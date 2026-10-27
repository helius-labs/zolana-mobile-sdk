//! Signature-scoped activity, using the protocol's existing readers. No
//! transaction or nullifier history queries belong on this path.

use std::collections::{BTreeMap, HashMap, HashSet};

use solana_rpc_client_api::request::RpcRequest;
use solana_transaction_status_client_types::EncodedConfirmedTransactionWithStatusMeta;
use zolana_client::{fetch_spl_asset_registrations, rpc::solana_rpc::ConfirmedInstructionGroups};
use zolana_interface::{
    event::OutputDataEncoding,
    instruction::{fetch_tag, settlement_accounts, tag, CircuitId, TransactIxData},
};
use zolana_transaction::{
    decrypt, serialization::confidential::Confidential, verify_owned, AssetRegistry,
    EncryptedScheme, HistoryEntry, HistoryKind, ShieldedKeys, ShieldedTransaction, SOL_MINT,
};

use super::*;
use crate::activity::{TransactionLookup, TransactionLookupStatus as Status};

type Confirmed = EncodedConfirmedTransactionWithStatusMeta;

pub(super) fn transaction(
    wallet: &MobileWallet,
    signature: String,
) -> Result<TransactionLookup, WalletError> {
    let signature = Signature::from_str(&signature).map_err(|_| WalletError::SignatureInvalid)?;
    // A single confirmed RPC read, including a real `null` when unavailable.
    // The convenience transaction fetcher polls missing signatures for 30s.
    let (indexed, confirmed) = beside(
        || {
            wallet
                .client
                .get_shielded_transactions_by_signature(signature, None)
        },
        || {
            wallet.client.rpc().client().send::<Option<Confirmed>>(
                RpcRequest::GetTransaction,
                serde_json::json!([signature.to_string(), {
                    "encoding": "json", "commitment": "confirmed", "maxSupportedTransactionVersion": 1
                }]),
            )
        },
    );
    let indexed = indexed?;
    let confirmed = confirmed.map_err(|error| ClientError::Rpc(error.to_string()))?;
    let mut event_ids = HashSet::new();
    let mut transactions = Vec::with_capacity(indexed.transactions.len());
    let slot = confirmed
        .as_ref()
        .map(|tx| tx.slot)
        .or_else(|| indexed.transactions.first().map(|tx| tx.transaction.slot));
    for event in indexed.transactions {
        let tx = event.transaction;
        if tx.tx_signature != signature
            || Some(tx.slot) != slot
            || !event_ids.insert(event.event_index)
        {
            return Err(ClientError::Rpc("inconsistent signature lookup response".into()).into());
        }
        transactions.push(tx);
    }
    let result = |status, entries| TransactionLookup {
        status,
        entries,
        slot,
        indexed_slot: indexed.context.slot,
    };
    if slot.is_none() {
        return Ok(result(Status::NotFound, vec![]));
    }
    if slot.is_some_and(|slot| slot > indexed.context.slot) {
        return Ok(result(Status::PendingIndexing, vec![]));
    }
    if transactions.is_empty() {
        return Ok(result(Status::Ready, vec![]));
    }
    if confirmed
        .as_ref()
        .and_then(|tx| tx.transaction.meta.as_ref())
        .is_some_and(|meta| meta.err.is_some())
    {
        return Err(ClientError::Rpc(
            "indexer returned shielded events for a failed transaction".into(),
        )
        .into());
    }
    let (discover, mut registry) = {
        let mut assets = wallet.assets();
        (assets.discovers(), assets.registry(&wallet.client)?.clone())
    };
    let mut details = wallet.lookup_details(
        signature,
        &transactions,
        &registry,
        discover,
        confirmed.as_ref(),
    )?;
    if matches!(details, Details::NeedAssets) && discover {
        for mint in fetch_spl_asset_registrations(&wallet.client, slot)? {
            if registry.asset_id(&mint.asset).ok() != Some(mint.asset_id) {
                registry.insert(mint.asset_id, mint.asset)?;
            }
        }
        details = wallet.lookup_details(
            signature,
            &transactions,
            &registry,
            false,
            confirmed.as_ref(),
        )?;
    }
    match details {
        Details::Complete(entries) => {
            wallet.assets().remember_registry(registry);
            Ok(result(Status::Ready, entries))
        }
        Details::Incomplete | Details::NeedAssets => Ok(result(Status::Incomplete, vec![])),
    }
}

