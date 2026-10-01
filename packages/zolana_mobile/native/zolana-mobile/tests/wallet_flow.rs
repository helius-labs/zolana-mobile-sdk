//! The whole wallet flow against a live cluster and indexer, proving on this
//! machine with the pinned keys: register, deposit, private transfer sent with
//! a refreshed blockhash, the recipient's balance, a withdrawal from a session
//! opened before the transfer from the sender's saved keys, with its fee paid
//! by another account, and two transfers prepared before either is sent. Each
//! step is the newest entry of its wallets' history.
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
use zolana_mobile::{
    derivation_message, ActivityKind, MobileWallet, PendingTransaction, RegistrationStatus,
    WalletConfig,
};

const FUNDING: u64 = 1_000_000_000;
const DEPOSIT: u64 = 50_000_000;
const TRANSFER: u64 = 20_000_000;
const WITHDRAWAL: u64 = 10_000_000;
/// Each of two transfers prepared together, and the deposit that makes sure
/// the sender has a second note for them.
const SPLIT: u64 = 1_000_000;
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

fn config() -> WalletConfig {
    let key_dir = env::var("ZOLANA_E2E_KEY_DIR").unwrap_or_else(|_| {
        env::temp_dir()
            .join("zolana-e2e-keys")
            .display()
            .to_string()
    });
    WalletConfig {
        rpc_url: required("ZOLANA_E2E_RPC_URL"),
        rpc_headers: None,
        indexer_url: required("ZOLANA_E2E_INDEXER_URL"),
        indexer_headers: None,
        proving_key_dir: key_dir,
        proving_key_url: env::var("ZOLANA_E2E_KEY_URL").ok(),
        allow_insecure_http: true,
        mints: Vec::new(),
    }
}

fn open(signer: &Keypair) -> MobileWallet {
    let pubkey = signer.pubkey().to_string();
    let signature = signer.sign_message(&derivation_message(pubkey.clone()).unwrap());
    MobileWallet::open(config(), pubkey, signature.as_ref().to_vec()).expect("open wallet")
}

/// Sign as `signers`, which must be the required signers in order.
fn submit(wallet: &MobileWallet, signers: &[&Keypair], pending: PendingTransaction) -> String {
    let expected: Vec<String> = signers.iter().map(|s| s.pubkey().to_string()).collect();
    assert_eq!(pending.signers(), expected);
    let signatures = signers
        .iter()
        .map(|s| s.sign_message(&pending.message_bytes()).as_ref().to_vec())
        .collect();
    println!("{}", pending.summary());
    wallet.submit(&pending, signatures).expect("submit")
}

/// The private SOL balance, read from the indexer now.
fn private_sol(wallet: &mut MobileWallet) -> u64 {
    wallet.private_balance(None).unwrap()
}

/// The newest history entry is the transaction `signature`, moving `amount`
/// SOL as `kind`.
fn newest(wallet: &mut MobileWallet, signature: &str, kind: ActivityKind, amount: u64) {
    let activity = wallet.activity().unwrap();
    let entry = activity.first().expect("an activity entry");
    assert_eq!(
        (
            entry.signature.as_str(),
            entry.kind,
            entry.mint.as_deref(),
            entry.amount
        ),
        (signature, kind, None, amount)
    );
}

fn register(wallet: &mut MobileWallet, signer: &Keypair) {
    if let Some(pending) = wallet.prepare_registration().expect("prepare registration") {
        submit(wallet, &[signer], pending);
    }
    assert_eq!(
        wallet.registration_status().unwrap(),
        RegistrationStatus::Registered
    );
}

