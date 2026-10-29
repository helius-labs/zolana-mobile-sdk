//! Signature-scoped activity, without scanning the wallet's history.
use super::{
    prepared::PreparedTransaction,
    types::OutputOpening,
    util::{beside, invalid},
    MobileWallet,
};
use crate::{
    activity::{self, TransactionLookup, TransactionLookupStatus as Status},
    error::WalletError,
};
use solana_commitment_config::CommitmentConfig;
use solana_pubkey::Pubkey;
use solana_rpc_client::api::{config::RpcTransactionConfig, request::RpcRequest};
use solana_signature::Signature;
use solana_transaction_status_client_types::{
    EncodedConfirmedTransactionWithStatusMeta as Confirmed, UiTransactionEncoding,
};
use std::{
    collections::{BTreeMap, HashMap, HashSet},
    str::FromStr,
};
use zolana_client::{
    fetch_spl_asset_registrations, rpc::solana_rpc::ConfirmedInstructionGroups, ClientError,
    GetShieldedTransactionsBySignatureResponse, Rpc,
};
use zolana_interface::{
    event::OutputDataEncoding,
    instruction::{fetch_tag, settlement_accounts, tag, CircuitId, TransactIxData},
    PROGRAM_ID_PUBKEY,
};
use zolana_keypair::PublicKey;
use zolana_transaction::{
    decrypt, serialization::confidential::Confidential, verify_owned, AssetRegistry,
    EncryptedScheme, HistoryEntry, HistoryKind, ShieldedKeys, ShieldedTransaction,
    SppProofOutputUtxo, TransactionError, TransactionKeyRequest, SOL_MINT,
};

pub(super) fn transaction(
    wallet: &MobileWallet,
    signature: String,
    prepared: Option<&PreparedTransaction>,
) -> Result<TransactionLookup, WalletError> {
    let signature = Signature::from_str(&signature).map_err(|_| WalletError::SignatureInvalid)?;
    let (discover_assets, mut registry) = {
        let mut assets = wallet.assets();
        (assets.discovers(), assets.registry(&wallet.client)?.clone())
    };
    let lookup = Lookup {
        keys: &wallet.keys,
        discover_assets,
        padding: padding_commitments(prepared.map_or(&[], |pending| pending.outputs.as_slice()))?,
    };
    let (indexed, confirmed) = beside(
        || {
            wallet
                .client
                .get_shielded_transactions_by_signature(signature, None)
        },
        || {
            // The typed Solana getter cannot represent an RPC null. Read once:
            // missing and malformed responses must remain distinct.
            wallet
                .client
                .rpc()
                .client()
                .send::<Option<Confirmed>>(
                    RpcRequest::GetTransaction,
                    serde_json::json!([
                        signature.to_string(),
                        RpcTransactionConfig {
                            encoding: Some(UiTransactionEncoding::Json),
                            commitment: Some(CommitmentConfig::confirmed()),
                            max_supported_transaction_version: Some(1),
                        }
                    ]),
                )
                .map_err(|error| ClientError::Rpc(format!("get_transaction {signature}: {error}")))
        },
    );
    let result = lookup.read(
        &wallet.client,
        signature,
        &mut registry,
        indexed?,
        confirmed?,
    )?;
    if result.status == Status::Ready {
        wallet.assets().remember_registry(registry);
    }
    Ok(result)
}

fn padding_commitments(outputs: &[OutputOpening]) -> Result<HashSet<(u16, [u8; 32])>, WalletError> {
    outputs
        .iter()
        .filter(|output| {
            output.owner_signing_key.is_none()
                && output.nullifier_key.is_none()
                && output.amount == 0
                && !output.compact
        })
        .map(|output| {
            let blinding = output
                .blinding
                .as_slice()
                .try_into()
                .map_err(|_| invalid("invalid output blinding"))?;
            let dummy = SppProofOutputUtxo {
                blinding,
                ..Default::default()
            };
            Ok((output.tree_id, dummy.hash(output.tree_id)?))
        })
        .collect()
}

