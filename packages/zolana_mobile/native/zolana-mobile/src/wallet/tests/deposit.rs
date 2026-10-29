use super::*;
use crate::wallet::spend::Leg;

/// A deposit is one instruction this account pays for and signs, and the
/// operation names what it moves.
#[test]
fn a_deposit_is_one_instruction_this_account_signs() {
    let signer = Keypair::new();
    let (mut wallet, _) = rpc_wallet(&signer, |method| match method {
        "getLatestBlockhash" => serde_json::json!({"result": {
            "context": {"slot": 1},
            "value": {
                "blockhash": "11111111111111111111111111111111",
                "lastValidBlockHeight": 9
            }
        }}),
        _ => serde_json::json!({"error": {"code": -32603, "message": "unexpected"}}),
    });
    let pending = wallet.prepare_deposit(None, 100, None).unwrap();
    assert_eq!(
        pending.operation(),
        Operation::Deposit {
            mint: None,
            amount: 100,
            creates_spl_interface: false,
        }
    );
    assert_eq!(pending.message.instructions().len(), 1);
    assert_eq!(pending.signers(), [signer.pubkey().to_string()]);
    assert_eq!(pending.last_valid_block_height(), 9);
}

/// The first deposit of a mint the pool has not registered creates its
/// registry entry and vault first, in the same transaction this account
/// signs; once registered, a deposit is one instruction again.
#[test]
fn a_first_token_deposit_creates_the_mints_registry_entry_and_vault() {
    let signer = Keypair::new();
    let other = Keypair::new().pubkey();
    let deposit = |registry, config| {
        let (mut wallet, _) = token_wallet(&signer, registry, config);
        wallet.prepare_deposit(Some(MINT.to_string()), 100, None)
    };
    let registered = account(
        PROGRAM_ID_PUBKEY,
        &zolana_interface::state::SplAssetRegistry::account_bytes(MINT, 2),
    );
    // Lamports sent to the registry address make no registry entry.
    let lamports_only = account(Pubkey::default(), &[]);
    for (registry, config, creates) in [
        (serde_json::Value::Null, protocol_config(true, other), true),
        (lamports_only, protocol_config(true, other), true),
        (
            serde_json::Value::Null,
            protocol_config(false, signer.pubkey()),
            true,
        ),
        (registered, serde_json::Value::Null, false),
    ] {
        let pending = deposit(registry, config).unwrap();
        assert_eq!(
            pending.operation(),
            Operation::Deposit {
                mint: Some(MINT.to_string()),
                amount: 100,
                creates_spl_interface: creates,
            }
        );
        let instructions = pending.message.instructions();
        assert_eq!(instructions.len(), if creates { 2 } else { 1 });
        let keys = pending.message.static_account_keys();
        if creates {
            let create = &instructions[0];
            assert_eq!(
                keys[usize::from(create.program_id_index)],
                PROGRAM_ID_PUBKEY
            );
            assert_eq!(
                create.data,
                [zolana_interface::instruction::tag::CREATE_SPL_INTERFACE]
            );
        }
        assert_eq!(pending.signers(), [signer.pubkey().to_string()]);
    }
}

/// A pool that lets only its authority register mints refuses another
/// account's first deposit before the blockhash is read.
#[test]
fn a_first_token_deposit_the_pool_restricts_is_refused_before_the_blockhash() {
    let signer = Keypair::new();
    let (mut wallet, asked) = token_wallet(
        &signer,
        serde_json::Value::Null,
        protocol_config(false, Keypair::new().pubkey()),
    );
    assert_eq!(
        wallet
            .prepare_deposit(Some(MINT.to_string()), 100, None)
            .err(),
        Some(WalletError::AssetNotSupported {
            mint: MINT.to_string()
        })
    );
    assert!(!asked
        .lock()
        .unwrap()
        .iter()
        .any(|method| method == "getLatestBlockhash"));
    let (mut wallet, _) = token_wallet(&signer, serde_json::Value::Null, serde_json::Value::Null);
    assert!(matches!(
        wallet.prepare_deposit(Some(MINT.to_string()), 100, None),
        Err(WalletError::Client { message }) if message.contains("protocol config")
    ));
}

