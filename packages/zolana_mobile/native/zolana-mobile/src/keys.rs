//! Proving keys, fetched on first use and pinned by the Zolana proving-key
//! lockfile.
//!
//! `proving-keys.lock` is the upstream lockfile at the pinned Zolana revision,
//! copied verbatim (`scripts/check-upstream.sh` checks it). Its version prefix
//! and per-file `size` + `sha256` are the only trust anchor: a key is used only
//! after the bytes on disk hash to the pinned value, and the gnark loader
//! reads a container before it can cross-check its sections.

use std::{
    collections::{HashMap, HashSet},
    fs::{self, File},
    io::{Read, Write},
    path::{Path, PathBuf},
    sync::{Mutex, OnceLock},
};

use serde::Deserialize;
use sha2::{Digest, Sha256};
use zolana_client::prover::ExpectedProvingKey;

use crate::transport::{Transport, TransportRequest};

/// The upstream prover's default key host.
pub const DEFAULT_PROVING_KEYS_URL: &str = "https://d3gbdb0egjwcw9.cloudfront.net";

const LOCKFILE: &str = include_str!("../proving-keys.lock");

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

/// Downloads and verifies proving keys into one directory.
pub struct KeyStore {
    dir: PathBuf,
    base_url: String,
    transport: Transport,
    /// Keys whose digest was checked this process. Re-hashing a 240 MB merge
    /// key before every proof would cost more than the proof.
    verified: Mutex<HashSet<String>>,
}

impl KeyStore {
    pub fn new(dir: impl Into<PathBuf>, base_url: Option<String>, transport: Transport) -> Self {
        let base_url = base_url.unwrap_or_else(|| DEFAULT_PROVING_KEYS_URL.to_string());
        Self {
            dir: dir.into(),
            base_url: base_url.trim_end_matches('/').to_string(),
            transport,
            verified: Mutex::new(HashSet::new()),
        }
    }

    /// Path of `key` in the store, downloading it first if it is missing or
    /// does not match the lockfile. The lockfile must pin the sha256 that the
    /// client expects, the one next to the on-chain verifying key.
    pub fn ensure(&self, key: &ExpectedProvingKey) -> Result<PathBuf, String> {
        let name = key.name.as_str();
        let entry = lockfile()
            .keys
            .get(name)
            .ok_or_else(|| "proving_key_unknown".to_string())?;
        if entry.sha256 != hex(&key.sha256) {
            return Err("proving_key_mismatch".to_string());
        }
        let path = self.dir.join(name);
        if self
            .verified
            .lock()
            .map_err(|_| "key_store_poisoned")?
            .contains(name)
        {
            return Ok(path);
        }
        if !matches_entry(&path, entry)? {
            fs::create_dir_all(&self.dir).map_err(|_| "proving_key_dir_unwritable")?;
            self.download(name, entry, &path)?;
        }
        self.verified
            .lock()
            .map_err(|_| "key_store_poisoned")?
            .insert(name.to_string());
        Ok(path)
    }

    fn download(&self, name: &str, entry: &LockEntry, path: &Path) -> Result<(), String> {
        let url = format!("{}/{}/{name}", self.base_url, lockfile().prefix);
        let body = self.fetch(url, entry.size)?;
        let partial = path.with_extension("key.partial");
        let result = (|| {
            let mut file = File::create(&partial).map_err(|_| "proving_key_dir_unwritable")?;
            let digest = copy_bounded(&mut body.as_slice(), &mut file, entry.size)?;
            file.sync_all().map_err(|_| "proving_key_dir_unwritable")?;
            if digest != entry.sha256 {
                return Err("proving_key_checksum_mismatch".to_string());
            }
            fs::rename(&partial, path).map_err(|_| "proving_key_dir_unwritable".to_string())
        })();
        if result.is_err() {
            let _ = fs::remove_file(&partial);
        }
        result
    }

    /// The body of `url`, whole: it is checked before it is written. The
    /// transport is asked to stop past `size` bytes.
    fn fetch(&self, url: String, size: u64) -> Result<Vec<u8>, String> {
        let response = self
            .transport
            .send_blocking(TransportRequest {
                method: "GET".to_string(),
                url,
                headers: HashMap::new(),
                body: Vec::new(),
                max_response_bytes: u32::try_from(size).ok(),
            })
            .map_err(download_failed)?;
        if !(200..300).contains(&response.status) {
            return Err(download_failed(response.status));
        }
        Ok(response.body)
    }
}

