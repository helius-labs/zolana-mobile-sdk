use super::*;

/// The notes and history wait for the indexer to reach the RPC's slot,
/// and a stalled indexer fails as `IndexerBehind` instead of answering
/// with notes another session may have spent.
#[test]
fn notes_are_read_once_the_indexer_has_the_rpcs_slot() {
    let signer = Keypair::new();
    let at_slot = |indexed: u64| {
        move |method: &str| match method {
            "getSlot" => serde_json::json!({"result": 100}),
            _ => serde_json::json!({"result": {
                "context": {"blockTime": 0, "slot": indexed},
                "transactions": [],
                "matches": [],
                "nextCursor": null,
            }}),
        }
    };

    let (mut wallet, asked) = rpc_wallet(&signer, at_slot(100));
    assert!(wallet.balances().expect("caught up").is_empty());
    assert!(wallet.activity().expect("caught up").is_empty());
    asked.lock().unwrap().clear();
    assert_eq!(
        wallet.overview().expect("caught up"),
        WalletOverview {
            balances: Vec::new(),
            activity: Vec::new(),
            unresolved_assets: UnresolvedAssets {
                asset_ids: Vec::new(),
                mints: Vec::new(),
            },
        }
    );
    // One read of each stream serves both halves.
    let tag_reads = asked
        .lock()
        .unwrap()
        .iter()
        .filter(|method| *method == "getShieldedTransactionsByTags")
        .count();
    assert_eq!(tag_reads, 1);

    let (mut wallet, asked) = rpc_wallet(&signer, at_slot(90));
    let behind = WalletError::IndexerBehind {
        indexer_slot: 90,
        rpc_slot: 100,
    };
    assert_eq!(wallet.balances().unwrap_err(), behind);
    let reads = asked
        .lock()
        .unwrap()
        .iter()
        .filter(|method| *method == "getShieldedTransactionsByTags")
        .count();
    assert_eq!(reads, INDEXER_CATCH_UP.num_retries as usize + 1);
}

#[test]
fn balances_count_their_notes_and_what_prepared_spends_hold() {
    let notes = || vec![note(50, 1, 0), note(30, 2, 0), note(0, 3, 0)];
    assert_eq!(
        token_balance(&balance(notes()), &reserved(&[]), MAX_SPEND_INPUTS),
        TokenBalance {
            mint: None,
            amount: 80,
            notes: 3,
            reserved: 0,
            limits: [2, 4, 8, 16]
                .map(|outputs| SpendLimit {
                    outputs,
                    amount: 80
                })
                .to_vec(),
        }
    );
    // A reserved note stays in the amount and leaves the limits.
    let held = token_balance(&balance(notes()), &reserved(&[1]), MAX_SPEND_INPUTS);
    assert_eq!((held.amount, held.reserved), (80, 50));
    assert_eq!(limits(&held), [(2, 30), (4, 30), (8, 30), (16, 30)]);
}

/// Each width's limit is what selection takes from the free notes,
/// largest first: at most the width's cap, or the configured one, and
/// nothing from a third tree.
#[test]
fn limits_stop_at_the_cap_and_before_a_third_tree() {
    let three = || vec![note(50, 1, 0), note(30, 2, 0), note(20, 3, 0)];
    assert_eq!(
        limits(&token_balance(&balance(three()), &reserved(&[]), 2)),
        [(2, 80), (4, 80), (8, 80), (16, 80)]
    );
    assert_eq!(
        limits(&token_balance(&balance(three()), &reserved(&[]), 1)),
        [(2, 50), (4, 50), (8, 50), (16, 50)]
    );
    // The note on the third tree stops the count, and the smaller note
    // on the first tree after it is not taken either.
    let trees = vec![
        note(30, 1, 0),
        note(20, 2, 1),
        note(10, 3, 2),
        note(5, 4, 0),
    ];
    assert_eq!(
        token_balance(&balance(trees), &reserved(&[]), MAX_SPEND_INPUTS).limits[0].amount,
        50
    );
    // Of 25 equal notes, the 2-output keys take all, the 4-output keys
    // 24, the 8-output keys 16 and the 16-output keys 8.
    let many = balance((1..=25).map(|n| note(10, n, 0)).collect());
    assert_eq!(
        limits(&token_balance(&many, &reserved(&[]), MAX_SPEND_INPUTS)),
        [(2, 250), (4, 240), (8, 160), (16, 80)]
    );
    assert_eq!(proof_widths(), [2, 4, 8, 16]);
}

