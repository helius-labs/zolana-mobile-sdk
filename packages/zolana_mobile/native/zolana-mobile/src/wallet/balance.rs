//! Balance views, device input limits and short-lived note reads.
use super::{
    spend::{max_spend_inputs, spend_failure, within_cap},
    MobileWallet,
};
use crate::{
    activity::{self, ActivityEntry},
    asset::{mint_name, Asset, Assets},
    error::WalletError,
};
use std::{cmp::Reverse, collections::HashSet, time::Instant};
use zolana_client::{IndexerPollConfig, IndexerRpcConfig, Rpc, UtxoScan, UtxoScanResult};
use zolana_interface::MAX_INPUT_TREES;
use zolana_transaction::{
    select_spend_excluding, AssetBalance, SpendableDecryptionResult, WalletUtxo,
};

/// The private balances, one per asset held, and the history, newest first.
/// Notes in [`Self::unresolved_assets`] are excluded from both.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WalletOverview {
    pub balances: Vec<TokenBalance>,
    pub activity: Vec<ActivityEntry>,
    pub unresolved_assets: UnresolvedAssets,
}

/// Assets of wallet notes that could not be resolved after refreshing the
/// pool's registrations. Their notes are excluded from balances and activity.
/// This also includes mints excluded by [`super::WalletConfig::mints`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UnresolvedAssets {
    /// Unresolved asset ids, sorted and deduplicated. These are supplied by
    /// senders and may not correspond to a registered asset.
    pub asset_ids: Vec<u64>,
    /// Base58 mints from deposits or merge outputs with no known asset id.
    pub mints: Vec<String>,
}

/// A spendable private balance. `mint` is `None` for SOL.
///
/// `amount` is what the indexer has confirmed for this wallet, `reserved`
/// included: a payment or change on its way appears once its transaction is
/// confirmed and indexed. Reservations live in this open wallet's memory
/// only.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TokenBalance {
    pub mint: Option<String>,
    pub amount: u64,
    /// How many spendable notes the amount is in: what a spend of all of it
    /// takes, and what a merge reduces.
    pub notes: u64,
    /// The part of `amount` in notes this wallet's prepared spends hold.
    pub reserved: u64,
    /// One entry per proof width the pinned shapes offer (2, 4, 8 and 16
    /// outputs), ascending. A spend with `k` private outputs proves with
    /// `k + 1` outputs, its change included, and public outputs take no
    /// slot: a withdrawal, a withdrawal with one private fee, or a send to
    /// one wallet reads the entry with `outputs: 2`, a send to one wallet
    /// with a fee the entry with `outputs: 4`. The limits include any
    /// application fee the spend pays.
    pub limits: Vec<SpendLimit>,
}

/// The most one spend of up to `outputs` outputs, change included, can take
/// now without a merge: from the unreserved notes, largest first, within the
/// note cap of that width and two trees.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SpendLimit {
    pub outputs: u64,
    pub amount: u64,
}

/// `balance` with what prepared spends reserve of it and its limit per proof
/// width, each capped at `configured_cap` notes.
pub(super) fn token_balance(
    balance: &AssetBalance,
    reserved: &HashSet<[u8; 32]>,
    configured_cap: usize,
) -> TokenBalance {
    TokenBalance {
        mint: mint_name(&balance.mint),
        amount: balance.amount,
        notes: balance.utxos.len() as u64,
        reserved: balance
            .utxos
            .iter()
            .filter(|note| reserved.contains(&note.nullifier))
            .map(|note| note.utxo.amount)
            .sum(),
        limits: proof_widths()
            .into_iter()
            .map(|outputs| SpendLimit {
                outputs: outputs as u64,
                amount: spend_limit(
                    &balance.utxos,
                    reserved,
                    configured_cap.min(max_spend_inputs(outputs)),
                ),
            })
            .collect(),
    }
}

/// The distinct output counts of the pinned shapes, ascending.
pub(super) fn proof_widths() -> Vec<usize> {
    let mut widths: Vec<usize> = zolana_client::SPP_SUPPORTED_SHAPES
        .iter()
        .map(|shape| shape.n_outputs())
        .collect();
    widths.sort_unstable();
    widths.dedup();
    widths
}

