use super::*;

#[test]
fn refreshed_spends_keep_their_openings() {
    let signer = Keypair::new();
    let (wallet, _) = rpc_wallet(&signer, |_| {
        serde_json::json!({"result": {
            "context": {"slot": 5}, "value": {
                "blockhash": "11111111111111111111111111111111", "lastValidBlockHeight": 250
            }
        }})
    });
    let mut pending = unsigned(&signer);
    pending.outputs = vec![OutputOpening::from_output(
        &SppProofOutputUtxo::default(),
        0,
    )];
    let refreshed = wallet.refresh(&pending).unwrap();
    assert_eq!(refreshed.outputs(), pending.outputs());
    assert_eq!(refreshed.last_valid_block_height(), 250);
}

#[test]
fn openings_preserve_all_commitment_fields() {
    use solana_address::Address;
    use zolana_transaction::Mint;

    let wallet = open(&Keypair::new()).unwrap();
    let mut real = SppProofOutputUtxo::new(Mint::SOL, 5, wallet.address).unwrap();
    real.data_hash = Some([7; 32]);
    real.ring_data_hash = Some([8; 32]);
    real.ring_program_id = Some(Address::new_from_array([9; 32]));
    let dummy = SppProofOutputUtxo::default();
    let compact = SppProofOutputUtxo {
        compact: true,
        ..dummy.clone()
    };
    for output in [real, dummy, compact] {
        let opening = OutputOpening::from_output(&output, 4);
        assert_eq!(
            commitment_of(&opening, wallet.address.viewing_pubkey),
            output.hash(4).unwrap()
        );
        assert_eq!(opening.owner_signing_key.is_none(), output.is_dummy());
        assert_eq!(opening.nullifier_key.is_none(), output.is_dummy());
        assert_eq!(opening.compact, output.is_compact());
    }
}

