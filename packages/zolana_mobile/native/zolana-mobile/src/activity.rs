//! The wallet's history, read from the indexer: every transaction that
//! created or spent one of its notes, seen from this wallet's side.
//!
//! The Zolana SDK has no history API. This reads what
//! [`zolana_client::SpendableUtxos::fetch`] reads, keeps the transactions as
//! well as the notes, and classifies each transaction by the notes it moved.

use std::{
    cmp::Reverse,
    collections::{BTreeMap, BTreeSet, HashMap, HashSet},
};

use solana_pubkey::Pubkey;
use solana_signature::Signature;
use zolana_client::{ClientError, EncryptedUtxoMatch, Rpc, ShieldedTransaction};
use zolana_keypair::P256Pubkey;
use zolana_transaction::{
    verify_spendable, AssetRegistry, DecryptionResult, ShieldedKeys, WalletUtxo,
};

use crate::asset::mint_name;

const PAGE_LIMIT: u32 = 1_000;

/// What a history entry did, from this wallet's side.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ActivityKind {
    /// Public funds moved into the private balance.
    Shielded,
    /// Private funds moved to a public account.
    Unshielded,
    Sent,
    Received,
    /// Notes rearranged within this wallet: a merge, or a transfer to itself.
    Internal,
}

/// Amounts are in base units: lamports for SOL, the mint's smallest unit
/// otherwise. `mint` is `None` for SOL. Sent and unshielded amounts are what
/// left the private balance, change excluded.
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
    let (transactions, notes) = transactions_and_notes(keys, assets, indexer)?;
    Ok(entries(effects(&transactions, &notes)))
}

/// Every transaction that created or spent a note of `keys`, and those notes,
/// spent or not. The reads match `SpendableUtxos::fetch`: the transactions
/// tagged for the wallet, then the spends of each note found, until a round
/// finds nothing new.
fn transactions_and_notes<K: ShieldedKeys + ?Sized, I: Rpc + ?Sized>(
    keys: &K,
    assets: &AssetRegistry,
    indexer: &I,
) -> Result<(Vec<ShieldedTransaction>, Vec<WalletUtxo>), ClientError> {
    let address = keys.address()?;
    let mut tags = vec![address.signing_pubkey.confidential_view_tag()?];
    tags.extend(keys.viewing_public_keys().iter().map(P256Pubkey::x));

    let mut seen = HashSet::new();
    let mut transactions = Vec::new();
    let mut batch = unseen(&mut seen, tagged_transactions(indexer, &tags)?);
    let mut decrypted = DecryptionResult::default();
    let mut queried = HashSet::new();
    while !batch.is_empty() {
        decrypted.extend(keys, &batch, assets)?;
        transactions.append(&mut batch);
        let nullifiers: Vec<_> = owned_notes(keys, &decrypted)?
            .iter()
            .map(|note| note.nullifier)
            .filter(|nullifier| queried.insert(*nullifier))
            .collect();
        batch = unseen(&mut seen, spending_transactions(indexer, &nullifiers)?);
    }
    let notes = owned_notes(keys, &decrypted)?;
    Ok((transactions, notes))
}

/// The decrypted notes this wallet owns, spent or not: the ones
/// `verify_spendable` checks against their commitments, before it drops the
/// spent ones.
fn owned_notes<K: ShieldedKeys + ?Sized>(
    keys: &K,
    decrypted: &DecryptionResult,
) -> Result<Vec<WalletUtxo>, ClientError> {
    let unspent = DecryptionResult {
        spent_nullifiers: HashSet::new(),
        ..decrypted.clone()
    };
    Ok(verify_spendable(keys, &unspent)?.utxos().cloned().collect())
}

/// A proofless deposit arrives one output at a time, so its leaf is part of
/// its identity.
fn unseen(
    seen: &mut HashSet<(Signature, Option<u16>, Option<u64>)>,
    transactions: Vec<ShieldedTransaction>,
) -> Vec<ShieldedTransaction> {
    transactions
        .into_iter()
        .filter(|tx| {
            let leaf = tx
                .proofless
                .then(|| tx.output_slots.first())
                .flatten()
                .map(|slot| slot.output_context.leaf_index);
            seen.insert((tx.tx_signature, tx.event_index, leaf))
        })
        .collect()
}

