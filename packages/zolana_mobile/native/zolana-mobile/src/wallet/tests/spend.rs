use super::*;

#[test]
fn a_merge_expires_ten_minutes_after_it_is_prepared() {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let expiry = merge_expiry();
    assert!(
        (now + 600..=now + 601).contains(&expiry),
        "{expiry} vs {now}"
    );
}

/// A prepared transaction pays the priority fee its call names, or else
/// the config's, in its v1 header, and keeps it through `refresh`.
#[test]
fn a_transaction_pays_the_priority_fee_its_call_or_the_config_names() {
    let signer = Keypair::new();
    let blockhash = |method: &str| match method {
        "getLatestBlockhash" => serde_json::json!({"result": {
            "context": {"slot": 1},
            "value": {
                "blockhash": "11111111111111111111111111111111",
                "lastValidBlockHeight": 9
            }
        }}),
        _ => serde_json::json!({"error": {"code": -32603, "message": "unexpected"}}),
    };
    let fee_in_header = |pending: &PreparedTransaction| {
        let VersionedMessage::V1(message) =
            wincode::deserialize::<VersionedMessage>(&pending.message()).expect("a message")
        else {
            panic!("not a v1 message");
        };
        message.config.priority_fee
    };

    let (mut wallet, _) = rpc_wallet(&signer, blockhash);
    let pending = wallet.prepare_deposit(None, 100, None).unwrap();
    assert_eq!(pending.priority_fee_lamports(), 0);
    assert_eq!(fee_in_header(&pending), None);

    let config = WalletConfig {
        priority_fee_lamports: Some(5_000),
        ..config()
    };
    let (mut wallet, _) = rpc_wallet_with(&signer, config, blockhash);
    let pending = wallet.prepare_deposit(None, 100, None).unwrap();
    assert_eq!(pending.priority_fee_lamports(), 5_000);
    assert_eq!(fee_in_header(&pending), Some(5_000));
    let named = wallet.prepare_deposit(None, 100, Some(12_000)).unwrap();
    assert_eq!(named.priority_fee_lamports(), 12_000);
    let refreshed = wallet.refresh(&named).unwrap();
    assert_eq!(fee_in_header(&refreshed), Some(12_000));
    let free = wallet.prepare_deposit(None, 100, Some(0)).unwrap();
    assert_eq!(fee_in_header(&free), Some(0));
}

/// The fee goes into a built message, a proved one included, and changes
/// nothing else of it.
#[test]
fn the_priority_fee_changes_only_the_header() {
    let wallet = open(&Keypair::new()).unwrap();
    let built = compile_message(
        &wallet.owner,
        &[set_merge_encryption_instruction(
            wallet.owner,
            MergeEncryptionScheme::PoseidonCtrV1,
        )],
        Default::default(),
        ComputeBudgetConfig::new(450_000),
    )
    .unwrap();
    let priced = wallet.with_priority_fee(built.clone(), Some(7)).unwrap();
    let (VersionedMessage::V1(built), VersionedMessage::V1(priced)) = (built, priced) else {
        panic!("not v1 messages");
    };
    assert_eq!(priced.config.priority_fee, Some(7));
    assert_eq!(priced.config.compute_unit_limit, Some(450_000));
    assert_eq!(
        v1::Message {
            config: built.config.with_priority_fee(7),
            ..built.clone()
        },
        priced
    );
    assert_eq!(
        wallet
            .with_priority_fee(VersionedMessage::V1(built.clone()), None)
            .unwrap(),
        VersionedMessage::V1(built)
    );
}

/// The outputs are checked before anything is asked of the network: no
/// outputs, a zero amount and a total past `u64` each fail at once.
#[test]
fn a_spend_checks_its_outputs_before_the_network() {
    let signer = Keypair::new();
    let (mut wallet, asked) = rpc_wallet(
        &signer,
        |_| serde_json::json!({"error": {"code": -32603, "message": "unexpected"}}),
    );
    let recipient = Keypair::new().pubkey().to_string();
    let spend = |wallet: &mut MobileWallet, outputs| {
        wallet
            .prepare_spend(outputs, None, None, None, None)
            .err()
            .expect("fails before the network")
    };
    assert_eq!(spend(&mut wallet, vec![]), WalletError::NoOutputs);
    assert_eq!(
        spend(
            &mut wallet,
            vec![private(&recipient, 1), public(&recipient, 0)]
        ),
        WalletError::AmountZero
    );
    let overflow = spend(
        &mut wallet,
        vec![private(&recipient, u64::MAX), public(&recipient, 1)],
    );
    assert!(
        matches!(&overflow, WalletError::Client { message } if message.contains("overflow")),
        "{overflow:?}"
    );
    assert!(asked.lock().unwrap().is_empty());
}

/// Private outputs and the change take the proof's output slots, public
/// outputs none; the slots pick the widest keys, and with them the notes
/// a spend can take.
#[test]
fn private_outputs_and_the_change_take_the_proof_slots() {
    let recipient = Pubkey::new_unique().to_string();
    let slots = |outputs: &[Output]| proof_outputs(outputs);
    assert_eq!(slots(&[public(&recipient, 1)]), 2);
    assert_eq!(slots(&[public(&recipient, 1), public(&recipient, 1)]), 2);
    assert_eq!(slots(&[private(&recipient, 1)]), 2);
    assert_eq!(slots(&[private(&recipient, 1), public(&recipient, 1)]), 2);
    assert_eq!(slots(&[private(&recipient, 1), private(&recipient, 1)]), 3);
    assert_eq!(
        slots(&[
            public(&recipient, 1),
            private(&recipient, 1),
            private(&recipient, 1),
            private(&recipient, 1),
        ]),
        4
    );
    // One private output and change: the 2-output keys, up to the SDK's
    // cap. Two or three: the 4-output keys, which stop at 24 notes.
    assert_eq!(max_spend_inputs(2), MAX_SPEND_INPUTS);
    assert_eq!(max_spend_inputs(3), 24);
    assert_eq!(max_spend_inputs(4), 24);
    assert_eq!(max_spend_inputs(5), 16);
    assert_eq!(max_spend_inputs(9), 8);
    assert!(canonical_shape(24, 3).is_ok());
    assert!(canonical_shape(25, 3).is_err());
}
