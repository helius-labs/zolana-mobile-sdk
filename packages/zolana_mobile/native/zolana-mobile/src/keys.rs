//! Proving keys, fetched on first use or ahead of it, and pinned by the
//! Zolana proving-key lockfile.
//!
//! `proving-keys.lock` is the upstream lockfile at the pinned Zolana revision,
//! copied verbatim (`scripts/check-upstream.sh` checks it). Its version prefix
//! and per-file `size` + `sha256` are the only trust anchor: a key is used only
//! after the bytes on disk hash to the pinned value, and the gnark loader
//! reads a container before it can cross-check its sections.
//!
//! The keys of one lockfile live in a directory of their own under the
//! configured one, named after its prefix; opening a wallet removes the
//! directories of other lockfiles. A key downloads in ranged parts appended to
//! `<key>.partial`, so an interrupted download resumes where it stopped, and
//! is moved into place once it hashes to the lockfile. `<key>.verified` then
//! records its size and modification time, so a later process uses it without
//! hashing it again.

use std::{
    collections::{BTreeSet, HashMap, HashSet},
    fs::{self, File, OpenOptions},
    io::{self, Read, Write},
    path::{Path, PathBuf},
    sync::{Arc, Mutex, OnceLock, PoisonError, RwLock},
    time::UNIX_EPOCH,
};

use flutter_rust_bridge::DartFnFuture;
use serde::Deserialize;
use sha2::{Digest, Sha256};
use zolana_client::prover::ExpectedProvingKey;
use zolana_transaction::instructions::{merge::merge_circuit_width, transact::canonical_shape};

use crate::error::WalletError;
use crate::transport::{Transport, TransportRequest};

/// The upstream prover's default key host.
pub const DEFAULT_PROVING_KEYS_URL: &str = "https://d3gbdb0egjwcw9.cloudfront.net";

const LOCKFILE: &str = include_str!("../proving-keys.lock");

/// Bytes per ranged request of a download: what a resumed download repeats
/// at most, and how often progress is reported and a cancel is seen.
const PART: u64 = 8 << 20;

/// Marks a directory as a key set this store manages, so pruning removes
/// nothing else under the configured directory.
const SET_MARKER: &str = ".zolana-proving-keys";

#[derive(Deserialize)]
struct Lockfile {
    prefix: String,
    keys: HashMap<String, LockEntry>,
}

#[derive(Deserialize)]
struct LockEntry {
    sha256: String,
    size: u64,
}

fn lockfile() -> &'static Lockfile {
    static LOCK: OnceLock<Lockfile> = OnceLock::new();
    LOCK.get_or_init(|| serde_json::from_str(LOCKFILE).expect("embedded lockfile is valid JSON"))
}

/// The directory name of this lockfile's keys: the last part of its prefix.
fn key_set() -> &'static str {
    let prefix = lockfile().prefix.trim_end_matches('/');
    prefix.rsplit('/').next().unwrap_or(prefix)
}

/// A key this wallet proves with on the device: a confidential transfer or a
/// merge. The lockfile also pins ring and custom-ring keys it never loads.
fn is_wallet_key(name: &str) -> bool {
    name.starts_with("transfer_confidential_")
        || (name.starts_with("merge_") && name.ends_with("_1.key") && !name.contains("ring"))
}

/// A proving key of the wallet and how much of it is on the device.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProvingKeyStatus {
    pub name: String,
    /// The key's size in the lockfile.
    pub size: u64,
    /// Bytes on the device: `size` once it downloaded, what an interrupted
    /// download left otherwise.
    pub downloaded: u64,
}

/// Where a [`ProvingKeys::prefetch`] is.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProvingKeyProgress {
    /// The key downloading now.
    pub name: String,
    pub key_downloaded: u64,
    pub key_size: u64,
    /// Keys complete so far, of `keys_total` that were not on the device
    /// when the prefetch started.
    pub keys_done: u32,
    pub keys_total: u32,
    /// Bytes on the device of those keys, of `total`.
    pub downloaded: u64,
    pub total: u64,
}

/// The wallet's proving keys: which are on the device, a download ahead of
/// the first proof that needs them, and their removal. It works beside the
/// wallet's other calls: a proof that needs a key a prefetch is downloading
/// waits for that download instead of starting its own.
pub struct ProvingKeys {
    pub(crate) store: Arc<KeyStore>,
}

impl ProvingKeys {
    /// Every key the wallet can prove with on the device, smallest first.
    pub fn status(&self) -> Vec<ProvingKeyStatus> {
        self.store.status()
    }

