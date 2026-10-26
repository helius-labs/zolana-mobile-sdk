use std::sync::{Arc, Mutex};

use base64::{engine::general_purpose::STANDARD, Engine};
use serde_json::{json, Value};
use solana_keypair::Keypair;
use solana_signer::Signer;
use zolana_client::rpc::{OutputContext, OutputSlot};
use zolana_interface::instruction::{CircuitId, InputUtxo, TransactProof, TreeContext};
use zolana_program::instruction::Transact;
use zolana_transaction::{AssetRegistry, History, Mint, Utxo};

use super::*;
use crate::{ActivityKind, WalletConfig};

fn config() -> WalletConfig {
    WalletConfig {
        rpc_url: "https://rpc.example".into(),
        indexer_url: "https://indexer.example".into(),
        proving_key_dir: "/unused".into(),
        proving_key_url: None,
        proving: None,
        prover_url: None,
        max_spend_inputs: None,
        reuse_notes_ms: None,
        mints: None,
    }
}

fn wallet(
    signer: &Keypair,
    handler: impl Fn(&str, &Value) -> Value + Send + Sync + 'static,
) -> (MobileWallet, Arc<Mutex<Vec<String>>>) {
    let asked = Arc::new(Mutex::new(Vec::new()));
    let calls = Arc::clone(&asked);
    let (transport, _) = crate::transport::tests::fake(move |request| {
        let request: Value = serde_json::from_slice(&request.body).unwrap();
        let method = request["method"].as_str().unwrap();
        calls.lock().unwrap().push(method.to_owned());
        let mut answer = handler(method, &request["params"]);
        answer["jsonrpc"] = json!("2.0");
        answer["id"] = request["id"].clone();
        crate::transport::tests::ok(&answer.to_string())
    });
    let key = signer.pubkey().to_string();
    let signature = signer.sign_message(&derivation_message(key.clone()).unwrap());
    (
        MobileWallet::open(config(), key, signature.as_ref().to_vec(), transport).unwrap(),
        asked,
    )
}

fn identity(signer: &Keypair) -> MobileWallet {
    wallet(signer, |method, _| panic!("unexpected request: {method}")).0
}

struct Fixture {
    signature: Signature,
    chain: Value,
    events: Vec<ShieldedTransaction>,
    registry: AssetRegistry,
    notes: Vec<WalletUtxo>,
    records: HashMap<Pubkey, zolana_user_registry_interface::UserRecord>,
}

