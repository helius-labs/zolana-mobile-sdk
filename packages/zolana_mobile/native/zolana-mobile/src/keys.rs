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

use crate::error::WalletError;
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
    pub fn ensure(&self, key: &ExpectedProvingKey) -> Result<PathBuf, WalletError> {
        let name = key.name.as_str();
        let entry = lockfile()
            .keys
            .get(name)
            .ok_or_else(|| WalletError::ProvingKeyUnknown { name: name.into() })?;
        if entry.sha256 != hex(&key.sha256) {
            return Err(WalletError::ProvingKeyMismatch { name: name.into() });
        }
        let path = self.dir.join(name);
        if self
            .verified
            .lock()
            .map_err(|_| WalletError::ProverUnavailable)?
            .contains(name)
        {
            return Ok(path);
        }
        if !matches_entry(name, &path, entry)? {
            fs::create_dir_all(&self.dir).map_err(|_| store_failed(&self.dir))?;
            self.download(name, entry, &path)?;
        }
        self.verified
            .lock()
            .map_err(|_| WalletError::ProverUnavailable)?
            .insert(name.to_string());
        Ok(path)
    }

    /// Download key `name` next to `path`, check it against `entry` as it
    /// lies on disk, and move it into place. The transport writes the body to
    /// the file itself, so the key is never held in memory.
    fn download(&self, name: &str, entry: &LockEntry, path: &Path) -> Result<(), WalletError> {
        let url = format!("{}/{}/{name}", self.base_url, lockfile().prefix);
        let partial = path.with_extension("key.partial");
        let result = (|| {
            let _ = fs::remove_file(&partial);
            let failed = || WalletError::ProvingKeyDownloadFailed { name: name.into() };
            let response = self
                .transport
                .send_blocking(TransportRequest {
                    method: "GET".to_string(),
                    url,
                    headers: HashMap::new(),
                    body: Vec::new(),
                    max_response_bytes: u32::try_from(entry.size).ok(),
                    timeout_ms: None,
                    download_path: Some(partial.display().to_string()),
                })
                .map_err(|_| failed())?;
            if !(200..300).contains(&response.status) {
                return Err(failed());
            }
            // A transport that returned the body instead of writing the file.
            if !response.body.is_empty() || !partial.exists() {
                let mut file = File::create(&partial).map_err(|_| store_failed(path))?;
                file.write_all(&response.body)
                    .map_err(|_| store_failed(path))?;
            }
            let mut file = File::open(&partial).map_err(|_| store_failed(path))?;
            let digest = copy_bounded(&mut file, &mut std::io::sink(), entry.size, name, path)?;
            if digest != entry.sha256 {
                return Err(WalletError::ProvingKeyCorrupt { name: name.into() });
            }
            File::open(&partial)
                .and_then(|file| file.sync_all())
                .map_err(|_| store_failed(path))?;
            fs::rename(&partial, path).map_err(|_| store_failed(path))
        })();
        if result.is_err() {
            let _ = fs::remove_file(&partial);
        }
        result
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
    Ok(copy_bounded(&mut file, &mut std::io::sink(), entry.size, name, path)? == entry.sha256)
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
        assert!(matches_entry("k.key", &path, &entry).unwrap());
        fs::write(&path, b"abd").unwrap();
        assert!(!matches_entry("k.key", &path, &entry).unwrap());
        fs::write(&path, b"abcd").unwrap();
        assert!(!matches_entry("k.key", &path, &entry).unwrap());
        assert!(!matches_entry("missing.key", &dir.join("missing.key"), &entry).unwrap());
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
        let partial = path.with_extension("key.partial");
        assert_eq!(request.download_path, Some(partial.display().to_string()));

        // A transport that writes the file itself returns no body.
        fs::remove_file(&path).unwrap();
        let streaming = |body: &'static [u8]| {
            fake(move |request| {
                fs::write(request.download_path.as_ref().unwrap(), body).unwrap();
                Ok(crate::TransportResponse {
                    status: 200,
                    headers: Default::default(),
                    body: Vec::new(),
                })
            })
            .0
        };
        store(streaming(b"abc"))
            .download("k.key", &entry, &path)
            .unwrap();
        assert_eq!(fs::read(&path).unwrap(), b"abc");
        assert!(!partial.exists());
        fs::remove_file(&path).unwrap();
        assert_eq!(
            store(streaming(b"abd"))
                .download("k.key", &entry, &path)
                .unwrap_err(),
            WalletError::ProvingKeyCorrupt {
                name: "k.key".into()
            }
        );
        assert!(!path.exists() && !partial.exists());
        fs::write(&path, b"abc").unwrap();

        fs::remove_file(&path).unwrap();
        let (missing, _) = fake(|_| {
            Ok(crate::TransportResponse {
                status: 404,
                headers: Default::default(),
                body: b"abc".to_vec(),
            })
        });
        let (tampered, _) = fake(|_| ok("abd"));
        let download_failed = || WalletError::ProvingKeyDownloadFailed {
            name: "k.key".into(),
        };
        // The default transport fails a body that stalls for its bound, after
        // the status.
        let (stalled, _) = fake(|_| {
            Err(crate::TransportFailure {
                message: "TimeoutException after 0:00:30.000000: No stream event".to_string(),
                status: Some(200),
            })
        });
        for (transport, expected) in [
            (missing, download_failed()),
            (
                tampered,
                WalletError::ProvingKeyCorrupt {
                    name: "k.key".into(),
                },
            ),
            (stalled, download_failed()),
        ] {
            assert_eq!(
                store(transport)
                    .download("k.key", &entry, &path)
                    .unwrap_err(),
                expected
            );
            assert!(!path.exists());
            assert!(!path.with_extension("key.partial").exists());
        }
        fs::remove_dir_all(&dir).unwrap();
    }
}