/// Confidential outputs carry the owner tag, deposits a viewing key's tag.
/// Deposits come from the encrypted-output stream, one transaction per
/// output: a deposit transaction can shield several outputs.
fn tagged_transactions<I: Rpc + ?Sized>(
    indexer: &I,
    tags: &[[u8; 32]],
) -> Result<Vec<ShieldedTransaction>, ClientError> {
    let mut transactions = paged(|cursor| {
        let page = indexer.get_shielded_transactions_by_tags(
            tags.to_vec(),
            cursor,
            Some(PAGE_LIMIT),
            None,
        )?;
        Ok((page.transactions, page.next_cursor))
    })?;
    transactions.retain(|tx| !tx.proofless);
    let deposits = paged(|cursor| {
        let page =
            indexer.get_encrypted_utxos_by_tags(tags.to_vec(), cursor, Some(PAGE_LIMIT), None)?;
        Ok((page.matches, page.next_cursor))
    })?;
    transactions.extend(
        deposits
            .into_iter()
            .filter_map(EncryptedUtxoMatch::into_proofless_transaction),
    );
    Ok(transactions)
}

fn spending_transactions<I: Rpc + ?Sized>(
    indexer: &I,
    nullifiers: &[[u8; 32]],
) -> Result<Vec<ShieldedTransaction>, ClientError> {
    let mut transactions = Vec::new();
    for chunk in nullifiers.chunks(PAGE_LIMIT as usize) {
        transactions.extend(paged(|cursor| {
            let page = indexer.get_shielded_transactions_by_nullifiers(
                chunk.to_vec(),
                cursor,
                Some(PAGE_LIMIT),
                None,
            )?;
            Ok((page.transactions, page.next_cursor))
        })?);
    }
    Ok(transactions)
}

/// Every item of a paged indexer read: `page` returns one page and the
/// cursor of the next.
fn paged<T>(
    mut page: impl FnMut(Option<Vec<u8>>) -> Result<(Vec<T>, Option<Vec<u8>>), ClientError>,
) -> Result<Vec<T>, ClientError> {
    let mut items = Vec::new();
    let mut cursor = None;
    loop {
        let (page_items, next) = page(cursor)?;
        items.extend(page_items);
        let Some(next) = next else {
            return Ok(items);
        };
        cursor = Some(next);
    }
}

/// What one Solana transaction did to this wallet's notes, over all its
/// shielded-pool events.
#[derive(Debug, Default)]
struct Effect {
    slot: u64,
    deposit: bool,
    /// An output this wallet cannot read: another wallet's note.
    pays_another: bool,
    received: BTreeMap<Pubkey, u64>,
    spent: BTreeMap<Pubkey, u64>,
}

fn effects(
    transactions: &[ShieldedTransaction],
    notes: &[WalletUtxo],
) -> BTreeMap<Signature, Effect> {
    let by_nullifier: HashMap<_, _> = notes.iter().map(|note| (note.nullifier, note)).collect();
    let by_hash: HashMap<_, _> = notes.iter().map(|note| (note.utxo_hash, note)).collect();
    let mut effects: BTreeMap<Signature, Effect> = BTreeMap::new();
    for tx in transactions {
        let effect = effects.entry(tx.tx_signature).or_default();
        effect.slot = tx.slot;
        effect.deposit |= tx.proofless;
        for slot in &tx.output_slots {
            match by_hash.get(&slot.output_context.hash) {
                Some(note) => add(&mut effect.received, note),
                None => effect.pays_another = true,
            }
        }
        for note in tx.nullifiers.iter().filter_map(|n| by_nullifier.get(n)) {
            add(&mut effect.spent, note);
        }
    }
    effects
}

fn add(amounts: &mut BTreeMap<Pubkey, u64>, note: &WalletUtxo) {
    let amount = amounts.entry(note.utxo.asset.asset).or_default();
    *amount = amount.saturating_add(note.utxo.amount);
}

