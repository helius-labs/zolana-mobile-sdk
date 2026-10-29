use super::*;

#[test]
fn opens_only_from_a_signature_over_the_derivation_message() {
    let signer = Keypair::new();
    let wallet = open(&signer).expect("derivation signature opens the wallet");
    assert_eq!(wallet.owner, signer.pubkey());
    assert_eq!(
        wallet.shielded_address(),
        open(&signer).unwrap().shielded_address(),
        "the shielded identity is deterministic in the signer"
    );

    let pubkey = signer.pubkey().to_string();
    let other_message = signer.sign_message(b"not the derivation message");
    let other_signer = Keypair::new().sign_message(&derivation_message(pubkey.clone()).unwrap());
    for signature in [
        other_message.as_ref().to_vec(),
        other_signer.as_ref().to_vec(),
        vec![0; 63],
    ] {
        assert_eq!(
            MobileWallet::open(config(), pubkey.clone(), signature, unreachable()).err(),
            Some(WalletError::InvalidDerivationSignature)
        );
    }
}

#[test]
fn errors_mask_api_keys_in_urls() {
    let config = WalletConfig {
        rpc_url: "https://rpc.example/?api-key=secret".to_string(),
        indexer_url: "https://indexer.example/v1/zolana?api-key=secret".to_string(),
        ..config()
    };
    let mut wallet = open_with(&Keypair::new(), config).unwrap();
    for error in [
        wallet.registration_status().unwrap_err(),
        wallet.private_balance(None).unwrap_err(),
    ] {
        let WalletError::Client { message } = &error else {
            panic!("{error:?}");
        };
        assert!(message.contains("api-key=redacted"), "{message}");
        assert!(!message.contains("secret"), "{message}");
    }
}

#[test]
fn reopens_from_exported_keys_and_refuses_others() {
    let signer = Keypair::new();
    let wallet = open(&signer).unwrap();
    let reopen = |solana_pubkey: String, keys| {
        MobileWallet::open_with_keys(config(), solana_pubkey, keys, unreachable())
    };
    let keys = wallet.export_keys();
    let lengths = [
        keys.viewing_private_key.len(),
        keys.viewing_public_key.len(),
        keys.nullifier_private_key.len(),
        keys.nullifier_public_key.len(),
    ];
    assert_eq!(lengths, [32, 33, 31, 32]);
    let reopened = reopen(signer.pubkey().to_string(), keys).unwrap();
    assert_eq!(reopened.shielded_address(), wallet.shielded_address());
    assert_eq!(
        reopened.export_keys().viewing_private_key,
        wallet.export_keys().viewing_private_key
    );

    // Another account's keys open, under another address: the registry
    // check reports them as a conflict.
    let other = Keypair::new();
    let foreign = reopen(other.pubkey().to_string(), wallet.export_keys()).unwrap();
    assert_ne!(foreign.address, open(&other).unwrap().address);

    fn field(keys: &mut WalletKeys, index: usize) -> &mut Vec<u8> {
        match index {
            0 => &mut keys.viewing_private_key,
            1 => &mut keys.viewing_public_key,
            2 => &mut keys.nullifier_private_key,
            _ => &mut keys.nullifier_public_key,
        }
    }
    let mut broken = Vec::new();
    for index in 0..4 {
        let mut flipped = wallet.export_keys();
        field(&mut flipped, index)[0] ^= 1;
        broken.push(flipped);
        let mut short = wallet.export_keys();
        field(&mut short, index).pop();
        broken.push(short);
    }
    let mut zero = wallet.export_keys();
    zero.viewing_private_key = vec![0; 32];
    broken.push(zero);
    for keys in broken {
        assert_eq!(
            reopen(signer.pubkey().to_string(), keys).err(),
            Some(WalletError::InvalidWalletKeys)
        );
    }
}

#[test]
fn a_record_with_other_keys_is_a_conflict() {
    let identity = open(&Keypair::new()).unwrap().address;
    let other = open(&Keypair::new()).unwrap().address;
    let status = |published: Option<Result<ShieldedAddress, ()>>| {
        registration_status_of(published, &identity)
    };
    assert_eq!(status(None), RegistrationStatus::NotRegistered);
    assert_eq!(status(Some(Ok(identity))), RegistrationStatus::Registered);
    assert_eq!(status(Some(Ok(other))), RegistrationStatus::Conflict);
    assert_eq!(status(Some(Err(()))), RegistrationStatus::Conflict);
    let owner = Pubkey::new_unique();
    assert_eq!(
        WalletError::from(ClientError::UserRegistryKeysMismatch { owner }),
        WalletError::RegistrationConflict {
            owner: owner.to_string()
        }
    );
}