/// Every amount up to a width's limit selects within that width's cap,
/// and one more fails: short, too many notes or too many trees.
#[test]
fn every_amount_up_to_a_limit_selects_and_one_more_does_not() {
    let sets: [Vec<WalletUtxo>; 5] = [
        vec![note(50, 1, 0), note(30, 2, 0), note(20, 3, 0)],
        vec![
            note(30, 1, 0),
            note(20, 2, 1),
            note(10, 3, 2),
            note(5, 4, 0),
        ],
        (1..=41).map(|n| note(10, n, 0)).collect(),
        vec![note(7, 1, 0), note(7, 2, 1), note(7, 3, 2)],
        vec![note(0, 1, 0), note(9, 2, 0)],
    ];
    let free = reserved(&[]);
    for notes in sets {
        let spendable = spendable(notes);
        let balance = &spendable.balances.assets[0];
        for width in proof_widths() {
            let cap = max_spend_inputs(width);
            let limit = spend_limit(&balance.utxos, &free, cap);
            let select = |amount| {
                select_spend_excluding(
                    spendable.utxos(),
                    zolana_transaction::SOL_MINT,
                    amount,
                    &free,
                )
                .map_err(|failure| spend_failure(amount, failure))
                .and_then(|inputs| within_cap(inputs, amount, cap))
            };
            for amount in 1..=limit {
                assert!(select(amount).is_ok(), "{amount} of {limit} at {width}");
            }
            assert!(
                select(limit + 1).is_err(),
                "{} past {limit} at {width}",
                limit + 1
            );
        }
    }
}

#[test]
fn a_spend_takes_at_most_the_configured_notes() {
    let notes = vec![note(50, 1, 0), note(30, 2, 0), note(20, 3, 0)];
    assert_eq!(within_cap(notes.clone(), 100, 3).unwrap().len(), 3);
    assert_eq!(
        within_cap(notes, 100, 2).unwrap_err(),
        WalletError::MergeRequired {
            amount: 100,
            max_inputs: 2
        }
    );

    let signer = Keypair::new();
    let with = |max_spend_inputs| {
        open_with(
            &signer,
            WalletConfig {
                max_spend_inputs,
                ..config()
            },
        )
    };
    for (max, cap) in [(None, MAX_SPEND_INPUTS), (Some(4), 4), (Some(40), 40)] {
        assert_eq!(with(max).unwrap().max_spend_inputs, cap);
    }
    for max in [0, 41] {
        assert_eq!(
            with(Some(max)).err(),
            Some(WalletError::InvalidMaxSpendInputs {
                max_inputs: max.into(),
                limit: 40
            })
        );
    }
}

