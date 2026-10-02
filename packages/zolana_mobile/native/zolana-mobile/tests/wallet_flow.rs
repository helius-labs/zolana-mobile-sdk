//! The whole wallet flow against a live cluster and indexer, proving on this
//! machine with the pinned keys: register, deposit, private transfer sent with
//! a refreshed blockhash, the recipient's balance, a withdrawal from a session
//! opened before the transfer from the sender's saved keys, with its fee paid
//! by another account, two transfers prepared before either is sent, and a
//! transfer proved by the application's backend. Each step is the newest entry
//! of its wallets' history.
//!
//! Run against a Zolana localnet (`just` in the zolana repository starts
//! surfpool, Photon, the prover and the programs), or any cluster that runs the
//! pinned program revision:
//!
//! ```sh
//! ZOLANA_E2E_RPC_URL=http://127.0.0.1:8899 \
//! ZOLANA_E2E_INDEXER_URL=http://127.0.0.1:8784 \
//! ZOLANA_E2E_PROVER_URL=http://127.0.0.1:3001 \
//! cargo test -p zolana-mobile --test wallet_flow -- --ignored --nocapture
//! ```
//!
//! The backend in this test forwards each request to the prover at
//! `ZOLANA_E2E_PROVER_URL`, such as the Helius gateway
//! (`https://beta-devnet.helius-rpc.com/v1/zolana?api-key=...`).
//!
//! `ZOLANA_E2E_KEY_DIR` keeps downloaded proving keys between runs.
//! `ZOLANA_E2E_SENDER_SEED` / `ZOLANA_E2E_RECIPIENT_SEED` (32-byte hex) reuse
//! funded accounts, such as the example's demo accounts, where airdrops are
//! rate limited; otherwise fresh accounts are airdropped.

use std::{env, future::Future, pin::Pin, thread::sleep, time::Duration};

use solana_keypair::Keypair;
use solana_signer::Signer;
use zolana_client::{Rpc, SolanaRpc};
use zolana_mobile::{
    derivation_message, ActivityKind, MobileWallet, PendingTransaction, Proving,
    RegistrationStatus, Transport, TransportOutcome, TransportRequest, TransportResponse,
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
        indexer_url: required("ZOLANA_E2E_INDEXER_URL"),
        proving_key_dir: key_dir,
        proving_key_url: env::var("ZOLANA_E2E_KEY_URL").ok(),
        proving: None,
        mints: Vec::new(),
    }
}

/// An application's transport: a blocking HTTP client, answering on its own
/// thread as the application's event loop does. The Solana RPC client calls
/// the transport inside its own async runtime, where a blocking client on the
/// same thread would panic.
fn transport() -> Transport {
    let http = reqwest::blocking::Client::builder()
        .timeout(None)
        .build()
        .expect("HTTP client");
    Transport::new(move |request| {
        let http = http.clone();
        let outcome = match std::thread::spawn(move || send(&http, request))
            .join()
            .expect("transport thread")
        {
            Ok(response) => TransportOutcome {
                response: Some(response),
                failure: None,
            },
            Err(error) => TransportOutcome {
                response: None,
                failure: Some(error.to_string()),
            },
        };
        Box::pin(async move { outcome })
    })
}

fn send(
    http: &reqwest::blocking::Client,
    request: TransportRequest,
) -> reqwest::Result<TransportResponse> {
    let mut outgoing = http
        .request(
            reqwest::Method::from_bytes(request.method.as_bytes()).expect("HTTP method"),
            &request.url,
        )
        .body(request.body);
    for (name, value) in request.headers {
        outgoing = outgoing.header(name, value);
    }
    let response = outgoing.send()?;
    Ok(TransportResponse {
        status: response.status().as_u16(),
        body: response.bytes()?.to_vec(),
    })
}

