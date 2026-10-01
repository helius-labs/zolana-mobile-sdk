//! The [`Prover`] the wallet hands to `ZolanaClient::with_prover`: the
//! device's own, or the application's backend for a spend that asks for it.

use std::sync::{
    atomic::{AtomicU64, Ordering},
    Arc, Mutex, PoisonError,
};

use flutter_rust_bridge::DartFnFuture;
use zolana_client::{prover::ExpectedProvingKey, ClientError, Proof, ProveRequest, Prover};

use crate::{init_gnark, keys, Loaded, PREPARED};

/// Where a spend is proved.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Proving {
    /// On the device, with the pinned proving key for its shape. The witness
    /// stays in this process.
    Local,
    /// By the application's backend, given with
    /// [`MobileWallet::set_remote_prover`](crate::MobileWallet::set_remote_prover).
    /// The backend receives the witness, the wallet's nullifier secret
    /// included.
    Remote,
}

/// The `/prove` request body in, the prover's proof out, `None` when it failed.
type ProveRemotely = dyn Fn(Vec<u8>) -> DartFnFuture<Option<Vec<u8>>> + Send + Sync;

/// The application's backend. It receives the `/prove` request body the Zolana
/// SDK's prover client sends and returns its prover's proof. The client
/// verifies the proof against the pinned verifying key and the public input it
/// computed itself before the wallet builds the message.
pub(crate) struct RemoteProver(Box<ProveRemotely>);

impl RemoteProver {
    pub(crate) fn new(
        prove: impl Fn(Vec<u8>) -> DartFnFuture<Option<Vec<u8>>> + Send + Sync + 'static,
    ) -> Self {
        Self(Box::new(prove))
    }
}

/// Waits in the calling thread, a flutter_rust_bridge worker, while the Dart
/// thread runs the callback.
impl Prover for RemoteProver {
    fn prove(&self, request: &dyn ProveRequest) -> Result<Proof, ClientError> {
        let body = request.body()?;
        let response = futures_executor::block_on((self.0)(body.as_bytes().to_vec()))
            .ok_or_else(|| ClientError::Prover("remote_prover_failed".to_string()))?;
        proof_from_response(&response)
            .ok_or_else(|| ClientError::Prover("proof_malformed".to_string()))
    }
}

/// The gnark proof JSON in a prover's response, alone or as its `proof`, as
/// the SDK's prover client reads it.
fn proof_from_response(response: &[u8]) -> Option<Proof> {
    let response: serde_json::Value = serde_json::from_slice(response).ok()?;
    let proof = response.get("proof").unwrap_or(&response);
    Proof::from_gnark_json(&proof.to_string()).ok()
}

/// The backend proving the current spend, or `None` to prove it on the device.
/// The wallet sets it for each spend; its client owns the [`WalletProver`]
/// that reads it.
pub(crate) type SpendProver = Arc<Mutex<Option<Arc<RemoteProver>>>>;

/// The prover the wallet's client holds.
pub(crate) struct WalletProver {
    pub(crate) native: NativeProver,
    pub(crate) remote: SpendProver,
}

impl Prover for WalletProver {
    fn prove(&self, request: &dyn ProveRequest) -> Result<Proof, ClientError> {
        let remote = self
            .remote
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone();
        match remote {
            Some(remote) => remote.prove(request),
            None => self.native.prove(request),
        }
    }
}

