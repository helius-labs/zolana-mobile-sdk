use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        Mutex, PoisonError,
    },
    time::Instant,
};

use zeroize::Zeroizing;

use zolana_hasher::{Hasher, Poseidon};

// On Apple's systems a freed large block stays charged to the app with the
// system allocator; zopro's gives it back (a key's first load left 200 MB
// charged on an iPad). Elsewhere it is the system allocator.
#[global_allocator]
static ALLOCATOR: zopro_zolana::zopro_core::alloc::Allocator = zopro_zolana::zopro_core::alloc::Allocator;

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

static PREPARED: Mutex<ProverState> = Mutex::new(ProverState {
    next_id: 1,
    loaded: None,
});

/// The one proving key loaded at a time: a phone has room for one.
struct ProverState {
    next_id: u64,
    loaded: Option<Loaded>,
}

struct Loaded {
    id: u64,
    /// The lockfile key [`prover::NativeProver`] loaded, or `None` for a key
    /// the application loaded itself as a [`LocalProver`]. The native prover
    /// switches keys only in the first case.
    key: Option<String>,
    prover: zopro_zolana::Prover,
}

impl ProverState {
    fn next_id(&mut self) -> Result<u64, WalletError> {
        let id = self.next_id;
        self.next_id = id.checked_add(1).ok_or(WalletError::ProverUnavailable)?;
        Ok(id)
    }
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

/// A proof by a [`LocalProver`] and where its time went, in milliseconds.
#[derive(Debug, Clone)]
pub struct LocalProofResult {
    pub proof_json: String,
    /// Every proof is verified before it is returned.
    pub verified: bool,
    pub inputs: u32,
    pub outputs: u32,
    pub proof_ms: u64,
    pub witness_ms: u64,
    pub verify_ms: u64,
    pub total_ms: u64,
}

/// A zopro key the application loaded from a file, to prove structured
/// Zolana `/prove` requests with on the device, such as for a benchmark.
/// One key is loaded at a time: while this one is, the wallet's own proofs
/// fail with `prover_busy`, so close it before the wallet proves.
pub struct LocalProver {
    id: u64,
    load_ms: u64,
    closed: AtomicBool,
}

impl LocalProver {
    /// Load the zopro key at `key_path`. Fails with `prover_busy` while
    /// another key is loaded or proving.
    pub fn load(key_path: String) -> Result<LocalProver, WalletError> {
        let started = Instant::now();
        let mut state = PREPARED.try_lock().map_err(|_| WalletError::ProverBusy)?;
        if state.loaded.is_some() {
            return Err(WalletError::ProverBusy);
        }
        let id = state.next_id()?;
        let prover = prover::load_key(std::path::Path::new(&key_path))?;
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

    /// How long loading the key took.
    #[flutter_rust_bridge::frb(sync, getter)]
    pub fn load_ms(&self) -> u64 {
        self.load_ms
    }

    /// Prove `request_json`, a Zolana `/prove` request; the proof is
    /// verified. One proof at a time: another fails with `prover_busy`.
    /// Fails with `prover_closed` after `close`, also when the close came
    /// while this proof ran.
    pub fn prove(&self, request_json: String) -> Result<LocalProofResult, WalletError> {
        let request_json = Zeroizing::new(request_json);
        let started = Instant::now();
        if self.closed.load(Ordering::Acquire) {
            return Err(WalletError::ProverClosed);
        }
        let mut state = PREPARED.try_lock().map_err(|_| WalletError::ProverBusy)?;
        let loaded = state
            .loaded
            .as_mut()
            .filter(|loaded| loaded.id == self.id)
            .ok_or(WalletError::ProverClosed)?;
        let (inputs, outputs) = loaded.prover.shape();
        let proof = loaded.prover.prove(&request_json);
        if self.closed.load(Ordering::Acquire) {
            state.loaded = None;
            return Err(WalletError::ProverClosed);
        }
        let proof = proof.map_err(|error| match error {
            zopro_zolana::Error::Unsupported | zopro_zolana::Error::Shape => {
                WalletError::UnsupportedCircuit
            }
            _ => WalletError::ProofFailed,
        })?;
        Ok(LocalProofResult {
            proof_json: proof.proof_json.to_string(),
            verified: true,
            inputs,
            outputs,
            proof_ms: proof.prove_ms,
            witness_ms: proof.witness_ms,
            verify_ms: proof.verify_ms,
            total_ms: elapsed_ms(started),
        })
    }

    /// Release the key, after the proof in progress, which then fails with
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
}