/// The most a spend of up to `cap` notes can take from `notes`, those in
/// `reserved` left out, as selection would take them: the largest first,
/// stopping before a note past the cap or on a third tree. Every amount up
/// to it selects; one more fails. The candidates are the Zolana SDK's, whose
/// filter is not public: positive notes the default-ring circuit spends.
pub(super) fn spend_limit(notes: &[WalletUtxo], reserved: &HashSet<[u8; 32]>, cap: usize) -> u64 {
    let mut candidates: Vec<&WalletUtxo> = notes
        .iter()
        .filter(|note| {
            note.utxo.amount > 0
                && note.is_default_ring_spendable()
                && !reserved.contains(&note.nullifier)
        })
        .collect();
    candidates.sort_by_key(|note| Reverse(note.utxo.amount));
    let mut trees: Vec<u16> = Vec::with_capacity(MAX_INPUT_TREES);
    let mut limit = 0u64;
    for note in candidates.into_iter().take(cap) {
        if !trees.contains(&note.tree_id()) {
            if trees.len() == MAX_INPUT_TREES {
                break;
            }
            trees.push(note.tree_id());
        }
        limit = limit.saturating_add(note.utxo.amount);
    }
    limit
}

/// A read of the wallet's notes and history, and when it finished.
pub(super) struct RecentRead {
    pub(super) at: Instant,
    pub(super) scan: UtxoScanResult,
}

