//! Signature validation, submission and outcome reconciliation.
use super::{account::signed_by, prepared::PreparedTransaction, types::Operation, MobileWallet};
use crate::error::{client_message, settles_the_send, WalletError};
use solana_commitment_config::CommitmentConfig;
use solana_signature::Signature;
use solana_transaction::versioned::VersionedTransaction;
use std::{str::FromStr, thread::sleep, time::Duration};
use zolana_client::{ClientError, IndexerPollConfig, Rpc, SolanaRpc, ZolanaClient};

impl MobileWallet {
    /// Attach the signatures, send, and wait as [`Self::confirm`] does.
    /// Returns the signature.
    ///
    /// `signatures` follows [`PreparedTransaction::signers`]; each is checked
    /// before anything is sent. A spend's notes are released once the send
    /// has an outcome: spent when it landed, free again when the chain
    /// refused it or it spent a note the chain already had. An unknown
    /// outcome does not release them. Ordinary preparation reservations expire
    /// at the last valid block height; cached and restored ones remain held.
    pub fn submit(
        &self,
        pending: &PreparedTransaction,
        signatures: Vec<Vec<u8>>,
    ) -> Result<String, WalletError> {
        let message_bytes = pending.message();
        let required = usize::from(pending.message.header().num_required_signatures);
        if signatures.len() != required {
            return Err(WalletError::SignatureCountMismatch {
                expected: required as u64,
                got: signatures.len() as u64,
            });
        }
        let mut transaction_signatures = Vec::with_capacity(required);
        for (signer, signature) in pending.message.static_account_keys()[..required]
            .iter()
            .zip(&signatures)
        {
            let signature: [u8; 64] = signature
                .as_slice()
                .try_into()
                .map_err(|_| WalletError::SignatureInvalid)?;
            if !signed_by(signer, &message_bytes, &signature) {
                return Err(WalletError::SignatureInvalid);
            }
            transaction_signatures.push(Signature::from(signature));
        }
        let signature = transaction_signatures[0];
        let transaction = VersionedTransaction {
            signatures: transaction_signatures,
            message: pending.message.clone(),
        };
        self.forget_notes();
        match self.client.process_transaction(transaction) {
            Ok(_) => {}
            // The chain refused it, in simulation or after it ran: it never
            // spent its notes.
            Err(failure) if settles_the_send(&failure) => {
                self.release(pending);
                return Err(failure.into());
            }
            // A send can fail after the transaction went out, such as when
            // the RPC loses the confirmation: look for it before failing.
            Err(_) => self.outcome(pending, signature)?,
        }
        self.settle(pending, signature, true)?;
        Ok(signature.to_string())
    }
    /// Wait for a transaction the application sent itself, as
    /// [`Self::submit`] waits for a send whose response was lost: until Solana
    /// confirms it and, for shielded-pool transactions, the indexer has it,
    /// so the next balance or spend reads the notes it created and not the
    /// ones it spent; the chain's error when the chain refused it;
    /// [`WalletError::TransactionOutcomeUnknown`] when history cannot
    /// establish its outcome, even after blockhash expiry. A spend's notes
    /// are released with every outcome but the unknown one.
    ///
    /// `signature` is the transaction signature, the fee payer's; it is
    /// checked against `pending` before anything is asked.
    pub fn confirm(
        &self,
        pending: &PreparedTransaction,
        signature: String,
    ) -> Result<(), WalletError> {
        let signature =
            Signature::from_str(&signature).map_err(|_| WalletError::SignatureInvalid)?;
        let fee_payer = &pending.message.static_account_keys()[0];
        if !signed_by(fee_payer, &pending.message(), signature.as_array()) {
            return Err(WalletError::SignatureInvalid);
        }
        self.settle(pending, signature, false)
    }
    /// Recover a shielded-pool transaction by its signature, including after
    /// an application restart. Searches RPC history and accepts a matching
    /// indexed event when RPC history is unavailable. Success waits for the
    /// indexer so balances include the transaction; confirmed chain errors
    /// are returned immediately.
    ///
    /// Missing history returns [`WalletError::TransactionOutcomeUnknown`],
    /// even after blockhash expiry: absence does not prove it never landed.
    /// `last_valid_block_height` can stop the wait early, but does not make a
    /// replacement payment safe. Keep the signature to check again.
    pub fn wait_for_transaction(
        &self,
        signature: String,
        last_valid_block_height: Option<u64>,
    ) -> Result<(), WalletError> {
        let signature =
            Signature::from_str(&signature).map_err(|_| WalletError::SignatureInvalid)?;
        self.forget_notes();
        reconcile(
            &self.client,
            signature,
            last_valid_block_height,
            self.send_lookup,
        )?;
        wait_for_indexer(&self.client, signature, self.send_lookup)
    }
    /// Wait for `pending`, sent as `signature`: a registration or a merging
    /// change until Solana confirms it, a shielded-pool transaction until its
    /// outcome is known and the indexer has it. `landed` when the send
    /// already confirmed it, so its status is not asked again.
    pub(super) fn settle(
        &self,
        pending: &PreparedTransaction,
        signature: Signature,
        landed: bool,
    ) -> Result<(), WalletError> {
        self.forget_notes();
        if landed {
            self.release(pending);
        } else {
            self.outcome(pending, signature)?;
        }
        if matches!(
            pending.operation,
            Operation::Registration | Operation::Merging { .. }
        ) {
            return Ok(());
        }
        wait_for_indexer(&self.client, signature, self.send_lookup)
    }
    /// What became of `pending`, sent as `signature`, as [`reconcile`] tells
    /// it. Every outcome but the unknown one releases its notes: spent once
    /// it landed, free again when the chain refused it or it spent a note
    /// the chain already had. An unknown outcome leaves reservations alone.
    pub(super) fn outcome(
        &self,
        pending: &PreparedTransaction,
        signature: Signature,
    ) -> Result<(), WalletError> {
        let outcome = reconcile(
            &self.client,
            signature,
            Some(pending.last_valid_block_height),
            self.send_lookup,
        );
        if !matches!(outcome, Err(WalletError::TransactionOutcomeUnknown { .. })) {
            self.release(pending);
        }
        outcome
    }
}

