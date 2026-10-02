use std::{
    sync::{Mutex, OnceLock},
    time::Instant,
};

use zeroize::Zeroizing;
use zolana_hasher::{Hasher, Poseidon};

mod activity;
mod asset;
mod keys;
mod prover;
mod transport;
mod wallet;

pub use activity::{ActivityEntry, ActivityKind};
pub use keys::DEFAULT_PROVING_KEYS_URL;
pub use prover::Proving;
pub use transport::{Transport, TransportOutcome, TransportRequest, TransportResponse};
pub use wallet::{
    derivation_message, MobileWallet, PendingTransaction, PendingTransactionKind,
    RegistrationStatus, TokenBalance, WalletConfig, WalletKeys,
};

static GNARK_INIT: OnceLock<Result<(), String>> = OnceLock::new();
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
    fn next_id(&mut self) -> Result<u64, String> {
        let id = self.next_id;
        self.next_id = id.checked_add(1).ok_or("prover_id_exhausted")?;
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

pub fn poseidon_hash(inputs: Vec<Vec<u8>>) -> Result<Vec<u8>, String> {
    if inputs.is_empty() || inputs.len() > 12 {
        return Err("Poseidon expects between 1 and 12 field elements".to_string());
    }
    if let Some((index, input)) = inputs
        .iter()
        .enumerate()
        .find(|(_, input)| input.len() != 32)
    {
        return Err(format!(
            "Poseidon input {index} has {} bytes, expected 32",
            input.len()
        ));
    }
    let refs = inputs.iter().map(Vec::as_slice).collect::<Vec<_>>();
    Poseidon::hashv(&refs)
        .map(|hash| hash.to_vec())
        .map_err(|error| error.to_string())
}

pub fn load_prover(
    r1cs_path: String,
    proving_key_path: String,
    verifying_key_path: String,
) -> Result<PreparedProverInfo, String> {
    let started = Instant::now();
    let mut state = PREPARED.try_lock().map_err(|_| "prover_busy".to_string())?;
    if state.loaded.is_some() {
        return Err("prover_busy".to_string());
    }
    let id = state.next_id()?;
    init_gnark()?;
    let prover =
        rust_gnark::PreparedProver::load(&r1cs_path, &proving_key_path, &verifying_key_path)
            .map_err(|_| "prover_load_failed".to_string())?;
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

pub fn release_prover(id: u64) -> Result<(), String> {
    let mut state = PREPARED
        .lock()
        .map_err(|_| "prover_unavailable".to_string())?;
    match &state.loaded {
        Some(loaded) if loaded.id == id => {
            state.loaded = None;
            Ok(())
        }
        _ => Err("prover_closed".to_string()),
    }
}

/// Prove a structured Zolana `/prove` request with the prepared prover `id`.
pub fn prove_prepared(id: u64, input_json: String) -> Result<LocalProofResult, String> {
    let input_json = Zeroizing::new(input_json);
    let started = Instant::now();
    let state = PREPARED.try_lock().map_err(|_| "prover_busy".to_string())?;
    let prover = &state
        .loaded
        .as_ref()
        .filter(|loaded| loaded.id == id)
        .ok_or("prover_closed")?
        .prover;
    let proof = prover
        .prove_request(&input_json)
        .map_err(|_| "proof_failed".to_string())?;
    let proof_ms = proof.prove_ms;
    if !proof.shape_known {
        return Err("unsupported_circuit".to_string());
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

fn init_gnark() -> Result<(), String> {
    GNARK_INIT
        .get_or_init(|| rust_gnark::init().map_err(|_| "prover_init_failed".to_string()))
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
                .join(format!("transfer_confidential_2_3.{extension}"))
                .display()
                .to_string()
        };
        let load = || load_prover(asset("r1cs"), asset("pk"), asset("vk"));
        let prepared = load().unwrap();
        assert_eq!(load().unwrap_err(), "prover_busy");

        let request = std::fs::read_to_string(assets.join("prove-request-2x3.json")).unwrap();
        let proof = prove_prepared(prepared.id, request).unwrap();
        assert!(proof.verified);
        assert_eq!((proof.inputs, proof.outputs), (2, 3));
        let json: serde_json::Value = serde_json::from_str(&proof.proof_json).unwrap();
        assert!(json["ar"].is_array());
        assert!(json["bs"].is_array());
        assert!(json["krs"].is_array());
        // A failed proof says nothing about its input.
        let error = prove_prepared(prepared.id, "{\"Secret\":\"private-sentinel\"}".to_string())
            .unwrap_err();
        assert_eq!(error, "proof_failed");

        release_prover(prepared.id).unwrap();
        assert_eq!(
            prove_prepared(prepared.id, "{}".to_string()).unwrap_err(),
            "prover_closed"
        );
        assert_eq!(release_prover(prepared.id).unwrap_err(), "prover_closed");
    }
}
