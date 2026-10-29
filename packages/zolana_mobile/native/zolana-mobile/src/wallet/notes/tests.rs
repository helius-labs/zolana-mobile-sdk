use super::*;
use crate::{
    error::WalletError,
    wallet::{
        tests::{note, open, unsigned},
        util::nullifiers,
    },
};
use solana_keypair::Keypair;
use solana_pubkey::Pubkey;
use std::collections::HashSet;
use zolana_transaction::{
    instructions::merge::MergeTransaction, Mint, TransactionError, WalletUtxo,
};

#[test]
fn descriptions_keep_tree_and_mint_and_exclude_unavailable_notes() {
    let mut data = note(30, 3, 1);
    data.data_hash = Some([1; 32]);
    let mut token = note(40, 4, 1);
    token.utxo.asset = Mint::new(Pubkey::new_unique(), 7);
    let mut ring = note(50, 5, 0);
    ring.ring_data_hash = Some([1; 32]);
    let notes = vec![
        note(10, 1, 0),
        note(20, 2, 1),
        data,
        token.clone(),
        ring,
        note(0, 6, 0),
    ];
    let listed = describe_notes(notes.iter(), &HashSet::from([[2; 32]]));
    assert_eq!(
        listed
            .iter()
            .map(|note| (note.amount, note.tree_id, note.mergeable))
            .collect::<Vec<_>>(),
        [(10, 0, true), (30, 1, false), (40, 1, true)]
    );
    assert_eq!(listed[0].mint, None);
    assert_eq!(listed[2].mint, Some(token.utxo.asset.asset.to_string()));
    assert_eq!(listed[0].id, hex::encode([1; 32]));
    let mut reordered = notes.clone();
    reordered.reverse();
    // Indexer order and metadata are not note identity.
    reordered.iter_mut().for_each(|note| {
        note.slot += 1;
        note.leaf_index += 1;
    });
    let mut again = describe_notes(reordered.iter(), &HashSet::from([[2; 32]]));
    again.reverse();
    assert_eq!(again, listed);
}

#[test]
fn exact_selection_does_not_substitute_missing_reserved_or_ineligible_notes() {
    let ids = [[1; 32], [2; 32]];
    let notes = [note(10, 1, 0), note(20, 2, 0), note(1, 3, 0)];
    assert!(select_merge_inputs(notes[1..].iter(), &ids, 0, &HashSet::new()).is_err());
    assert_eq!(
        select_merge_inputs(notes.iter(), &ids, 0, &HashSet::from([[2; 32]])).unwrap_err(),
        WalletError::NotesReserved { amount: 20 }
    );
    assert!(select_merge_inputs(notes.iter(), &ids, 1, &HashSet::new()).is_err());
    for kind in 0..4 {
        let mut notes = notes.clone();
        match kind {
            0 => notes[1].utxo.amount = 0,
            1 => notes[1].data_hash = Some([1; 32]),
            2 => notes[1].ring_data_hash = Some([1; 32]),
            _ => notes[1].utxo.ring_program_id = Some(Pubkey::new_unique()),
        }
        assert!(select_merge_inputs(notes.iter(), &ids, 0, &HashSet::new()).is_err());
    }
    let mut mixed = notes;
    mixed[1].utxo.asset = Mint::new(Pubkey::new_unique(), 7);
    let selected = select_merge_inputs(mixed.iter(), &ids, 0, &HashSet::new()).unwrap();
    assert!(matches!(
        MergeTransaction::new(selected),
        Err(TransactionError::MergeInputAssetMismatch { .. })
    ));
}

#[test]
fn input_ids_are_bounded_canonical_and_unique() {
    let ids: Vec<_> = (1..=51).map(|id| hex::encode([id; 32])).collect();
    assert_eq!(merge_input_ids(&ids).unwrap().len(), 51);
    assert_eq!(merge_input_ids(&ids[..2]).unwrap(), [[1; 32], [2; 32]]);
    for invalid in [
        vec![],
        ids[..1].to_vec(),
        vec![ids[0].clone(); 2],
        vec![ids[0].clone(); 52],
        vec!["AB".repeat(32), ids[0].clone()],
        vec!["0".repeat(63), ids[0].clone()],
        vec!["gg".repeat(32), ids[0].clone()],
    ] {
        assert!(merge_input_ids(&invalid).is_err());
    }
}

