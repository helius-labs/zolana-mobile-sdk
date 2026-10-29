//! Cache-targeted merges and authenticated restart state. The cache is an
//! acceleration path: the same merge output is also published to the tree.
use super::*;
use hmac::{Hmac, Mac};
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use zolana_client::{
    prover::MergeCacheTarget, MergeProver, ProofCompressed, ProverExt, WitnessReader,
};
use zolana_interface::{
    instruction::instruction_data::CreateCacheData,
    state::cache::{CacheAccount, CACHE_CAPACITY},
};
use zolana_program::instruction::{CacheWriteAccounts, CreateCache, MergeTransact};
use zolana_transaction::{Data, Utxo};
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

pub(super) fn invalid(message: &str) -> WalletError {
    WalletError::Client {
        message: message.to_owned(),
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
    pub fn prepare_cached_merge(
        &mut self,
        target: CacheTarget,
        mint: Option<String>,
        max_inputs: Option<u32>,
        proving: Option<Proving>,
        priority_fee_lamports: Option<u64>,
    ) -> Result<PreparedTransaction, WalletError> {
        target.validate()?;
        let fee = priority_fee_lamports.or(self.priority_fee).unwrap_or(0);
        if fee > target.max_priority_fee_lamports {
            return Err(invalid("priority fee exceeds sponsored run limit"));
        }
        let proving = self.proving(proving)?;
        let payer = parse_pubkey(&target.sponsor)?;
        let asset = self.asset(mint)?;
        let record = fetch_user_record_optional_checked(&self.client, self.owner)?
            .ok_or(WalletError::MergingDisabled)?;
        if record.merge_encryption != MergeEncryptionScheme::PoseidonCtrV1 {
            return Err(WalletError::MergingDisabled);
        }
        if record.nullifier_pubkey != self.address.nullifier_pubkey
            || record.viewing_pubkey != *self.address.viewing_pubkey.as_bytes()
        {
            return Err(WalletError::RegistrationConflict {
                owner: self.owner.to_string(),
            });
        }
        let reserved = self.reserved()?;
        let notes: Vec<_> = self
            .recent_notes()?
            .utxos()
            .filter(|note| note.tree_id() == target.tree_id)
            .cloned()
            .collect();
        // The pinned example fits 51 real inputs with CreateCache in a 4096-byte
        // v1 message. Also check the actual compiled transaction below.
        let cap = max_inputs.unwrap_or(24).min(51) as usize;
        let inputs = select_merge(notes.iter(), asset.mint, cap, &reserved)?;
        let spends = nullifiers(&inputs);
        let expiry = merge_expiry().min(target.expires_at as u64);
        let merge = MergeTransaction::new(inputs)?
            .with_output_tree_id(target.tree_id)
            .with_expiry(expiry)
            .encrypt(&self.keys)?;
        let cached = CachedOutput {
            target: target.clone(),
            owner: self.owner.to_string(),
            mint: mint_name(&asset.mint),
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
        verify_merge(&result, &proof)?;
        let merge_ix = MergeTransact {
            input_tree: pda::tree(target.tree_id),
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
                "cached merge exceeds transaction limits; reduce maxInputs",
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
            if cache.utxo_hashes[usize::from(output.target.slot)] != output.commitment {
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

    /// Versioned, wallet-authenticated snapshot. Contains private amounts and
    /// blindings: store securely with the run and submission signatures.
    pub fn export_prepared_transaction(
        &self,
        pending: &PreparedTransaction,
    ) -> Result<String, WalletError> {
        let state = SavedTransaction {
            version: 2,
            operation: pending.operation.clone(),
            message: pending.message.serialize(),
            last_valid_block_height: pending.last_valid_block_height,
            spends: pending.spends.clone(),
            outputs: pending.outputs.clone(),
            cached_output: pending.cached_output.clone(),
            cache_inputs: pending.cache_inputs.clone(),
        };
        let payload = serde_json::to_string(&state)
            .map_err(|_| invalid("cannot serialize prepared transaction"))?;
        let mut mac = self.state_mac();
        mac.update(payload.as_bytes());
        serde_json::to_string(&Envelope {
            payload,
            tag: mac.finalize().into_bytes().to_vec(),
        })
        .map_err(|_| invalid("cannot serialize prepared transaction"))
    }

    /// Restore before allowing new spends. Restored reservations stay until
    /// explicitly released or the transaction outcome is settled; expiry alone
    /// does not establish that an interrupted submission failed.
    pub fn restore_prepared_transaction(
        &self,
        snapshot: String,
    ) -> Result<PreparedTransaction, WalletError> {
        if snapshot.len() > 1_000_000 {
            return Err(invalid("snapshot too large"));
        }
        let envelope: Envelope =
            serde_json::from_str(&snapshot).map_err(|_| invalid("invalid snapshot"))?;
        let mut mac = self.state_mac();
        mac.update(envelope.payload.as_bytes());
        mac.verify_slice(&envelope.tag)
            .map_err(|_| invalid("snapshot authentication failed"))?;
        let state: SavedTransaction =
            serde_json::from_str(&envelope.payload).map_err(|_| invalid("invalid snapshot"))?;
        if state.version != 2 {
            return Err(invalid("unsupported snapshot version"));
        }
        let message: VersionedMessage = wincode::deserialize_exact(&state.message)
            .map_err(|_| invalid("invalid saved message"))?;
        let mut pending = self.proved(
            state.operation,
            UnsignedMessage {
                message,
                last_valid_block_height: state.last_valid_block_height,
            },
        );
        pending.outputs = state.outputs;
        pending.cached_output = state.cached_output;
        pending.cache_inputs = state.cache_inputs;
        self.reserve_until(pending, state.spends, u64::MAX)
    }

    fn state_mac(&self) -> Hmac<Sha256> {
        let mut mac = Hmac::<Sha256>::new_from_slice(self.nullifier_key.secret().as_ref())
            .expect("HMAC accepts all key lengths");
        mac.update(b"zolana-mobile/prepared/v2/8ae430a1");
        mac.update(&self.owner.to_bytes());
        mac.update(self.recovery_domain.as_bytes());
        mac
    }
}

#[derive(Serialize, Deserialize)]
struct Envelope {
    payload: String,
    tag: Vec<u8>,
}
#[derive(Serialize, Deserialize)]
struct SavedTransaction {
    version: u8,
    operation: Operation,
    message: Vec<u8>,
    last_valid_block_height: u64,
    spends: Vec<[u8; 32]>,
    outputs: Vec<OutputOpening>,
    cached_output: Option<CachedOutput>,
    cache_inputs: Vec<CachedOutput>,
}

// Use the pinned upstream verifier for the locally computed merge statement.
fn verify_merge(
    result: &zolana_client::MergeProofResult,
    proof: &zolana_client::Proof,
) -> Result<(), WalletError> {
    use zolana_interface::verifying_keys::{merge_24_1, merge_52_1, merge_8_1};
    let key = match result.inputs.inputs.len() {
        8 => &merge_8_1::VERIFYINGKEY,
        24 => &merge_24_1::VERIFYINGKEY,
        52 => &merge_52_1::VERIFYINGKEY,
        _ => return Err(WalletError::UnsupportedCircuit),
    };
    zolana_client::prover::verify_proof_statement(proof, result.public_input_hash, key)
        .map_err(Into::into)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::wallet::tests::{open, unsigned};
    use solana_keypair::Keypair;
    use solana_signer::Signer;

    fn target() -> CacheTarget {
        CacheTarget {
            sponsor: Pubkey::new_unique().to_string(),
            nonce: 7,
            tree_id: 0,
            expires_at: now() + 600,
            slot: 0,
            max_priority_fee_lamports: 5_000,
            compute_unit_limit: 400_000,
        }
    }

    #[test]
    fn cache_terms_fail_before_network_or_proof() {
        let mut wallet = open(&Keypair::new()).unwrap();
        let mut terms = target();
        terms.slot = CACHE_CAPACITY as u8;
        assert!(wallet
            .prepare_cached_merge(terms, None, None, None, None)
            .is_err());
        let mut terms = target();
        terms.expires_at = now() - 1;
        assert!(wallet
            .prepare_cached_merge(terms, None, None, None, None)
            .is_err());
        assert!(wallet
            .prepare_cached_merge(target(), None, None, None, Some(5_001))
            .is_err());
    }

    #[test]
    fn snapshots_authenticate_message_metadata_wallet_and_restore_reservations() {
        let signer = Keypair::new();
        let wallet = open(&signer).unwrap();
        let mut pending = unsigned(&signer);
        pending.spends = vec![[9; 32]];
        pending.last_valid_block_height = 123;
        let snapshot = wallet.export_prepared_transaction(&pending).unwrap();
        let reopened = open(&signer).unwrap();
        let restored = reopened
            .restore_prepared_transaction(snapshot.clone())
            .unwrap();
        assert_eq!(restored.message(), pending.message());
        assert_eq!(restored.last_valid_block_height(), 123);
        {
            let held = reopened.reservations();
            assert_eq!(held.reserved[&restored.id].spends, vec![[9; 32]]);
            assert_eq!(held.reserved[&restored.id].until, u64::MAX);
        }
        let duplicate = reopened
            .restore_prepared_transaction(snapshot.clone())
            .unwrap();
        assert_eq!(duplicate.id, restored.id);
        assert_eq!(reopened.reservations().reserved.len(), 1);
        let other = open(&Keypair::new()).unwrap();
        assert!(other
            .restore_prepared_transaction(snapshot.clone())
            .is_err());
        let mut envelope: Envelope = serde_json::from_str(&snapshot).unwrap();
        envelope.payload = envelope.payload.replace("123", "124");
        assert!(reopened
            .restore_prepared_transaction(serde_json::to_string(&envelope).unwrap())
            .is_err());
        // A different signed message may spend the same inputs. Releasing
        // the first must not release the second after a restart.
        pending.message.set_recent_blockhash([8; 32].into());
        let other_message = reopened
            .restore_prepared_transaction(wallet.export_prepared_transaction(&pending).unwrap())
            .unwrap();
        assert_ne!(other_message.id, restored.id);
        reopened.release(&restored);
        assert_eq!(reopened.reservations().reserved.len(), 1);
        reopened.release(&other_message);
        assert!(reopened.reservations().reserved.is_empty());
    }

    #[test]
    fn cache_reads_check_owner_terms_commitments_and_duplicate_slots() {
        use base64::{engine::general_purpose::STANDARD, Engine};
        use bytemuck::Zeroable;
        use serde_json::json;
        let signer = Keypair::new();
        let identity = open(&signer).unwrap();
        let terms = target();
        let mint = zolana_transaction::Mint::SOL;
        let utxo = Utxo {
            owner: identity.address.signing_pubkey,
            asset: mint,
            amount: 50,
            blinding: [2; 32],
            ring_program_id: None,
            data: Data::default(),
        };
        let commitment = utxo
            .hash(&identity.address.nullifier_pubkey, &[0; 32], &[0; 32], 0)
            .unwrap();
        let output = CachedOutput {
            target: terms.clone(),
            owner: signer.pubkey().to_string(),
            mint: None,
            asset_id: mint.asset_id,
            amount: 50,
            blinding: [2; 32],
            commitment,
        };
        let mut cache = CacheAccount::zeroed();
        cache.discriminator = zolana_interface::state::discriminator::CACHE;
        cache.rent_sponsor = parse_pubkey(&terms.sponsor).unwrap();
        cache.write_authority = cache.rent_sponsor;
        cache.expires_at = terms.expires_at.to_le_bytes();
        cache.utxo_hashes[0] = commitment;
        let value = json!({"context":{"slot":1},"value":{"data":[STANDARD.encode(bytemuck::bytes_of(&cache)),"base64"],
            "owner":PROGRAM_ID_PUBKEY.to_string(),"lamports":1,"executable":false,"rentEpoch":0}});
        let response = Arc::new(Mutex::new(value.clone()));
        let shared = response.clone();
        let (wallet, _) = crate::wallet::tests::rpc_wallet(&signer, move |method| {
            if method == "getBlockHeight" {
                return json!({"result":1});
            }
            assert_eq!(method, "getAccountInfo");
            json!({"result":shared.lock().unwrap().clone()})
        });
        wallet.verify_cached_outputs(vec![output.clone()]).unwrap();
        assert!(wallet
            .verify_cached_outputs(vec![output.clone(), output.clone()])
            .is_err());
        let mut changed = output.clone();
        changed.commitment[0] ^= 1;
        assert!(wallet.verify_cached_outputs(vec![changed]).is_err());
        response.lock().unwrap()["value"]["owner"] = json!(Pubkey::new_unique().to_string());
        assert!(wallet.verify_cached_outputs(vec![output.clone()]).is_err());
        *response.lock().unwrap() = value;
        let mut pending = unsigned(&signer);
        pending.cached_output = Some(output.clone());
        let pending = wallet.reserve(pending, vec![[9; 32]]).unwrap();
        let predicted = wallet.cached_note(&output).unwrap().nullifier;
        assert!(wallet.reserved().unwrap().contains(&predicted));
        // Only the real input is a spend reservation: the prediction can be
        // consumed explicitly from cache, but not selected again from the tree.
        assert!(!wallet.reservations().reserved[&pending.id]
            .spends
            .contains(&predicted));
        wallet.release(&pending);
        assert!(!wallet.reserved().unwrap().contains(&predicted));
    }

    #[test]
    fn snapshots_preserve_cache_prediction_and_run_terms() {
        let signer = Keypair::new();
        let wallet = open(&signer).unwrap();
        let mut pending = unsigned(&signer);
        let mint = zolana_transaction::Mint::SOL;
        let utxo = Utxo {
            owner: wallet.address.signing_pubkey,
            asset: mint,
            amount: 42,
            blinding: [3; 32],
            ring_program_id: None,
            data: Data::default(),
        };
        let commitment = utxo
            .hash(&wallet.address.nullifier_pubkey, &[0; 32], &[0; 32], 0)
            .unwrap();
        pending.cached_output = Some(CachedOutput {
            target: target(),
            owner: signer.pubkey().to_string(),
            mint: None,
            asset_id: mint.asset_id,
            amount: 42,
            blinding: [3; 32],
            commitment,
        });
        let state = wallet.export_prepared_transaction(&pending).unwrap();
        let reopened = open(&signer).unwrap();
        let restored = reopened.restore_prepared_transaction(state).unwrap();
        let output = restored.cached_output().unwrap();
        assert_eq!(output.amount(), 42);
        assert_eq!(output.commitment(), commitment.to_vec());
        assert_eq!(output.target(), pending.cached_output().unwrap().target());
        reopened.release(&restored);
        pending.cached_output.as_mut().unwrap().commitment[0] ^= 1;
        assert!(reopened
            .restore_prepared_transaction(wallet.export_prepared_transaction(&pending).unwrap())
            .is_err());
        assert!(reopened.reservations().reserved.is_empty());
    }
}
