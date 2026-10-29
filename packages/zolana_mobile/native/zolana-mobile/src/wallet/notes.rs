//! Public note descriptions and exact input selection. Private openings stay
//! in the wallet; the application refers to notes by their commitments.
use super::{util::invalid, MobileWallet};
use crate::{asset::mint_name, error::WalletError};
use std::collections::{HashMap, HashSet};
use zolana_transaction::WalletUtxo;

/// An unreserved, nonzero default-ring note from a confirmed indexer scan.
/// This description does not reserve the note or guarantee future availability.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SpendableNote {
    /// The published commitment as 64 lowercase hex characters. Stable across
    /// scans and restarts of the same wallet on the same deployment.
    pub id: String,
    /// `None` for SOL; otherwise the SPL mint's base58 address.
    pub mint: Option<String>,
    /// Integer base units, never a floating-point token amount.
    pub amount: u64,
    pub tree_id: u16,
    /// Merges require plain notes, without attached data or ring bindings.
    pub mergeable: bool,
}

impl MobileWallet {
    /// Read spendable notes now, excluding this wallet's reservations (including
    /// predicted cache outputs). As with balances, unresolved assets are omitted.
    /// Restore pending transactions before listing notes after a restart.
    pub fn spendable_notes(&mut self) -> Result<Vec<SpendableNote>, WalletError> {
        let spendable = self.spendable()?;
        Ok(describe_notes(spendable.utxos(), &self.reserved()?))
    }
}

fn describe_notes<'a>(
    notes: impl Iterator<Item = &'a WalletUtxo>,
    reserved: &HashSet<[u8; 32]>,
) -> Vec<SpendableNote> {
    notes
        .filter(|note| {
            note.utxo.amount > 0
                && note.is_default_ring_spendable()
                && !reserved.contains(&note.nullifier)
        })
        .map(|note| SpendableNote {
            id: hex::encode(note.utxo_hash),
            mint: mint_name(&note.utxo.asset.asset),
            amount: note.utxo.amount,
            tree_id: note.tree_id,
            mergeable: note.is_plain(),
        })
        .collect()
}

// CreateCache + MergeTransact fits at most 51 real inputs in the pinned v1
// transaction. The compiled transaction is also checked before returning it.
pub(super) const MAX_CACHED_MERGE_INPUTS: usize = 51;

pub(super) fn merge_input_ids(ids: &[String]) -> Result<Vec<[u8; 32]>, WalletError> {
    if !(2..=MAX_CACHED_MERGE_INPUTS).contains(&ids.len()) {
        return Err(invalid("cached merge requires 2 to 51 inputIds"));
    }
    let mut seen = HashSet::new();
    ids.iter()
        .map(|id| {
            let mut hash = [0; 32];
            if id.bytes().any(|byte| byte.is_ascii_uppercase())
                || hex::decode_to_slice(id, &mut hash).is_err()
            {
                return Err(invalid(
                    "note ID must be a 32-byte lowercase hex commitment",
                ));
            }
            if !seen.insert(hash) {
                return Err(invalid("duplicate merge input ID"));
            }
            Ok(hash)
        })
        .collect()
}

pub(super) fn select_merge_inputs<'a>(
    notes: impl Iterator<Item = &'a WalletUtxo>,
    ids: &[[u8; 32]],
    tree_id: u16,
    reserved: &HashSet<[u8; 32]>,
) -> Result<Vec<WalletUtxo>, WalletError> {
    let by_hash: HashMap<_, _> = notes.map(|note| (note.utxo_hash, note)).collect();
    ids.iter()
        .map(|id| {
            let note = by_hash
                .get(id)
                .ok_or_else(|| invalid("merge input is no longer available"))?;
            if reserved.contains(&note.nullifier) {
                return Err(WalletError::NotesReserved {
                    amount: note.utxo.amount,
                });
            }
            if note.tree_id != tree_id {
                return Err(invalid("merge input is not on the cache target tree"));
            }
            if note.utxo.amount == 0 || !note.is_plain() {
                return Err(invalid("merge input must be a nonzero plain note"));
            }
            Ok((*note).clone())
        })
        .collect()
}

#[cfg(test)]
mod tests;
