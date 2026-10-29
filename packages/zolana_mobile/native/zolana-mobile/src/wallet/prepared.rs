//! Unsigned messages, reservations and prepared transaction lifetime.
use super::{
    cache::CachedOutput,
    types::{Operation, OutputOpening},
    util::invalid,
    MobileWallet,
};
use crate::error::WalletError;
use hmac::{Hmac, Mac};
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use solana_instruction::Instruction;
use solana_message::VersionedMessage;
use std::collections::{HashMap, HashSet};
use zolana_client::{compile_message, ClientError, ComputeBudgetConfig, Rpc, UnsignedMessage};

/// A transaction the wallet built and, where needed, proved, awaiting
/// signatures: each of [`Self::signers`] signs [`Self::message`] with
/// Ed25519, in order. [`Self::operation`] and [`Self::fee_payer`] are what to
/// show the user before signing.
pub struct PreparedTransaction {
    /// Unique per wallet: the key of the notes a spend reserves.
    pub(super) id: u64,
    pub(super) operation: Operation,
    pub(super) message: VersionedMessage,
    pub(super) last_valid_block_height: u64,
    /// Nullifiers of the notes it spends; empty unless it is a spend.
    pub(super) spends: Vec<[u8; 32]>,
    /// Output slots of a spend, in proof order. Empty for any other transaction.
    pub(super) outputs: Vec<OutputOpening>,
    pub(super) cached_output: Option<CachedOutput>,
    pub(super) cache_inputs: Vec<CachedOutput>,
}

impl PreparedTransaction {
    /// What it does, with what it moves and to whom.
    #[flutter_rust_bridge::frb(sync, getter)]
    pub fn operation(&self) -> Operation {
        self.operation.clone()
    }

    /// The base58 account that pays the network fee: the first signer.
    #[flutter_rust_bridge::frb(sync, getter)]
    pub fn fee_payer(&self) -> String {
        self.message.static_account_keys()[0].to_string()
    }

    /// The last block height at which the message can still land. Past it,
    /// `refresh` gives it a new blockhash without a new proof.
    #[flutter_rust_bridge::frb(sync, getter)]
    pub fn last_valid_block_height(&self) -> u64 {
        self.last_valid_block_height
    }

    /// The serialized Solana message every signer signs with Ed25519.
    #[flutter_rust_bridge::frb(sync, getter)]
    pub fn message(&self) -> Vec<u8> {
        self.message.serialize()
    }

    /// Total priority fee in lamports, added to the base transaction fee.
    /// Read from the prepared message header; zero when unset.
    #[flutter_rust_bridge::frb(sync, getter)]
    pub fn priority_fee_lamports(&self) -> u64 {
        match &self.message {
            VersionedMessage::V1(message) => message.config.priority_fee.unwrap_or(0),
            _ => 0,
        }
    }

    /// Base58 public keys that must sign, in signature order. The first is
    /// the fee payer, whose signature is the transaction signature.
    #[flutter_rust_bridge::frb(sync, getter)]
    pub fn signers(&self) -> Vec<String> {
        let required = usize::from(self.message.header().num_required_signatures);
        self.message.static_account_keys()[..required]
            .iter()
            .map(ToString::to_string)
            .collect()
    }

    /// Predicted output of a cache-targeted merge; persist with the transaction.
    #[flutter_rust_bridge::frb(sync, getter)]
    pub fn cached_output(&self) -> Option<CachedOutput> {
        self.cached_output.clone()
    }

    /// Output slots of a spend, in proof order, including padding. Empty
    /// unless this is a spend. Do not log them.
    #[flutter_rust_bridge::frb(sync, getter)]
    pub fn outputs(&self) -> Vec<OutputOpening> {
        self.outputs.clone()
    }
}

/// Notes reserved by prepared spends. The next spend selects other notes
/// until a spend's outcome is known, it is released, or it is past its
/// `last_valid_block_height`, when it can no longer land.
#[derive(Default)]
pub(super) struct Reservations {
    pub(super) next_id: u64,
    pub(super) reserved: HashMap<u64, Reservation>,
}

pub(super) struct Reservation {
    pub(super) message: Vec<u8>,
    pub(super) spends: Vec<[u8; 32]>,
    /// Exclude the cached output's tree copy from later merge rounds.
    pub(super) cached_output: Option<[u8; 32]>,
    pub(super) until: u64,
}