impl Fixture {
    /// Real encrypted default-ring spend, with known source notes only in the
    /// fixture. The lookup receives neither those notes nor any history page.
    fn spend(
        sender: &MobileWallet,
        recipient: &MobileWallet,
        private: u64,
        withdrawal: u64,
        assets: &[Mint],
    ) -> Self {
        let payer = sender.owner;
        let mut registry = AssetRegistry::default();
        for mint in assets {
            if mint.asset != SOL_MINT {
                registry.insert(mint.asset_id, mint.asset).unwrap();
            }
        }
        let mut notes = Vec::new();
        for (index, asset) in assets.iter().enumerate() {
            let utxo = Utxo {
                owner: sender.address.signing_pubkey,
                asset: *asset,
                amount: 100,
                blinding: [index as u8 + 1; 32],
                ring_program_id: None,
                data: Default::default(),
            };
            let hash = utxo
                .hash(&sender.address.nullifier_pubkey, &[0; 32], &[0; 32], 0)
                .unwrap();
            let nullifier = utxo.nullifier(&hash, &sender.nullifier_key).unwrap();
            notes.push(WalletUtxo {
                utxo,
                nullifier_pubkey: sender.address.nullifier_pubkey,
                utxo_hash: hash,
                nullifier,
                tx_viewing_key: None,
                data_hash: None,
                ring_data_hash: None,
                tree_id: 0,
                leaf_index: index as u64,
                latest_tree_id: None,
                slot: 1,
                tx_signature: Signature::from([1; 64]),
                slot_index: index as u32,
            });
        }
        let mut transaction = ConfidentialTransaction::new(notes.clone(), payer).unwrap();
        let mut settlements = Vec::new();
        for asset in assets {
            if private > 0 {
                if asset.asset == SOL_MINT {
                    transaction
                        .transfer_sol(&recipient.address, private)
                        .unwrap();
                } else {
                    transaction
                        .transfer(&recipient.address, asset.asset, private)
                        .unwrap();
                }
            }
            if withdrawal > 0 {
                settlements.push(
                    transaction
                        .withdraw_to(
                            asset.asset,
                            withdrawal,
                            recipient.owner,
                            (asset.asset != SOL_MINT).then(pda::spl_token_program_id),
                        )
                        .unwrap(),
                );
            }
        }
        let inputs = transaction.encrypt(&sender.keys).unwrap();
        let tree_id = inputs.output_tree_id;
        // Read back only wire-visible data, just as the indexer does.
        let data = TransactIxData {
            expiry_unix_ts: u64::MAX,
            tx_viewing_pk: inputs.external_data.tx_viewing_pk,
            salt: inputs.external_data.salt,
            interface_transfers: inputs
                .external_data
                .interface_transfers
                .iter()
                .map(|leg| leg.interface_transfer())
                .collect(),
            data_hash: None,
            ring_data_hash: None,
            outputs: inputs.external_data.outputs.clone(),
            messages: vec![],
            private_tx_hash: [0; 32],
            circuit: CircuitId::ConfidentialEddsa(
                inputs.input_utxos.len() as u8,
                inputs.output_utxos.len() as u8,
                3,
            ),
            proof: TransactProof::zeroed(),
            inputs: inputs
                .input_utxos
                .iter()
                .filter(|input| !input.compact)
                .map(|input| InputUtxo {
                    nullifier_hash: input.nullifier(),
                    tree_index: 0,
                })
                .collect(),
            tree_contexts: vec![TreeContext {
                utxo_tree_root_index: 0,
                nullifier_tree_root_index: 0,
            }],
        };
        let builder = Transact {
            payer,
            input_trees: vec![pda::tree(0)],
            output_tree: pda::tree(tree_id),
            owner_signers: vec![sender.owner],
            interface_transfer_accounts: settlements,
            data,
        };
        let ix = builder.instruction();
        let signature = Signature::from([12; 64]);
        let message = compile_message(
            &payer,
            &[ix.clone()],
            Default::default(),
            ComputeBudgetConfig::new(1_000_000),
        )
        .unwrap();
        let signed = VersionedTransaction {
            signatures: vec![signature],
            message,
        };
        let chain = json!({"slot": 10, "blockTime": 0, "version": 1,
            "transaction": {"signatures": [signature.to_string()], "message": {
                "header": signed.message.header(),
                "accountKeys": signed.message.static_account_keys().iter().map(ToString::to_string).collect::<Vec<_>>(),
                "recentBlockhash": signed.message.recent_blockhash().to_string(),
                "instructions": signed.message.instructions().iter().map(|ix| solana_transaction_status_client_types::UiCompiledInstruction::from(ix, None)).collect::<Vec<_>>(),
                "addressTableLookups": []
            }},
            "meta": {"err": null, "status": {"Ok": null}, "fee": 5000,
                "preBalances": [], "postBalances": [], "innerInstructions": [], "logMessages": [],
                "preTokenBalances": [], "postTokenBalances": [], "rewards": []}
        });
        let event = ShieldedTransaction {
            slot: 10,
            tx_signature: signature,
            event_index: Some(0),
            tx_viewing_pk: Some(
                zolana_keypair::X25519Pubkey::from_bytes(builder.data.tx_viewing_pk).unwrap(),
            ),
            salt: Some(builder.data.salt),
            nullifiers: builder
                .data
                .inputs
                .iter()
                .map(|input| input.nullifier_hash)
                .collect(),
            output_slots: builder
                .data
                .outputs
                .iter()
                .enumerate()
                .map(|(index, output)| OutputSlot {
                    view_tag: fetch_tag(&output.owner_tag, |i| {
                        ix.accounts
                            .get(usize::from(i))
                            .map(|account| account.pubkey.to_bytes())
                    })
                    .unwrap(),
                    output_context: OutputContext {
                        hash: output.utxo_hash,
                        tree_id,
                        leaf_index: 100 + index as u64,
                    },
                    payload: output.data.clone().unwrap_or_default(),
                })
                .collect(),
            messages: vec![],
            proofless: false,
            ring_config: None,
            ring_program_id: None,
        };
        let record = zolana_user_registry_interface::UserRecord {
            owner: recipient.owner,
            bump: 0,
            nullifier_pubkey: recipient.address.nullifier_pubkey,
            viewing_pubkey: *recipient.address.viewing_pubkey.as_bytes(),
            merge_encryption: MergeEncryptionScheme::Disabled,
        };
        Self {
            signature,
            chain,
            events: vec![event],
            registry,
            notes,
            records: HashMap::from([(recipient.owner, record)]),
        }
    }

