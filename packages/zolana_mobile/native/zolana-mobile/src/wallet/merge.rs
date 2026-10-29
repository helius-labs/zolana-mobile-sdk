//! Ordinary tree merges through the shared SDK builder.
use super::{prepared::PreparedTransaction, types::Operation, util::nullifiers, MobileWallet};
use crate::{asset::mint_name, error::WalletError, prover::Proving};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use zolana_client::MergeSubmission;
use zolana_transaction::{
    instructions::merge::{MergeTransaction, MAX_MERGE_INPUTS, MERGE_DEFAULT_INPUT_COUNT},
    select_merge,
};

impl MobileWallet {
    /// Build and prove a merge of the smallest notes of `mint` (SOL for
    /// `None`) on one tree into one: at most `max_inputs` of them, 24 when
    /// `None`, 52 at most. Notes that prepared spends reserve are left out,
    /// and the merge reserves its own. Fails with
    /// [`WalletError::NothingToMerge`] below two notes, and with
    /// [`WalletError::MergingDisabled`] unless [`Self::prepare_merging`] turned
    /// merging on.
    ///
    /// The merge needs no signature of this account: `fee_payer` pays and
    /// signs alone, this account when `None`. `proving` works as in
    /// [`Self::prepare_spend`]; a merge is proved with this wallet's
    /// nullifier secret, which a remote prover receives. The proof expires
    /// after ten minutes, so a merge cannot be sent later by whoever holds it;
    /// [`Self::refresh`] does not extend it.
    pub fn prepare_merge(
        &mut self,
        mint: Option<String>,
        max_inputs: Option<u32>,
        fee_payer: Option<String>,
        proving: Option<Proving>,
        priority_fee_lamports: Option<u64>,
    ) -> Result<PreparedTransaction, WalletError> {
        let payer = self.fee_payer(fee_payer)?;
        let proving = self.proving(proving)?;
        let asset = self.asset(mint)?;
        let reserved = self.reserved()?;
        let max_inputs = max_inputs.map_or(MERGE_DEFAULT_INPUT_COUNT, |max| {
            usize::try_from(max).unwrap_or(MAX_MERGE_INPUTS)
        });
        let inputs = select_merge(
            self.recent_notes()?.utxos(),
            asset.mint,
            max_inputs,
            &reserved,
        )?;
        let spends = nullifiers(&inputs);
        let tree_id = inputs[0].tree_id();
        let merge = MergeTransaction::new(inputs)?
            .with_output_tree_id(tree_id)
            .with_expiry(merge_expiry())
            .encrypt(&self.keys)?;
        let amount = merge.output_utxo.amount;
        let mut submission =
            MergeSubmission::new(&merge, self.owner, &self.address, &self.nullifier_key);
        if proving == Proving::Remote {
            let remote = self
                .remote_prover
                .clone()
                .ok_or(WalletError::RemoteProverMissing)?;
            submission = submission.with_prover(remote);
        }
        let mut unsigned = submission.finish_unsigned_sync(&self.client, payer)?;
        unsigned.message = self.with_priority_fee(unsigned.message, priority_fee_lamports)?;
        let pending = self.proved(
            Operation::Merge {
                mint: mint_name(&asset.mint),
                amount,
            },
            unsigned,
        );
        self.reserve(pending, spends)
    }
}

/// How long a merge proof can be sent. Anyone who holds it can send it, and
/// the program refuses it after this.
pub(super) const MERGE_LIFETIME: Duration = Duration::from_secs(600);

/// The unix time a merge prepared now expires at.
pub(super) fn merge_expiry() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .saturating_add(MERGE_LIFETIME)
        .as_secs()
}
