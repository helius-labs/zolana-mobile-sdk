//! The on-device [`Prover`] the wallet hands to `ZolanaClient::with_prover`.

use std::sync::atomic::{AtomicU64, Ordering};

use zolana_client::{ClientError, Delivery, Proof, Prover};

use crate::{init_gnark, keys, Loaded, PREPARED};

/// Proves each `/prove` body with the pinned key for its shape, loading that
/// key into the shared prepared slot when the shape changes. Request bodies
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

    fn prove_json(&self, body: &str) -> Result<String, String> {
        let name = keys::key_name_for_request(body)?;
        let path = self.keys.ensure(&name)?;
        let mut state = PREPARED
            .lock()
            .map_err(|_| "prover_unavailable".to_string())?;
        let loaded_key = state.loaded.as_ref().map(|loaded| loaded.key.as_deref());
        if loaded_key != Some(Some(name.as_str())) {
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
                key: Some(name),
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

impl Prover for NativeProver {
    fn prove_body(&self, body: &str, _delivery: Delivery) -> Result<Proof, ClientError> {
        let proof_json = self.prove_json(body).map_err(ClientError::Prover)?;
        Proof::from_gnark_json(&proof_json)
    }
}

#[cfg(test)]
mod tests {
    use zolana_client::{Delivery, Prover};

    use super::*;

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
        let request = include_str!("../../../../../fixtures/prove-request-2x3.json");
        let proof = prover
            .prove_body(request, Delivery::InResponse)
            .expect("prove the captured client request");
        assert!(
            proof.commitment.is_none(),
            "the eddsa rail has no BSB22 commitment"
        );
        // The same key serves the next proof without reloading.
        prover.prove_body(request, Delivery::InResponse).unwrap();
    }
}
