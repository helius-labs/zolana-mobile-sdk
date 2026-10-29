//! Cache-targeted merges and authenticated restart state. The cache is an
//! acceleration path: the same merge output is also published to the tree.
use super::{
    merge::merge_expiry,
    notes::{merge_input_ids, select_merge_inputs, MAX_CACHED_MERGE_INPUTS},
    prepared::PreparedTransaction,
    spend::{max_spend_inputs, within_cap},
    types::{Operation, Output},
    util::{invalid, nullifiers, parse_pubkey},
    MobileWallet,
};
use crate::{
    asset::{mint_name, Asset},
    error::WalletError,
    prover::Proving,
};
use serde::{Deserialize, Serialize};
use solana_message::VersionedMessage;
use solana_pubkey::Pubkey;
use solana_signature::Signature;
use std::{
    collections::HashSet,
    time::{SystemTime, UNIX_EPOCH},
};
use zolana_client::{
    check_merge_record, compile_message,
    prover::{MergeCacheTarget, MergeProver, ProofCompressed},
    user_registry::fetch_user_record_optional_checked,
    ClientError, ComputeBudgetConfig, ProverExt, Rpc, WitnessReader,
};
use zolana_interface::{
    instruction::instruction_data::CreateCacheData,
    pda,
    state::cache::{CacheAccount, CACHE_CAPACITY},
    PROGRAM_ID_PUBKEY,
};
use zolana_program::instruction::{CacheWriteAccounts, CreateCache, MergeTransact};
use zolana_transaction::{
    instructions::merge::MergeTransaction, select_merge, Data, Utxo, WalletUtxo,
};
use zolana_user_registry_interface::user_record_pda;

/// Terms supplied by the sponsor. This version uses the sponsor as payer and
/// cache writer, so the unsigned merge requires only its signature.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[flutter_rust_bridge::frb(non_opaque)]
pub struct CacheTarget {
    pub sponsor: String,
    pub nonce: u64,
    pub tree_id: u16,
    pub expires_at: i64,
    pub slot: u8,
    pub max_priority_fee_lamports: u64,
    pub compute_unit_limit: u32,
}

impl CacheTarget {
    pub fn cache_address(&self) -> Result<String, WalletError> {
        Ok(self.address()?.to_string())
    }
    pub(super) fn address(&self) -> Result<Pubkey, WalletError> {
        Ok(self.create()?.cache())
    }
    fn create(&self) -> Result<CreateCache, WalletError> {
        let sponsor = parse_pubkey(&self.sponsor)?;
        Ok(CreateCache {
            payer: sponsor,
            data: CreateCacheData {
                write_authority: sponsor,
                nonce: self.nonce,
                tree_id: self.tree_id,
                expires_at: self.expires_at,
            },
        })
    }
    fn validate(&self) -> Result<(), WalletError> {
        self.create()?;
        if usize::from(self.slot) >= CACHE_CAPACITY {
            return Err(invalid("cache slot out of range"));
        }
        if self.expires_at <= now() {
            return Err(invalid("cache expired"));
        }
        if self.compute_unit_limit == 0 || self.compute_unit_limit > 1_400_000 {
            return Err(invalid("invalid merge compute limit"));
        }
        Ok(())
    }
    fn same_cache(&self, other: &Self) -> bool {
        self.sponsor == other.sponsor
            && self.nonce == other.nonce
            && self.tree_id == other.tree_id
            && self.expires_at == other.expires_at
    }
}

/// Wallet-derived predicted merge output. Private fields prevent an app from
/// substituting an amount or blinding. Export its prepared transaction for
/// restart recovery; never log that snapshot.
#[derive(Clone, Serialize, Deserialize)]
pub struct CachedOutput {
    pub(super) target: CacheTarget,
    owner: String,
    mint: Option<String>,
    asset_id: u64,
    amount: u64,
    blinding: [u8; 32],
    pub(super) commitment: [u8; 32],
}
impl CachedOutput {
    /// An independent handle for bridge calls that take ownership of inputs.
    #[flutter_rust_bridge::frb(sync)]
    pub fn copy(&self) -> Self {
        self.clone()
    }