fn download_failed<E>(_: E) -> String {
    "proving_key_download_failed".to_string()
}

/// Whether the file at `path` exists and hashes to `entry`.
fn matches_entry(path: &Path, entry: &LockEntry) -> Result<bool, String> {
    let Ok(mut file) = File::open(path) else {
        return Ok(false);
    };
    if file.metadata().map(|m| m.len()).ok() != Some(entry.size) {
        return Ok(false);
    }
    Ok(copy_bounded(&mut file, &mut std::io::sink(), entry.size)? == entry.sha256)
}

/// Copy exactly `size` bytes, failing on a short or long source, and return
/// their lowercase hex SHA-256.
fn copy_bounded(
    source: &mut impl Read,
    sink: &mut impl Write,
    size: u64,
) -> Result<String, String> {
    let mut hasher = Sha256::new();
    let mut buffer = vec![0u8; 1 << 20];
    let mut copied = 0u64;
    loop {
        let read = source
            .read(&mut buffer)
            .map_err(|_| "proving_key_read_failed".to_string())?;
        if read == 0 {
            break;
        }
        copied += read as u64;
        if copied > size {
            return Err("proving_key_size_mismatch".to_string());
        }
        hasher.update(&buffer[..read]);
        sink.write_all(&buffer[..read])
            .map_err(|_| "proving_key_dir_unwritable".to_string())?;
    }
    if copied != size {
        return Err("proving_key_size_mismatch".to_string());
    }
    Ok(hex(&hasher.finalize()))
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::transport::tests::{fake, ok, unreachable};

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
        let store = KeyStore::new(std::env::temp_dir(), None, unreachable());
        let unknown = ExpectedProvingKey {
            name: "transfer_confidential_9_9.key".into(),
            sha256: [0; 32],
        };
        assert_eq!(store.ensure(&unknown).unwrap_err(), "proving_key_unknown");
        let foreign = ExpectedProvingKey {
            name: "transfer_confidential_2_3.key".into(),
            sha256: [0; 32],
        };
        assert_eq!(store.ensure(&foreign).unwrap_err(), "proving_key_mismatch");
    }

    #[test]
    fn keys_are_used_only_when_they_match_the_lockfile() {
        let dir = std::env::temp_dir().join(format!("zolana-keys-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let entry = LockEntry {
            // sha256("abc")
            sha256: "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad".into(),
            size: 3,
        };
        let path = dir.join("k.key");
        fs::write(&path, b"abc").unwrap();
        assert!(matches_entry(&path, &entry).unwrap());
        fs::write(&path, b"abd").unwrap();
        assert!(!matches_entry(&path, &entry).unwrap());
        fs::write(&path, b"abcd").unwrap();
        assert!(!matches_entry(&path, &entry).unwrap());
        assert!(!matches_entry(&dir.join("missing.key"), &entry).unwrap());
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn downloads_through_the_transport() {
        let dir = std::env::temp_dir().join(format!("zolana-key-transport-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("k.key");
        let entry = LockEntry {
            // sha256("abc")
            sha256: "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad".into(),
            size: 3,
        };
        let store = |transport| KeyStore::new(&dir, None, transport);

        let (transport, requests) = fake(|_| ok("abc"));
        store(transport).download("k.key", &entry, &path).unwrap();
        assert_eq!(fs::read(&path).unwrap(), b"abc");
        let request = requests.lock().unwrap().pop().unwrap();
        assert_eq!(request.method, "GET");
        assert_eq!(
            request.url,
            format!("{DEFAULT_PROVING_KEYS_URL}/{}/k.key", lockfile().prefix)
        );
        assert!(request.headers.is_empty() && request.body.is_empty());
        assert_eq!(request.max_response_bytes, Some(3));

        fs::remove_file(&path).unwrap();
        let (missing, _) = fake(|_| {
            Ok(crate::TransportResponse {
                status: 404,
                body: b"abc".to_vec(),
            })
        });
        let (tampered, _) = fake(|_| ok("abd"));
        for (transport, expected) in [
            (missing, "proving_key_download_failed"),
            (tampered, "proving_key_checksum_mismatch"),
        ] {
            assert_eq!(
                store(transport)
                    .download("k.key", &entry, &path)
                    .unwrap_err(),
                expected
            );
            assert!(!path.exists());
        }
        fs::remove_dir_all(&dir).unwrap();
    }
}
