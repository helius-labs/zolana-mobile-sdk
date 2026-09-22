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
    /// Keys whose digest was checked this process. Re-hashing a 240 MB merge
    /// key before every proof would cost more than the proof.
    verified: Mutex<HashSet<String>>,
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
        })
    }

    /// Path of `name` in the store, downloading it first if it is missing or
    /// does not match the lockfile.
    pub fn ensure(&self, name: &str) -> Result<PathBuf, String> {
        let entry = lockfile()
            .keys
            .get(name)
            .ok_or_else(|| "proving_key_unknown".to_string())?;
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
            .timeout(None)
            .build()
            .and_then(|client| client.get(url).send())
            .and_then(reqwest::blocking::Response::error_for_status)
            .map_err(|_| "proving_key_download_failed".to_string())?;
        let partial = path.with_extension("key.partial");
        let result = (|| {
            let mut file = File::create(&partial).map_err(|_| "proving_key_dir_unwritable")?;
            let digest = copy_bounded(&mut response, &mut file, entry.size)?;
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
    Ok(hasher
        .finalize()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect())
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

/// Lockfile name of the key a `/prove` request needs.
pub fn key_name_for_request(body: &str) -> Result<String, String> {
    #[derive(Deserialize)]
    struct Envelope {
        #[serde(rename = "circuitType")]
        circuit_type: String,
        #[serde(rename = "nInputs")]
        n_inputs: Option<u32>,
        #[serde(rename = "nOutputs")]
        n_outputs: Option<u32>,
        inputs: Option<Vec<serde::de::IgnoredAny>>,
    }
    let envelope: Envelope =
        serde_json::from_str(body).map_err(|_| "proof_request_invalid".to_string())?;
    match (
        envelope.circuit_type.as_str(),
        envelope.n_inputs,
        envelope.n_outputs,
    ) {
        ("transfer-confidential", Some(inputs), Some(outputs)) => {
            Ok(format!("transfer_confidential_{inputs}_{outputs}.key"))
        }
        ("merge", _, _) => {
            let inputs = envelope.inputs.map_or(0, |inputs| inputs.len());
            Ok(format!("merge_{inputs}_1.key"))
        }
        _ => Err("unsupported_circuit".to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
    fn requests_map_to_their_key() {
        let transfer = r#"{"circuitType":"transfer-confidential","nInputs":2,"nOutputs":3}"#;
        assert_eq!(
            key_name_for_request(transfer).unwrap(),
            "transfer_confidential_2_3.key"
        );
        let merge = r#"{"circuitType":"merge","inputs":[1,2,3,4,5,6,7,8]}"#;
        assert_eq!(key_name_for_request(merge).unwrap(), "merge_8_1.key");
        for unsupported in [
            r#"{"circuitType":"transfer-p256-ring","nInputs":2,"nOutputs":3}"#,
            r#"{"circuitType":"transfer-confidential"}"#,
            "not json",
        ] {
            assert!(key_name_for_request(unsupported).is_err());
        }
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
    fn plaintext_key_hosts_are_refused() {
        assert!(KeyStore::new("/tmp", Some("http://keys.example.com".into())).is_err());
        assert!(KeyStore::new("/tmp", Some("http://localhost.evil.com".into())).is_err());
        assert!(KeyStore::new("/tmp", Some("http://127.0.0.1:9000".into())).is_ok());
        assert!(KeyStore::new("/tmp", None).is_ok());
    }
}