impl MobileWallet {
    /// Apply the call's fee or the wallet default before signing. The header
    /// is not part of the proof, so this also works after proving.
    pub(super) fn with_priority_fee(
        &self,
        mut message: VersionedMessage,
        priority_fee_lamports: Option<u64>,
    ) -> Result<VersionedMessage, WalletError> {
        let Some(fee) = priority_fee_lamports.or(self.priority_fee) else {
            return Ok(message);
        };
        let VersionedMessage::V1(v1) = &mut message else {
            return Err(ClientError::UnsupportedTransactionVersion.into());
        };
        v1.config.priority_fee = Some(fee);
        Ok(message)
    }
    /// A message with `instructions`, paid by this account. Its blockhash is
    /// set by [`Self::pending`].
    pub(super) fn message(
        &self,
        instructions: &[Instruction],
    ) -> Result<VersionedMessage, WalletError> {
        Ok(compile_message(
            &self.owner,
            instructions,
            Default::default(),
            ComputeBudgetConfig::for_instruction_count(instructions.len()),
        )?)
    }
    /// `message` with the latest blockhash, set last so that it is as young
    /// as it can be when the application gets it.
    pub(super) fn pending(
        &self,
        operation: Operation,
        mut message: VersionedMessage,
    ) -> Result<PreparedTransaction, WalletError> {
        let (blockhash, last_valid_block_height) = self.client.get_latest_blockhash()?;
        message.set_recent_blockhash(blockhash);
        Ok(self.proved(
            operation,
            UnsignedMessage {
                message,
                last_valid_block_height,
            },
        ))
    }
    /// A message the SDK proved and built with the latest blockhash, fetched
    /// after proving.
    pub(super) fn proved(
        &self,
        operation: Operation,
        unsigned: UnsignedMessage,
    ) -> PreparedTransaction {
        let mut reservations = self.reservations();
        let id = reservations.next_id;
        reservations.next_id += 1;
        PreparedTransaction {
            id,
            operation,
            message: unsigned.message,
            last_valid_block_height: unsigned.last_valid_block_height,
            spends: Vec::new(),
            outputs: Vec::new(),
            cached_output: None,
            cache_inputs: Vec::new(),
        }
    }
    /// `pending`, reserving the notes with `spends` as nullifiers until it
    /// can no longer land.
    pub(super) fn reserve(
        &self,
        pending: PreparedTransaction,
        spends: Vec<[u8; 32]>,
    ) -> Result<PreparedTransaction, WalletError> {
        let until = if pending.cached_output.is_some() || !pending.cache_inputs.is_empty() {
            u64::MAX
        } else {
            pending.last_valid_block_height
        };
        self.reserve_until(pending, spends, until)
    }
    pub(super) fn reserve_until(
        &self,
        mut pending: PreparedTransaction,
        spends: Vec<[u8; 32]>,
        until: u64,
    ) -> Result<PreparedTransaction, WalletError> {
        let cached_output = pending
            .cached_output
            .as_ref()
            .map(|output| self.cached_note(output).map(|note| note.nullifier))
            .transpose()?;
        for output in &pending.cache_inputs {
            self.cached_note(output)?;
        }
        let message = pending.message();
        let mut reservations = self.reservations();
        // Restoring the same message is idempotent. Sharing inputs does not
        // make two different transactions the same pending submission.
        if let Some((&id, _)) = reservations
            .reserved
            .iter()
            .find(|(_, held)| held.message == message)
        {
            pending.id = id;
        }
        reservations.reserved.insert(
            pending.id,
            Reservation {
                message,
                spends: spends.clone(),
                cached_output,
                until,
            },
        );
        pending.spends = spends;
        Ok(pending)
    }
    /// Release the notes `pending` reserves, so the next spend can select
    /// them: for a prepared spend that will not be sent, such as one the user
    /// declined. [`Self::submit`] and [`Self::confirm`] do this once the
    /// outcome is known.
    pub fn release(&self, pending: &PreparedTransaction) {
        self.reservations().reserved.remove(&pending.id);
    }
    pub(super) fn reservations(&self) -> std::sync::MutexGuard<'_, Reservations> {
        self.reservations
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }
    /// The nullifiers of the notes prepared spends and merges reserve.
    /// Reservations past their last valid block height are dropped: those
    /// transactions can no longer land.
    pub(super) fn reserved(&self) -> Result<HashSet<[u8; 32]>, WalletError> {
        {
            let reservations = self.reservations();
            if reservations.reserved.is_empty() {
                return Ok(HashSet::new());
            }
        }
        let height = self.client.get_block_height()?;
        let mut reservations = self.reservations();
        reservations.reserved.retain(|_, held| held.until >= height);
        Ok(reservations
            .reserved
            .values()
            .flat_map(|held| held.spends.iter().copied().chain(held.cached_output))
            .collect())
    }
    /// `pending` with a new blockhash and the same proof, for an approval that
    /// outlived [`PreparedTransaction::last_valid_block_height`]. Signatures
    /// over the old message do not apply to the new one.
    ///
    /// The proof stays valid while the trees still hold the roots it was built
    /// on: a state tree keeps its last 500 roots, about its last 500
    /// transactions, and a nullifier tree its last 100 batch roots. After that
    /// the program rejects the proof as stale, and the spend is prepared again.
    ///
    /// A refreshed spend reserves its notes again until its new last valid
    /// block height.
    pub fn refresh(
        &self,
        pending: &PreparedTransaction,
    ) -> Result<PreparedTransaction, WalletError> {
        self.check_cached_transaction(pending)?;
        let mut refreshed = self.pending(pending.operation.clone(), pending.message.clone())?;
        refreshed.cached_output = pending.cached_output.clone();
        refreshed.cache_inputs = pending.cache_inputs.clone();
        refreshed.outputs = pending.outputs.clone();
        if !pending.spends.is_empty() {
            refreshed = self.reserve(refreshed, pending.spends.clone())?;
        }
        if refreshed.id != pending.id {
            self.release(pending);
        }
        Ok(refreshed)
    }
}