    /// The keys spends of up to `max_inputs` notes with `outputs` outputs,
    /// and merges of up to `max_merge_inputs` notes, prove with, smallest
    /// first. A transfer has two outputs, the payment and the change; a
    /// withdrawal pads its change to two.
    pub fn needed(
        &self,
        max_inputs: u32,
        outputs: Vec<u32>,
        max_merge_inputs: u32,
    ) -> Result<Vec<String>, WalletError> {
        let mut names = BTreeSet::new();
        for n_in in 1..=max_inputs as usize {
            for &n_out in &outputs {
                let shape = canonical_shape(n_in, n_out as usize).map_err(|_| {
                    WalletError::ProvingKeyUnknown {
                        name: format!("transfer_confidential_{n_in}_{n_out}.key"),
                    }
                })?;
                names.insert(format!(
                    "transfer_confidential_{}_{}.key",
                    shape.n_inputs(),
                    shape.n_outputs()
                ));
            }
        }
        for count in 2..=max_merge_inputs as usize {
            let width = merge_circuit_width(count).ok_or(WalletError::ProvingKeyUnknown {
                name: format!("merge_{count}_1.key"),
            })?;
            names.insert(format!("merge_{width}_1.key"));
        }
        let mut names: Vec<String> = names.into_iter().collect();
        for name in &names {
            if !lockfile().keys.contains_key(name) {
                return Err(WalletError::ProvingKeyUnknown { name: name.clone() });
            }
        }
        names.sort_by_key(|name| lockfile().keys[name].size);
        Ok(names)
    }

    /// Download `names` (see [`Self::needed`]) that are not on the device,
    /// each checked against the lockfile. `progress` hears of every part, and
    /// stops the prefetch by answering `false`; what it downloaded stays, and
    /// the next download of that key resumes from it. Returns whether every
    /// key is on the device.
    pub fn prefetch(
        &self,
        names: Vec<String>,
        progress: impl Fn(ProvingKeyProgress) -> DartFnFuture<bool> + Send + Sync + 'static,
    ) -> Result<bool, WalletError> {
        let keys = names
            .iter()
            .map(|name| Ok((name.as_str(), entry(name)?)))
            .collect::<Result<Vec<_>, WalletError>>()?;
        self.store.prefetch(&keys, |update| {
            futures_executor::block_on(progress(update.clone()))
        })
    }

    /// Remove the keys and partial downloads from the device. A prefetch or
    /// proof that is downloading a key finishes its part first.
    pub fn clear(&self) -> Result<(), WalletError> {
        self.store.clear()
    }
}

/// Downloads and verifies proving keys into one directory.
pub struct KeyStore {
    /// This lockfile's key set under the configured directory.
    dir: PathBuf,
    base_url: String,
    transport: Transport,
    /// [`PART`] outside tests.
    part: u64,
    shared: Arc<Shared>,
}

/// What the stores of one directory share in this process, however many
/// wallets are open on it.
#[derive(Default)]
struct Shared {
    /// One lock per key, held while it is checked or downloaded: a second
    /// caller waits and then finds it on disk.
    downloads: Mutex<HashMap<String, Arc<Mutex<()>>>>,
    /// Keys checked against the lockfile this process.
    verified: Mutex<HashSet<String>>,
    /// Read while a key is checked or downloaded, written to clear the set.
    files: RwLock<()>,
}

fn shared(dir: &Path) -> Arc<Shared> {
    static STORES: OnceLock<Mutex<HashMap<PathBuf, Arc<Shared>>>> = OnceLock::new();
    let mut stores = STORES
        .get_or_init(Mutex::default)
        .lock()
        .unwrap_or_else(PoisonError::into_inner);
    Arc::clone(stores.entry(dir.to_path_buf()).or_default())
}

impl KeyStore {
    /// The keys of this lockfile under `root`. Removes the key sets of other
    /// lockfiles there.
    pub fn new(root: impl Into<PathBuf>, base_url: Option<String>, transport: Transport) -> Self {
        let root = root.into();
        prune(&root);
        let dir = root.join(key_set());
        let base_url = base_url.unwrap_or_else(|| DEFAULT_PROVING_KEYS_URL.to_string());
        Self {
            shared: shared(&dir),
            dir,
            base_url: base_url.trim_end_matches('/').to_string(),
            transport,
            part: PART,
        }
    }