    fn indexer(&self, slot: u64) -> Value {
        json!({"context": {"slot": slot, "blockTime": 0}, "transactions": self.events.iter().enumerate().map(|(index, tx)| json!({
            "eventIndex": index, "transaction": {
                "slot": tx.slot, "txSignature": tx.tx_signature.to_string(), "eventIndex": tx.event_index,
                "txViewingPk": tx.tx_viewing_pk.map(|key| STANDARD.encode(key.as_bytes())),
                "salt": tx.salt.map(|salt| STANDARD.encode(salt)), "proofless": tx.proofless,
                "nullifiers": tx.nullifiers.iter().map(|hash| zolana_api::Hash::from(*hash).to_base58()).collect::<Vec<_>>(),
                "messages": [], "outputSlots": tx.output_slots.iter().map(|slot| json!({
                    "viewTag": zolana_api::Hash::from(slot.view_tag).to_base58(), "payload": STANDARD.encode(&slot.payload),
                    "outputContext": {"hash": zolana_api::Hash::from(slot.output_context.hash).to_base58(),
                        "tree": pda::tree(slot.output_context.tree_id).to_string(), "treeId": slot.output_context.tree_id,
                        "leafIndex": slot.output_context.leaf_index}
                })).collect::<Vec<_>>()
            }
        })).collect::<Vec<_>>()})
    }

    fn open(self, signer: &Keypair) -> (MobileWallet, Arc<Mutex<Vec<String>>>) {
        self.open_with_registry(signer, true)
    }

    fn open_with_registry(
        self,
        signer: &Keypair,
        preload: bool,
    ) -> (MobileWallet, Arc<Mutex<Vec<String>>>) {
        let registry = self.registry.clone();
        let indexed = self.indexer(10);
        let (wallet, calls) = wallet(signer, move |method, params| match method {
            "getShieldedTransactionsBySignature" => json!({"result": indexed}),
            "getTransaction" => json!({"result": self.chain}),
            "getProgramAccounts" => {
                json!({"result": self.notes.iter().filter(|note| note.utxo.asset.asset != SOL_MINT).map(|note| {
                let mint = note.utxo.asset;
                json!({"pubkey": pda::spl_asset_registry(&mint.asset).to_string(), "account": {
                    "owner": PROGRAM_ID_PUBKEY.to_string(), "lamports": 1000000, "executable": false, "rentEpoch": 0,
                    "data": [STANDARD.encode(zolana_interface::state::SplAssetRegistry::account_bytes(mint.asset, mint.asset_id)), "base64"]
                }})
            }).collect::<Vec<_>>()})
            }
            "getUserRecords" => {
                let records = params["owners"].as_array().unwrap().iter().map(|owner| {
                    let owner = Pubkey::from_str(owner.as_str().unwrap()).unwrap();
                    self.records.get(&owner).map(|record| json!({"owner": record.owner.to_string(),
                        "nullifierPubkey": zolana_api::Hash::from(record.nullifier_pubkey).to_base58(),
                        "viewingPubkey": STANDARD.encode(record.viewing_pubkey), "mergeEncryption": 0}))
                }).collect::<Vec<_>>();
                json!({"result": {"context": {"slot": 10, "blockTime": 0}, "records": records}})
            }
            _ => panic!("lookup must not fetch history: {method}"),
        });
        if preload {
            wallet.assets().remember_registry(registry);
        }
        (wallet, calls)
    }
}