/// A public token recipient without its associated token account gets
/// one in the spend; a recipient that has one, and SOL, need none.
#[test]
fn a_public_token_recipient_without_its_account_gets_one() {
    let mint = Pubkey::new_unique();
    let token = Asset {
        mint,
        token_program: Some(pda::spl_token_program_id()),
    };
    let (without, with) = (Pubkey::new_unique(), Pubkey::new_unique());
    let existing = token.token_account(&with).unwrap().to_string();
    let (transport, _) = crate::transport::tests::fake(move |request| {
        let body: serde_json::Value = serde_json::from_slice(&request.body).unwrap();
        assert_eq!(body["method"], "getAccountInfo");
        let value = if body["params"][0] == existing.as_str() {
            serde_json::json!({
                "data": ["", "base64"], "executable": false, "lamports": 2_039_280,
                "owner": pda::spl_token_program_id().to_string(), "rentEpoch": 0, "space": 0
            })
        } else {
            serde_json::Value::Null
        };
        crate::transport::tests::ok(
            &serde_json::json!({
                "jsonrpc": "2.0", "id": body["id"],
                "result": {"context": {"slot": 1}, "value": value}
            })
            .to_string(),
        )
    });
    let signer = Keypair::new();
    let pubkey = signer.pubkey().to_string();
    let signature = signer.sign_message(&derivation_message(pubkey.clone()).unwrap());
    let wallet =
        MobileWallet::open(config(), pubkey, signature.as_ref().to_vec(), transport).unwrap();
    let creates = |asset: Asset, recipient: Pubkey| {
        let legs = wallet
            .legs(asset, &[public(&recipient.to_string(), 1)], &[recipient])
            .unwrap();
        match legs[..] {
            [Leg::Public {
                creates_token_account,
                ..
            }] => creates_token_account,
            _ => panic!("one public leg"),
        }
    };
    assert!(creates(token, without));
    assert!(!creates(token, with));
    let sol = Asset {
        mint: zolana_transaction::SOL_MINT,
        token_program: None,
    };
    assert!(!creates(sol, without));
}

/// The created accounts go ahead of the proved message, which compiles
/// again unchanged behind them: the same blockhash, the transact
/// instruction's accounts and roles, and a compute limit raised for each
/// account, at most to the ceiling.
#[test]
fn created_token_accounts_go_ahead_of_the_proved_message() {
    let payer = Pubkey::new_unique();
    let transact = Instruction {
        program_id: Pubkey::new_unique(),
        accounts: vec![
            AccountMeta::new(payer, true),
            AccountMeta::new_readonly(Pubkey::new_unique(), true),
            AccountMeta::new(Pubkey::new_unique(), false),
            AccountMeta::new_readonly(Pubkey::new_unique(), false),
        ],
        data: vec![1, 2, 3],
    };
    let create = CreateAssociatedTokenAccount {
        payer,
        owner: Pubkey::new_unique(),
        mint: Pubkey::new_unique(),
        token_program: pda::spl_token_2022_program_id(),
    }
    .instruction();
    let blockhash = Default::default();
    let proved = |limit: u32| UnsignedMessage {
        message: compile_message(
            &payer,
            core::slice::from_ref(&transact),
            blockhash,
            ComputeBudgetConfig::new(limit),
        )
        .unwrap(),
        last_valid_block_height: 9,
    };
    let expected = |limit: u32| {
        compile_message(
            &payer,
            &[create.clone(), transact.clone()],
            blockhash,
            ComputeBudgetConfig::new(limit),
        )
        .unwrap()
    };

    let rebuilt = with_setup(proved(1_000_000), &payer, core::slice::from_ref(&create)).unwrap();
    assert_eq!(rebuilt.message, expected(1_040_000));
    assert_eq!(rebuilt.last_valid_block_height, 9);
    let capped = with_setup(proved(1_390_000), &payer, core::slice::from_ref(&create)).unwrap();
    assert_eq!(capped.message, expected(MAX_COMPUTE_UNITS));
    assert_eq!(
        with_setup(proved(1_000_000), &payer, &[]).unwrap().message,
        proved(1_000_000).message
    );
}