/// Golden v1 messages built with the actual SDK wire serializers, plus
/// Poseidon vectors for the Dart policy tests. No real funds or proofs.
#[test]
fn spend_check_vectors_match_example() {
    use solana_address::Address;
    use zolana_hasher::{primitives::right_align, Hasher, Poseidon};
    use zolana_interface::instruction::{
        CircuitId, InputUtxo, InterfaceTransfer, OwnerTag, TransactIxData, TransactOutput,
        TransactProof, TreeContext,
    };
    use zolana_program::instruction::{
        Transact, TransactSolTransferAccounts, TransactSplWithdrawalAccounts,
    };
    use zolana_transaction::{
        derive_output_blinding_seed, utxo::derive_transact_output_blinding, Mint,
    };

    let signers = [1, 2, 3].map(|byte| Keypair::new_from_array([byte; 32]));
    let wallets = signers.each_ref().map(|signer| open(signer).unwrap());
    let payer = signers[0].pubkey();
    let recipient = signers[1].pubkey();
    let token_program = pda::spl_token_program_id();
    let mint = Pubkey::new_from_array([8; 32]);
    let mut hashes = serde_json::Map::new();
    let mut hash = |inputs: Vec<[u8; 32]>| {
        let result = Poseidon::hashv(
            &inputs
                .iter()
                .map(|field| field.as_slice())
                .collect::<Vec<_>>(),
        )
        .unwrap();
        let key = inputs
            .iter()
            .map(|field| {
                field
                    .iter()
                    .map(|byte| format!("{byte:02x}"))
                    .collect::<String>()
            })
            .collect::<Vec<_>>()
            .join(":");
        hashes.insert(key, serde_json::json!(result));
        result
    };
    let mut cases = Vec::new();
    for spl in [false, true] {
        for withdraw in [false, true] {
            let asset = if spl {
                Mint::new(Address::new_from_array(mint.to_bytes()), 7)
            } else {
                Mint::SOL
            };
            let first = [1; 32];
            let seed = derive_output_blinding_seed(&first, &[2; 32]).unwrap();
            let mut outputs = vec![
                SppProofOutputUtxo::new(asset, 7, wallets[0].address).unwrap(),
                SppProofOutputUtxo::new(asset, 5, wallets[2].address).unwrap(),
            ];
            if !withdraw {
                outputs.push(SppProofOutputUtxo::new(asset, 100, wallets[1].address).unwrap());
            }
            outputs.push(SppProofOutputUtxo::default());
            while outputs.len() < 4 {
                outputs.push(SppProofOutputUtxo {
                    compact: true,
                    ..Default::default()
                });
            }
            for (index, output) in outputs.iter_mut().enumerate() {
                output.blinding =
                    derive_transact_output_blinding(&first, &seed, index as u32).unwrap();
                if output.is_compact() {
                    continue;
                }
                let dummy = output.is_dummy();
                let owner = match output.owner_address {
                    None => [0; 32],
                    Some(address) => {
                        let mut tagged = vec![0x53];
                        tagged.extend_from_slice(&address.signing_pubkey.as_ed25519().unwrap());
                        let identity = hash(vec![
                            right_align::<31>(tagged[..31].try_into().unwrap()),
                            right_align::<2>(tagged[31..].try_into().unwrap()),
                        ]);
                        hash(vec![identity, address.nullifier_pubkey])
                    }
                };
                let asset_hash = if dummy {
                    [0; 32]
                } else {
                    hash(vec![
                        right_align::<31>(asset.asset.as_array()[..31].try_into().unwrap()),
                        right_align::<1>(asset.asset.as_array()[31..].try_into().unwrap()),
                    ])
                };
                let ring = hash(vec![[0; 32], [0; 32]]);
                let owner_utxo = hash(vec![owner, output.blinding]);
                let commitment = hash(vec![
                    right_align(&(if dummy { 1u16 } else { 3u16 }).to_be_bytes()),
                    [0; 32],
                    asset_hash,
                    right_align(&(if dummy { 0 } else { output.amount }).to_be_bytes()),
                    [0; 32],
                    ring,
                    owner_utxo,
                ]);
                assert_eq!(commitment, output.hash(0).unwrap());
            }
            let user_token =
                pda::associated_token_address_with_program(&recipient, &mint, &token_program);
            let (transfers, settlement) = if !withdraw {
                (vec![], vec![])
            } else if spl {
                let (spl_interface, bump) = pda::spl_interface_with_bump(&mint);
                (
                    vec![InterfaceTransfer::SplWithdrawal {
                        amount: 100,
                        spl_interface_bump: bump,
                    }],
                    vec![TransactInterfaceTransferAccounts::SplWithdrawal(
                        TransactSplWithdrawalAccounts {
                            mint,
                            spl_interface,
                            user_token_account: user_token,
                            token_program,
                        },
                    )],
                )
            } else {
                (
                    vec![InterfaceTransfer::SolWithdrawal { amount: 100 }],
                    vec![TransactInterfaceTransferAccounts::Sol(
                        TransactSolTransferAccounts { recipient },
                    )],
                )
            };
            let mut builder = Transact {
                payer,
                input_trees: vec![pda::tree(0)],
                output_tree: pda::tree(0),
                owner_signers: vec![payer],
                interface_transfer_accounts: settlement,
                data: TransactIxData {
                    expiry_unix_ts: u64::MAX,
                    tx_viewing_pk: [0; 33],
                    salt: [0; 16],
                    interface_transfers: transfers,
                    data_hash: None,
                    ring_data_hash: None,
                    outputs: outputs
                        .iter()
                        .filter(|output| !output.is_compact())
                        .map(|output| TransactOutput {
                            utxo_hash: output.hash(0).unwrap(),
                            owner_tag: match output.owner_address {
                                Some(address) if address == wallets[0].address => {
                                    OwnerTag::Account(0)
                                }
                                Some(address) => OwnerTag::Inline(
                                    address.signing_pubkey.confidential_view_tag().unwrap(),
                                ),
                                None => OwnerTag::Inline([9; 32]),
                            },
                            data: Some(vec![42; 8]),
                        })
                        .collect(),
                    messages: vec![],
                    private_tx_hash: [0; 32],
                    circuit: CircuitId::ConfidentialSigner(2, 4, 3),
                    proof: TransactProof::zeroed(),
                    inputs: vec![
                        InputUtxo {
                            nullifier_hash: first,
                            tree_index: 0,
                        },
                        InputUtxo {
                            nullifier_hash: [3; 32],
                            tree_index: 0,
                        },
                    ],
                    tree_contexts: vec![TreeContext {
                        utxo_tree_root_index: 0,
                        nullifier_tree_root_index: 0,
                    }],
                },
            };
            for create in [false, true] {
                if create && !(spl && withdraw) {
                    continue;
                }
                let mut instructions = Vec::new();
                if create {
                    instructions.push(
                        CreateAssociatedTokenAccount {
                            payer,
                            owner: recipient,
                            mint,
                            token_program,
                        }
                        .instruction(),
                    );
                }
                instructions.push(builder.instruction());
                let message = compile_message(
                    &payer,
                    &instructions,
                    Default::default(),
                    ComputeBudgetConfig::new(1_000_000),
                )
                .unwrap();
                let openings = outputs
                    .iter()
                    .map(|output| {
                        let o = OutputOpening::from_output(output, 0);
                        serde_json::json!({
                            "ownerSigningKey": o.owner_signing_key, "nullifierKey": o.nullifier_key,
                            "mint": o.mint, "amount": o.amount.to_string(), "blinding": o.blinding,
                            "treeId": o.tree_id, "compact": o.compact,
                        })
                    })
                    .collect::<Vec<_>>();
                cases.push(serde_json::json!({
                    "name": format!("{}-{}{}", if spl { "spl" } else { "sol" }, if withdraw { "withdraw" } else { "private" }, if create { "-create" } else { "" }),
                    "message": message.serialize(), "openings": openings,
                    "mint": spl.then(|| mint.to_string()), "withdraw": withdraw,
                    "tokenAccount": user_token.to_string(), "tokenProgram": token_program.to_string(),
                }));
                let cache = pda::cache(&payer, 42).0;
                builder.data.circuit = CircuitId::ConfidentialSignerCached(
                    2,
                    4,
                    3,
                    zolana_interface::verifying_keys::CacheAccess {
                        read_bitmap: 5,
                        write_slots: zolana_interface::verifying_keys::CacheAccess::NO_WRITES,
                    },
                );
                instructions.pop();
                instructions.push(builder.instruction_with_cache_read(cache));
                let cached_message = compile_message(
                    &payer,
                    &instructions,
                    Default::default(),
                    ComputeBudgetConfig::new(1_000_000),
                )
                .unwrap();
                let mut cached_case = cases.last().unwrap().clone();
                cached_case["name"] =
                    serde_json::json!(format!("{}-cached", cached_case["name"].as_str().unwrap()));
                cached_case["message"] = serde_json::json!(cached_message.serialize());
                cached_case["cacheAddress"] = serde_json::json!(cache.to_string());
                cached_case["cacheSlots"] = serde_json::json!([0, 2]);
                cases.push(cached_case);
                builder.data.circuit = CircuitId::ConfidentialSigner(2, 4, 3);
            }
        }
    }
    let fixture = serde_json::json!({
        "sender": payer.to_string(), "recipient": recipient.to_string(),
        "collector": signers[2].pubkey().to_string(), "tree": pda::tree(0).to_string(),
        "nullifiers": signers.iter().zip(&wallets).map(|(signer, wallet)| (signer.pubkey().to_string(), wallet.address.nullifier_pubkey.to_vec())).collect::<std::collections::BTreeMap<_, _>>(),
        "hashes": hashes, "cases": cases,
    });
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../example/test/fixtures/spend_checks.json");
    if std::env::var_os("UPDATE_SPEND_CHECK_FIXTURE").is_some() {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, serde_json::to_string(&fixture).unwrap() + "\n").unwrap();
    }
    let expected: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
    assert_eq!(
        fixture, expected,
        "regenerate with UPDATE_SPEND_CHECK_FIXTURE=1"
    );
}