    #[flutter_rust_bridge::frb(sync, getter)]
    pub fn target(&self) -> CacheTarget {
        self.target.clone()
    }
    #[flutter_rust_bridge::frb(sync, getter)]
    pub fn amount(&self) -> u64 {
        self.amount
    }
    #[flutter_rust_bridge::frb(sync, getter)]
    pub fn commitment(&self) -> Vec<u8> {
        self.commitment.to_vec()
    }
    #[flutter_rust_bridge::frb(sync, getter)]
    pub fn mint(&self) -> Option<String> {
        self.mint.clone()
    }
}

fn now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}

impl MobileWallet {
    /// Build an unsigned [CreateCache, MergeTransact], including an idempotent
    /// cache creation in every round. Notes come from the target's one tree.
    /// With `input_ids`, use exactly those notes in that order, infer the mint,
    /// and reject `mint` or `max_inputs`. Missing or reserved inputs fail rather
    /// than selecting replacements. Save the ordered IDs with the run.
    pub fn prepare_cached_merge(
        &mut self,
        target: CacheTarget,
        mint: Option<String>,
        max_inputs: Option<u32>,
        proving: Option<Proving>,
        priority_fee_lamports: Option<u64>,
        input_ids: Option<Vec<String>>,
    ) -> Result<PreparedTransaction, WalletError> {
        target.validate()?;
        if input_ids.is_some() && (mint.is_some() || max_inputs.is_some()) {
            return Err(invalid(
                "inputIds cannot be combined with mint or maxInputs",
            ));
        }
        let ids = input_ids.as_deref().map(merge_input_ids).transpose()?;
        let fee = priority_fee_lamports.or(self.priority_fee).unwrap_or(0);
        if fee > target.max_priority_fee_lamports {
            return Err(invalid("priority fee exceeds sponsored run limit"));
        }
        let proving = self.proving(proving)?;
        let payer = parse_pubkey(&target.sponsor)?;
        let record = fetch_user_record_optional_checked(&self.client, self.owner)?
            .ok_or(WalletError::MergingDisabled)?;
        check_merge_record(&record, self.owner, &self.address).map_err(|error| match error {
            ClientError::MergeNullifierKeyMismatch => WalletError::RegistrationConflict {
                owner: self.owner.to_string(),
            },
            error => error.into(),
        })?;
        let reserved = self.reserved()?;
        let inputs = if let Some(ids) = ids {
            select_merge_inputs(
                self.recent_notes()?.utxos(),
                &ids,
                target.tree_id,
                &reserved,
            )?
        } else {
            let asset = self.asset(mint)?;
            select_merge(
                self.recent_notes()?
                    .utxos()
                    .filter(|note| note.tree_id == target.tree_id),
                asset.mint,
                max_inputs.unwrap_or(24).min(MAX_CACHED_MERGE_INPUTS as u32) as usize,
                &reserved,
            )?
        };
        let spends = nullifiers(&inputs);
        let expiry = merge_expiry().min(target.expires_at as u64);
        let merge = MergeTransaction::new(inputs)?
            .with_output_tree_id(target.tree_id)
            .with_expiry(expiry)
            .encrypt(&self.keys)?;
        let cached = CachedOutput {
            target: target.clone(),
            owner: self.owner.to_string(),
            mint: mint_name(&merge.output_utxo.asset.asset),
            asset_id: merge.output_utxo.asset.asset_id,
            amount: merge.output_utxo.amount,
            blinding: merge.output_utxo.blinding,
            commitment: [0; 32],
        };
        let witnesses = self.client.indexer().input_witnesses(
            &merge.input_utxo_hashes()?,
            &merge.dummy_nullifiers(),
            None,
        )?;
        let input_tree = pda::tree(target.tree_id);
        let trees = witnesses
            .spend_proofs
            .iter()
            .flat_map(|proof| {
                [
                    proof.state.merkle_context.tree,
                    proof.nullifier.merkle_context.tree,
                ]
            })
            .chain(
                witnesses
                    .dummy_nullifier_proofs
                    .iter()
                    .map(|proof| proof.merkle_context.tree),
            );
        for proof_tree in trees {
            if proof_tree != input_tree {
                return Err(ClientError::MergeInputTreeMismatch {
                    proof_tree: proof_tree.to_bytes(),
                    input_tree: input_tree.to_bytes(),
                }
                .into());
            }
        }
        let result = MergeProver {
            transaction: merge,
            nullifier_key: self.nullifier_key.clone(),
            proofs: witnesses.spend_proofs,
            dummy_nullifier_proofs: witnesses.dummy_nullifier_proofs,
            cache: Some(MergeCacheTarget {
                address: target.address()?,
                slot: target.slot,
            }),
        }
        .build()?;
        let proof = match proving {
            Proving::Local => self.native_prover.prove_merge(&result.inputs)?,
            Proving::Remote => self
                .remote_prover
                .as_ref()
                .ok_or(WalletError::RemoteProverMissing)?
                .prove_merge(&result.inputs)?,
        };
        use zolana_interface::verifying_keys::{merge_24_1, merge_52_1, merge_8_1};
        let key = match result.inputs.inputs.len() {
            8 => &merge_8_1::VERIFYINGKEY,
            24 => &merge_24_1::VERIFYINGKEY,
            52 => &merge_52_1::VERIFYINGKEY,
            _ => return Err(WalletError::UnsupportedCircuit),
        };
        zolana_client::prover::verify_proof_statement(&proof, result.public_input_hash, key)?;
        let merge_ix = MergeTransact {
            input_tree,
            output_tree: pda::tree(target.tree_id),
            payer,
            user_record: user_record_pda(&self.owner).0,
            data: result.instruction_data(ProofCompressed::try_from(proof)?.to_merge_proof()?),
            cache: Some(CacheWriteAccounts {
                cache: target.address()?,
                writer: payer,
            }),
        }
        .instruction();
        let instructions = [target.create()?.instruction(), merge_ix];
        let budget = ComputeBudgetConfig::new(target.compute_unit_limit).with_priority_fee(fee);
        if !zolana_client::transaction_size(&payer, &instructions, budget)?.fits() {
            return Err(invalid(
                "cached merge exceeds transaction limits; use fewer inputs",
            ));
        }
        let message = compile_message(&payer, &instructions, Default::default(), budget)?;
        let mut pending = self.pending(
            Operation::Merge {
                mint: cached.mint.clone(),
                amount: cached.amount,
            },
            message,
        )?;
        pending.cached_output = Some(CachedOutput {
            commitment: result.output_hash,
            ..cached
        });
        target.validate()?; // Proving may have outlived the run.
        self.reserve(pending, spends)
    }

