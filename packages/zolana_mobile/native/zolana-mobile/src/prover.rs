//! The on-device [`Prover`] the wallet hands to `ZolanaClient::with_prover`.

use std::sync::atomic::{AtomicU64, Ordering};

use zolana_client::{ClientError, Proof, ProveRequest, Prover};

use crate::{init_gnark, keys, Loaded, PREPARED};

/// Proves each request with the key it names, loading that key into the
/// shared prepared slot when the shape changes. Request bodies carry
/// nullifier secrets and never leave the process.
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

    /// Proves `body` with the key `name`, which must be the key the body's
    /// circuit and shape need, pinned to `sha256`, the digest the on-chain
    /// verifier expects.
    fn prove_json(&self, body: &str, name: &str, sha256: &[u8; 32]) -> Result<String, String> {
        if keys::key_name_for_request(body)? != name {
            return Err("proving_key_mismatch".to_string());
        }
        let path = self.keys.ensure(name, sha256)?;
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
        let proof_json = self
            .prove_json(&body, &key.name, &key.sha256)
            .map_err(ClientError::Prover)?;
        Proof::from_gnark_json(&proof_json)
    }
}

#[cfg(test)]
mod tests {
    use zeroize::Zeroizing;
    use zolana_client::prover::{known_proving_keys, ExpectedProvingKey};

    use super::*;

    const KEY: &str = "transfer_confidential_2_3.key";

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

    /// The key the on-chain verifier pins for the captured body.
    fn pinned() -> ExpectedProvingKey {
        let (name, sha256) = known_proving_keys()
            .find(|(name, _)| *name == KEY)
            .expect("the verifier pins the 2→3 key");
        ExpectedProvingKey {
            name: name.to_string(),
            sha256,
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
            .prove(&Captured(pinned()))
            .expect("prove the captured client request");
        assert!(
            proof.commitment.is_none(),
            "the eddsa rail has no BSB22 commitment"
        );
        // The same key serves the next proof without reloading.
        prover.prove(&Captured(pinned())).unwrap();
    }

    #[test]
    fn refuses_a_key_the_verifier_does_not_pin() {
        let prover = NativeProver::new(keys::KeyStore::new(std::env::temp_dir(), None).unwrap());
        let mut key = pinned();
        key.sha256[0] ^= 1;
        let error = prover.prove(&Captured(key)).unwrap_err();
        assert!(
            matches!(&error, ClientError::Prover(message) if message == "proving_key_mismatch"),
            "{error:?}"
        );
    }
}