/// With `reuse_notes_ms`, a spend takes its notes from a read made within
/// the window instead of reading again; a send or a confirmation ends
/// the reuse, and without the window every spend reads.
#[test]
fn a_spend_reuses_the_last_read_within_its_window() {
    let signer = Keypair::new();
    let caught_up = |method: &str| match method {
        "getSlot" => serde_json::json!({"result": 100}),
        "getBlockHeight" => serde_json::json!({"result": 5}),
        _ => serde_json::json!({"result": {
            "context": {"blockTime": 0, "slot": 100},
            "transactions": [],
            "matches": [],
            "nextCursor": null,
        }}),
    };
    let tag_reads = |asked: &Arc<Mutex<Vec<String>>>| {
        asked
            .lock()
            .unwrap()
            .iter()
            .filter(|method| *method == "getShieldedTransactionsByTags")
            .count()
    };
    let (mut wallet, asked) = rpc_wallet_with(
        &signer,
        WalletConfig {
            reuse_notes_ms: Some(60_000),
            ..config()
        },
        caught_up,
    );
    wallet.balances().unwrap();
    assert_eq!(tag_reads(&asked), 1);
    // The merge finds nothing to take, in the notes of the read.
    assert!(matches!(
        wallet.prepare_merge(None, None, None, None, None).err(),
        Some(WalletError::NothingToMerge { .. })
    ));
    assert_eq!(tag_reads(&asked), 1, "no read within the window");
    wallet.forget_notes();
    let _ = wallet.prepare_merge(None, None, None, None, None);
    assert_eq!(tag_reads(&asked), 2, "read again once forgotten");
    let stale = Instant::now().checked_sub(Duration::from_secs(61)).unwrap();
    wallet.recent().as_mut().unwrap().at = stale;
    let _ = wallet.prepare_merge(None, None, None, None, None);
    assert_eq!(tag_reads(&asked), 3, "a read older than the window");

    let (mut wallet, asked) = rpc_wallet_with(&signer, config(), caught_up);
    wallet.balances().unwrap();
    let _ = wallet.prepare_merge(None, None, None, None, None);
    assert_eq!(tag_reads(&asked), 2, "every spend reads without a window");
}

#[test]
fn selection_leaves_out_reserved_notes_and_reports_what_it_lacks() {
    let notes = spendable(vec![note(50, 1, 0), note(30, 2, 0), note(20, 3, 0)]);
    let select = |notes: &SpendableDecryptionResult, mint, amount, reserved: &[u8]| {
        let reserved = reserved.iter().map(|&n| [n; 32]).collect();
        select_spend_excluding(notes.utxos(), mint, amount, &reserved)
            .map(|picked| nullifiers(&picked))
            .map_err(|failure| spend_failure(amount, failure))
    };
    let sol = zolana_transaction::SOL_MINT;
    assert_eq!(select(&notes, sol, 40, &[]), Ok(vec![[1; 32]]));
    // The largest note is reserved: the others cover it.
    assert_eq!(select(&notes, sol, 40, &[1]), Ok(vec![[2; 32], [3; 32]]));
    assert_eq!(
        select(&notes, sol, 60, &[1]).unwrap_err(),
        WalletError::NotesReserved { amount: 60 }
    );
    assert_eq!(
        select(&notes, sol, 200, &[1]).unwrap_err(),
        WalletError::InsufficientPrivateBalance {
            requested: 200,
            available: 50
        }
    );
    assert_eq!(
        select(&notes, Pubkey::new_unique(), 1, &[]).unwrap_err(),
        WalletError::InsufficientPrivateBalance {
            requested: 1,
            available: 0
        }
    );
    assert_eq!(
        select(&notes, sol, 0, &[]).unwrap_err(),
        WalletError::AmountZero
    );
    // A spend takes notes from two trees, not from three.
    let two_trees = spendable(vec![note(50, 1, 0), note(30, 2, 1)]);
    assert_eq!(select(&two_trees, sol, 70, &[]), Ok(vec![[1; 32], [2; 32]]));
    let three_trees = spendable(vec![note(30, 1, 0), note(20, 2, 1), note(10, 3, 2)]);
    assert_eq!(
        select(&three_trees, sol, 55, &[]).unwrap_err(),
        WalletError::TooManyInputTrees {
            trees: 3,
            max_trees: 2
        }
    );
    let many = spendable((1..=41).map(|n| note(10, n, 0)).collect());
    assert_eq!(
        select(&many, sol, 410, &[]).unwrap_err(),
        WalletError::MergeRequired {
            amount: 410,
            max_inputs: 40
        }
    );
}