fn commitment_of(opening: &OutputOpening, viewing: zolana_keypair::X25519Pubkey) -> [u8; 32] {
    use std::str::FromStr;

    use solana_address::Address;
    use zolana_keypair::PublicKey;
    use zolana_transaction::{Mint, SppProofOutputUtxo};

    let owner_address = match (&opening.owner_signing_key, &opening.nullifier_key) {
        (Some(signing), Some(nullifier)) => Some(zolana_keypair::ShieldedAddress {
            signing_pubkey: PublicKey::from_bytes(signing.as_slice().try_into().unwrap()).unwrap(),
            nullifier_pubkey: nullifier.as_slice().try_into().unwrap(),
            viewing_pubkey: viewing,
        }),
        _ => None,
    };
    let asset = match &opening.mint {
        None => Mint::SOL,
        Some(mint) => Mint::new(Address::from_str(mint).unwrap(), 0),
    };
    let output = SppProofOutputUtxo {
        asset,
        amount: opening.amount,
        blinding: opening.blinding.as_slice().try_into().unwrap(),
        ring_program_id: opening
            .ring_program_id
            .as_deref()
            .map(|id| Address::from_str(id).unwrap()),
        ring_data_hash: opening
            .ring_data_hash
            .as_deref()
            .map(|hash| hash.try_into().unwrap()),
        data_hash: opening
            .data_hash
            .as_deref()
            .map(|hash| hash.try_into().unwrap()),
        owner_address,
        compact: opening.compact,
        ..SppProofOutputUtxo::default()
    };
    output.hash(opening.tree_id).unwrap()
}
