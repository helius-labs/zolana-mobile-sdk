//! The wallet's history as the Zolana SDK reads and classifies it, in the
//! types the bindings carry.

use zolana_client::{ClientError, Rpc, SpendableUtxos};
use zolana_transaction::{AssetRegistry, HistoryEntry, HistoryKind, ShieldedKeys};

use crate::asset::mint_name;

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
}

/// The history of `keys`, newest first: one entry per asset a transaction
/// moved.
pub(crate) fn fetch<K: ShieldedKeys + ?Sized, I: Rpc + ?Sized>(
    keys: &K,
    assets: &AssetRegistry,
    indexer: &I,
) -> Result<Vec<ActivityEntry>, ClientError> {
    let history = SpendableUtxos::new(keys, assets).fetch_history(indexer)?;
    Ok(history.entries().into_iter().map(activity_entry).collect())
}

fn activity_entry(entry: HistoryEntry) -> ActivityEntry {
    ActivityEntry {
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
    }
}
