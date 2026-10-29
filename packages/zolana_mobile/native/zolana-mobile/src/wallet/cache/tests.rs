use super::*;
use crate::wallet::tests::{open, unsigned};
use solana_keypair::Keypair;
use solana_pubkey::Pubkey;
use solana_signer::Signer;
use std::sync::{Arc, Mutex};
use zolana_interface::{state::cache::CacheAccount, PROGRAM_ID_PUBKEY};

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
        .prepare_cached_merge(terms, None, None, None, None, None)
        .is_err());
    let mut terms = target();
    terms.expires_at = now() - 1;
    assert!(wallet
        .prepare_cached_merge(terms, None, None, None, None, None)
        .is_err());
    assert!(wallet
        .prepare_cached_merge(target(), None, None, None, Some(5_001), None)
        .is_err());
}

#[test]
fn explicit_merge_arguments_fail_before_network_or_proof() {
    let mut wallet = open(&Keypair::new()).unwrap();
    let ids = vec![hex::encode([1; 32]), hex::encode([2; 32])];
    for (mint, max_inputs, ids) in [
        (Some("mint".to_owned()), None, ids.clone()),
        (None, Some(2), ids.clone()),
        (None, None, vec![ids[0].clone()]),
        (None, None, vec![ids[0].clone(); 2]),
        (None, None, vec!["bad ID".to_owned(); 2]),
    ] {
        assert!(wallet
            .prepare_cached_merge(target(), mint, max_inputs, None, None, Some(ids))
            .is_err());
        assert!(wallet.reservations().reserved.is_empty());
    }
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
    let mut envelope: serde_json::Value = serde_json::from_str(&snapshot).unwrap();
    envelope["payload"] =
        serde_json::json!(envelope["payload"].as_str().unwrap().replace("123", "124"));
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
    for mutation in 0..8 {
        let mut changed = cache;
        match mutation {
            0 => changed.discriminator = 0,
            1 => changed.rent_sponsor = Pubkey::new_unique(),
            2 => changed.write_authority = Pubkey::new_unique(),
            3 => changed.tree_id = 1u16.to_le_bytes(),
            4 => changed.expires_at = (terms.expires_at + 1).to_le_bytes(),
            5 => changed.utxo_hashes[0] = [0; 32],
            _ => {}
        }
        let mut bytes = bytemuck::bytes_of(&changed).to_vec();
        if mutation == 6 {
            bytes.push(0);
        }
        let mut changed = value.clone();
        changed["value"]["data"] = json!([STANDARD.encode(bytes), "base64"]);
        if mutation == 7 {
            changed["value"] = serde_json::Value::Null;
        }
        *response.lock().unwrap() = changed;
        assert!(
            wallet.verify_cached_outputs(vec![output.clone()]).is_err(),
            "mutation {mutation}"
        );
    }
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