#[test]
fn cold_lookup_matches_history_for_sends_withdrawals_and_multiple_assets() {
    let sender = Keypair::new();
    let recipient = Keypair::new();
    let owner = identity(&sender);
    let recipient_wallet = identity(&recipient);
    for (private, withdrawal) in [(20, 0), (0, 100), (10, 20)] {
        for assets in [
            vec![Mint::SOL],
            vec![Mint::SOL, Mint::new(Pubkey::new_unique(), 7)],
        ] {
            let fixture = Fixture::spend(&owner, &recipient_wallet, private, withdrawal, &assets);
            let decoded = decrypt(&owner.keys, &fixture.events, &fixture.registry).unwrap();
            let mut utxos = fixture.notes.clone();
            utxos.extend(verify_owned(&owner.keys, &decoded).unwrap().utxos);
            let expected = activity::entries(
                &History {
                    transactions: fixture.events.clone(),
                    utxos,
                    view_tags: [owner
                        .address
                        .signing_pubkey
                        .confidential_view_tag()
                        .unwrap()]
                    .into_iter()
                    .collect(),
                    ..Default::default()
                },
                &fixture.registry,
            )
            .unwrap();
            let confirmed: Confirmed = serde_json::from_value(fixture.chain.clone()).unwrap();
            let groups = ConfirmedInstructionGroups::try_from(confirmed)
                .expect("fixture instruction groups");
            assert!(
                transact_context(&fixture.events[0], &groups).is_some(),
                "fixture event matches its instruction"
            );
            let signature = fixture.signature.to_string();
            let (wallet, asked) = fixture.open(&sender);
            let result = wallet.transaction(signature).unwrap();
            assert_eq!(
                result.status,
                Status::Ready,
                "private={private}, withdrawal={withdrawal}, assets={assets:?}, calls={:?}",
                asked.lock().unwrap()
            );
            assert_eq!(result.entries, expected);
            assert!(!result.entries.is_empty());
            assert_eq!(result.indexed_slot, 10);
            assert!(asked.lock().unwrap().iter().all(|method| [
                "getTransaction",
                "getShieldedTransactionsBySignature",
                "getUserRecords"
            ]
            .contains(&method.as_str())));
        }
    }
}

#[test]
fn incoming_unrelated_and_self_transfers_are_read_without_history() {
    let sender = Keypair::new();
    let recipient = Keypair::new();
    let owner = identity(&sender);
    let other = identity(&recipient);
    let fixture = Fixture::spend(&owner, &other, 20, 0, &[Mint::SOL]);
    let signature = fixture.signature.to_string();
    let (wallet, _) = fixture.open(&recipient);
    let incoming = wallet.transaction(signature).unwrap();
    assert_eq!(incoming.status, Status::Ready);
    assert_eq!(incoming.entries.len(), 1);
    assert_eq!(
        (incoming.entries[0].kind, incoming.entries[0].amount),
        (ActivityKind::Received, 20)
    );

    let fixture = Fixture::spend(&owner, &other, 20, 0, &[Mint::SOL]);
    let signature = fixture.signature.to_string();
    let (wallet, _) = fixture.open(&Keypair::new());
    let unrelated = wallet.transaction(signature).unwrap();
    assert_eq!(unrelated.status, Status::Ready);
    assert!(unrelated.entries.is_empty());

    let fixture = Fixture::spend(&owner, &owner, 20, 0, &[Mint::SOL]);
    let signature = fixture.signature.to_string();
    let (wallet, _) = fixture.open(&sender);
    let own = wallet.transaction(signature).unwrap();
    assert_eq!(own.status, Status::Ready);
    assert_eq!(
        (own.entries[0].kind, own.entries[0].amount),
        (ActivityKind::SelfTransfer, 100)
    );
}

#[test]
fn indexing_delay_not_found_and_indexed_empty_are_distinct() {
    let signer = Keypair::new();
    for (chain_slot, indexed_slot, expected) in [
        (Some(10), 9, Status::PendingIndexing),
        (Some(10), 10, Status::Ready),
        (None, 10, Status::NotFound),
    ] {
        let owner = identity(&signer);
        let mut fixture = Fixture::spend(&owner, &owner, 20, 0, &[Mint::SOL]);
        fixture.events.clear();
        let indexed = fixture.indexer(indexed_slot);
        let chain = chain_slot.map(|_| fixture.chain).unwrap_or(Value::Null);
        let (wallet, asked) = wallet(&signer, move |method, _| match method {
            "getShieldedTransactionsBySignature" => json!({"result": indexed}),
            "getTransaction" => json!({"result": chain}),
            _ => panic!("unexpected request: {method}"),
        });
        let result = wallet
            .transaction(Signature::from([12; 64]).to_string())
            .unwrap();
        assert_eq!(result.status, expected);
        assert_eq!(result.slot, chain_slot);
        assert!(result.entries.is_empty());
        assert_eq!(asked.lock().unwrap().len(), 2);
    }
}