// Shared context for a read and its optional asset-discovery retry.
struct Lookup<'a> {
    keys: &'a dyn ShieldedKeys,
    discover_assets: bool,
    padding: HashSet<(u16, [u8; 32])>,
}

impl Lookup<'_> {
    fn read(
        &self,
        client: &impl Rpc,
        signature: Signature,
        registry: &mut AssetRegistry,
        indexed: GetShieldedTransactionsBySignatureResponse,
        confirmed: Option<Confirmed>,
    ) -> Result<TransactionLookup, WalletError> {
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
                return Err(invalid("inconsistent signature lookup response"));
            }
            transactions.push(tx);
        }
        let result = |status, entries: Vec<HistoryEntry>, registry: &AssetRegistry| {
            Ok(TransactionLookup {
                status,
                entries: entries
                    .into_iter()
                    .map(|entry| activity::activity_entry(entry, registry))
                    .collect::<Result<_, _>>()?,
                slot,
                indexed_slot: indexed.context.slot,
            })
        };
        if slot.is_none() {
            return result(Status::NotFound, vec![], registry);
        }
        if slot.is_some_and(|slot| slot > indexed.context.slot) {
            return result(Status::PendingIndexing, vec![], registry);
        }
        if transactions.is_empty() {
            return result(Status::Ready, vec![], registry);
        }
        if confirmed
            .as_ref()
            .and_then(|tx| tx.transaction.meta.as_ref())
            .is_some_and(|meta| meta.err.is_some())
        {
            return Err(invalid(
                "indexer returned shielded events for a failed transaction",
            ));
        }
        let groups = confirmed.and_then(|tx| ConfirmedInstructionGroups::try_from(tx).ok());
        let mut details = self.lookup_details(
            client,
            signature,
            &transactions,
            registry,
            self.discover_assets,
            groups.as_ref(),
        )?;
        if matches!(details, Details::NeedAssets) && self.discover_assets {
            for mint in fetch_spl_asset_registrations(client, slot)? {
                if registry.asset_id(&mint.asset).ok() != Some(mint.asset_id) {
                    registry.insert(mint.asset_id, mint.asset)?;
                }
            }
            details = self.lookup_details(
                client,
                signature,
                &transactions,
                registry,
                false,
                groups.as_ref(),
            )?;
        }
        match details {
            Details::Complete(entries) => result(Status::Ready, entries, registry),
            Details::Incomplete | Details::NeedAssets => {
                result(Status::Incomplete, vec![], registry)
            }
        }
    }
}
enum Details {
    Complete(Vec<HistoryEntry>),
    NeedAssets,
    Incomplete,
}

#[derive(Default)]
struct Movement {
    received: u128,
    spent: u128,
}