    /// Path of `key` in the store, downloading it first if it is missing or
    /// does not match the lockfile. The lockfile must pin the sha256 that the
    /// client expects, the one next to the on-chain verifying key.
    pub fn ensure(&self, key: &ExpectedProvingKey) -> Result<PathBuf, WalletError> {
        let name = key.name.as_str();
        let entry = entry(name)?;
        if entry.sha256 != hex(&key.sha256) {
            return Err(WalletError::ProvingKeyMismatch { name: name.into() });
        }
        self.fetch(name, entry, &mut |_| true)?
            .ok_or_else(|| WalletError::ProvingKeyDownloadFailed { name: name.into() })
    }

    fn status(&self) -> Vec<ProvingKeyStatus> {
        let mut keys: Vec<ProvingKeyStatus> = lockfile()
            .keys
            .iter()
            .filter(|(name, _)| is_wallet_key(name))
            .map(|(name, entry)| ProvingKeyStatus {
                name: name.clone(),
                size: entry.size,
                downloaded: self.on_disk(name, entry),
            })
            .collect();
        keys.sort_by(|a, b| a.size.cmp(&b.size).then_with(|| a.name.cmp(&b.name)));
        keys
    }

    /// Bytes of key `name` on disk, without hashing it.
    fn on_disk(&self, name: &str, entry: &LockEntry) -> u64 {
        let path = self.dir.join(name);
        if self.is_verified(name) || Verified::matches(&path, entry) {
            return entry.size;
        }
        let len = |path: &Path| fs::metadata(path).map_or(0, |m| m.len());
        match len(&path) {
            0 => len(&partial(&path)).min(entry.size),
            whole => whole.min(entry.size),
        }
    }

    fn prefetch(
        &self,
        keys: &[(&str, &LockEntry)],
        mut progress: impl FnMut(&ProvingKeyProgress) -> bool,
    ) -> Result<bool, WalletError> {
        let mut pending = Vec::new();
        for &(name, entry) in keys {
            let on_disk = self.on_disk(name, entry);
            let done = self.is_verified(name) || Verified::matches(&self.dir.join(name), entry);
            if !done {
                pending.push((name, entry, on_disk));
            }
        }
        let mut update = ProvingKeyProgress {
            name: String::new(),
            key_downloaded: 0,
            key_size: 0,
            keys_done: 0,
            keys_total: pending.len() as u32,
            downloaded: pending.iter().map(|(_, _, on_disk)| on_disk).sum(),
            total: pending.iter().map(|(_, entry, _)| entry.size).sum(),
        };
        for (name, entry, on_disk) in pending {
            let before = update.downloaded - on_disk;
            update.name = name.to_string();
            update.key_size = entry.size;
            let fetched = self.fetch(name, entry, &mut |key_downloaded| {
                update.key_downloaded = key_downloaded;
                update.downloaded = before + key_downloaded;
                progress(&update)
            })?;
            if fetched.is_none() {
                return Ok(false);
            }
            update.keys_done += 1;
            update.key_downloaded = entry.size;
            update.downloaded = before + entry.size;
            if !progress(&update) {
                return Ok(update.keys_done == update.keys_total);
            }
        }
        Ok(true)
    }

    fn clear(&self) -> Result<(), WalletError> {
        let _files = self
            .shared
            .files
            .write()
            .unwrap_or_else(PoisonError::into_inner);
        self.shared
            .verified
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clear();
        match fs::remove_dir_all(&self.dir) {
            Err(error) if error.kind() != io::ErrorKind::NotFound => Err(store_failed(&self.dir)),
            _ => Ok(()),
        }
    }

    fn is_verified(&self, name: &str) -> bool {
        self.shared
            .verified
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .contains(name)
    }

    /// Key `name` on disk and checked, downloading what is missing. `None`
    /// when `progress` stopped the download.
    fn fetch(
        &self,
        name: &str,
        entry: &LockEntry,
        progress: &mut dyn FnMut(u64) -> bool,
    ) -> Result<Option<PathBuf>, WalletError> {
        let _files = self
            .shared
            .files
            .read()
            .unwrap_or_else(PoisonError::into_inner);
        let lock = Arc::clone(
            self.shared
                .downloads
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .entry(name.to_string())
                .or_default(),
        );
        let _key = lock.lock().unwrap_or_else(PoisonError::into_inner);
        let path = self.dir.join(name);
        if self.is_verified(name) || Verified::matches(&path, entry) {
            return Ok(self.verified(name, path));
        }
        if !matches_entry(name, &path, entry)? {
            let _ = fs::remove_file(&path);
            self.create_dir()?;
            if !self.download(name, entry, &path, progress)? {
                return Ok(None);
            }
        }
        Verified::record(&path, entry);
        Ok(self.verified(name, path))
    }

