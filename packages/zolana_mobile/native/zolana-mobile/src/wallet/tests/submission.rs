use super::*;

/// A send whose confirmation the RPC lost, for a transaction that landed
/// (or did not): the wallet looks for it before it reports a failure.
#[test]
fn a_send_error_is_checked_against_the_signature_status() {
    let signer = Keypair::new();
    let internal = || serde_json::json!({"error": {"code": -32603, "message": "Internal error"}});
    let registration = |signer: &Keypair| PreparedTransaction {
        operation: Operation::Registration,
        ..unsigned(signer)
    };
    let signed = |pending: &PreparedTransaction| {
        vec![signer.sign_message(&pending.message()).as_ref().to_vec()]
    };

    let (wallet, asked) = rpc_wallet(&signer, move |method| match method {
        "sendTransaction" => internal(),
        _ => status(true),
    });
    let pending = registration(&signer);
    let signature = wallet
        .submit(&pending, signed(&pending))
        .expect("it landed");
    assert_eq!(
        signature,
        signer.sign_message(&pending.message()).to_string()
    );
    assert!(asked
        .lock()
        .unwrap()
        .contains(&"getSignatureStatuses".to_string()));

    // Expiry stops waiting, but does not establish whether it landed.
    let (wallet, _) = rpc_wallet(&signer, move |method| match method {
        "sendTransaction" => internal(),
        "getBlockHeight" => serde_json::json!({"result": 10}),
        _ => status(false),
    });
    assert!(matches!(
        wallet.submit(&pending, signed(&pending)),
        Err(WalletError::TransactionOutcomeUnknown { .. })
    ));
}

#[test]
fn retries_missing_processed_and_failed_history_lookups() {
    let mut processed = status(true);
    processed["result"]["value"][0]["confirmationStatus"] = "processed".into();
    let failure = serde_json::json!({"error": {"code": -32603, "message": "lookup timed out"}});
    for first in [status(false), processed, failure] {
        let (wallet, asked) = recovery_wallet(vec![first, status(true)], 99);
        assert_eq!(
            reconcile(&wallet.client, Signature::from([3; 64]), Some(100), QUICK),
            Ok(())
        );
        assert_eq!(
            asked
                .lock()
                .unwrap()
                .iter()
                .filter(|method| *method == "getSignatureStatuses")
                .count(),
            2
        );
    }
}

#[test]
fn inconclusive_history_remains_unknown() {
    let mut processed = status(true);
    processed["result"]["value"][0]["confirmationStatus"] = "processed".into();
    let failure = serde_json::json!({"error": {"code": -32603, "message": "lookup timed out"}});
    for (status, height, message) in [
        (status(false), 101, "not found"),
        (status(false), 99, "not found"),
        (processed, 99, "not confirmed"),
        (failure, 99, "lookup timed out"),
    ] {
        let (wallet, _) = recovery_wallet(vec![status], height);
        assert!(matches!(
            wallet.wait_for_transaction(Signature::from([3; 64]).to_string(), Some(100)),
            Err(WalletError::TransactionOutcomeUnknown { message: actual, .. }) if actual.contains(message)
        ));
    }
}