enum Details {
    Complete(Vec<ActivityEntry>),
    NeedAssets,
    Incomplete,
}

#[derive(Default)]
struct Movement {
    received: u128,
    spent: u128,
}

impl MobileWallet {
    fn lookup_details(
        &self,
        signature: Signature,
        transactions: &[ShieldedTransaction],
        assets: &AssetRegistry,
        discover: bool,
        confirmed: Option<&Confirmed>,
    ) -> Result<Details, WalletError> {
        let decoded = decrypt(&self.keys, transactions, assets)?;
        if discover && (!decoded.unknown_asset_ids.is_empty() || !decoded.unknown_mints.is_empty())
        {
            return Ok(Details::NeedAssets);
        }
        if !discover
            && self.assets().discovers()
            && (!decoded.unknown_asset_ids.is_empty() || !decoded.unknown_mints.is_empty())
        {
            return Ok(Details::Incomplete);
        }
        let owned = verify_owned(&self.keys, &decoded)?;
        let by_hash: HashMap<_, _> = owned
            .utxos
            .iter()
            .map(|utxo| (utxo.utxo_hash, utxo))
            .collect();
        let mut movements = BTreeMap::<Pubkey, Movement>::new();
        let mut counted = HashSet::new();
        let mut spends = HashSet::new();
        let own_tag = self.address.signing_pubkey.confidential_view_tag()?;
        let own_hash = self.address.owner_hash()?;
        let mut tags: HashSet<_> = self
            .keys
            .viewing_public_keys()
            .iter()
            .map(|key| key.u())
            .collect();
        tags.insert(own_tag);
        let mut pays_another = false;
        let deposit = transactions.iter().any(|tx| tx.proofless);
        let groups = confirmed
            .cloned()
            .and_then(|tx| ConfirmedInstructionGroups::try_from(tx).ok());

        for tx in transactions {
            let mut received = BTreeMap::<Pubkey, u128>::new();
            for slot in &tx.output_slots {
                if let Some(note) = by_hash.get(&slot.output_context.hash) {
                    *received.entry(note.utxo.asset.asset).or_default() +=
                        u128::from(note.utxo.amount);
                    if counted.insert(note.utxo_hash) {
                        movements.entry(note.utxo.asset.asset).or_default().received +=
                            u128::from(note.utxo.amount);
                    }
                } else if matches!(slot.output_data(), Some(OutputDataEncoding::Encrypted(ref blob))
                    if blob.first() == Some(&EncryptedScheme::Confidential.as_byte()) &&
                    Confidential::embedded_viewing_pk(&blob[1..]).ok() == Some(self.address.viewing_pubkey))
                    || slot.proofless_output().is_some_and(|output| {
                        output.owner == own_hash
                            && assets
                                .asset_id(&Pubkey::new_from_array(output.asset))
                                .is_ok()
                    })
                {
                    return Ok(Details::Incomplete);
                } else if !tags.contains(&slot.view_tag) {
                    pays_another = true;
                }
            }
            if tx.proofless {
                continue;
            }
            if tx.may_be_merge() && tx.output_slots.iter().any(|slot| {
                matches!(slot.output_data(), Some(OutputDataEncoding::VerifiablyEncrypted(ref blob) | OutputDataEncoding::Encrypted(ref blob))
                    if blob.first() == Some(&EncryptedScheme::Merge.as_byte()))
            }) {
                if received.is_empty() && decoded.unknown_mints.is_empty() &&
                    tx.output_slots.iter().any(|slot| tags.contains(&slot.view_tag)) {
                    return Ok(Details::Incomplete);
                }
                // The existing merge decoder derives the key from the owner's
                // nullifier secret and accepts only commitment-matching notes.
                if !received.is_empty() && spends.insert(tx.nullifiers.clone()) {
                    for (mint, amount) in received {
                        movements.entry(mint).or_default().spent += amount;
                    }
                }
                continue;
            }
            let Some(first) = tx.nullifiers.first() else {
                continue;
            };
            let key = self.viewing_key.get_transaction_viewing_key(first)?;
            let context = groups
                .as_ref()
                .and_then(|groups| transact_context(tx, groups));
            if groups.is_some() && tx.ring_config.is_none() && context.is_none() {
                return Ok(Details::Incomplete);
            }
            if Some(key.pubkey()) != tx.tx_viewing_pk {
                // A sponsor signing as fee payer owns no inputs merely by
                // paying. An actual owner whose key cannot read the spend
                // needs context this bounded reader does not have.
                if context
                    .as_ref()
                    .is_some_and(|cx| cx.owners.contains(&self.owner))
                {
                    return Ok(Details::Incomplete);
                }
                continue;
            }
            let Some(cx) = context else {
                return Ok(Details::Incomplete);
            };
            if !matches!(cx.data.circuit.uncached(), CircuitId::ConfidentialEddsa(..))
                || cx.owners != [self.owner]
            {
                return Ok(Details::Incomplete);
            }
            if !spends.insert(tx.nullifiers.clone()) {
                continue;
            }
            let owners: Vec<_> = tx
                .output_slots
                .iter()
                .filter(|slot| slot.view_tag != own_tag)
                .map(|slot| Pubkey::new_from_array(slot.view_tag))
                .collect::<HashSet<_>>()
                .into_iter()
                .collect();
            let records = if owners.is_empty() {
                HashMap::new()
            } else {
                let response = self.client.get_user_records(owners.clone(), None)?;
                if response.records.len() != owners.len() {
                    return Ok(Details::Incomplete);
                }
                let mut records = HashMap::new();
                for (owner, record) in owners.into_iter().zip(response.records) {
                    let Some(record) = record else {
                        return Ok(Details::Incomplete);
                    };
                    if record.owner != owner {
                        return Ok(Details::Incomplete);
                    }
                    records.insert(owner, record);
                }
                records
            };
            // Outputs owned by somebody else and public settlement legs are
            // the outflow. Change cancels out, so no input-note history is
            // required. Self-transfers instead spend exactly their kept value.
            let mut net = BTreeMap::<Pubkey, i128>::new();
            let Some(salt) = tx.salt else {
                return Ok(Details::Incomplete);
            };
            for (index, slot) in tx.output_slots.iter().enumerate() {
                if slot.view_tag == own_tag {
                    continue;
                }
                let Some(OutputDataEncoding::Encrypted(blob)) = slot.output_data() else {
                    return Ok(Details::Incomplete);
                };
                let Some((&scheme, body)) = blob.split_first() else {
                    return Ok(Details::Incomplete);
                };
                if scheme != EncryptedScheme::Confidential.as_byte() {
                    return Ok(Details::Incomplete);
                }
                let Ok(plaintext) =
                    Confidential::decrypt_with_tx_key(&key, body, salt, index as u32)
                else {
                    return Ok(Details::Incomplete);
                };
                let record = &records[&Pubkey::new_from_array(slot.view_tag)];
                let owner = PublicKey::from_ed25519(record.owner.as_array());
                let note = match plaintext.into_utxo(owner, assets) {
                    Ok(note) => note,
                    Err(TransactionError::UnknownAsset(_)) if discover => {
                        return Ok(Details::NeedAssets)
                    }
                    Err(TransactionError::UnknownAsset(_)) if !self.assets().discovers() => {
                        continue
                    }
                    Err(_) => return Ok(Details::Incomplete),
                };
                if note.ring_program_id.is_some()
                    || note.data.utxo_data().is_some()
                    || note.data.ring_data().is_some()
                    || note.hash(
                        &record.nullifier_pubkey,
                        &[0; 32],
                        &[0; 32],
                        slot.output_context.tree_id,
                    )? != slot.output_context.hash
                {
                    return Ok(Details::Incomplete);
                }
                *net.entry(note.asset.asset).or_default() += i128::from(note.amount);
            }
            let Some(settlements) =
                settlement_accounts(&cx.data.interface_transfers, cx.data.circuit, &cx.accounts)
            else {
                return Ok(Details::Incomplete);
            };
            let mut offset = 0;
            for leg in &cx.data.interface_transfers {
                let mint = leg
                    .mint_account_position()
                    .map_or(SOL_MINT, |i| settlements[offset + i]);
                offset += leg.settlement_account_count();
                if assets.asset_id(&mint).is_err() {
                    if discover {
                        return Ok(Details::NeedAssets);
                    }
                    if self.assets().discovers() {
                        return Ok(Details::Incomplete);
                    }
                    continue;
                }
                *net.entry(mint).or_default() += if leg.is_deposit() {
                    -i128::from(leg.amount())
                } else {
                    i128::from(leg.amount())
                };
            }
            for mint in received.keys() {
                net.entry(*mint).or_default();
            }
            for (mint, net) in net {
                let kept = received.get(&mint).copied().unwrap_or_default();
                let spent = kept as i128 + net;
                if spent < 0 {
                    return Ok(Details::Incomplete);
                }
                movements.entry(mint).or_default().spent += spent as u128;
            }
        }
        let mut entries = Vec::new();
        for (mint, movement) in movements {
            let (received, spent) = (movement.received, movement.spent);
            let (kind, amount) = if spent == 0 {
                (
                    if deposit {
                        HistoryKind::Deposit
                    } else {
                        HistoryKind::Received
                    },
                    received,
                )
            } else if spent == received {
                (HistoryKind::SelfTransfer, spent)
            } else if received > spent {
                (HistoryKind::Deposit, received - spent)
            } else {
                (
                    if pays_another {
                        HistoryKind::Sent
                    } else {
                        HistoryKind::Withdrawal
                    },
                    spent - received,
                )
            };
            if amount == 0 {
                continue;
            }
            let amount = u64::try_from(amount)
                .map_err(|_| ClientError::Rpc("activity amount exceeds u64".into()))?;
            entries.push(activity::activity_entry(
                HistoryEntry {
                    kind,
                    mint,
                    amount,
                    tx_signature: signature,
                    slot: transactions[0].slot,
                },
                assets,
            )?);
        }
        Ok(Details::Complete(entries))
    }
}

