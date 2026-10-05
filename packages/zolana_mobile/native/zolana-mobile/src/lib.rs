use std::{
    sync::{Mutex, OnceLock},
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
pub use keys::DEFAULT_PROVING_KEYS_URL;
pub use prover::Proving;
pub use transport::{
    Transport, TransportFailure, TransportOutcome, TransportRequest, TransportResponse,
};
pub use wallet::{
    derivation_message, MobileWallet, PendingTransaction, PendingTransactionKind,
    RegistrationStatus, TokenBalance, WalletConfig, WalletKeys,
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
    /// the application loaded itself through [`load_prover`]. The native
    /// prover switches keys only in the first case.
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
pub struct PreparedProverInfo {
    pub id: u64,
    pub load_ms: u64,
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

pub fn load_prover(
    r1cs_path: String,
    proving_key_path: String,
    verifying_key_path: String,
) -> Result<PreparedProverInfo, WalletError> {
    let started = Instant::now();
    let mut state = PREPARED.try_lock().map_err(|_| WalletError::ProverBusy)?;
    if state.loaded.is_some() {
        return Err(WalletError::ProverBusy);
    }
    let id = state.next_id()?;
    init_gnark()?;
    let prover =
        rust_gnark::PreparedProver::load(&r1cs_path, &proving_key_path, &verifying_key_path)
            .map_err(|_| WalletError::ProverLoadFailed)?;
    state.loaded = Some(Loaded {
        id,
        key: None,
        prover,
    });
    Ok(PreparedProverInfo {
        id,
        load_ms: elapsed_ms(started),
    })
}

pub fn release_prover(id: u64) -> Result<(), WalletError> {
    let mut state = PREPARED
        .lock()
        .map_err(|_| WalletError::ProverUnavailable)?;
    match &state.loaded {
        Some(loaded) if loaded.id == id => {
            state.loaded = None;
            Ok(())
        }
        _ => Err(WalletError::ProverClosed),
    }
}

/// Prove a structured Zolana `/prove` request with the prepared prover `id`.
pub fn prove_prepared(id: u64, input_json: String) -> Result<LocalProofResult, WalletError> {
    let input_json = Zeroizing::new(input_json);
    let started = Instant::now();
    let state = PREPARED.try_lock().map_err(|_| WalletError::ProverBusy)?;
    let prover = &state
        .loaded
        .as_ref()
        .filter(|loaded| loaded.id == id)
        .ok_or(WalletError::ProverClosed)?
        .prover;
    let proof = prover
        .prove_request(&input_json)
        .map_err(|_| WalletError::ProofFailed)?;
    let proof_ms = proof.prove_ms;
    if !proof.shape_known {
        return Err(WalletError::UnsupportedCircuit);
    }
    Ok(LocalProofResult {
        proof_json: proof.proof_json,
        verified: true,
        inputs: proof.inputs,
        outputs: proof.outputs,
        proof_ms,
        witness_ms: proof.witness_ms,
        verify_ms: proof.verify_ms,
        total_ms: elapsed_ms(started),
    })
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
        let load = || load_prover(asset("r1cs"), asset("pk"), asset("vk"));
        let prepared = load().unwrap();
        assert_eq!(load().unwrap_err(), WalletError::ProverBusy);

        let request = std::fs::read_to_string(assets.join("prove-request-2x2.json")).unwrap();
        let proof = prove_prepared(prepared.id, request).unwrap();
        assert!(proof.verified);
        assert_eq!((proof.inputs, proof.outputs), (2, 2));
        let json: serde_json::Value = serde_json::from_str(&proof.proof_json).unwrap();
        assert!(json["ar"].is_array());
        assert!(json["bs"].is_array());
        assert!(json["krs"].is_array());
        // A failed proof says nothing about its input.
        let error = prove_prepared(prepared.id, "{\"Secret\":\"private-sentinel\"}".to_string())
            .unwrap_err();
        assert_eq!(error, WalletError::ProofFailed);

        release_prover(prepared.id).unwrap();
        assert_eq!(
            prove_prepared(prepared.id, "{}".to_string()).unwrap_err(),
            WalletError::ProverClosed
        );
        assert_eq!(
            release_prover(prepared.id).unwrap_err(),
            WalletError::ProverClosed
        );
    }
}