    fn verified(&self, name: &str, path: PathBuf) -> Option<PathBuf> {
        self.shared
            .verified
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .insert(name.to_string());
        Some(path)
    }

    fn create_dir(&self) -> Result<(), WalletError> {
        fs::create_dir_all(&self.dir).map_err(|_| store_failed(&self.dir))?;
        let marker = self.dir.join(SET_MARKER);
        if !marker.exists() {
            File::create(&marker).map_err(|_| store_failed(&marker))?;
        }
        Ok(())
    }

    /// Download key `name` into `<path>.partial` in ranged parts, resuming
    /// from what is there, check it against `entry` as it lies on disk, and
    /// move it into place. The transport writes each part to a file of its
    /// own, so a key is never held in memory. `false` when `progress` stopped
    /// it; a failed part leaves what came before it for the next attempt.
    fn download(
        &self,
        name: &str,
        entry: &LockEntry,
        path: &Path,
        progress: &mut dyn FnMut(u64) -> bool,
    ) -> Result<bool, WalletError> {
        let partial = partial(path);
        let part = path.with_extension("key.part");
        let failed = || WalletError::ProvingKeyDownloadFailed { name: name.into() };
        let mut offset = fs::metadata(&partial).map_or(0, |m| m.len());
        if offset > entry.size {
            fs::remove_file(&partial).map_err(|_| store_failed(path))?;
            offset = 0;
        }
        while offset < entry.size {
            if !progress(offset) {
                return Ok(false);
            }
            let end = (offset + self.part).min(entry.size) - 1;
            let _ = fs::remove_file(&part);
            let response = self
                .transport
                .send_blocking(TransportRequest {
                    method: "GET".to_string(),
                    url: format!("{}/{}/{name}", self.base_url, lockfile().prefix),
                    headers: HashMap::from([(
                        "range".to_string(),
                        format!("bytes={offset}-{end}"),
                    )]),
                    body: Vec::new(),
                    // A host without ranges answers the first part with the
                    // whole key.
                    max_response_bytes: u32::try_from(if offset == 0 {
                        entry.size
                    } else {
                        end + 1 - offset
                    })
                    .ok(),
                    timeout_ms: None,
                    download_path: Some(part.display().to_string()),
                })
                .map_err(|_| failed());
            let appended = response.and_then(|response| {
                // A transport that returned the body instead of writing the file.
                if !response.body.is_empty() || !part.exists() {
                    fs::write(&part, &response.body).map_err(|_| store_failed(path))?;
                }
                match response.status {
                    206 if content_range(&response.headers) == Some((offset, end, entry.size)) => {
                        append(&part, &partial, end + 1 - offset).map_err(|_| failed())
                    }
                    200 if offset == 0 => {
                        fs::rename(&part, &partial).map_err(|_| store_failed(path))
                    }
                    _ => Err(failed()),
                }
            });
            let _ = fs::remove_file(&part);
            appended?;
            offset = fs::metadata(&partial).map_or(0, |m| m.len());
        }
        progress(entry.size);
        let mut file = File::open(&partial).map_err(|_| store_failed(path))?;
        let digest = copy_bounded(&mut file, &mut io::sink(), entry.size, name, path);
        if digest.as_deref().ok() != Some(entry.sha256.as_str()) {
            let _ = fs::remove_file(&partial);
            return Err(digest
                .err()
                .unwrap_or(WalletError::ProvingKeyCorrupt { name: name.into() }));
        }
        file.sync_all().map_err(|_| store_failed(path))?;
        fs::rename(&partial, path).map_err(|_| store_failed(path))?;
        Ok(true)
    }
}

fn entry(name: &str) -> Result<&'static LockEntry, WalletError> {
    lockfile()
        .keys
        .get(name)
        .ok_or_else(|| WalletError::ProvingKeyUnknown { name: name.into() })
}

fn partial(path: &Path) -> PathBuf {
    path.with_extension("key.partial")
}

/// Remove the key sets of other lockfiles under `root`. Best effort: a set
/// that cannot be removed now is removed by a later open.
fn prune(root: &Path) {
    let Ok(entries) = fs::read_dir(root) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if entry.file_name() != key_set() && path.join(SET_MARKER).is_file() {
            let _ = fs::remove_dir_all(&path);
        }
    }
}