#[test]
fn registration_enables_merging_only_when_requested() {
    use zolana_user_registry_interface::{instruction::discriminator, user_registry_program_id};

    let signer = Keypair::new();
    for enable_merging in [None, Some(false), Some(true)] {
        for fee in [None, Some(7), Some(0)] {
            let (wallet, _) = rpc_wallet_with(
                &signer,
                WalletConfig {
                    priority_fee_lamports: Some(5),
                    ..config()
                },
                |method| match method {
                    "getAccountInfo" => serde_json::json!({"result": {
                        "context": {"slot": 1}, "value": null
                    }}),
                    "getLatestBlockhash" => serde_json::json!({"result": {
                        "context": {"slot": 1}, "value": {
                            "blockhash": "11111111111111111111111111111111",
                            "lastValidBlockHeight": 9
                        }
                    }}),
                    _ => panic!("unexpected request: {method}"),
                },
            );
            let pending = wallet
                .prepare_registration(fee, enable_merging)
                .unwrap()
                .unwrap();
            assert_eq!(pending.operation(), Operation::Registration);
            assert_eq!(pending.signers(), [signer.pubkey().to_string()]);
            assert_eq!(pending.last_valid_block_height(), 9);
            let VersionedMessage::V1(message) = &pending.message else {
                panic!("not a v1 message");
            };
            let enabled = enable_merging.unwrap_or(false);
            assert_eq!(message.instructions.len(), if enabled { 2 } else { 1 });
            assert_eq!(message.config.priority_fee, Some(fee.unwrap_or(5)));
            assert_eq!(
                message.config.compute_unit_limit,
                Some(if enabled { 400_000 } else { 200_000 })
            );
            let keys = &message.account_keys;
            let register = &message.instructions[0];
            assert_eq!(
                keys[usize::from(register.program_id_index)],
                user_registry_program_id()
            );
            let mut data = vec![discriminator::REGISTER];
            data.extend_from_slice(&wallet.address.nullifier_pubkey);
            data.extend_from_slice(wallet.address.viewing_pubkey.as_bytes());
            assert_eq!(register.data, data);
            let record = user_record_pda(&signer.pubkey()).0;
            let accounts = |indices: &[u8]| {
                indices
                    .iter()
                    .map(|&i| keys[usize::from(i)])
                    .collect::<Vec<_>>()
            };
            assert_eq!(
                accounts(&register.accounts),
                [record, signer.pubkey(), signer.pubkey(), Pubkey::default()]
            );
            assert!(pending.message.is_maybe_writable_with_reserved_addresses(
                usize::from(register.accounts[0]),
                Some(&HashSet::<Pubkey>::new()),
            ));
            if enabled {
                let merge = &message.instructions[1];
                assert_eq!(
                    keys[usize::from(merge.program_id_index)],
                    user_registry_program_id()
                );
                assert_eq!(
                    merge.data,
                    [
                        discriminator::SET_MERGE_ENCRYPTION,
                        MergeEncryptionScheme::PoseidonCtrV1 as u8
                    ]
                );
                assert_eq!(accounts(&merge.accounts), [record, signer.pubkey()]);
            }
        }
    }
}