#[test]
#[ignore = "needs a Zolana cluster and indexer"]
fn register_deposit_transfer_and_receive() {
    let sender = account("ZOLANA_E2E_SENDER_SEED");
    let recipient = account("ZOLANA_E2E_RECIPIENT_SEED");
    let mut rpc = SolanaRpc::new(required("ZOLANA_E2E_RPC_URL"));
    // The sender pays the deposits and fees, the recipient registration and
    // the withdrawal fee.
    for (signer, needed) in [(&sender, DEPOSIT + SPLIT + FEES), (&recipient, FEES)] {
        if rpc.get_balance(signer.pubkey()).expect("balance") < needed {
            rpc.airdrop(&signer.pubkey(), FUNDING).expect("airdrop");
        }
    }

    let mut sender_wallet = open(&sender);
    let mut recipient_wallet = open(&recipient);
    register(&mut sender_wallet, &sender);
    register(&mut recipient_wallet, &recipient);
    // Reused accounts may already hold private balances from earlier runs.
    let sender_before = private_sol(&mut sender_wallet);
    let recipient_before = private_sol(&mut recipient_wallet);

    let pending = sender_wallet.prepare_deposit(None, DEPOSIT).unwrap();
    let deposit = submit(&sender_wallet, &[&sender], pending);
    assert_eq!(private_sol(&mut sender_wallet), sender_before + DEPOSIT);
    newest(
        &mut sender_wallet,
        &deposit,
        ActivityKind::Shielded,
        DEPOSIT,
    );

    // A second session of the sender, opened before the transfer spends its
    // notes from the keys the first one exported: the same account on another
    // device.
    let mut stale = MobileWallet::open_with_keys(
        config(),
        sender.pubkey().to_string(),
        sender_wallet.export_keys(),
    )
    .expect("open from saved keys");
    assert_eq!(stale.shielded_address(), sender_wallet.shielded_address());

    let started = std::time::Instant::now();
    let pending = sender_wallet
        .prepare_transfer(recipient.pubkey().to_string(), None, TRANSFER, None)
        .expect("prove transfer");
    println!("built and proved on device in {:?}", started.elapsed());
    // A slow approval: the same proof under a new blockhash.
    let refreshed = (0..40)
        .find_map(|_| {
            std::thread::sleep(std::time::Duration::from_millis(500));
            let refreshed = sender_wallet.refresh(&pending).expect("refresh");
            (refreshed.message_bytes() != pending.message_bytes()).then_some(refreshed)
        })
        .expect("a new blockhash");
    assert!(refreshed.last_valid_block_height() >= pending.last_valid_block_height());
    assert_eq!(refreshed.signers(), pending.signers());
    assert_eq!(refreshed.summary(), pending.summary());
    let transfer = submit(&sender_wallet, &[&sender], refreshed);

    assert_eq!(
        private_sol(&mut sender_wallet),
        sender_before + DEPOSIT - TRANSFER
    );
    assert_eq!(
        private_sol(&mut recipient_wallet),
        recipient_before + TRANSFER
    );
    newest(&mut sender_wallet, &transfer, ActivityKind::Sent, TRANSFER);
    // After a restart the signature alone is enough to wait for it.
    recipient_wallet
        .wait_for_transaction(transfer.clone())
        .expect("wait for a landed transaction");
    newest(
        &mut recipient_wallet,
        &transfer,
        ActivityKind::Received,
        TRANSFER,
    );

    // The other session must not pick a note the transfer already spent.
    // The recipient pays the fee, so the sender receives the full amount.
    let public_before = rpc.get_balance(sender.pubkey()).unwrap();
    let pending = stale
        .prepare_withdrawal(
            sender.pubkey().to_string(),
            None,
            WITHDRAWAL,
            Some(recipient.pubkey().to_string()),
        )
        .expect("another session still builds a spendable withdrawal");
    let withdrawal = submit(&stale, &[&recipient, &sender], pending);
    assert_eq!(
        rpc.get_balance(sender.pubkey()).unwrap(),
        public_before + WITHDRAWAL
    );
    newest(
        &mut stale,
        &withdrawal,
        ActivityKind::Unshielded,
        WITHDRAWAL,
    );
    assert_eq!(
        stale.private_balance(None).unwrap(),
        sender_before + DEPOSIT - TRANSFER - WITHDRAWAL
    );

    // Two transfers prepared before either is sent take different notes, so
    // both land: the first reserves the notes it spends.
    let pending = sender_wallet.prepare_deposit(None, SPLIT).unwrap();
    submit(&sender_wallet, &[&sender], pending);
    let recipient_before = private_sol(&mut recipient_wallet);
    let prepare = |wallet: &mut MobileWallet| {
        wallet
            .prepare_transfer(recipient.pubkey().to_string(), None, SPLIT, None)
            .expect("prove transfer")
    };
    let first = prepare(&mut sender_wallet);
    let second = prepare(&mut sender_wallet);
    submit(&sender_wallet, &[&sender], first);
    submit(&sender_wallet, &[&sender], second);
    assert_eq!(
        private_sol(&mut recipient_wallet),
        recipient_before + 2 * SPLIT
    );
    assert_eq!(
        private_sol(&mut sender_wallet),
        sender_before + DEPOSIT - TRANSFER - WITHDRAWAL - SPLIT
    );
}