/// The `start`, `end` and `size` of a `Content-Range: bytes start-end/size`.
fn content_range(headers: &HashMap<String, String>) -> Option<(u64, u64, u64)> {
    let value = headers
        .iter()
        .find(|(name, _)| name.eq_ignore_ascii_case("content-range"))?
        .1;
    let (range, size) = value.trim().strip_prefix("bytes ")?.split_once('/')?;
    let (start, end) = range.split_once('-')?;
    Some((start.parse().ok()?, end.parse().ok()?, size.parse().ok()?))
}

/// Append the part at `part`, exactly `len` bytes, to `partial`.
fn append(part: &Path, partial: &Path, len: u64) -> io::Result<()> {
    let mut source = File::open(part)?;
    if source.metadata()?.len() != len {
        return Err(io::ErrorKind::UnexpectedEof.into());
    }
    let mut target = OpenOptions::new().create(true).append(true).open(partial)?;
    io::copy(&mut source, &mut target)?;
    target.flush()
}

/// What `<key>.verified` records of a key that hashed to the lockfile: its
/// digest, size and modification time. A key whose size or time changed is
/// hashed again.
struct Verified;

impl Verified {
    fn path(key: &Path) -> PathBuf {
        key.with_extension("key.verified")
    }

    fn stamp(key: &Path, entry: &LockEntry) -> Option<String> {
        let metadata = fs::metadata(key).ok()?;
        let modified = metadata.modified().ok()?.duration_since(UNIX_EPOCH).ok()?;
        (metadata.len() == entry.size).then(|| {
            format!(
                "{} {} {}",
                entry.sha256,
                metadata.len(),
                modified.as_nanos()
            )
        })
    }

    fn matches(key: &Path, entry: &LockEntry) -> bool {
        let recorded = fs::read_to_string(Self::path(key)).ok();
        recorded.is_some() && recorded == Self::stamp(key, entry)
    }

    /// Best effort: without the record, the key is hashed again next time.
    fn record(key: &Path, entry: &LockEntry) {
        if let Some(stamp) = Self::stamp(key, entry) {
            let _ = fs::write(Self::path(key), stamp);
        }
    }
}

/// Whether the file of key `name` at `path` exists and hashes to `entry`.
fn matches_entry(name: &str, path: &Path, entry: &LockEntry) -> Result<bool, WalletError> {
    let Ok(mut file) = File::open(path) else {
        return Ok(false);
    };
    if file.metadata().map(|m| m.len()).ok() != Some(entry.size) {
        return Ok(false);
    }
    Ok(copy_bounded(&mut file, &mut io::sink(), entry.size, name, path)? == entry.sha256)
}

/// Copy exactly `size` bytes of key `name`, failing on a short or long
/// source, and return their lowercase hex SHA-256.
fn copy_bounded(
    source: &mut impl Read,
    sink: &mut impl Write,
    size: u64,
    name: &str,
    path: &Path,
) -> Result<String, WalletError> {
    let corrupt = || WalletError::ProvingKeyCorrupt { name: name.into() };
    let mut hasher = Sha256::new();
    let mut buffer = vec![0u8; 1 << 20];
    let mut copied = 0u64;
    loop {
        let read = source.read(&mut buffer).map_err(|_| store_failed(path))?;
        if read == 0 {
            break;
        }
        copied += read as u64;
        if copied > size {
            return Err(corrupt());
        }
        hasher.update(&buffer[..read]);
        sink.write_all(&buffer[..read])
            .map_err(|_| store_failed(path))?;
    }
    if copied != size {
        return Err(corrupt());
    }
    Ok(hex(&hasher.finalize()))
}