impl Lookup<'_> {
    fn lookup_details<R: Rpc + ?Sized>(
        &self,
        client: &R,
        signature: Signature,
        transactions: &[ShieldedTransaction],
        assets: &AssetRegistry,
        discover: bool,
        groups: Option<&ConfirmedInstructionGroups>,
    ) -> Result<Details, ClientError> {
        let address = self.keys.address()?;
        let own_tag = address.signing_pubkey.confidential_view_tag()?;
        let owner = Pubkey::new_from_array(own_tag);
        let decoded = decrypt(self.keys, transactions, assets)?;
        if discover && (!decoded.unknown_asset_ids.is_empty() || !decoded.unknown_mints.is_empty())
        {
            return Ok(Details::NeedAssets);
        }
        if !discover
            && self.discover_assets
            && (!decoded.unknown_asset_ids.is_empty() || !decoded.unknown_mints.is_empty())
        {
            return Ok(Details::Incomplete);
        }
        let owned = verify_owned(self.keys, &decoded)?;
        let by_hash: HashMap<_, _> = owned
            .utxos
            .iter()
            .map(|utxo| (utxo.utxo_hash, utxo))
            .collect();
        let mut movements = BTreeMap::<Pubkey, Movement>::new();
        let mut counted = HashSet::new();
        let mut spends = HashSet::new();
        let own_hash = address.owner_hash()?;
        let mut tags: HashSet<_> = self
            .keys
            .viewing_public_keys()
            .iter()
            .map(|key| key.u())
            .collect();
        tags.insert(own_tag);
        let mut pays_another = false;
        let deposit = transactions.iter().any(|tx| tx.proofless);
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
                    Confidential::embedded_viewing_pk(&blob[1..]).ok() == Some(address.viewing_pubkey))
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
            let key = self
                .keys
                .transaction_keys(&[TransactionKeyRequest {
                    viewing_pubkey: address.viewing_pubkey,
                    first_nullifier: *first,
                }])?
                .into_iter()
                .next()
                .ok_or(TransactionError::IncompleteDerivation { got: 0, want: 1 })?;
            let context = groups.and_then(|groups| transact_context(tx, groups));
            if groups.is_some() && tx.ring_config.is_none() && context.is_none() {
                return Ok(Details::Incomplete);
            }
            if Some(key.pubkey()) != tx.tx_viewing_pk {
                // A sponsor signing as fee payer owns no inputs merely by
                // paying. An actual owner whose key cannot read the spend
                // needs context this bounded reader does not have.
                if context
                    .as_ref()
                    .is_some_and(|cx| cx.owners.contains(&owner))
                {
                    return Ok(Details::Incomplete);
                }
                continue;
            }
            let Some(cx) = context else {
                return Ok(Details::Incomplete);
            };
            if !matches!(
                cx.data.circuit.uncached(),
                CircuitId::ConfidentialSigner(..)
            ) || cx.owners != [owner]
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
                let response = client.get_user_records(owners.clone(), None)?;
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
                if slot.view_tag == own_tag
                    || self
                        .padding
                        .contains(&(slot.output_context.tree_id, slot.output_context.hash))
                {
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
                    Err(TransactionError::UnknownAsset(_)) if !self.discover_assets => continue,
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
                    if self.discover_assets {
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
            entries.push(HistoryEntry {
                kind,
                mint,
                amount,
                tx_signature: signature,
                slot: transactions[0].slot,
            });
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

impl MobileWallet {
    /// Fetch only the shielded events of `signature` and their confirmed
    /// Solana transaction, then decrypt this wallet's rows. Does not scan or
    /// warm the wallet's history or spendable-note cache. Asset/recipient
    /// registry reads may be needed, but no history pages are fetched.
    ///
    /// A cold wallet can read its default-ring outgoing transfers and public
    /// withdrawals using its transaction viewing key, without old input notes.
    /// `Incomplete` reports unsupported spends or missing metadata rather than
    /// returning an incorrect empty result. Recipient-tagged dummy padding
    /// (possible on an exact-balance private send) is also incomplete: a cold
    /// reader cannot authenticate it as padding without an output opening.
    /// Use [`Self::transaction_with_prepared`] when the prepared spend was saved.
    /// Only `Ready` with no entries means
    /// there are no activity rows for this wallet's configured assets.
    pub fn transaction(&self, signature: String) -> Result<TransactionLookup, WalletError> {
        transaction(self, signature, None)
    }
    /// Like [`Self::transaction`], with output openings from a prepared spend
    /// (including one restored after restart). Matching dummy commitments let
    /// an exact-balance send be read without input-note history. Other outputs
    /// and public settlements are verified as usual; saved amounts are not
    /// used to infer the result. Unmatched openings cannot hide unreadable outputs.
    ///
    /// This reads activity details; use [`Self::confirm`] to confirm a payment.
    pub fn transaction_with_prepared(
        &self,
        signature: String,
        pending: &PreparedTransaction,
    ) -> Result<TransactionLookup, WalletError> {
        transaction(self, signature, Some(pending))
    }
}

#[cfg(test)]
mod tests;