/// Retry schedule for confirmation and indexing after a send.
pub(super) const SEND_LOOKUP: IndexerPollConfig = IndexerPollConfig {
    num_retries: 10,
    delay_ms: 400,
    max_delay_ms: 8_000,
};

/// Recover from RPC history or a confirmed indexed event. Missing history
/// cannot establish that a transaction never landed, even after expiry.
pub(super) fn reconcile(
    client: &ZolanaClient<SolanaRpc>,
    signature: Signature,
    last_valid_block_height: Option<u64>,
    poll: IndexerPollConfig,
) -> Result<(), WalletError> {
    let mut message = String::new();
    for delay in std::iter::once(Duration::ZERO).chain(poll.backoff()) {
        sleep(delay);
        let status = client
            .rpc()
            .client()
            .get_signature_statuses_with_history(&[signature])
            .map(|response| response.value.into_iter().next().flatten());
        message = match status {
            Ok(Some(status)) if status.satisfies_commitment(CommitmentConfig::confirmed()) => {
                return match status.err {
                    None => Ok(()),
                    Some(failure) => Err(ClientError::SolanaRpcTransaction {
                        operation: "process_transaction",
                        source: failure.into(),
                    }
                    .into()),
                };
            }
            Ok(Some(_)) => "landed but not confirmed".to_string(),
            Ok(None) => "transaction not found in RPC history".to_string(),
            Err(failure) => client_message(&ClientError::Rpc(failure.to_string())),
        };
        // An indexed event is positive evidence even if the RPC has pruned
        // its history. An empty response is not evidence of failure.
        if client
            .get_shielded_transactions_by_signature(signature, None)
            .is_ok_and(|response| {
                response.transactions.iter().any(|event| {
                    event.transaction.tx_signature == signature
                        && event.transaction.slot <= response.context.slot
                })
            })
        {
            return Ok(());
        }
        if let Some(height) = last_valid_block_height {
            if client
                .get_block_height()
                .is_ok_and(|current| current > height)
            {
                break;
            }
        }
    }
    Err(WalletError::TransactionOutcomeUnknown {
        signature: signature.to_string(),
        message,
    })
}

/// Confirmation is already established. Wait only for the indexed notes;
/// checking the recent RPC cache again would lose historical successes.
pub(super) fn wait_for_indexer(
    client: &ZolanaClient<SolanaRpc>,
    signature: Signature,
    poll: IndexerPollConfig,
) -> Result<(), WalletError> {
    let mut last_error = None;
    for delay in std::iter::once(Duration::ZERO).chain(poll.backoff()) {
        sleep(delay);
        match client.get_shielded_transactions_by_signature(signature, None) {
            Ok(response)
                if response.transactions.iter().any(|event| {
                    event.transaction.tx_signature == signature
                        && event.transaction.slot <= response.context.slot
                }) =>
            {
                return Ok(())
            }
            Ok(_) => last_error = None,
            Err(failure) if client.indexer().should_retry(&failure) => {
                last_error = Some(client_message(&failure));
            }
            Err(failure) => return Err(failure.into()),
        }
    }
    Err(match last_error {
        Some(last_error) => ClientError::PollTimedOut {
            attempts: poll.num_retries.saturating_add(1),
            last_error: Some(last_error),
        },
        None => ClientError::IndexerTimeout,
    }
    .into())
}