fn store_failed(path: &Path) -> WalletError {
    WalletError::ProvingKeyStoreFailed {
        path: path.display().to_string(),
    }
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

#[cfg(test)]
mod tests {
    use std::{
        sync::atomic::{AtomicUsize, Ordering},
        thread,
        time::Duration,
    };

    use super::*;
    use crate::transport::tests::{fake, refused, unreachable, Requests};

    fn needed(
        max_inputs: u32,
        outputs: Vec<u32>,
        max_merge_inputs: u32,
    ) -> Result<Vec<String>, WalletError> {
        ProvingKeys {
            store: Arc::new(KeyStore::new(scratch("needed"), None, unreachable())),
        }
        .needed(max_inputs, outputs, max_merge_inputs)
    }
    use crate::{TransportFailure, TransportResponse};

    /// A fresh directory for one test.
    fn scratch(test: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("zolana-keys-{test}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    /// 100 bytes of key and their lockfile entry.
    fn key() -> (Vec<u8>, LockEntry) {
        let data: Vec<u8> = (0..100).collect();
        let entry = LockEntry {
            sha256: hex(&Sha256::digest(&data)),
            size: data.len() as u64,
        };
        (data, entry)
    }

    fn store(root: &Path, transport: Transport) -> KeyStore {
        KeyStore {
            part: 30,
            ..KeyStore::new(root, None, transport)
        }
    }

    /// A key host that answers a `range` with that part of `data`, written
    /// to the request's file as a streaming transport does. `answer` can
    /// replace the answer to the request of that index.
    fn host(
        data: Vec<u8>,
        answer: impl Fn(usize) -> Option<Result<TransportResponse, TransportFailure>>
            + Send
            + Sync
            + 'static,
    ) -> (Transport, Requests) {
        let count = AtomicUsize::new(0);
        fake(move |request| {
            if let Some(answer) = answer(count.fetch_add(1, Ordering::SeqCst)) {
                return answer;
            }
            let (start, end) = request.headers["range"]
                .strip_prefix("bytes=")
                .and_then(|range| range.split_once('-'))
                .map(|(start, end)| {
                    (
                        start.parse::<usize>().unwrap(),
                        end.parse::<usize>().unwrap(),
                    )
                })
                .unwrap();
            fs::write(request.download_path.as_ref().unwrap(), &data[start..=end]).unwrap();
            Ok(TransportResponse {
                status: 206,
                headers: HashMap::from([(
                    "content-range".to_string(),
                    format!("bytes {start}-{end}/{}", data.len()),
                )]),
                body: Vec::new(),
            })
        })
    }

    fn ranges(requests: &Requests) -> Vec<String> {
        requests
            .lock()
            .unwrap()
            .iter()
            .map(|request| request.headers["range"].clone())
            .collect()
    }

    #[test]
    fn every_transfer_shape_has_a_pinned_key() {
        for shape in zolana_client::SPP_SUPPORTED_SHAPES {
            let name = format!(
                "transfer_confidential_{}_{}.key",
                shape.n_inputs(),
                shape.n_outputs()
            );
            assert!(lockfile().keys.contains_key(&name), "{name} is not pinned");
        }
    }

    #[test]
    fn pinned_keys_match_the_verifying_keys() {
        for (name, sha256) in zolana_interface::verifying_keys::PROVING_KEY_SHA256S {
            if let Some(entry) = lockfile().keys.get(*name) {
                assert_eq!(entry.sha256, hex(sha256), "{name}");
            }
        }
    }

    #[test]
    fn a_key_the_verifying_key_does_not_pin_is_refused() {
        let store = KeyStore::new(scratch("refused"), None, unreachable());
        let unknown = ExpectedProvingKey {
            name: "transfer_confidential_9_9.key".into(),
            sha256: [0; 32],
        };
        assert_eq!(
            store.ensure(&unknown).unwrap_err(),
            WalletError::ProvingKeyUnknown {
                name: "transfer_confidential_9_9.key".into()
            }
        );
        let foreign = ExpectedProvingKey {
            name: "transfer_confidential_2_2.key".into(),
            sha256: [0; 32],
        };
        assert_eq!(
            store.ensure(&foreign).unwrap_err(),
            WalletError::ProvingKeyMismatch {
                name: "transfer_confidential_2_2.key".into()
            }
        );
    }

    #[test]
    fn the_keys_of_spends_and_merges_are_the_shapes_the_sdk_picks() {
        assert_eq!(
            needed(4, vec![2], 0).unwrap(),
            [
                "transfer_confidential_1_2.key",
                "transfer_confidential_2_2.key",
                "transfer_confidential_3_2.key",
                "transfer_confidential_4_2.key",
            ]
        );
        // Seven notes prove with the eight-input shape, three outputs with four.
        let wide = needed(7, vec![3], 25).unwrap();
        assert!(wide.contains(&"transfer_confidential_8_4.key".to_string()));
        assert!(!wide.contains(&"transfer_confidential_7_4.key".to_string()));
        assert_eq!(
            wide.iter()
                .filter(|name| name.starts_with("merge_"))
                .collect::<Vec<_>>(),
            ["merge_8_1.key", "merge_24_1.key", "merge_54_1.key"]
        );
        assert_eq!(
            needed(1, vec![17], 0).unwrap_err(),
            WalletError::ProvingKeyUnknown {
                name: "transfer_confidential_1_17.key".into()
            }
        );
        assert!(needed(0, vec![2], 0).unwrap().is_empty());
    }

    #[test]
    fn a_key_downloads_in_parts_and_resumes_where_it_stopped() {
        let root = scratch("resume");
        let (data, entry) = key();
        let path = root.join(key_set()).join("k.key");
        let failed = WalletError::ProvingKeyDownloadFailed {
            name: "k.key".into(),
        };

        // The third part stalls: the first two stay on disk.
        let (transport, requests) = host(data.clone(), |index| {
            (index == 2).then(|| refused("stalled".into()))
        });
        let store = store(&root, transport);
        assert_eq!(
            store.fetch("k.key", &entry, &mut |_| true).unwrap_err(),
            failed
        );
        assert_eq!(
            ranges(&requests),
            ["bytes=0-29", "bytes=30-59", "bytes=60-89"]
        );
        assert_eq!(fs::read(partial(&path)).unwrap(), &data[..60]);

        let (transport, requests) = host(data.clone(), |_| None);
        let mut seen = Vec::new();
        let fetched = super::KeyStore { transport, ..store }
            .fetch("k.key", &entry, &mut |downloaded| {
                seen.push(downloaded);
                true
            })
            .unwrap();
        assert_eq!(fetched, Some(path.clone()));
        assert_eq!(ranges(&requests), ["bytes=60-89", "bytes=90-99"]);
        assert_eq!(seen, [60, 90, 100]);
        assert_eq!(fs::read(&path).unwrap(), data);
        assert!(!partial(&path).exists());
        assert!(Verified::matches(&path, &entry));
        assert!(root.join(key_set()).join(SET_MARKER).is_file());
        fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn a_host_without_ranges_sends_the_whole_key_at_once() {
        let root = scratch("whole");
        let (data, entry) = key();
        let body = data.clone();
        let (transport, requests) = fake(move |_| {
            Ok(TransportResponse {
                status: 200,
                headers: HashMap::new(),
                body: body.clone(),
            })
        });
        let store = store(&root, transport);
        let path = store
            .fetch("k.key", &entry, &mut |_| true)
            .unwrap()
            .unwrap();
        assert_eq!(fs::read(path).unwrap(), data);
        let request = requests.lock().unwrap().pop().unwrap();
        assert_eq!(request.method, "GET");
        assert_eq!(
            request.url,
            format!("{DEFAULT_PROVING_KEYS_URL}/{}/k.key", lockfile().prefix)
        );
        assert_eq!(request.max_response_bytes, Some(100));
        assert!(request.body.is_empty());
        fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn a_stopped_download_keeps_its_parts() {
        let root = scratch("stopped");
        let (data, entry) = key();
        let (transport, requests) = host(data.clone(), |_| None);
        let store = store(&root, transport);
        assert_eq!(
            store
                .fetch("k.key", &entry, &mut |downloaded| downloaded < 30)
                .unwrap(),
            None
        );
        assert_eq!(requests.lock().unwrap().len(), 1);
        let path = root.join(key_set()).join("k.key");
        assert_eq!(fs::read(partial(&path)).unwrap(), &data[..30]);
        assert_eq!(store.on_disk("k.key", &entry), 30);
        fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn a_wrong_key_or_part_is_refused() {
        let root = scratch("refused-parts");
        let (data, entry) = key();
        let path = root.join(key_set()).join("k.key");

        let mut tampered = data.clone();
        tampered[50] ^= 1;
        let corrupt = store(&root, host(tampered, |_| None).0);
        assert_eq!(
            corrupt.fetch("k.key", &entry, &mut |_| true).unwrap_err(),
            WalletError::ProvingKeyCorrupt {
                name: "k.key".into()
            }
        );
        assert!(!path.exists() && !partial(&path).exists());

        // A part of another range, or of another file, is not appended.
        for content_range in ["bytes 30-59/100", "bytes 0-29/101"] {
            let (transport, _) = host(data.clone(), move |_| {
                Some(Ok(TransportResponse {
                    status: 206,
                    headers: HashMap::from([(
                        "Content-Range".to_string(),
                        content_range.to_string(),
                    )]),
                    body: vec![0; 30],
                }))
            });
            assert_eq!(
                KeyStore {
                    transport,
                    ..store(&root, unreachable())
                }
                .fetch("k.key", &entry, &mut |_| true)
                .unwrap_err(),
                WalletError::ProvingKeyDownloadFailed {
                    name: "k.key".into()
                }
            );
            assert!(!path.exists());
            assert_eq!(fs::metadata(partial(&path)).map_or(0, |m| m.len()), 0);
        }
        fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn a_verified_key_is_hashed_again_only_after_it_changed() {
        let root = scratch("verified");
        let (data, entry) = key();
        let path = root.join("k.key");
        fs::write(&path, &data).unwrap();
        assert!(!Verified::matches(&path, &entry));
        Verified::record(&path, &entry);
        assert!(Verified::matches(&path, &entry));
        thread::sleep(Duration::from_millis(20));
        fs::write(&path, &data).unwrap();
        assert!(!Verified::matches(&path, &entry), "rewritten");
        Verified::record(&path, &entry);
        fs::write(&path, &data[..99]).unwrap();
        assert!(!Verified::matches(&path, &entry), "truncated");
        fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn two_callers_share_one_download() {
        let root = scratch("single-flight");
        let (data, entry) = key();
        let (transport, requests) = host(data, |_| {
            thread::sleep(Duration::from_millis(20));
            None
        });
        let store = Arc::new(store(&root, transport));
        let entry = Arc::new(entry);
        let callers: Vec<_> = (0..2)
            .map(|_| {
                let (store, entry) = (Arc::clone(&store), Arc::clone(&entry));
                thread::spawn(move || store.fetch("k.key", &entry, &mut |_| true).unwrap())
            })
            .collect();
        for caller in callers {
            assert!(caller.join().unwrap().is_some());
        }
        assert_eq!(requests.lock().unwrap().len(), 4);
        fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn a_prefetch_reports_each_part_and_skips_keys_on_disk() {
        let root = scratch("prefetch");
        let (data, entry) = key();
        let (transport, requests) = host(data, |_| None);
        let store = store(&root, transport);
        store.fetch("k.key", &entry, &mut |_| true).unwrap();
        requests.lock().unwrap().clear();
        let mut updates = Vec::new();
        let done = store
            .prefetch(&[("k.key", &entry), ("l.key", &entry)], |update| {
                updates.push((update.name.clone(), update.keys_done, update.downloaded));
                true
            })
            .unwrap();
        assert!(done);
        assert_eq!(
            updates,
            [
                ("l.key".to_string(), 0, 0),
                ("l.key".to_string(), 0, 30),
                ("l.key".to_string(), 0, 60),
                ("l.key".to_string(), 0, 90),
                ("l.key".to_string(), 0, 100),
                ("l.key".to_string(), 1, 100),
            ]
        );
        assert_eq!(requests.lock().unwrap().len(), 4);

        let stopped = store
            .prefetch(&[("m.key", &entry)], |update| update.downloaded == 0)
            .unwrap();
        assert!(!stopped);
        fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn other_key_sets_are_pruned_and_clear_removes_this_one() {
        let root = scratch("prune");
        let old = root.join("0123456789abcdef");
        fs::create_dir_all(&old).unwrap();
        fs::write(old.join(SET_MARKER), b"").unwrap();
        fs::write(old.join("transfer_confidential_1_2.key"), b"old").unwrap();
        let app = root.join("app-data");
        fs::create_dir_all(&app).unwrap();

        let (data, entry) = key();
        let store = store(&root, host(data, |_| None).0);
        assert!(!old.exists(), "another lockfile's keys are removed");
        assert!(app.exists(), "a directory without the marker stays");

        let path = store
            .fetch("k.key", &entry, &mut |_| true)
            .unwrap()
            .unwrap();
        assert!(path.exists());
        store.clear().unwrap();
        assert!(!root.join(key_set()).exists());
        assert!(!store.is_verified("k.key"));
        store.clear().unwrap();
        fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn status_lists_the_wallet_keys_and_what_is_on_disk() {
        let root = scratch("status");
        let store = KeyStore::new(&root, None, unreachable());
        let status = store.status();
        assert!(status.iter().all(|key| is_wallet_key(&key.name)));
        assert!(status.iter().any(|key| key.name == "merge_54_1.key"));
        assert!(!status.iter().any(|key| key.name.contains("ring")));
        assert!(status.windows(2).all(|pair| pair[0].size <= pair[1].size));
        let dir = root.join(key_set());
        fs::create_dir_all(&dir).unwrap();
        fs::write(partial(&dir.join("transfer_confidential_1_2.key")), [0; 7]).unwrap();
        let first = &store.status()[0];
        assert_eq!(
            (first.name.as_str(), first.downloaded),
            ("transfer_confidential_1_2.key", 7)
        );
        fs::remove_dir_all(&root).unwrap();
    }
}