#[test]
fn bad_signatures_and_network_failures_are_not_empty_results() {
    let signer = Keypair::new();
    let (wallet, asked) = wallet(
        &signer,
        |_, _| json!({"error": {"code": -32603, "message": "unavailable"}}),
    );
    assert_eq!(
        wallet.transaction("not-a-signature".into()),
        Err(WalletError::SignatureInvalid)
    );
    assert!(asked.lock().unwrap().is_empty());
    assert!(matches!(
        wallet.transaction(Signature::from([12; 64]).to_string()),
        Err(WalletError::Client { .. })
    ));
}

#[test]
fn missing_metadata_and_corrupt_ciphertexts_never_return_partial_rows() {
    let sender = Keypair::new();
    let recipient = Keypair::new();
    let owner = identity(&sender);
    let other = identity(&recipient);
    for mutation in [0, 1, 2] {
        let mut fixture = Fixture::spend(&owner, &other, 20, 0, &[Mint::SOL]);
        match mutation {
            0 => fixture.chain = Value::Null,
            1 => fixture.events[0].output_slots[0].payload[10] ^= 1,
            _ => fixture.records.clear(),
        }
        let signature = fixture.signature.to_string();
        let (wallet, _) = fixture.open(&sender);
        let result = wallet.transaction(signature).unwrap();
        assert_eq!(result.status, Status::Incomplete, "mutation {mutation}");
        assert!(result.entries.is_empty());
    }
    // A recipient can still authenticate its received notes when the chain
    // RPC has pruned the metadata required for sender-side settlements.
    let mut fixture = Fixture::spend(&owner, &other, 20, 0, &[Mint::SOL]);
    fixture.chain = Value::Null;
    let signature = fixture.signature.to_string();
    let (wallet, _) = fixture.open(&recipient);
    let result = wallet.transaction(signature).unwrap();
    assert_eq!(result.status, Status::Ready);
    assert_eq!(result.entries[0].amount, 20);
}

#[test]
fn deposit_events_are_aggregated_deduplicated_and_have_stable_asset_ids() {
    use zolana_transaction::serialization::{
        OwnerCx, Proofless, ProoflessEncode, UtxoSerialization,
    };
    let signer = Keypair::new();
    let owner = identity(&signer);
    let token = Mint::new(Pubkey::new_unique(), 7);
    let mut fixture = Fixture::spend(&owner, &owner, 20, 0, &[Mint::SOL, token]);
    let tag = owner
        .address
        .signing_pubkey
        .confidential_view_tag()
        .unwrap();
    let mut slots = Vec::new();
    for (index, asset) in [Mint::SOL, token].iter().enumerate() {
        let utxo = Utxo {
            owner: owner.address.signing_pubkey,
            asset: *asset,
            amount: 5 + index as u64,
            blinding: [index as u8; 32],
            ring_program_id: None,
            data: Default::default(),
        };
        let hash = utxo
            .hash(&owner.address.nullifier_pubkey, &[0; 32], &[0; 32], 0)
            .unwrap();
        let encoded = Proofless::encode(
            &[utxo],
            &OwnerCx {
                owner: owner.address.signing_pubkey,
                assets: &fixture.registry,
                ring_program_id: None,
                first_nullifier: None,
            },
            tag,
            &ProoflessEncode {
                owner_hash: owner.address.owner_hash().unwrap(),
                data_hash: None,
                ring_data_hash: None,
            },
        )
        .unwrap();
        slots.push(OutputSlot {
            view_tag: tag,
            output_context: OutputContext {
                hash,
                tree_id: 0,
                leaf_index: 100 + index as u64,
            },
            payload: encoded.data,
        });
    }
    fixture.events[0].output_slots = slots;
    fixture.events[0].proofless = true;
    fixture.events[0].tx_viewing_pk = None;
    fixture.events[0].salt = None;
    fixture.events[0].nullifiers.clear();
    fixture.chain = Value::Null;
    let mut duplicate = fixture.events[0].clone();
    duplicate.event_index = Some(1);
    fixture.events.push(duplicate);
    let signature = fixture.signature.to_string();
    let (wallet, _) = fixture.open(&signer);
    let result = wallet.transaction(signature).unwrap();
    assert_eq!(result.status, Status::Ready);
    assert_eq!(result.entries.len(), 2);
    assert_eq!(
        result
            .entries
            .iter()
            .map(|entry| (entry.index, entry.amount))
            .collect::<Vec<_>>(),
        vec![(1, 5), (7, 6)]
    );
    assert!(result
        .entries
        .iter()
        .all(|entry| entry.kind == ActivityKind::Deposit));
}