impl MobileWallet {
    /// Spendable private balances, read from the indexer now: one per asset
    /// held in SOL and the mints the wallet knows, each with what prepared
    /// spends reserve of it and the most one spend can take per proof width.
    pub fn balances(&mut self) -> Result<Vec<TokenBalance>, WalletError> {
        let spendable = self.spendable()?;
        self.token_balances(&spendable)
    }
    /// [`Self::balances`] and [`Self::activity`] from one read of the
    /// indexer, and the assets of notes that could not be included in either.
    pub fn overview(&mut self) -> Result<WalletOverview, WalletError> {
        let scan = self.scan()?;
        Ok(WalletOverview {
            balances: self.token_balances(&scan.spendable)?,
            activity: activity::entries(&scan.history, &scan.asset_registry)?,
            unresolved_assets: UnresolvedAssets {
                asset_ids: scan.history.unknown_asset_ids.iter().copied().collect(),
                mints: scan
                    .history
                    .unknown_mints
                    .iter()
                    .map(ToString::to_string)
                    .collect(),
            },
        })
    }
    /// [`TokenBalance`]s of `spendable`, against the notes prepared spends
    /// reserve now.
    pub(super) fn token_balances(
        &self,
        spendable: &SpendableDecryptionResult,
    ) -> Result<Vec<TokenBalance>, WalletError> {
        let reserved = self.reserved()?;
        Ok(spendable
            .balances
            .assets
            .iter()
            .map(|balance| token_balance(balance, &reserved, self.max_spend_inputs))
            .collect())
    }
    /// Spendable private balance of SOL (`mint` `None`) or `mint`, read from
    /// the indexer now.
    pub fn private_balance(&mut self, mint: Option<String>) -> Result<u64, WalletError> {
        // A mint the pool has not registered holds nothing: zero, not a
        // failure.
        let asset = self.assets().asset(&self.client, mint.as_deref())?;
        Ok(self
            .spendable()?
            .balances
            .get_balance(asset.mint)
            .map_or(0, |balance| balance.amount))
    }
    /// Transaction history, read from the indexer now, newest first: one
    /// entry per asset each transaction moved, in SOL and every mint
    /// [`Self::balances`] reports.
    ///
    /// The indexer does not say which spends were withdrawals: a spend whose
    /// outputs are all this wallet's own is listed as a withdrawal, one with
    /// another wallet's output as sent.
    pub fn activity(&mut self) -> Result<Vec<ActivityEntry>, WalletError> {
        let scan = self.scan()?;
        activity::entries(&scan.history, &scan.asset_registry)
    }
    /// The notes a spend of `amount` with `outputs` outputs takes, other than
    /// those prepared spends reserve: at most the configured cap and what the
    /// pinned shapes allow, [`WalletError::MergeRequired`] beyond.
    /// Reservations past their last valid block height are dropped: those
    /// spends can no longer land.
    pub(super) fn select_notes(
        &self,
        asset: Asset,
        amount: u64,
        outputs: usize,
    ) -> Result<Vec<WalletUtxo>, WalletError> {
        let spendable = self.recent_notes()?;
        let reserved = self.reserved()?;
        let inputs = select_spend_excluding(spendable.utxos(), asset.mint, amount, &reserved)
            .map_err(|failure| spend_failure(amount, failure))?;
        within_cap(
            inputs,
            amount,
            self.max_spend_inputs.min(max_spend_inputs(outputs)),
        )
    }
    /// SOL for `None`, otherwise a configured or discovered mint.
    pub(super) fn asset(&self, mint: Option<String>) -> Result<Asset, WalletError> {
        self.assets().resolve(&self.client, mint.as_deref())
    }
    pub(super) fn assets(&self) -> std::sync::MutexGuard<'_, Assets> {
        self.assets
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }
    /// The wallet's spendable notes, in SOL and every mint it knows, once
    /// the indexer has caught up with the RPC.
    pub(super) fn spendable(&self) -> Result<SpendableDecryptionResult, WalletError> {
        Ok(self.scan()?.spendable)
    }
    /// The wallet's notes and history, in SOL and every mint it knows, once
    /// the indexer has caught up with the RPC. Kept for the spends
    /// [`super::WalletConfig::reuse_notes_ms`] lets take their notes from it. A
    /// discovering wallet delegates registry lookup and re-decryption to the
    /// SDK, retaining its resolved mappings only after a successful scan.
    pub(super) fn scan(&self) -> Result<UtxoScanResult, WalletError> {
        let indexed = self.indexed()?;
        let scan = {
            let mut assets = self.assets();
            let discover = assets.discovers();
            let registry = assets.registry(&self.client)?;
            let scan = UtxoScan::new(&self.keys, registry).with_indexer_config(indexed);
            let scan = if discover {
                scan.with_asset_discovery()
            } else {
                scan
            };
            let scan = scan.fetch(&self.client)?;
            assets.remember_registry(scan.asset_registry.clone());
            scan
        };
        if !self.reuse_notes.is_zero() {
            *self.recent() = Some(RecentRead {
                at: Instant::now(),
                scan: scan.clone(),
            });
        }
        Ok(scan)
    }
    /// The notes a spend selects from: the last read's, within
    /// [`super::WalletConfig::reuse_notes_ms`] of it, or a read now.
    pub(super) fn recent_notes(&self) -> Result<SpendableDecryptionResult, WalletError> {
        if let Some(read) = self.recent().as_ref() {
            if read.at.elapsed() <= self.reuse_notes {
                return Ok(read.scan.spendable.clone());
            }
        }
        self.spendable()
    }
    /// The last read no longer stands for the chain: a transaction of this
    /// wallet went out or landed.
    pub(super) fn forget_notes(&self) {
        *self.recent() = None;
    }
    pub(super) fn recent(&self) -> std::sync::MutexGuard<'_, Option<RecentRead>> {
        self.recent
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }
    /// Indexer reads wait for the RPC's confirmed slot, the commitment Photon
    /// indexes at: a spend that slot holds, from this session or another, is
    /// read rather than selected again. An indexer still behind after
    /// [`INDEXER_CATCH_UP`] fails with [`WalletError::IndexerBehind`].
    pub(super) fn indexed(&self) -> Result<IndexerRpcConfig, WalletError> {
        Ok(IndexerRpcConfig {
            poll: INDEXER_CATCH_UP,
            require_slot: Some(self.client.get_slot()?),
        })
    }
}

/// About ten seconds for the indexer to reach the RPC's slot, asked again
/// after a tenth of a second at first: the indexer is a slot or two behind
/// at most times, and a spend waits for that, not for a stall. A stalled
/// indexer fails after the schedule.
pub(super) const INDEXER_CATCH_UP: IndexerPollConfig = IndexerPollConfig {
    num_retries: 7,
    delay_ms: 100,
    max_delay_ms: 4_000,
};