/// A transaction that spent none of the wallet's notes deposited or paid it.
/// One that spent some sent them to another wallet when it has an output the
/// wallet cannot read; otherwise every output is the wallet's own, and what
/// did not come back as change was withdrawn.
fn entries(effects: BTreeMap<Signature, Effect>) -> Vec<ActivityEntry> {
    let mut entries = Vec::new();
    for (signature, effect) in effects {
        let mints: BTreeSet<_> = effect.received.keys().chain(effect.spent.keys()).collect();
        for mint in mints {
            let received = effect.received.get(mint).copied().unwrap_or(0);
            let spent = effect.spent.get(mint).copied().unwrap_or(0);
            let (kind, amount) = if spent == 0 {
                let kind = if effect.deposit {
                    ActivityKind::Shielded
                } else {
                    ActivityKind::Received
                };
                (kind, received)
            } else if received >= spent {
                (ActivityKind::Internal, spent)
            } else if effect.pays_another {
                (ActivityKind::Sent, spent - received)
            } else {
                (ActivityKind::Unshielded, spent - received)
            };
            if amount > 0 {
                entries.push(ActivityEntry {
                    kind,
                    mint: mint_name(mint),
                    amount,
                    signature: signature.to_string(),
                    slot: effect.slot,
                });
            }
        }
    }
    entries.sort_by(|a, b| {
        (Reverse(a.slot), &a.signature, &a.mint).cmp(&(Reverse(b.slot), &b.signature, &b.mint))
    });
    entries
}

#[cfg(test)]
mod tests {
    use zolana_transaction::SOL_MINT;

    use super::*;

    fn effect(deposit: bool, pays_another: bool, received: u64, spent: u64) -> Effect {
        let amounts = |amount| match amount {
            0 => BTreeMap::new(),
            amount => BTreeMap::from([(SOL_MINT, amount)]),
        };
        Effect {
            slot: 1,
            deposit,
            pays_another,
            received: amounts(received),
            spent: amounts(spent),
        }
    }

    fn kinds(effect: Effect) -> Vec<(ActivityKind, u64)> {
        entries(BTreeMap::from([(Signature::default(), effect)]))
            .into_iter()
            .map(|entry| (entry.kind, entry.amount))
            .collect()
    }

    #[test]
    fn classifies_by_the_notes_moved() {
        use ActivityKind::*;
        // A deposit, and a payment from another wallet with its change.
        assert_eq!(kinds(effect(true, false, 50, 0)), [(Shielded, 50)]);
        assert_eq!(kinds(effect(false, true, 7, 0)), [(Received, 7)]);
        // A send keeps the change; a whole note sent leaves none.
        assert_eq!(kinds(effect(false, true, 20, 30)), [(Sent, 10)]);
        assert_eq!(kinds(effect(false, true, 0, 100)), [(Sent, 100)]);
        // Every output is this wallet's: withdrawn, or merged.
        assert_eq!(kinds(effect(false, false, 40, 50)), [(Unshielded, 10)]);
        assert_eq!(kinds(effect(false, false, 0, 50)), [(Unshielded, 50)]);
        assert_eq!(kinds(effect(false, false, 60, 60)), [(Internal, 60)]);
    }

    #[test]
    fn lists_newest_first_with_one_entry_per_asset() {
        let mint = Pubkey::new_unique();
        let mut older = effect(false, true, 0, 0);
        older.received = BTreeMap::from([(SOL_MINT, 5), (mint, 9)]);
        let mut newer = effect(false, false, 0, 0);
        newer.slot = 2;
        newer.spent = BTreeMap::from([(mint, 4)]);
        let listed = entries(BTreeMap::from([
            (Signature::from([1; 64]), older),
            (Signature::from([2; 64]), newer),
        ]));
        let summary: Vec<_> = listed
            .iter()
            .map(|entry| (entry.slot, entry.kind, entry.mint.clone(), entry.amount))
            .collect();
        assert_eq!(
            summary,
            [
                (2, ActivityKind::Unshielded, Some(mint.to_string()), 4),
                (1, ActivityKind::Received, None, 5),
                (1, ActivityKind::Received, Some(mint.to_string()), 9),
            ]
        );
    }
}
