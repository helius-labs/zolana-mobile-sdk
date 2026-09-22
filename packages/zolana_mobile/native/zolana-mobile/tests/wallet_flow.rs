//! The whole wallet flow against a live cluster and indexer, proving on this
//! machine with the pinned keys: register, deposit, sync, private transfer,
//! the recipient's sync, and a withdrawal from a session that is stale.
//!
//! Run against a Zolana localnet (`just` in the zolana repository starts
//! surfpool, Photon and the programs), or any cluster that runs the pinned
//! program revision:
//!
//! ```sh
//! ZOLANA_E2E_RPC_URL=http://127.0.0.1:8899 \
//! ZOLANA_E2E_INDEXER_URL=http://127.0.0.1:8784 \
//! cargo test -p zolana-mobile --test wallet_flow -- --ignored --nocapture
//! ```
//!
//! `ZOLANA_E2E_KEY_DIR` keeps downloaded proving keys between runs.
//! `ZOLANA_E2E_SENDER_SEED` / `ZOLANA_E2E_RECIPIENT_SEED` (32-byte hex) reuse
//! funded accounts, such as the example's demo accounts, where airdrops are
//! rate limited; otherwise fresh accounts are airdropped.

use std::env;

use solana_keypair::Keypair;
use solana_signer::Signer;
use zolana_client::{Rpc, SolanaRpc};
use zolana_mobile::{derivation_message, MobileWallet, PendingTransaction, WalletConfig};

const FUNDING: u64 = 1_000_000_000;
const DEPOSIT: u64 = 50_000_000;
const TRANSFER: u64 = 20_000_000;
const WITHDRAWAL: u64 = 10_000_000;
const FEES: u64 = 20_000_000;

fn required(name: &str) -> String {
    env::var(name).unwrap_or_else(|_| panic!("set {name}"))
}

fn account(seed_var: &str) -> Keypair {
    let Ok(seed) = env::var(seed_var) else {
        return Keypair::new();
    };
    let bytes: Vec<u8> = (0..seed.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&seed[i..i + 2], 16).expect("hex seed"))
        .collect();
    Keypair::new_from_array(bytes.try_into().expect("32-byte seed"))
}

fn open(signer: &Keypair) -> MobileWallet {
    let key_dir = env::var("ZOLANA_E2E_KEY_DIR").unwrap_or_else(|_| {
        env::temp_dir()
            .join("zolana-e2e-keys")
            .display()
            .to_string()
    });
    let pubkey = signer.pubkey().to_string();
    let signature = signer.sign_message(&derivation_message(pubkey.clone()).unwrap());
    MobileWallet::open(
        WalletConfig {
            rpc_url: required("ZOLANA_E2E_RPC_URL"),
            indexer_url: required("ZOLANA_E2E_INDEXER_URL"),
            proving_key_dir: key_dir,
            proving_key_url: env::var("ZOLANA_E2E_KEY_URL").ok(),
            allow_insecure_http: true,
        },
        pubkey,
        signature.as_ref().to_vec(),
    )
    .expect("open wallet")
}

/// Sign as every required signer, which in this flow is the wallet owner.
fn submit(wallet: &mut MobileWallet, signer: &Keypair, pending: PendingTransaction) -> String {
    assert_eq!(pending.signers(), vec![signer.pubkey().to_string()]);
    let signature = signer.sign_message(&pending.message_bytes());
    println!("{}", pending.summary());
    wallet
        .submit(pending, vec![signature.as_ref().to_vec()])
        .expect("submit")
}

fn register(wallet: &mut MobileWallet, signer: &Keypair) {
    if let Some(pending) = wallet.prepare_registration().expect("prepare registration") {
        submit(wallet, signer, pending);
    }
    assert!(wallet.is_registered().unwrap());
}

#[test]
#[ignore = "needs a Zolana cluster and indexer"]
fn register_deposit_transfer_and_receive() {
    let sender = account("ZOLANA_E2E_SENDER_SEED");
    let recipient = account("ZOLANA_E2E_RECIPIENT_SEED");
    let mut rpc = SolanaRpc::new(required("ZOLANA_E2E_RPC_URL"));
    // The sender pays the deposit and fees, the recipient only registration.
    for (signer, needed) in [(&sender, DEPOSIT + FEES), (&recipient, FEES)] {
        if rpc.get_balance(signer.pubkey()).expect("balance") < needed {
            rpc.airdrop(&signer.pubkey(), FUNDING).expect("airdrop");
        }
    }

    let mut sender_wallet = open(&sender);
    let mut recipient_wallet = open(&recipient);
    register(&mut sender_wallet, &sender);
    register(&mut recipient_wallet, &recipient);
    // Reused accounts may already hold private balances from earlier runs.
    let sender_before = sender_wallet.sync().unwrap().private_lamports;
    let recipient_before = recipient_wallet.sync().unwrap().private_lamports;

    let pending = sender_wallet.prepare_deposit(DEPOSIT).unwrap();
    submit(&mut sender_wallet, &sender, pending);
    assert_eq!(
        sender_wallet.sync().unwrap().private_lamports,
        sender_before + DEPOSIT
    );

    // A second session of the sender, synced before the transfer spends its
    // notes: the same account on another device.
    let mut stale = open(&sender);
    stale.sync().unwrap();

    let started = std::time::Instant::now();
    let pending = sender_wallet
        .prepare_transfer(recipient.pubkey().to_string(), TRANSFER)
        .expect("prove transfer");
    println!("built and proved on device in {:?}", started.elapsed());
    submit(&mut sender_wallet, &sender, pending);

    assert_eq!(
        sender_wallet.sync().unwrap().private_lamports,
        sender_before + DEPOSIT - TRANSFER
    );
    assert_eq!(
        recipient_wallet.sync().unwrap().private_lamports,
        recipient_before + TRANSFER
    );

    // The stale session must not pick a note the transfer already spent.
    let pending = stale
        .prepare_withdrawal(sender.pubkey().to_string(), WITHDRAWAL)
        .expect("a stale session still builds a spendable withdrawal");
    submit(&mut stale, &sender, pending);
    assert_eq!(
        stale.private_lamports().unwrap(),
        sender_before + DEPOSIT - TRANSFER - WITHDRAWAL
    );
}
