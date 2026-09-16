use std::{
    fs,
    path::{Path, PathBuf},
    str::FromStr,
    sync::{Mutex, OnceLock},
    time::Instant,
};

use zeroize::Zeroizing;
use zolana_hasher::{Hasher, Poseidon};
use zolana_keypair::{ShieldedAddress, ShieldedKeypair, SigningKey};
use zolana_transaction::{
    instructions::{transact::ConfidentialTransfer, types::SppProofInputUtxo},
    Address, AssetRegistry, Data, Utxo, SOL_MINT,
};

static GNARK_INIT: OnceLock<Result<(), String>> = OnceLock::new();
static PREPARED: Mutex<ProverState> = Mutex::new(ProverState {
    next_id: 1,
    loaded: None,
});

struct ProverState {
    next_id: u64,
    loaded: Option<(u64, rust_gnark::PreparedProver)>,
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

#[derive(Debug, Clone)]
pub struct GnarkProofResult {
    pub proof: String,
    pub public_inputs: String,
}

pub struct TransferDraftRequest {
    pub sender_seed: Vec<u8>,
    pub recipient: String,
    pub input_lamports: u64,
    pub transfer_lamports: u64,
}

#[derive(Debug, Clone)]
pub struct TransferDraftOutput {
    pub owner: String,
    pub lamports: u64,
    pub is_change: bool,
    pub is_dummy: bool,
    pub commitment_hex: String,
}

#[derive(Debug, Clone)]
pub struct TransferDraft {
    pub sender: String,
    pub recipient: String,
    pub input_lamports: u64,
    pub transfer_lamports: u64,
    pub change_lamports: u64,
    pub shape: String,
    pub first_nullifier_hex: String,
    pub external_data_hash_hex: String,
    pub outputs: Vec<TransferDraftOutput>,
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

pub fn generate_gnark_proof(
    r1cs_path: String,
    proving_key_path: String,
    witness_json: String,
) -> Result<GnarkProofResult, String> {
    let witness_json = Zeroizing::new(witness_json);
    let _state = PREPARED.try_lock().map_err(|_| "prover_busy".to_string())?;
    init_gnark()?;
    rust_gnark::groth16_prove(&r1cs_path, &proving_key_path, &witness_json)
        .map(|result| GnarkProofResult {
            proof: result.proof,
            public_inputs: result.public_inputs,
        })
        .map_err(|_| "proof_failed".to_string())
}

pub fn verify_gnark_proof(
    r1cs_path: String,
    verifying_key_path: String,
    proof_result: GnarkProofResult,
) -> Result<bool, String> {
    let _state = PREPARED.try_lock().map_err(|_| "prover_busy".to_string())?;
    init_gnark()?;
    rust_gnark::groth16_verify(
        &r1cs_path,
        &verifying_key_path,
        &rust_gnark::Groth16ProofResult {
            proof: proof_result.proof,
            public_inputs: proof_result.public_inputs,
            ..Default::default()
        },
    )
    .map_err(|_| "verification_failed".to_string())
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
    let id = state.next_id;
    state.next_id = id.checked_add(1).ok_or("prover_id_exhausted")?;
    init_gnark()?;
    let prover =
        rust_gnark::PreparedProver::load(&r1cs_path, &proving_key_path, &verifying_key_path)
            .map_err(|_| "prover_load_failed".to_string())?;
    state.loaded = Some((id, prover));
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
        Some((loaded_id, _)) if *loaded_id == id => {
            state.loaded = None;
            Ok(())
        }
        _ => Err("prover_closed".to_string()),
    }
}

pub fn prove_prepared(
    id: u64,
    input_json: String,
    structured_request: bool,
) -> Result<LocalProofResult, String> {
    let input_json = Zeroizing::new(input_json);
    let started = Instant::now();
    let state = PREPARED.try_lock().map_err(|_| "prover_busy".to_string())?;
    let (_, prover) = state
        .loaded
        .as_ref()
        .filter(|(loaded_id, _)| *loaded_id == id)
        .ok_or("prover_closed")?;
    let proof = if structured_request {
        prover.prove_request(&input_json)
    } else {
        prover.prove(&input_json)
    }
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

pub fn prove_assignment(
    proving_key_path: String,
    assignment_path: String,
) -> Result<LocalProofResult, String> {
    let _state = PREPARED.try_lock().map_err(|_| "prover_busy".to_string())?;
    init_gnark()?;
    let total_started = Instant::now();
    let proving_key = PathBuf::from(&proving_key_path);
    let r1cs_path = sibling_asset(&proving_key, "r1cs")?;
    let verifying_key_path = sibling_asset(&proving_key, "vk")?;
    let witness_json = Zeroizing::new(
        fs::read_to_string(&assignment_path).map_err(|_| "witness_read_failed".to_string())?,
    );
    let proof = rust_gnark::groth16_prove(&r1cs_path, &proving_key_path, &witness_json)
        .map_err(|_| "proof_failed".to_string())?;
    let proof_ms = proof.prove_ms;
    let verify_started = Instant::now();
    let verified = rust_gnark::groth16_verify(&r1cs_path, &verifying_key_path, &proof)
        .map_err(|_| "verification_failed".to_string())?;
    if !verified {
        return Err("proof_invalid".to_string());
    }
    let verify_ms = elapsed_ms(verify_started);
    if !proof.shape_known {
        return Err("unsupported_circuit".to_string());
    }

    Ok(LocalProofResult {
        proof_json: serde_json::json!({
            "proof": proof.proof,
            "publicInputs": proof.public_inputs,
        })
        .to_string(),
        verified,
        inputs: proof.inputs,
        outputs: proof.outputs,
        proof_ms,
        witness_ms: proof.witness_ms,
        verify_ms,
        total_ms: elapsed_ms(total_started),
    })
}

pub fn shielded_address(seed: Vec<u8>) -> Result<String, String> {
    let seed = Zeroizing::new(seed);
    keypair_from_seed(&seed)?
        .shielded_address()
        .map(|address| address.to_string())
        .map_err(|error| error.to_string())
}

pub fn prepare_transfer(request: TransferDraftRequest) -> Result<TransferDraft, String> {
    let sender_seed = Zeroizing::new(request.sender_seed);
    if request.input_lamports == 0 {
        return Err("input_lamports must be greater than zero".to_string());
    }
    if request.transfer_lamports == 0 {
        return Err("transfer_lamports must be greater than zero".to_string());
    }
    if request.transfer_lamports > request.input_lamports {
        return Err("transfer_lamports exceeds the input balance".to_string());
    }

    let sender = keypair_from_seed(&sender_seed)?;
    let sender_address = sender
        .shielded_address()
        .map_err(|error| error.to_string())?;
    let recipient =
        ShieldedAddress::from_str(&request.recipient).map_err(|error| error.to_string())?;
    let payer = Address::new_from_array(
        sender
            .signing_pubkey()
            .as_ed25519()
            .map_err(|error| error.to_string())?,
    );
    let mut blinding = [0u8; 32];
    getrandom::fill(&mut blinding[1..])
        .map_err(|error| format!("generate input blinding: {error}"))?;
    let input = SppProofInputUtxo::new(
        Utxo {
            owner: sender.signing_pubkey(),
            asset: SOL_MINT,
            amount: request.input_lamports,
            blinding,
            ring_program_id: None,
            data: Data::default(),
        },
        &sender,
    );
    let mut transfer =
        ConfidentialTransfer::new(sender_address, vec![input], payer).with_compact_change();
    transfer
        .send(&recipient, SOL_MINT, request.transfer_lamports)
        .map_err(|error| error.to_string())?;
    let proof_inputs = transfer
        .sign(&sender, &AssetRegistry::default())
        .map_err(|error| error.to_string())?;
    let shape = proof_inputs
        .check_shape()
        .map_err(|error| error.to_string())?;
    let first_nullifier = proof_inputs.input_utxos[0]
        .nullifier()
        .map_err(|error| error.to_string())?;
    let external_data_hash = proof_inputs
        .external_data
        .hash()
        .map_err(|error| error.to_string())?;
    let sender_text = sender_address.to_string();
    let outputs = proof_inputs
        .output_utxos
        .iter()
        .map(|output| {
            let owner = output
                .owner_address
                .map(|address| address.to_string())
                .unwrap_or_default();
            let commitment = output.hash().map_err(|error| error.to_string())?;
            Ok(TransferDraftOutput {
                is_change: output.owner_address == Some(sender_address),
                is_dummy: output.is_dummy(),
                owner,
                lamports: output.amount,
                commitment_hex: hex(&commitment),
            })
        })
        .collect::<Result<Vec<_>, String>>()?;

    Ok(TransferDraft {
        sender: sender_text,
        recipient: recipient.to_string(),
        input_lamports: request.input_lamports,
        transfer_lamports: request.transfer_lamports,
        change_lamports: request.input_lamports - request.transfer_lamports,
        shape: format!("{}→{}", shape.n_inputs(), shape.n_outputs()),
        first_nullifier_hex: hex(&first_nullifier),
        external_data_hash_hex: hex(&external_data_hash),
        outputs,
    })
}

fn init_gnark() -> Result<(), String> {
    GNARK_INIT
        .get_or_init(|| rust_gnark::init().map_err(|_| "prover_init_failed".to_string()))
        .clone()
}

fn sibling_asset(proving_key: &Path, extension: &str) -> Result<String, String> {
    let path = proving_key.with_extension(extension);
    if !path.is_file() {
        return Err("prover_asset_missing".to_string());
    }
    Ok(path.display().to_string())
}

fn keypair_from_seed(seed: &[u8]) -> Result<ShieldedKeypair, String> {
    let bytes =
        Zeroizing::new(<[u8; 32]>::try_from(seed).map_err(|_| "invalid_seed_length".to_string())?);
    ShieldedKeypair::from_keypair(SigningKey::from_ed25519_bytes(&bytes))
        .map_err(|error| error.to_string())
}

fn elapsed_ms(started: Instant) -> u64 {
    started.elapsed().as_millis().try_into().unwrap_or(u64::MAX)
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prepares_compact_partial_transfer() {
        let recipient = shielded_address(vec![8; 32]).unwrap();
        let draft = prepare_transfer(TransferDraftRequest {
            sender_seed: vec![7; 32],
            recipient,
            input_lamports: 10,
            transfer_lamports: 4,
        })
        .unwrap();

        assert_eq!(draft.shape, "1→2");
        assert_eq!(draft.change_lamports, 6);
        assert_eq!(draft.outputs.len(), 2);
        assert!(draft.outputs[0].is_change);
        assert!(!draft.outputs[1].is_change);
    }

    #[test]
    fn transfer_drafts_do_not_reuse_blinding() {
        let recipient = shielded_address(vec![8; 32]).unwrap();
        let request = || TransferDraftRequest {
            sender_seed: vec![7; 32],
            recipient: recipient.clone(),
            input_lamports: 10,
            transfer_lamports: 4,
        };

        let first = prepare_transfer(request()).unwrap();
        let second = prepare_transfer(request()).unwrap();

        assert_ne!(first.first_nullifier_hex, second.first_nullifier_hex);
        assert_ne!(
            first.outputs[0].commitment_hex,
            second.outputs[0].commitment_hex
        );
    }

    #[test]
    fn poseidon_binding_validates_inputs() {
        assert!(poseidon_hash(vec![]).is_err());
        assert!(poseidon_hash(vec![vec![0; 31]]).is_err());
        assert_eq!(poseidon_hash(vec![vec![0; 32]]).unwrap().len(), 32);
    }

    #[test]
    #[ignore = "requires local proving fixtures"]
    fn proves_and_verifies_staged_mopro_witness() {
        let repository = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../..");
        let proving_key = std::env::var("ZOLANA_PROVING_KEY_PATH").unwrap_or_else(|_| {
            repository
                .join("packages/zolana_mobile/example/assets/proving/transfer_confidential_2_3.pk")
                .display()
                .to_string()
        });
        let assignment = std::env::var("ZOLANA_ASSIGNMENT_PATH").unwrap_or_else(|_| {
            repository
                .join("fixtures/witness-2x3.json")
                .display()
                .to_string()
        });
        let result = prove_assignment(proving_key.clone(), assignment.clone()).unwrap();

        assert!(result.verified);
        assert_eq!((result.inputs, result.outputs), (2, 3));

        let r1cs = Path::new(&proving_key)
            .with_extension("r1cs")
            .display()
            .to_string();
        let vk = Path::new(&proving_key)
            .with_extension("vk")
            .display()
            .to_string();
        let prepared = load_prover(r1cs.clone(), proving_key.clone(), vk.clone()).unwrap();
        assert_eq!(
            load_prover(r1cs, proving_key, vk).unwrap_err(),
            "prover_busy"
        );
        let witness = fs::read_to_string(assignment).unwrap();
        let flat_result = prove_prepared(prepared.id, witness, false).unwrap();
        assert!(flat_result.verified);
        let request =
            fs::read_to_string(repository.join("fixtures/prove-request-2x3.json")).unwrap();
        let structured = prove_prepared(prepared.id, request, true).unwrap();
        assert!(structured.verified);
        assert_eq!((structured.inputs, structured.outputs), (2, 3));
        let json: serde_json::Value = serde_json::from_str(&structured.proof_json).unwrap();
        assert!(json["ar"].is_array());
        assert!(json["bs"].is_array());
        assert!(json["krs"].is_array());
        let error = prove_prepared(
            prepared.id,
            "{\"Secret\":\"private-sentinel\"}".to_string(),
            false,
        )
        .unwrap_err();
        assert_eq!(error, "proof_failed");
        release_prover(prepared.id).unwrap();
        assert_eq!(
            prove_prepared(prepared.id, "{}".to_string(), false).unwrap_err(),
            "prover_closed"
        );
        assert_eq!(release_prover(prepared.id).unwrap_err(), "prover_closed");
    }
}