/// A send refused in simulation was never sent and settles at once; one
/// found already processed may have landed and is looked up.
#[test]
fn only_an_already_processed_simulation_is_looked_up() {
    let preflight = |err: &str| {
        let result: solana_rpc_client_api::response::RpcSimulateTransactionResult =
            serde_json::from_str(&format!(r#"{{"err":"{err}"}}"#)).unwrap();
        ClientError::SolanaRpcTransaction {
            operation: "process_transaction",
            source: solana_rpc_client_api::request::RpcError::RpcResponseError {
                code: -32002,
                message: "Transaction simulation failed".into(),
                data: solana_rpc_client_api::request::RpcResponseErrorData::SendTransactionPreflightFailure(result),
            }
            .into(),
        }
    };
    assert!(settles_the_send(&preflight("InsufficientFundsForFee")));
    assert!(!settles_the_send(&preflight("AlreadyProcessed")));
    assert!(!settles_the_send(&ClientError::Rpc(
        "connection reset".into()
    )));
}

/// A spend of a note the chain already spent fails as such, at once: the
/// node refused it, so it cannot have landed.
#[test]
fn a_spent_note_is_notes_already_spent() {
    let signer = Keypair::new();
    let (wallet, asked) = rpc_wallet(&signer, |method| match method {
        "sendTransaction" => nullifier_queued(),
        _ => status(true),
    });
    let pending = unsigned(&signer);
    let signatures = vec![signer.sign_message(&pending.message()).as_ref().to_vec()];
    assert_eq!(
        wallet.submit(&pending, signatures).unwrap_err(),
        WalletError::NotesAlreadySpent
    );
    assert_eq!(*asked.lock().unwrap(), ["sendTransaction"]);
}

#[test]
fn submit_checks_every_signature_before_sending() {
    let signer = Keypair::new();
    let mut wallet = open(&signer).unwrap();
    let pending = || unsigned(&signer);
    assert_eq!(pending().signers(), vec![signer.pubkey().to_string()]);
    let valid = signer.sign_message(&pending().message());
    let wrong_signer = Keypair::new().sign_message(&pending().message());
    let count = |got| WalletError::SignatureCountMismatch { expected: 1, got };
    for (signatures, expected) in [
        (vec![], count(0)),
        (vec![valid.as_ref().to_vec(); 2], count(2)),
        (
            vec![wrong_signer.as_ref().to_vec()],
            WalletError::SignatureInvalid,
        ),
        (
            vec![valid.as_ref()[..63].to_vec()],
            WalletError::SignatureInvalid,
        ),
    ] {
        assert_eq!(wallet.submit(&pending(), signatures).err(), Some(expected));
    }
    // A valid signature gets as far as the transport, which answers
    // neither the send nor the lookups after it.
    wallet.send_lookup = QUICK;
    let error = wallet
        .submit(&pending(), vec![valid.as_ref().to_vec()])
        .unwrap_err();
    assert!(
        matches!(error, WalletError::TransactionOutcomeUnknown { .. }),
        "{error:?}"
    );
}

#[test]
fn confirm_accepts_only_the_fee_payer_signature() {
    let signer = Keypair::new();
    let wallet = open(&signer).unwrap();
    let pending = unsigned(&signer);
    let other = Keypair::new().sign_message(&pending.message());
    for signature in ["not-a-signature".to_string(), other.to_string()] {
        assert_eq!(
            wallet.confirm(&pending, signature).err(),
            Some(WalletError::SignatureInvalid)
        );
    }
    // The fee payer's signature gets as far as the transport.
    let signature = signer.sign_message(&pending.message()).to_string();
    let error = wallet.confirm(&pending, signature).unwrap_err();
    assert_ne!(error, WalletError::SignatureInvalid);
}

#[test]
fn waits_for_a_transaction_by_its_signature() {
    let mut wallet = open(&Keypair::new()).unwrap();
    wallet.send_lookup = QUICK;
    assert_eq!(
        wallet
            .wait_for_transaction("not-a-signature".to_string(), None)
            .unwrap_err(),
        WalletError::SignatureInvalid
    );
    // A real signature gets as far as the transport, which answers no
    // lookup.
    let error = wallet
        .wait_for_transaction(Signature::default().to_string(), Some(100))
        .unwrap_err();
    assert!(
        matches!(error, WalletError::TransactionOutcomeUnknown { .. }),
        "{error:?}"
    );

    // Missing history does not prove that an old transaction never landed.
    let signer = Keypair::new();
    let (wallet, _) = rpc_wallet(&signer, |method| match method {
        "getBlockHeight" => serde_json::json!({"result": 10}),
        _ => status(false),
    });
    assert!(matches!(
        wallet.wait_for_transaction(Signature::default().to_string(), Some(5)),
        Err(WalletError::TransactionOutcomeUnknown { .. })
    ));
}

#[test]
fn recovers_a_success_missing_from_the_recent_status_cache() {
    let signature = Signature::from([3; 64]);
    let (mut wallet, asked) =
        rpc_wallet_asked(
            &Keypair::new(),
            config(),
            move |method, params| match method {
                "getSignatureStatuses" => {
                    assert_eq!(params[0], serde_json::json!([signature.to_string()]));
                    status(params[1]["searchTransactionHistory"] == true)
                }
                "getBlockHeight" => serde_json::json!({"result": 10}),
                "getShieldedTransactionsBySignature" => indexed(Some(signature)),
                _ => panic!("unexpected request: {method}"),
            },
        );
    wallet.send_lookup = QUICK;
    assert_eq!(
        wallet.wait_for_transaction(signature.to_string(), Some(5)),
        Ok(())
    );
    assert_eq!(
        *asked.lock().unwrap(),
        ["getSignatureStatuses", "getShieldedTransactionsBySignature"]
    );
}

#[test]
fn historical_failure_takes_precedence_over_indexed_events() {
    let signature = Signature::from([3; 64]);
    let (wallet, asked) = rpc_wallet_asked(&Keypair::new(), config(), move |method, params| {
        assert_eq!(method, "getSignatureStatuses");
        assert_eq!(params[1]["searchTransactionHistory"], true);
        let mut failed = status(true);
        failed["result"]["value"][0]["err"] = "InsufficientFundsForFee".into();
        failed["result"]["value"][0]["status"] =
            serde_json::json!({"Err": "InsufficientFundsForFee"});
        failed
    });
    assert!(
        matches!(wallet.wait_for_transaction(signature.to_string(), Some(5)),
            Err(WalletError::Client { message }) if message.contains("Insufficient funds for fee")
        )
    );
    assert_eq!(*asked.lock().unwrap(), ["getSignatureStatuses"]);
}

#[test]
fn indexed_events_recover_missing_or_unavailable_rpc_history() {
    let signature = Signature::from([3; 64]);
    for history in [
        status(false),
        serde_json::json!({"error": {"code": -32603, "message": "history unavailable"}}),
    ] {
        let (mut wallet, _) = rpc_wallet(&Keypair::new(), move |method| match method {
            "getSignatureStatuses" => history.clone(),
            "getShieldedTransactionsBySignature" => indexed(Some(signature)),
            _ => panic!("unexpected request: {method}"),
        });
        wallet.send_lookup = QUICK;
        assert_eq!(
            wallet.wait_for_transaction(signature.to_string(), Some(5)),
            Ok(())
        );
    }
}

#[test]
fn unrelated_or_unconfirmed_indexer_events_do_not_establish_success() {
    let signature = Signature::from([3; 64]);
    let mut ahead = indexed(Some(signature));
    ahead["result"]["context"]["slot"] = 4.into();
    for response in [indexed(Some(Signature::from([4; 64]))), ahead] {
        let (mut wallet, _) = rpc_wallet(&Keypair::new(), move |method| match method {
            "getSignatureStatuses" => status(false),
            "getBlockHeight" => serde_json::json!({"result": 10}),
            "getShieldedTransactionsBySignature" => response.clone(),
            _ => panic!("unexpected request: {method}"),
        });
        wallet.send_lookup = QUICK;
        assert!(matches!(
            wallet.wait_for_transaction(signature.to_string(), Some(5)),
            Err(WalletError::TransactionOutcomeUnknown { .. })
        ));
    }
}

#[test]
fn historical_success_still_waits_for_indexing() {
    let signature = Signature::from([3; 64]);
    let (mut wallet, asked) = rpc_wallet(&Keypair::new(), |method| match method {
        "getSignatureStatuses" => status(true),
        "getShieldedTransactionsBySignature" => indexed(None),
        _ => panic!("unexpected request: {method}"),
    });
    wallet.send_lookup = QUICK;
    assert_eq!(
        wallet.wait_for_transaction(signature.to_string(), Some(5)),
        Err(ClientError::IndexerTimeout.into())
    );
    let asked = asked.lock().unwrap();
    assert_eq!(
        asked
            .iter()
            .filter(|method| *method == "getSignatureStatuses")
            .count(),
        1
    );
    assert_eq!(
        asked
            .iter()
            .filter(|method| *method == "getShieldedTransactionsBySignature")
            .count(),
        QUICK.num_retries as usize + 1
    );
}

#[test]
fn a_transaction_never_seen_without_its_height_is_unknown() {
    let (wallet, asked) = recovery_wallet(vec![status(false)], 1_000);
    assert!(matches!(
        wallet.wait_for_transaction(Signature::from([3; 64]).to_string(), None),
        Err(WalletError::TransactionOutcomeUnknown { .. })
    ));
    assert_eq!(
        asked
            .lock()
            .unwrap()
            .iter()
            .filter(|method| *method == "getSignatureStatuses")
            .count(),
        QUICK.num_retries as usize + 1
    );
}

/// A refusal releases notes. Missing history leaves reservations alone,
/// through both `submit` and `confirm`.
#[test]
fn a_failed_send_releases_its_notes_unless_the_outcome_is_unknown() {
    let signer = Keypair::new();
    let internal = || serde_json::json!({"error": {"code": -32603, "message": "Internal error"}});
    let spend = |wallet: &MobileWallet| {
        let pending = PreparedTransaction {
            operation: Operation::Spend {
                mint: None,
                outputs: Vec::new(),
                created_token_accounts: Vec::new(),
            },
            last_valid_block_height: 5,
            ..unsigned(&signer)
        };
        wallet.reserve(pending, vec![[1; 32]]).unwrap()
    };
    let signed = |pending: &PreparedTransaction| {
        vec![signer.sign_message(&pending.message()).as_ref().to_vec()]
    };
    let held = |wallet: &MobileWallet| wallet.reservations().reserved.len();

    let (wallet, _) = rpc_wallet(&signer, |method| match method {
        "sendTransaction" => nullifier_queued(),
        _ => status(true),
    });
    let pending = spend(&wallet);
    assert_eq!(held(&wallet), 1);
    assert_eq!(
        wallet.submit(&pending, signed(&pending)).unwrap_err(),
        WalletError::NotesAlreadySpent
    );
    assert_eq!(held(&wallet), 0, "refused: the notes are free");

    let expired = move |method: &str| match method {
        "sendTransaction" => internal(),
        "getBlockHeight" => serde_json::json!({"result": 10}),
        _ => status(false),
    };
    let (wallet, _) = rpc_wallet(&signer, expired);
    let pending = spend(&wallet);
    assert!(matches!(
        wallet.submit(&pending, signed(&pending)),
        Err(WalletError::TransactionOutcomeUnknown { .. })
    ));
    assert_eq!(
        held(&wallet),
        1,
        "expired with missing history: the outcome is unknown"
    );

    let (wallet, _) = rpc_wallet(&signer, expired);
    let pending = spend(&wallet);
    let signature = signer.sign_message(&pending.message()).to_string();
    assert!(matches!(
        wallet.confirm(&pending, signature),
        Err(WalletError::TransactionOutcomeUnknown { .. })
    ));
    assert_eq!(held(&wallet), 1, "missing history after the app sent it");

    let (mut wallet, _) = rpc_wallet(&signer, move |_| internal());
    wallet.send_lookup = QUICK;
    let pending = spend(&wallet);
    assert!(matches!(
        wallet.submit(&pending, signed(&pending)),
        Err(WalletError::TransactionOutcomeUnknown { .. })
    ));
    assert_eq!(held(&wallet), 1, "unknown: the notes stay reserved");
}
