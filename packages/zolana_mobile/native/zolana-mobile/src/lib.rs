use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        Mutex, OnceLock, PoisonError,
    },
    time::Instant,
};

use zeroize::Zeroizing;
use zolana_hasher::{Hasher, Poseidon};

mod activity;
mod asset;
mod error;
mod keys;
mod prover;
mod transport;
mod wallet;

pub use activity::{ActivityEntry, ActivityKind};
pub use asset::MintConfig;
pub use error::WalletError;
pub use keys::{ProvingKeyProgress, ProvingKeyStatus, ProvingKeys, DEFAULT_PROVING_KEYS_URL};
pub use prover::Proving;
pub use transport::{
    Transport, TransportFailure, TransportOutcome, TransportRequest, TransportResponse,
};
pub use wallet::{
    derivation_message, MobileWallet, Operation, Output, PreparedTransaction, RegistrationStatus,
    SpendLimit, TokenBalance, WalletConfig, WalletKeys, WalletOverview,
};

static GNARK_INIT: OnceLock<Result<(), WalletError>> = OnceLock::new();
static PREPARED: Mutex<ProverState> = Mutex::new(ProverState {
    next_id: 1,
    loaded: None,
});

/// The one prepared proving system gnark holds at a time.
struct ProverState {
    next_id: u64,
    loaded: Option<Loaded>,
}

struct Loaded {
    id: u64,
    /// The lockfile key [`prover::NativeProver`] loaded, or `None` for a system
    /// the application loaded itself as a [`LocalProver`]. The native prover
    /// switches keys only in the first case.
    key: Option<String>,
    prover: rust_gnark::PreparedProver,
}

impl ProverState {
    fn next_id(&mut self) -> Result<u64, WalletError> {
        let id = self.next_id;
        self.next_id = id.checked_add(1).ok_or(WalletError::ProverUnavailable)?;
        Ok(id)
    }
}

#[derive(Debug, Clone)]
pub struct LocalProofResult {
    pub proof_json: String,
    pub verified: bool,
    pub inputs: u32,
    pub outputs: u32,
    pub proof_ms: u64,
    pub witness_ms: u64,
    pub verify_ms: u64,
    pub total_ms: u64,
}

/// A proof of a `/prove` request through the wallet's key store, by
/// [`ProvingKeys::prove`], and where its time went. Milliseconds; the
/// stages add up to about `total_ms`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProvedRequest {
    pub proof_json: String,
    /// The key it was proved with.
    pub key: String,
    /// Bytes of the key downloaded for this proof: its size when it was not
    /// on the device, what a partial download lacked, or none.
    pub downloaded_bytes: u64,
    /// Fetching and checking the key, download included.
    pub download_ms: u64,
    /// Loading the key into the prover; none when it was the loaded one.
    pub load_ms: u64,
    pub witness_ms: u64,
    pub prove_ms: u64,
    pub verify_ms: u64,
    pub total_ms: u64,
}

pub fn sdk_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

pub fn poseidon_hash(inputs: Vec<Vec<u8>>) -> Result<Vec<u8>, WalletError> {
    if inputs.is_empty() || inputs.len() > 12 {
        return Err(WalletError::PoseidonInputCountInvalid {
            count: inputs.len() as u64,
        });
    }
    if let Some((index, input)) = inputs
        .iter()
        .enumerate()
        .find(|(_, input)| input.len() != 32)
    {
        return Err(WalletError::PoseidonInputLengthInvalid {
            index: index as u64,
            length: input.len() as u64,
        });
    }
    let refs = inputs.iter().map(Vec::as_slice).collect::<Vec<_>>();
    Poseidon::hashv(&refs)
        .map(|hash| hash.to_vec())
        .map_err(|failure| zolana_client::ClientError::from(failure).into())
}

/// A circuit the application loaded from its r1cs and key files, to prove
/// structured Zolana `/prove` requests with on the device, such as for a
/// benchmark. gnark holds one prepared circuit at a time: while one is
/// loaded, the wallet's own proofs fail with `prover_busy`, so close it before
/// the wallet proves.
pub struct LocalProver {
    id: u64,
    load_ms: u64,
    closed: AtomicBool,
}

impl LocalProver {
    /// Load the circuit from its compiled constraints (`circuit_path`, the
    /// `.r1cs` file) and keys. Fails with `prover_busy` while another circuit
    /// is loaded or proving.
    pub fn load(
        circuit_path: String,
        proving_key_path: String,
        verifying_key_path: String,
    ) -> Result<LocalProver, WalletError> {
        let started = Instant::now();
        let mut state = PREPARED.try_lock().map_err(|_| WalletError::ProverBusy)?;
        if state.loaded.is_some() {
            return Err(WalletError::ProverBusy);
        }
        let id = state.next_id()?;
        init_gnark()?;
        let prover =
            rust_gnark::PreparedProver::load(&circuit_path, &proving_key_path, &verifying_key_path)
                .map_err(|_| WalletError::ProverLoadFailed)?;
        state.loaded = Some(Loaded {
            id,
            key: None,
            prover,
        });
        Ok(LocalProver {
            id,
            load_ms: elapsed_ms(started),
            closed: AtomicBool::new(false),
        })
    }