    /// Spend only these cache outputs. Deliberately does not silently add tree
    /// inputs: the input set stays the one the app approved for this run.
    pub fn prepare_cached_spend(
        &mut self,
        outputs: Vec<Output>,
        cached_inputs: Vec<CachedOutput>,
        mint: Option<String>,
        fee_payer: Option<String>,
        priority_fee_lamports: Option<u64>,
    ) -> Result<PreparedTransaction, WalletError> {
        if cached_inputs.is_empty() {
            return Err(invalid("cached inputs are empty"));
        }
        let fee = priority_fee_lamports.or(self.priority_fee).unwrap_or(0);
        if cached_inputs
            .iter()
            .any(|input| fee > input.target.max_priority_fee_lamports)
        {
            return Err(invalid("priority fee exceeds cached run limit"));
        }
        self.prepare_spend_inner(
            outputs,
            mint,
            fee_payer,
            Some(Proving::Local),
            priority_fee_lamports,
            cached_inputs,
        )
    }

    pub(super) fn select_cached_notes(
        &self,
        cached: &[CachedOutput],
        asset: Asset,
        amount: u64,
        outputs: usize,
    ) -> Result<Vec<WalletUtxo>, WalletError> {
        self.verify_cached_outputs(cached.to_vec())?;
        let mut ordered = cached.to_vec();
        ordered.sort_by_key(|output| output.target.slot);
        self.reserved()?; // expire ordinary prepared-message reservations
        let reserved: HashSet<_> = self
            .reservations()
            .reserved
            .values()
            .flat_map(|held| held.spends.iter().copied())
            .collect();
        let mut notes = Vec::new();
        let mut total = 0u64;
        for output in &ordered {
            if output.mint != mint_name(&asset.mint) {
                return Err(invalid("cached input mint mismatch"));
            }
            let note = self.cached_note(output)?;
            if reserved.contains(&note.nullifier) {
                return Err(WalletError::NotesReserved { amount });
            }
            total = total
                .checked_add(output.amount)
                .ok_or_else(|| invalid("cached balance overflow"))?;
            notes.push(note);
        }
        if total < amount {
            return Err(WalletError::InsufficientPrivateBalance {
                requested: amount,
                available: total,
            });
        }
        within_cap(
            notes,
            amount,
            self.max_spend_inputs.min(max_spend_inputs(outputs)),
        )
    }