fn open(signer: &Keypair) -> MobileWallet {
    let pubkey = signer.pubkey().to_string();
    let signature = signer.sign_message(&derivation_message(pubkey.clone()).unwrap());
    MobileWallet::open(config(), pubkey, signature.as_ref().to_vec(), transport())
        .expect("open wallet")
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

type ProverResponse = Pin<Box<dyn Future<Output = Option<Vec<u8>>> + Send>>;

/// An application's backend that proves through the prover at
/// `ZOLANA_E2E_PROVER_URL`.
fn backend() -> impl Fn(Vec<u8>) -> ProverResponse + Send + Sync + 'static {
    let prover = reqwest::Url::parse(&required("ZOLANA_E2E_PROVER_URL")).expect("prover URL");
    let http = reqwest::blocking::Client::new();
    move |request| {
        let proof = forward(&http, &prover, request);
        Box::pin(async move { proof })
    }
}

/// Post `request` to the prover as the SDK's prover client does, and wait for
/// the proof if the prover queues it.
fn forward(
    http: &reqwest::blocking::Client,
    prover: &reqwest::Url,
    request: Vec<u8>,
) -> Option<Vec<u8>> {
    let url = |path: &str| {
        let mut url = prover.clone();
        url.path_segments_mut()
            .ok()?
            .pop_if_empty()
            .extend(path.split('/'));
        Some(url)
    };
    let json = |response: reqwest::blocking::Response| -> Option<serde_json::Value> {
        serde_json::from_slice(&response.error_for_status().ok()?.bytes().ok()?).ok()
    };
    let response = json(
        http.post(url("prove")?)
            .header("Content-Type", "application/json")
            .header("X-Sync", "true")
            .body(request)
            .send()
            .ok()?,
    )?;
    let Some(job) = response["jobId"].as_str() else {
        return Some(response.to_string().into_bytes());
    };
    let mut status = url("prove/status")?;
    status.query_pairs_mut().append_pair("jobId", job);
    for _ in 0..240 {
        sleep(Duration::from_millis(250));
        let response = json(http.get(status.clone()).send().ok()?)?;
        match response["status"].as_str() {
            Some("completed") => return Some(response["result"].to_string().into_bytes()),
            Some("failed") => return None,
            _ => {}
        }
    }
    None
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
        transport(),
    )
    .expect("open from saved keys");
    assert_eq!(stale.shielded_address(), sender_wallet.shielded_address());

    let started = std::time::Instant::now();
    let pending = sender_wallet
        .prepare_transfer(recipient.pubkey().to_string(), None, TRANSFER, None, None)
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
            None,
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
            .prepare_transfer(recipient.pubkey().to_string(), None, SPLIT, None, None)
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

    // A backend proof the pinned verifying key rejects fails the spend before
    // a message is built: here a real proof of another transaction.
    let replayed = include_bytes!("../../../../../fixtures/prove-response-2x3.json");
    for (response, code) in [
        (&replayed[..], "proof_invalid"),
        (b"garbage", "proof_malformed"),
    ] {
        sender_wallet.set_remote_prover(move |_| Box::pin(async move { Some(response.to_vec()) }));
        let error = sender_wallet
            .prepare_transfer(
                recipient.pubkey().to_string(),
                None,
                SPLIT,
                None,
                Some(Proving::Remote),
            )
            .err();
        assert_eq!(error.as_deref(), Some(code));
    }
    // A transfer the backend proves lands like one proved on the device.
    sender_wallet.set_remote_prover(backend());
    let started = std::time::Instant::now();
    let pending = sender_wallet
        .prepare_transfer(
            recipient.pubkey().to_string(),
            None,
            SPLIT,
            None,
            Some(Proving::Remote),
        )
        .expect("prove transfer through the backend");
    println!("built and proved by the backend in {:?}", started.elapsed());
    let transfer = submit(&sender_wallet, &[&sender], pending);
    assert_eq!(
        private_sol(&mut recipient_wallet),
        recipient_before + 3 * SPLIT
    );
    newest(&mut sender_wallet, &transfer, ActivityKind::Sent, SPLIT);
}