impl MobileWallet {
    /// Versioned, wallet-authenticated snapshot. Contains private amounts and
    /// blindings: store securely with the run and submission signatures.
    pub fn export_prepared_transaction(
        &self,
        pending: &PreparedTransaction,
    ) -> Result<String, WalletError> {
        let state = SavedTransaction {
            version: 2,
            operation: pending.operation.clone(),
            message: pending.message.serialize(),
            last_valid_block_height: pending.last_valid_block_height,
            spends: pending.spends.clone(),
            outputs: pending.outputs.clone(),
            cached_output: pending.cached_output.clone(),
            cache_inputs: pending.cache_inputs.clone(),
        };
        let payload = serde_json::to_string(&state)
            .map_err(|_| invalid("cannot serialize prepared transaction"))?;
        let mut mac = self.state_mac();
        mac.update(payload.as_bytes());
        serde_json::to_string(&Envelope {
            payload,
            tag: mac.finalize().into_bytes().to_vec(),
        })
        .map_err(|_| invalid("cannot serialize prepared transaction"))
    }

    /// Restore before allowing new spends. Restored reservations stay until
    /// explicitly released or the transaction outcome is settled; expiry alone
    /// does not establish that an interrupted submission failed.
    pub fn restore_prepared_transaction(
        &self,
        snapshot: String,
    ) -> Result<PreparedTransaction, WalletError> {
        if snapshot.len() > 1_000_000 {
            return Err(invalid("snapshot too large"));
        }
        let envelope: Envelope =
            serde_json::from_str(&snapshot).map_err(|_| invalid("invalid snapshot"))?;
        let mut mac = self.state_mac();
        mac.update(envelope.payload.as_bytes());
        mac.verify_slice(&envelope.tag)
            .map_err(|_| invalid("snapshot authentication failed"))?;
        let state: SavedTransaction =
            serde_json::from_str(&envelope.payload).map_err(|_| invalid("invalid snapshot"))?;
        if state.version != 2 {
            return Err(invalid("unsupported snapshot version"));
        }
        let message: VersionedMessage = wincode::deserialize_exact(&state.message)
            .map_err(|_| invalid("invalid saved message"))?;
        let mut pending = self.proved(
            state.operation,
            UnsignedMessage {
                message,
                last_valid_block_height: state.last_valid_block_height,
            },
        );
        pending.outputs = state.outputs;
        pending.cached_output = state.cached_output;
        pending.cache_inputs = state.cache_inputs;
        self.reserve_until(pending, state.spends, u64::MAX)
    }

    fn state_mac(&self) -> Hmac<Sha256> {
        let mut mac = Hmac::<Sha256>::new_from_slice(self.nullifier_key.secret().as_ref())
            .expect("HMAC accepts all key lengths");
        mac.update(b"zolana-mobile/prepared/v2/8ae430a1");
        mac.update(&self.owner.to_bytes());
        mac.update(self.recovery_domain.as_bytes());
        mac
    }
}

#[derive(Serialize, Deserialize)]
struct Envelope {
    payload: String,
    tag: Vec<u8>,
}
#[derive(Serialize, Deserialize)]
struct SavedTransaction {
    version: u8,
    operation: Operation,
    message: Vec<u8>,
    last_valid_block_height: u64,
    spends: Vec<[u8; 32]>,
    outputs: Vec<OutputOpening>,
    cached_output: Option<CachedOutput>,
    cache_inputs: Vec<CachedOutput>,
}