struct TransactContext {
    data: TransactIxData,
    accounts: Vec<Pubkey>,
    owners: Vec<Pubkey>,
}

/// Bind the indexer's event to a real SPP invocation, including ciphertexts,
/// commitments and resolved owner tags. A matching byte sequence elsewhere in
/// the transaction is not an instruction or evidence for a settlement.
fn transact_context(
    tx: &ShieldedTransaction,
    groups: &ConfirmedInstructionGroups,
) -> Option<TransactContext> {
    for instruction in groups
        .groups
        .iter()
        .flat_map(|group| std::iter::once(&group.outer).chain(&group.inner))
    {
        if instruction.program_id != PROGRAM_ID_PUBKEY
            || instruction.data.first() != Some(&tag::TRANSACT)
        {
            continue;
        }
        let data = TransactIxData::deserialize(instruction.data.get(1..)?).ok()?;
        if data
            .inputs
            .iter()
            .map(|input| input.nullifier_hash)
            .collect::<Vec<_>>()
            != tx.nullifiers
            || data.outputs.len() != tx.output_slots.len()
            || Some(data.tx_viewing_pk) != tx.tx_viewing_pk.map(|key| *key.as_bytes())
            || Some(data.salt) != tx.salt
        {
            continue;
        }
        let same_outputs = data
            .outputs
            .iter()
            .zip(&tx.output_slots)
            .all(|(output, slot)| {
                output.utxo_hash == slot.output_context.hash
                    && output.data.as_deref() == Some(slot.payload.as_slice())
                    && fetch_tag(&output.owner_tag, |i| {
                        instruction
                            .accounts
                            .get(usize::from(i))
                            .map(|key| key.to_bytes())
                    })
                    .ok()
                        == Some(slot.view_tag)
            });
        if !same_outputs {
            continue;
        }
        let settlements = settlement_accounts(
            &data.interface_transfers,
            data.circuit,
            &instruction.accounts,
        )?;
        let start = 4 + data.tree_contexts.len() + data.inputs.len();
        let cache_accounts = data.circuit.cache_access().map_or(0, |access| {
            usize::from(access.read_bitmap != 0) + 2 * usize::from(access.writes_cache())
        });
        let end = instruction
            .accounts
            .len()
            .checked_sub(cache_accounts + settlements.len())?;
        let owners = instruction.accounts.get(start..end)?.to_vec();
        return Some(TransactContext {
            data,
            accounts: instruction.accounts.clone(),
            owners,
        });
    }
    None
}

#[cfg(test)]
mod tests;
