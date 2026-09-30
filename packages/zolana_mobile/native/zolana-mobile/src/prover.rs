//! The on-device [`Prover`] the wallet hands to `ZolanaClient::with_prover`.

use std::sync::atomic::{AtomicU64, Ordering};

use zolana_client::{prover::ExpectedProvingKey, ClientError, Proof, ProveRequest, Prover};

use crate::{init_gnark, keys, Loaded, PREPARED};

/// Proves each `/prove` body with the key the client names for it, loading
/// that key into the shared prepared slot when it changes. Request bodies
/// carry nullifier secrets and never leave the process.
pub(crate) struct NativeProver {
    keys: keys::KeyStore,
    /// Id of the prepared system this prover loaded last; 0 before the first.
    loaded: AtomicU64,
}

impl NativeProver {
    pub(crate) fn new(keys: keys::KeyStore) -> Self {
        Self {
            keys,
            loaded: AtomicU64::new(0),
        }
    }

    fn prove_json(&self, body: &str, key: &ExpectedProvingKey) -> Result<String, String> {
        let name = key.name.as_str();
        let path = self.keys.ensure(key)?;
        let mut state = PREPARED
            .lock()
            .map_err(|_| "prover_unavailable".to_string())?;
        let loaded_key = state.loaded.as_ref().map(|loaded| loaded.key.as_deref());
        if loaded_key != Some(Some(name)) {
            if loaded_key == Some(None) {
                // The application holds its own prepared system; leave it.
                return Err("prover_busy".to_string());
            }
            // Drop the previous key before loading the next: gnark holds one.
            state.loaded = None;
            init_gnark()?;
            let path = path.to_str().ok_or("proving_key_path_invalid")?;
            let prover = rust_gnark::PreparedProver::load_key(path)
                .map_err(|_| "prover_load_failed".to_string())?;
            let id = state.next_id()?;
            self.loaded.store(id, Ordering::Relaxed);
            state.loaded = Some(Loaded {
                id,
                key: Some(name.to_string()),
                prover,
            });
        }
        let loaded = state.loaded.as_ref().ok_or("prover_closed")?;
        let proof = loaded
            .prover
            .prove_request(body)
            .map_err(|_| "proof_failed".to_string())?;
        Ok(proof.proof_json)
    }
}

/// A closed wallet releases the proving key it loaded, unless another prover
/// has replaced it since.
impl Drop for NativeProver {
    fn drop(&mut self) {
        let id = *self.loaded.get_mut();
        let Ok(mut state) = PREPARED.lock() else {
            return;
        };
        if state.loaded.as_ref().is_some_and(|loaded| loaded.id == id) {
            state.loaded = None;
        }
    }
}

/// Proves in the calling thread, so [`ProveRequest::delivery`] does not apply.
impl Prover for NativeProver {
    fn prove(&self, request: &dyn ProveRequest) -> Result<Proof, ClientError> {
        let body = request.body()?;
        let key = request.proving_key()?;
        let proof_json = self.prove_json(&body, &key).map_err(ClientError::Prover)?;
        Proof::from_gnark_json(&proof_json)
    }
}

#[cfg(test)]
mod tests {
    use zeroize::Zeroizing;

    use super::*;

    /// The captured 2→3 client body, with the key it asks for.
    struct Captured(ExpectedProvingKey);

    impl ProveRequest for Captured {
        fn body(&self) -> Result<Zeroizing<String>, ClientError> {
            Ok(Zeroizing::new(
                include_str!("../../../../../fixtures/prove-request-2x3.json").to_string(),
            ))
        }

        fn proving_key(&self) -> Result<ExpectedProvingKey, ClientError> {
            Ok(self.0.clone())
        }
    }

    /// Downloads `transfer_confidential_2_3.key` into `ZOLANA_TEST_KEY_DIR`
    /// (or a temp directory) unless it is already there and pinned.
    #[test]
    #[ignore = "downloads a proving key"]
    fn proves_a_client_request_with_the_pinned_key() {
        let dir = std::env::var("ZOLANA_TEST_KEY_DIR").unwrap_or_else(|_| {
            std::env::temp_dir()
                .join("zolana-mobile-keys")
                .display()
                .to_string()
        });
        let prover = NativeProver::new(keys::KeyStore::new(dir, None).unwrap());
        let proof = prover
            .prove(&Captured(transfer_2_3_key()))
            .expect("prove the captured client request");
        assert!(
            proof.commitment.is_none(),
            "the eddsa rail has no BSB22 commitment"
        );
        // The same key serves the next proof without reloading.
        prover.prove(&Captured(transfer_2_3_key())).unwrap();
    }

    fn transfer_2_3_key() -> ExpectedProvingKey {
        let name = "transfer_confidential_2_3.key";
        let (_, sha256) = zolana_interface::verifying_keys::PROVING_KEY_SHA256S
            .iter()
            .find(|(file, _)| *file == name)
            .expect("pinned 2x3 key");
        ExpectedProvingKey {
            name: name.to_string(),
            sha256: *sha256,
        }
    }
}