/// A failed proof as the wallet reports it: the wallet's provers fail with a
/// code, and a proof the pinned verifying key rejects with `proof_invalid`.
pub(crate) fn proving_error(error: ClientError) -> String {
    match error {
        ClientError::Prover(code) => code,
        ClientError::ProofVerification(_) => "proof_invalid".to_string(),
        error => crate::wallet::error(error),
    }
}

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
    use std::{
        io::{BufRead, BufReader, Read, Write},
        net::TcpListener,
    };

    use solana_address::Address;
    use zeroize::Zeroizing;
    use zolana_client::{
        assemble, prover::verify_proof_statement, AssembledTransfer, MerkleContext, MerkleProof,
        NonInclusionProof, ProverClient, SpendProof, NULLIFIER_TREE_HEIGHT, STATE_TREE_HEIGHT,
    };
    use zolana_interface::{pda, verifying_keys::CircuitId, N_PUBLIC_SLOTS};
    use zolana_keypair::ShieldedKeypair;
    use zolana_transaction::{
        instructions::transact::{ConfidentialTransaction, Shape},
        Data, Mint, Utxo, WalletUtxo,
    };

    use super::*;

    const REQUEST: &str = include_str!("../../../../../fixtures/prove-request-2x3.json");
    /// The Helius prover's proof of [`REQUEST`].
    const RESPONSE: &[u8] = include_bytes!("../../../../../fixtures/prove-response-2x3.json");

    /// A backend that answers every request with `response`, and the last
    /// request it received.
    fn backend(response: Option<&'static [u8]>) -> (RemoteProver, Arc<Mutex<Vec<u8>>>) {
        let received = Arc::new(Mutex::new(Vec::new()));
        let sink = Arc::clone(&received);
        let prover = RemoteProver::new(move |request| {
            *sink.lock().unwrap() = request;
            Box::pin(async move { response.map(<[u8]>::to_vec) })
        });
        (prover, received)
    }

    #[test]
    fn a_backend_receives_the_client_request_and_fails_by_name() {
        let request = Captured(transfer_2_3_key());
        let (prover, received) = backend(Some(RESPONSE));
        let proof = prover.prove(&request).expect("the prover's proof");
        assert_eq!(*received.lock().unwrap(), REQUEST.as_bytes());
        // The fixture is a real proof of its request.
        let request: serde_json::Value = serde_json::from_str(REQUEST).unwrap();
        let public_input = request["publicInputHash"].as_str().unwrap();
        let public_input = format!("{:0>64}", public_input.trim_start_matches("0x"));
        let public_input: Vec<u8> = (0..64)
            .step_by(2)
            .map(|i| u8::from_str_radix(&public_input[i..i + 2], 16).unwrap())
            .collect();
        let circuit = CircuitId::ConfidentialEddsa(2, 3, N_PUBLIC_SLOTS as u8);
        verify_proof_statement(
            &proof,
            public_input.try_into().unwrap(),
            circuit.verifying_key().unwrap(),
        )
        .expect("the fixture proof verifies");

        let request = Captured(transfer_2_3_key());
        for (response, code) in [
            (None, "remote_prover_failed"),
            (Some(&b"not json"[..]), "proof_malformed"),
            (Some(&br#"{"proof":{"ar":["0x1"]}}"#[..]), "proof_malformed"),
        ] {
            let (prover, _) = backend(response);
            let error = proving_error(prover.prove(&request).unwrap_err());
            assert_eq!(error, code);
        }
    }

    /// The client verifies a backend's proof in `AssembledTransfer::prove`,
    /// which `finish_submission_unsigned_sync` calls before it fetches a
    /// blockhash and builds the message.
    #[test]
    fn a_backend_proof_is_verified_against_the_transaction() {
        let sender = ShieldedKeypair::new_ed25519().unwrap();
        let mut assembled = assembled_transfer(&sender);
        let shape = (
            assembled.prover_inputs.inputs.len(),
            assembled.prover_inputs.outputs.len(),
        );
        assert_eq!(shape, (2, 3), "the shape of the fixture proof");
        let sent = prover_client_body(&mut assembled, &sender);

        for (response, code) in [
            // A real proof, of another transaction.
            (RESPONSE, "proof_invalid"),
            (&b"{}"[..], "proof_malformed"),
        ] {
            let (backend, received) = backend(Some(response));
            let prover = WalletProver {
                native: NativeProver::new(
                    keys::KeyStore::new(std::env::temp_dir().display().to_string(), None, None)
                        .unwrap(),
                ),
                remote: Arc::new(Mutex::new(Some(Arc::new(backend)))),
            };
            let error = proving_error(assembled.prove(&prover, &sender).unwrap_err());
            assert_eq!(error, code);
            assert_eq!(
                *received.lock().unwrap(),
                sent,
                "the backend receives what the SDK's prover client sends"
            );
        }
    }

    /// A SOL transfer of `sender`'s one note, built and assembled as the wallet
    /// builds a spend, against a made-up witness, in the shape of the fixture
    /// proof.
    fn assembled_transfer(sender: &ShieldedKeypair) -> AssembledTransfer {
        let utxo = Utxo {
            owner: sender.signing_pubkey(),
            asset: Mint::SOL,
            amount: 10,
            blinding: [1; 32],
            ring_program_id: None,
            data: Data::default(),
        };
        let nullifier_pubkey = sender.nullifier_key.pubkey().unwrap();
        let utxo_hash = utxo.hash(&nullifier_pubkey, &[0; 32], &[0; 32], 0).unwrap();
        let note = WalletUtxo {
            nullifier: sender
                .nullifier_key
                .nullifier(&utxo_hash, &utxo.blinding)
                .unwrap(),
            utxo,
            nullifier_pubkey,
            utxo_hash,
            data_hash: None,
            ring_data_hash: None,
            tree_id: 0,
            leaf_index: 0,
            slot: 0,
            tx_signature: Default::default(),
            slot_index: 0,
        };
        let payer = Address::new_from_array(sender.signing_pubkey().as_ed25519().unwrap());
        let recipient = ShieldedKeypair::new_ed25519().unwrap();
        let mut transaction = ConfidentialTransaction::new(vec![note.clone()], payer).unwrap();
        transaction
            .transfer_sol(&recipient.shielded_address().unwrap(), 4)
            .unwrap();
        transaction
            .pad_utxos(Shape::IN2_OUT3, &sender.shielded_address().unwrap())
            .unwrap();
        let proof_inputs = transaction.encrypt(sender).unwrap();

        let context = MerkleContext {
            tree_type: 0,
            tree: pda::tree(0),
        };
        let nullifier = |leaf| NonInclusionProof {
            leaf,
            merkle_context: context.clone(),
            path: vec![[0; 32]; NULLIFIER_TREE_HEIGHT],
            low_element: [0; 32],
            low_element_index: 0,
            high_element: [0; 32],
            high_element_index: 0,
            root: [0; 32],
            root_seq: 0,
            root_index: 0,
        };
        let spend = SpendProof {
            state: MerkleProof {
                leaf: note.utxo_hash,
                merkle_context: context.clone(),
                path: vec![[0; 32]; STATE_TREE_HEIGHT],
                leaf_index: 0,
                root: [0; 32],
                root_seq: 0,
                root_index: 0,
            },
            nullifier: nullifier(note.nullifier),
        };
        let dummies: Vec<_> = proof_inputs
            .dummy_nullifiers()
            .into_iter()
            .map(nullifier)
            .collect();
        assemble(proof_inputs, &[spend], &dummies).unwrap()
    }

    /// The body the SDK's prover client posts for `assembled`, to a prover that
    /// answers with an error.
    fn prover_client_body(assembled: &mut AssembledTransfer, sender: &ShieldedKeypair) -> Vec<u8> {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let client = ProverClient::new(format!("http://{}", listener.local_addr().unwrap()));
        let server = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut reader = BufReader::new(stream.try_clone().unwrap());
            let mut length = 0;
            loop {
                let mut line = String::new();
                reader.read_line(&mut line).unwrap();
                if line == "\r\n" {
                    break;
                }
                if let Some(value) = line.to_lowercase().strip_prefix("content-length:") {
                    length = value.trim().parse().unwrap();
                }
            }
            let mut body = vec![0; length];
            reader.read_exact(&mut body).unwrap();
            stream
                .write_all(b"HTTP/1.1 500 Internal Server Error\r\ncontent-length: 0\r\n\r\n")
                .unwrap();
            body
        });
        assert!(assembled.prove(&client, sender).is_err());
        server.join().unwrap()
    }

    /// The captured 2→3 client body, with the key it asks for.
    struct Captured(ExpectedProvingKey);

    impl ProveRequest for Captured {
        fn body(&self) -> Result<Zeroizing<String>, ClientError> {
            Ok(Zeroizing::new(REQUEST.to_string()))
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
        let prover = NativeProver::new(keys::KeyStore::new(dir, None, None).unwrap());
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