fn owned(wallet: &MobileWallet, amount: u64, nonce: u8) -> WalletUtxo {
    let mut note = note(amount, nonce, 0);
    note.utxo.owner = wallet.address.signing_pubkey;
    note.utxo.blinding[31] = nonce;
    note.nullifier_pubkey = wallet.address.nullifier_pubkey;
    note.utxo_hash = note
        .utxo
        .hash(&note.nullifier_pubkey, &[0; 32], &[0; 32], 0)
        .unwrap();
    note.nullifier = wallet
        .nullifier_key
        .nullifier(&note.utxo_hash, &note.utxo.blinding)
        .unwrap();
    note
}

#[test]
fn saved_order_survives_new_notes_rescans_and_wallet_restart() {
    let signer = Keypair::new();
    let wallet = open(&signer).unwrap();
    // Equal amounts make scan-order tie breaking especially dangerous.
    let notes = vec![owned(&wallet, 8, 1), owned(&wallet, 8, 2)];
    let ids: Vec<_> = describe_notes(notes.iter(), &HashSet::new())
        .into_iter()
        .map(|note| note.id)
        .collect();
    let build = |wallet: &MobileWallet, notes: &[WalletUtxo], ids: &[String]| {
        let selected = select_merge_inputs(
            notes.iter(),
            &merge_input_ids(ids).unwrap(),
            0,
            &HashSet::new(),
        )
        .unwrap();
        MergeTransaction::new(selected)
            .unwrap()
            .with_output_tree_id(0)
            .encrypt(&wallet.keys)
            .unwrap()
    };
    let original = build(&wallet, &notes, &ids);
    let reopened = open(&signer).unwrap();
    let rescan = vec![owned(&reopened, 1, 3), notes[1].clone(), notes[0].clone()];
    let saved: Vec<String> = serde_json::from_str(&serde_json::to_string(&ids).unwrap()).unwrap();
    let retried = build(&reopened, &rescan, &saved);
    assert_eq!(
        original.output_hash().unwrap(),
        retried.output_hash().unwrap()
    );
    assert_eq!(retried.output_utxo.amount, 16);
    assert_eq!(retried.input_utxos[0].utxo_hash, notes[0].utxo_hash);
    assert_eq!(retried.input_utxos[1].utxo_hash, notes[1].utxo_hash);
    assert_ne!(
        original.output_ciphertext_salt,
        retried.output_ciphertext_salt
    );
    let swapped = build(&reopened, &rescan, &[ids[1].clone(), ids[0].clone()]);
    assert_ne!(
        original.output_hash().unwrap(),
        swapped.output_hash().unwrap()
    );
}

#[test]
fn restored_reservations_exclude_planned_inputs_until_reconciled() {
    let signer = Keypair::new();
    let wallet = open(&signer).unwrap();
    let notes = [owned(&wallet, 5, 1), owned(&wallet, 8, 2)];
    let pending = wallet
        .reserve_until(unsigned(&signer), nullifiers(&notes), u64::MAX)
        .unwrap();
    let snapshot = wallet.export_prepared_transaction(&pending).unwrap();
    let reopened = open(&signer).unwrap();
    let restored = reopened.restore_prepared_transaction(snapshot).unwrap();
    let held: HashSet<_> = reopened
        .reservations()
        .reserved
        .values()
        .flat_map(|r| r.spends.iter().copied())
        .collect();
    assert!(describe_notes(notes.iter(), &held).is_empty());
    assert!(matches!(
        select_merge_inputs(
            notes.iter(),
            &[notes[0].utxo_hash, notes[1].utxo_hash],
            0,
            &held
        ),
        Err(WalletError::NotesReserved { .. })
    ));
    reopened.release(&restored);
    assert!(reopened.reservations().reserved.is_empty());
}
