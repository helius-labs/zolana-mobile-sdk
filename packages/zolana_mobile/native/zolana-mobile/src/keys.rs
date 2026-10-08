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
    time::Duration,
};

use serde::Deserialize;
use sha2::{Digest, Sha256};
use zolana_client::prover::ExpectedProvingKey;

/// The upstream prover's default key host.
pub const DEFAULT_PROVING_KEYS_URL: &str = "https://d3gbdb0egjwcw9.cloudfront.net";

const LOCKFILE: &str = include_str!("../proving-keys.lock");

/// How long a key download may receive no data; it bounds a stall, not the download.
const KEY_DOWNLOAD_STALL_TIMEOUT: Duration = Duration::from_secs(30);

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
    /// Keys whose digest was checked this process. Re-hashing a 240 MB merge
    /// key before every proof would cost more than the proof.
    verified: Mutex<HashSet<String>>,
    stall_timeout: Duration,
}

impl KeyStore {
    pub fn new(dir: impl Into<PathBuf>, base_url: Option<String>) -> Result<Self, String> {
        let base_url = base_url.unwrap_or_else(|| DEFAULT_PROVING_KEYS_URL.to_string());
        if !is_allowed_url(&base_url) {
            return Err("proving_key_url_insecure".to_string());
        }
        Ok(Self {
            dir: dir.into(),
            base_url: base_url.trim_end_matches('/').to_string(),
            verified: Mutex::new(HashSet::new()),
            stall_timeout: KEY_DOWNLOAD_STALL_TIMEOUT,
        })
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
        let mut response = reqwest::blocking::Client::builder()
            .timeout(self.stall_timeout)
            .build()
            .and_then(|client| client.get(url).send())
            .and_then(reqwest::blocking::Response::error_for_status)
            .map_err(|_| "proving_key_download_failed".to_string())?;
        let partial = path.with_extension("key.partial");
        let result = (|| {
            let mut file = File::create(&partial).map_err(|_| "proving_key_dir_unwritable")?;
            let digest = copy_bounded(
                &mut response,
                &mut file,
                entry.size,
                "proving_key_download_failed",
            )?;
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
}

/// Whether the file at `path` exists and hashes to `entry`.
fn matches_entry(path: &Path, entry: &LockEntry) -> Result<bool, String> {
    let Ok(mut file) = File::open(path) else {
        return Ok(false);
    };
    if file.metadata().map(|m| m.len()).ok() != Some(entry.size) {
        return Ok(false);
    }
    let digest = copy_bounded(
        &mut file,
        &mut std::io::sink(),
        entry.size,
        "proving_key_read_failed",
    )?;
    Ok(digest == entry.sha256)
}

/// Copy exactly `size` bytes, failing on a short or long source and with
/// `read_failed` when the source fails, and return their lowercase hex SHA-256.
fn copy_bounded(
    source: &mut impl Read,
    sink: &mut impl Write,
    size: u64,
    read_failed: &str,
) -> Result<String, String> {
    let mut hasher = Sha256::new();
    let mut buffer = vec![0u8; 1 << 20];
    let mut copied = 0u64;
    loop {
        let read = source
            .read(&mut buffer)
            .map_err(|_| read_failed.to_string())?;
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

/// Keys are public setup parameters, but a tampered key yields proofs the
/// program rejects, so the transport still has to be authenticated.
fn is_allowed_url(url: &str) -> bool {
    let Ok(url) = reqwest::Url::parse(url) else {
        return false;
    };
    match url.scheme() {
        "https" => true,
        "http" => matches!(url.host_str(), Some("127.0.0.1" | "localhost")),
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use std::{
        io::{BufRead, BufReader},
        net::{TcpListener, TcpStream},
        thread,
        time::Instant,
    };

    use super::*;

    const TEST_STALL_TIMEOUT: Duration = Duration::from_secs(1);

    fn abc_entry() -> LockEntry {
        LockEntry {
            // sha256("abc")
            sha256: "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad".into(),
            size: 3,
        }
    }

    fn test_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("zolana-keys-{name}-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    /// A key host on 127.0.0.1 that answers one request with `respond`.
    fn key_host(respond: impl FnOnce(&mut TcpStream) + Send + 'static) -> String {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let url = format!("http://{}", listener.local_addr().unwrap());
        thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            stream.set_nodelay(true).unwrap();
            let mut request = BufReader::new(stream.try_clone().unwrap());
            let mut line = String::new();
            while request.read_line(&mut line).unwrap() > 2 {
                line.clear();
            }
            respond(&mut stream);
        });
        url
    }

    fn store_with_test_timeout(dir: &Path, url: String) -> KeyStore {
        let mut store = KeyStore::new(dir, Some(url)).unwrap();
        store.stall_timeout = TEST_STALL_TIMEOUT;
        store
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
        let store = KeyStore::new(std::env::temp_dir(), None).unwrap();
        let unknown = ExpectedProvingKey {
            name: "transfer_confidential_9_9.key".into(),
            sha256: [0; 32],
        };
        assert_eq!(store.ensure(&unknown).unwrap_err(), "proving_key_unknown");
        let foreign = ExpectedProvingKey {
            name: "transfer_confidential_2_2.key".into(),
            sha256: [0; 32],
        };
        assert_eq!(store.ensure(&foreign).unwrap_err(), "proving_key_mismatch");
    }

    #[test]
    fn keys_are_used_only_when_they_match_the_lockfile() {
        let dir = test_dir("match");
        let entry = abc_entry();
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
    fn a_stalled_download_fails_and_leaves_no_partial_key() {
        let dir = test_dir("stalled");
        let url = key_host(|stream| {
            stream
                .write_all(b"HTTP/1.1 200 OK\r\ncontent-length: 3\r\n\r\na")
                .unwrap();
            // Without a client timeout, the test fails after 10 s instead of hanging.
            stream
                .set_read_timeout(Some(Duration::from_secs(10)))
                .unwrap();
            let _ = stream.read(&mut [0; 1]);
        });
        let path = dir.join("k.key");
        let started = Instant::now();
        let error = store_with_test_timeout(&dir, url)
            .download("k.key", &abc_entry(), &path)
            .unwrap_err();
        let elapsed = started.elapsed();
        assert_eq!(error, "proving_key_download_failed");
        assert!(
            (TEST_STALL_TIMEOUT..Duration::from_secs(5)).contains(&elapsed),
            "{elapsed:?}"
        );
        assert!(!path.exists());
        assert!(!path.with_extension("key.partial").exists());
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn a_slow_download_that_keeps_receiving_data_completes() {
        let dir = test_dir("slow");
        let url = key_host(|stream| {
            stream
                .write_all(b"HTTP/1.1 200 OK\r\ncontent-length: 3\r\n\r\n")
                .unwrap();
            for byte in b"abc" {
                thread::sleep(TEST_STALL_TIMEOUT * 2 / 5);
                stream.write_all(&[*byte]).unwrap();
            }
        });
        let path = dir.join("k.key");
        let started = Instant::now();
        store_with_test_timeout(&dir, url)
            .download("k.key", &abc_entry(), &path)
            .unwrap();
        assert!(started.elapsed() > TEST_STALL_TIMEOUT);
        assert!(matches_entry(&path, &abc_entry()).unwrap());
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn plaintext_key_hosts_are_refused() {
        assert!(KeyStore::new("/tmp", Some("http://keys.example.com".into())).is_err());
        assert!(KeyStore::new("/tmp", Some("http://localhost.evil.com".into())).is_err());
        assert!(KeyStore::new("/tmp", Some("http://127.0.0.1:9000".into())).is_ok());
        assert!(KeyStore::new("/tmp", None).is_ok());
    }
}