#[test]
fn a_cold_wallet_reads_a_merge_as_a_self_transfer() {
    use zolana_interface::event::{encode_merge_output_data, MergeOutputDataRef};
    use zolana_transaction::instructions::merge::{
        merge_output_blinding, merge_output_key, MergeOutputPlaintext,
    };
    let signer = Keypair::new();
    let owner = identity(&signer);
    let mut fixture = Fixture::spend(&owner, &owner, 20, 0, &[Mint::SOL]);
    let first = [1; 32];
    let salt = [3; 16];
    let key = merge_output_key(&owner.nullifier_key, &first).unwrap();
    let blinding = merge_output_blinding(&owner.nullifier_key, &first).unwrap();
    let plaintext = MergeOutputPlaintext {
        amount: 50,
        mint: SOL_MINT,
    };
    let ciphertext = plaintext.encrypt(&key, &salt).unwrap();
    let utxo = plaintext.into_utxo(owner.address.signing_pubkey, Mint::SOL, blinding, None);
    let hash = utxo
        .hash(&owner.address.nullifier_pubkey, &[0; 32], &[0; 32], 0)
        .unwrap();
    fixture.events[0].output_slots = vec![OutputSlot {
        view_tag: owner
            .address
            .signing_pubkey
            .confidential_view_tag()
            .unwrap(),
        output_context: OutputContext {
            hash,
            tree_id: 0,
            leaf_index: 100,
        },
        payload: encode_merge_output_data(MergeOutputDataRef {
            salt: &salt,
            ciphertext: &ciphertext,
            prefix: &[],
        }),
    }];
    fixture.events[0].tx_viewing_pk = None;
    fixture.events[0].salt = None;
    fixture.events[0].nullifiers = vec![first];
    fixture.chain = Value::Null;
    let mut corrupt = Fixture {
        signature: fixture.signature,
        chain: fixture.chain.clone(),
        events: fixture.events.clone(),
        registry: fixture.registry.clone(),
        notes: fixture.notes.clone(),
        records: fixture.records.clone(),
    };
    corrupt.events[0].output_slots[0].output_context.hash = [0; 32];
    let signature = fixture.signature.to_string();
    let (wallet, _) = fixture.open(&signer);
    let result = wallet.transaction(signature.clone()).unwrap();
    assert_eq!(result.status, Status::Ready);
    assert_eq!(
        (result.entries[0].kind, result.entries[0].amount),
        (ActivityKind::SelfTransfer, 50)
    );
    let (wallet, _) = corrupt.open(&signer);
    let result = wallet.transaction(signature).unwrap();
    assert_eq!(result.status, Status::Incomplete);
    assert!(result.entries.is_empty());
}

#[test]
fn a_full_private_send_with_recipient_tagged_padding_is_explicitly_incomplete() {
    let sender = Keypair::new();
    let recipient = Keypair::new();
    let fixture = Fixture::spend(
        &identity(&sender),
        &identity(&recipient),
        100,
        0,
        &[Mint::SOL],
    );
    let signature = fixture.signature.to_string();
    let (wallet, _) = fixture.open(&sender);
    let result = wallet.transaction(signature).unwrap();
    assert_eq!(result.status, Status::Incomplete);
    assert!(result.entries.is_empty());
}

#[test]
fn a_cold_token_lookup_discovers_only_asset_metadata_and_keeps_existing_mappings() {
    let sender = Keypair::new();
    let receiver = Keypair::new();
    let token = Mint::new(Pubkey::new_unique(), 7);
    let fixture = Fixture::spend(
        &identity(&sender),
        &identity(&receiver),
        10,
        20,
        &[Mint::SOL, token],
    );
    let signature = fixture.signature.to_string();
    let (wallet, asked) = fixture.open_with_registry(&sender, false);
    let result = wallet.transaction(signature).unwrap();
    assert_eq!(result.status, Status::Ready);
    assert_eq!(
        result
            .entries
            .iter()
            .map(|row| (row.index, row.amount))
            .collect::<Vec<_>>(),
        vec![(1, 30), (7, 30)]
    );
    assert_eq!(
        asked
            .lock()
            .unwrap()
            .iter()
            .filter(|method| method.as_str() == "getProgramAccounts")
            .count(),
        1
    );
}