    pub(super) fn cached_note(&self, output: &CachedOutput) -> Result<WalletUtxo, WalletError> {
        if output.owner != self.owner.to_string() {
            return Err(invalid("cached output belongs to another wallet"));
        }
        let mint = output
            .mint
            .as_deref()
            .map(parse_pubkey)
            .transpose()?
            .unwrap_or(zolana_transaction::Mint::SOL.asset);
        let utxo = Utxo {
            owner: self.address.signing_pubkey,
            asset: zolana_transaction::Mint::new(mint, output.asset_id),
            amount: output.amount,
            blinding: output.blinding,
            ring_program_id: None,
            data: Data::default(),
        };
        let nullifier_pubkey = self.nullifier_key.pubkey()?;
        let utxo_hash = utxo.hash(&nullifier_pubkey, &[0; 32], &[0; 32], output.target.tree_id)?;
        if utxo_hash != output.commitment {
            return Err(invalid("cached output commitment mismatch"));
        }
        Ok(WalletUtxo {
            nullifier: self.nullifier_key.nullifier(&utxo_hash, &utxo.blinding)?,
            utxo,
            nullifier_pubkey,
            utxo_hash,
            tx_viewing_key: None,
            data_hash: None,
            ring_data_hash: None,
            tree_id: output.target.tree_id,
            leaf_index: 0,
            latest_tree_id: None,
            slot: 0,
            tx_signature: Signature::default(),
            slot_index: 0,
        })
    }

    /// Check account owner, layout, run metadata and every expected commitment
    /// at RPC confirmed commitment. Does not wait for the indexer or a prover job.
    pub fn verify_cached_outputs(&self, outputs: Vec<CachedOutput>) -> Result<(), WalletError> {
        let first = outputs
            .first()
            .ok_or_else(|| invalid("cached inputs are empty"))?;
        first.target.validate()?;
        let account = self
            .client
            .get_account(first.target.address()?)?
            .ok_or_else(|| invalid("cache is not available yet or has been closed"))?;
        if account.owner != PROGRAM_ID_PUBKEY || account.data.len() != CacheAccount::SIZE {
            return Err(invalid("invalid cache account owner or size"));
        }
        let cache = bytemuck::try_pod_read_unaligned::<CacheAccount>(&account.data)
            .map_err(|_| invalid("invalid cache account"))?;
        let sponsor = parse_pubkey(&first.target.sponsor)?;
        if !cache.has_discriminator()
            || cache.rent_sponsor != sponsor
            || cache.write_authority != sponsor
            || u16::from_le_bytes(cache.tree_id) != first.target.tree_id
            || cache.expiry_unix_ts() != first.target.expires_at
        {
            return Err(invalid("cache account does not match run terms"));
        }
        let mut slots = HashSet::new();
        for output in &outputs {
            output.target.validate()?;
            if !first.target.same_cache(&output.target) || !slots.insert(output.target.slot) {
                return Err(invalid(
                    "cached inputs must use distinct slots of one run cache",
                ));
            }
            self.cached_note(output)?;
            if output.commitment == [0; 32]
                || cache.utxo_hashes[usize::from(output.target.slot)] != output.commitment
            {
                return Err(invalid(
                    "cache slot does not hold the expected merge output",
                ));
            }
        }
        Ok(())
    }

    pub(super) fn check_cached_transaction(
        &self,
        pending: &PreparedTransaction,
    ) -> Result<(), WalletError> {
        if let Some(output) = &pending.cached_output {
            output.target.validate()?;
            if let VersionedMessage::V1(message) = &pending.message {
                let data = message
                    .instructions
                    .get(1)
                    .and_then(|ix| ix.data.get(1..))
                    .ok_or_else(|| invalid("invalid cached merge message"))?;
                let merge = zolana_interface::instruction::MergeTransactIxData::deserialize(data)
                    .map_err(|_| invalid("invalid cached merge message"))?;
                if merge.expiry_unix_ts <= now() as u64 {
                    return Err(invalid("merge proof expired; prepare again"));
                }
            }
        }
        if !pending.cache_inputs.is_empty() {
            self.verify_cached_outputs(pending.cache_inputs.clone())?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests;