    /// How long loading the circuit took.
    #[flutter_rust_bridge::frb(sync, getter)]
    pub fn load_ms(&self) -> u64 {
        self.load_ms
    }

    /// Prove `request_json`, a Zolana `/prove` request, and verify the proof.
    /// One proof at a time: another fails with `prover_busy`. Fails with
    /// `prover_closed` after `close`, also when the close came while this proof
    /// ran.
    pub fn prove(&self, request_json: String) -> Result<LocalProofResult, WalletError> {
        let request_json = Zeroizing::new(request_json);
        let started = Instant::now();
        if self.closed.load(Ordering::Acquire) {
            return Err(WalletError::ProverClosed);
        }
        let mut state = PREPARED.try_lock().map_err(|_| WalletError::ProverBusy)?;
        let prover = &state
            .loaded
            .as_ref()
            .filter(|loaded| loaded.id == self.id)
            .ok_or(WalletError::ProverClosed)?
            .prover;
        let proof = prover.prove_request(&request_json);
        if self.closed.load(Ordering::Acquire) {
            state.loaded = None;
            return Err(WalletError::ProverClosed);
        }
        let proof = proof.map_err(|_| WalletError::ProofFailed)?;
        if !proof.shape_known {
            return Err(WalletError::UnsupportedCircuit);
        }
        Ok(LocalProofResult {
            proof_json: proof.proof_json,
            verified: true,
            inputs: proof.inputs,
            outputs: proof.outputs,
            proof_ms: proof.prove_ms,
            witness_ms: proof.witness_ms,
            verify_ms: proof.verify_ms,
            total_ms: elapsed_ms(started),
        })
    }

    /// Release the circuit, after the proof in progress, which then fails with
    /// `prover_closed`. Later proofs fail the same way.
    pub fn close(&self) {
        self.closed.store(true, Ordering::Release);
        self.release();
    }

    fn release(&self) {
        let mut state = PREPARED.lock().unwrap_or_else(PoisonError::into_inner);
        if state
            .loaded
            .as_ref()
            .is_some_and(|loaded| loaded.id == self.id)
        {
            state.loaded = None;
        }
    }
}

impl Drop for LocalProver {
    fn drop(&mut self) {
        self.release();
    }
}

fn init_gnark() -> Result<(), WalletError> {
    GNARK_INIT
        .get_or_init(|| rust_gnark::init().map_err(|_| WalletError::ProverInitFailed))
        .clone()
}

fn elapsed_ms(started: Instant) -> u64 {
    started.elapsed().as_millis().try_into().unwrap_or(u64::MAX)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn poseidon_binding_validates_inputs() {
        assert!(poseidon_hash(vec![]).is_err());
        assert!(poseidon_hash(vec![vec![0; 31]]).is_err());
        assert_eq!(poseidon_hash(vec![vec![0; 32]]).unwrap().len(), 32);
    }

    #[test]
    #[ignore = "requires the staged proving assets"]
    fn proves_the_staged_request() {
        let repository = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../..");
        let assets = repository.join("packages/zolana_mobile/example/assets/proving");
        let asset = |extension: &str| {
            assets
                .join(format!("transfer_confidential_2_2.{extension}"))
                .display()
                .to_string()
        };
        let load = || LocalProver::load(asset("r1cs"), asset("pk"), asset("vk"));
        let prover = load().unwrap();
        assert_eq!(load().err(), Some(WalletError::ProverBusy));

        let request = std::fs::read_to_string(assets.join("prove-request-2x2.json")).unwrap();
        let proof = prover.prove(request.clone()).unwrap();
        assert!(proof.verified);
        assert_eq!((proof.inputs, proof.outputs), (2, 2));
        let json: serde_json::Value = serde_json::from_str(&proof.proof_json).unwrap();
        assert!(json["ar"].is_array());
        assert!(json["bs"].is_array());
        assert!(json["krs"].is_array());
        // A failed proof says nothing about its input.
        let error = prover
            .prove("{\"Secret\":\"private-sentinel\"}".to_string())
            .unwrap_err();
        assert_eq!(error, WalletError::ProofFailed);

        // A close during a proof fails that proof and frees the circuit.
        std::thread::scope(|scope| {
            let running = scope.spawn(|| prover.prove(request.clone()));
            std::thread::sleep(std::time::Duration::from_millis(proof.proof_ms / 2));
            prover.close();
            assert_eq!(
                running.join().unwrap().err(),
                Some(WalletError::ProverClosed)
            );
        });
        assert_eq!(prover.prove(request).err(), Some(WalletError::ProverClosed));
        drop(prover);
        load().unwrap().close();
    }
}
