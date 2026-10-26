//! The wallet's history as the Zolana SDK reads and classifies it, in the
//! types the bindings carry.

use zolana_transaction::{AssetRegistry, History, HistoryEntry, HistoryKind};

use crate::{asset::mint_name, error::WalletError};

/// What a history entry did, from this wallet's side.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ActivityKind {
    /// Public funds moved into the private balance.
    Deposit,
    /// A transfer from another wallet.
    Received,
    /// A transfer to another wallet.
    Sent,
    /// Private funds moved to a public account.
    Withdrawal,
    /// Notes moved within this wallet: a merge, or a transfer to itself.
    SelfTransfer,
}

/// Amounts are in base units: lamports for SOL, the mint's smallest unit
/// otherwise. `mint` is `None` for SOL. Sent and withdrawn amounts are what
/// left the private balance, change excluded; a self transfer's is what it
/// spent.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ActivityEntry {
    pub kind: ActivityKind,
    pub mint: Option<String>,
    pub amount: u64,
    pub signature: String,
    pub slot: u64,
    /// Stable asset-row index: the pool's asset id (1 for SOL), not the row's
    /// position in a list or the TS SDK's output-leaf index. Together with
    /// `signature` and `slot`, this identifies the same row across reads.
    pub index: u64,
}

/// `history` newest first: one entry per asset a transaction moved.
pub(crate) fn entries(
    history: &History,
    assets: &AssetRegistry,
) -> Result<Vec<ActivityEntry>, WalletError> {
    history
        .entries()
        .into_iter()
        .map(|entry| activity_entry(entry, assets))
        .collect()
}

pub(crate) fn activity_entry(
    entry: HistoryEntry,
    assets: &AssetRegistry,
) -> Result<ActivityEntry, WalletError> {
    let index = assets.asset_id(&entry.mint)?;
    Ok(ActivityEntry {
        kind: match entry.kind {
            HistoryKind::Deposit => ActivityKind::Deposit,
            HistoryKind::Received => ActivityKind::Received,
            HistoryKind::Sent => ActivityKind::Sent,
            HistoryKind::Withdrawal => ActivityKind::Withdrawal,
            HistoryKind::SelfTransfer => ActivityKind::SelfTransfer,
        },
        mint: mint_name(&entry.mint),
        amount: entry.amount,
        signature: entry.tx_signature.to_string(),
        slot: entry.slot,
        index,
    })
}

/// Whether a signature has a complete wallet result at the indexed slot.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TransactionLookupStatus {
    /// The complete result for configured assets. `entries` may be empty.
    Ready,
    /// The confirmed transaction's slot is ahead of the indexer. Retry later.
    PendingIndexing,
    /// Neither the indexer nor the confirmed RPC returned the transaction.
    /// This does not prove it never existed: the RPC may prune old history.
    NotFound,
    /// The transaction was found, but this SDK cannot reconstruct every
    /// wallet row from its format, ciphertexts or available RPC metadata.
    /// No partial entries are returned and no history scan is attempted.
    Incomplete,
}

/// A signature-scoped read. Network failures throw; they are never empty
/// wallet results. Only `ready` may carry entries.
#[derive(Clone, Debug, PartialEq, Eq)]
#[flutter_rust_bridge::frb(non_opaque)]
pub struct TransactionLookup {
    pub status: TransactionLookupStatus,
    pub entries: Vec<ActivityEntry>,
    pub slot: Option<u64>,
    /// Highest slot the indexer had persisted for this lookup.
    pub indexed_slot: u64,
}