#[test]
fn registration_keeps_existing_records_and_refuses_other_keys() {
    use zolana_user_registry_interface::{user_registry_program_id, UserRecord};

    let signer = Keypair::new();
    let address = open(&signer).unwrap().address;
    for enable_merging in [None, Some(false), Some(true)] {
        for scheme in [
            MergeEncryptionScheme::Disabled,
            MergeEncryptionScheme::PoseidonCtrV1,
        ] {
            for conflict in [None, Some(0), Some(32)] {
                // UserRecord's discriminator, owner, bump, keys and scheme.
                let mut data = vec![UserRecord::DISCRIMINATOR];
                data.extend_from_slice(signer.pubkey().as_ref());
                data.push(user_record_pda(&signer.pubkey()).1);
                let mut keys = address.nullifier_pubkey.to_vec();
                keys.extend_from_slice(address.viewing_pubkey.as_bytes());
                if let Some(index) = conflict {
                    keys[index] ^= 1;
                }
                data.extend_from_slice(&keys);
                data.push(scheme as u8);
                let record = account(user_registry_program_id(), &data);
                let (wallet, asked) = rpc_wallet(&signer, move |method| {
                    assert_eq!(method, "getAccountInfo");
                    serde_json::json!({"result": {"context": {"slot": 1}, "value": record}})
                });
                let result = wallet.prepare_registration(None, enable_merging);
                if conflict.is_some() {
                    assert_eq!(
                        result.err(),
                        Some(WalletError::RegistrationConflict {
                            owner: signer.pubkey().to_string()
                        })
                    );
                } else {
                    assert!(result.unwrap().is_none());
                }
                assert_eq!(*asked.lock().unwrap(), ["getAccountInfo"]);
            }
        }
    }
}

#[test]
fn each_spend_is_proved_where_it_asks_or_the_config_says() {
    let signer = Keypair::new();
    let recipient = Keypair::new().pubkey().to_string();
    let missing = |wallet: &mut MobileWallet, proving| {
        [
            wallet.prepare_spend(vec![private(&recipient, 1)], None, None, proving, None),
            wallet.prepare_spend(vec![public(&recipient, 1)], None, None, proving, None),
        ]
        .map(|result| result.err() == Some(WalletError::RemoteProverMissing))
    };

    let mut local = open(&signer).unwrap();
    assert_eq!(missing(&mut local, Some(Proving::Remote)), [true; 2]);
    // The others get as far as the transport.
    assert_eq!(missing(&mut local, None), [false; 2]);
    let config = |prover_url: Option<&str>| WalletConfig {
        proving: Some(Proving::Remote),
        prover_url: prover_url.map(str::to_string),
        prover_tee_policy: prover_url.map(|_| {
            include_str!("../../../../../example/assets/policies/devnet.json").to_string()
        }),
        ..config()
    };
    let mut remote = open_with(&signer, config(None)).unwrap();
    assert_eq!(missing(&mut remote, None), [true; 2]);
    assert_eq!(missing(&mut remote, Some(Proving::Local)), [false; 2]);
    let mut remote = open_with(&signer, config(Some("https://prover.example"))).unwrap();
    assert_eq!(missing(&mut remote, None), [false; 2]);
}

/// Every remote prover, the Helius gateway included, gets attested,
/// encrypted requests; only a loopback prover gets plaintext.
#[test]
fn only_a_loopback_prover_gets_plaintext_proof_requests() {
    let signer = Keypair::new();
    let tee = |url: &str, policy: Option<&str>| {
        let config = WalletConfig {
            prover_url: Some(url.to_string()),
            prover_tee_policy: policy.map(str::to_string),
            ..config()
        };
        let wallet = open_with(&signer, config).unwrap();
        wallet.remote_prover.as_ref().unwrap().has_tee()
    };
    let policy = Some(include_str!(
        "../../../../../example/assets/policies/devnet.json"
    ));
    assert!(tee("https://prover.example", policy));
    assert!(tee(
        "https://beta-devnet.helius-rpc.com/v1/zolana?api-key=k",
        policy
    ));
    assert!(!tee("http://127.0.0.1:3001", None));
}

#[test]
fn rejects_malformed_keys_before_the_network() {
    let mut wallet = open(&Keypair::new()).unwrap();
    let recipient = Keypair::new().pubkey().to_string();
    let bad = "not-a-pubkey".to_string();
    let invalid = Some(WalletError::InvalidPubkey { value: bad.clone() });
    let error = |result: Result<PreparedTransaction, WalletError>| result.err();
    assert_eq!(
        error(wallet.prepare_spend(vec![private(&bad, 1)], None, None, None, None)),
        invalid
    );
    assert_eq!(
        error(wallet.prepare_spend(vec![public(&bad, 1)], None, None, None, None)),
        invalid
    );
    assert_eq!(
        error(wallet.prepare_spend(vec![private(&recipient, 1)], None, Some(bad), None, None)),
        invalid
    );
}
