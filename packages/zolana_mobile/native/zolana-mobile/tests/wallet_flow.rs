//! The whole wallet flow against a live cluster and indexer, proving on this
//! machine with the pinned keys: register, deposit, sync, private transfer,
//! and the recipient's sync.
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

use std::env;

use solana_keypair::Keypair;
use solana_signer::Signer;
use zolana_client::SolanaRpc;
use zolana_mobile::{derivation_message, MobileWallet, PendingTransaction, WalletConfig};

const FUNDING: u64 = 2_000_000_000;
const DEPOSIT: u64 = 500_000_000;
const TRANSFER: u64 = 200_000_000;

fn required(name: &str) -> String {
    env::var(name).unwrap_or_else(|_| panic!("set {name}"))
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
fn submit(wallet: &MobileWallet, signer: &Keypair, pending: PendingTransaction) -> String {
    assert_eq!(pending.signers(), vec![signer.pubkey().to_string()]);
    let signature = signer.sign_message(&pending.message_bytes());
    println!("{}", pending.summary());
    wallet
        .submit(pending, vec![signature.as_ref().to_vec()])
        .expect("submit")
}

fn register(wallet: &MobileWallet, signer: &Keypair) {
    if let Some(pending) = wallet.prepare_registration().expect("prepare registration") {
        submit(wallet, signer, pending);
    }
    assert!(wallet.is_registered().unwrap());
}

#[test]
#[ignore = "needs a Zolana cluster and indexer"]
fn register_deposit_transfer_and_receive() {
    let sender = Keypair::new();
    let recipient = Keypair::new();
    let mut rpc = SolanaRpc::new(required("ZOLANA_E2E_RPC_URL"));
    for signer in [&sender, &recipient] {
        rpc.airdrop(&signer.pubkey(), FUNDING).expect("airdrop");
    }

    let mut sender_wallet = open(&sender);
    let mut recipient_wallet = open(&recipient);
    register(&sender_wallet, &sender);
    register(&recipient_wallet, &recipient);

    let pending = sender_wallet.prepare_deposit(DEPOSIT).unwrap();
    submit(&sender_wallet, &sender, pending);
    assert_eq!(sender_wallet.sync().unwrap().private_lamports, DEPOSIT);

    let started = std::time::Instant::now();
    let pending = sender_wallet
        .prepare_transfer(recipient.pubkey().to_string(), TRANSFER)
        .expect("prove transfer");
    println!("built and proved on device in {:?}", started.elapsed());
    submit(&sender_wallet, &sender, pending);

    assert_eq!(
        sender_wallet.sync().unwrap().private_lamports,
        DEPOSIT - TRANSFER
    );
    assert_eq!(recipient_wallet.sync().unwrap().private_lamports, TRANSFER);
}
