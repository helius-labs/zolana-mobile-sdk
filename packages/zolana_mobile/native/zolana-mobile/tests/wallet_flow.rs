//! The whole wallet flow against a live cluster and indexer, proving on this
//! machine with the pinned keys: register, deposit, private transfer sent with
//! a refreshed blockhash, the recipient's balance, a spend that pays the
//! sender's public account and the recipient's wallet in one proof from a
//! session opened before the transfer from the sender's saved keys, with its
//! network fee paid by another account, two transfers prepared before either
//! is sent, and a transfer proved by a remote prover. Each step is the newest
//! entry of its wallets' history. Another test merges notes, and another
//! deposits, transfers and withdraws an SPL token: `ZOLANA_E2E_MINT`, a mint
//! the sender is the mint authority of, or a new mint whose first deposit
//! registers it with the shielded pool, on a cluster that allows that without
//! the protocol authority, as devnet does.
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
//! The remote proof comes from the prover at `ZOLANA_E2E_PROVER_URL`, such as
//! devnet-d's attested prover (`https://d1unet9hn0mci0.cloudfront.net`),
//! asked through the test's transport.
//!
//! `ZOLANA_E2E_KEY_DIR` keeps downloaded proving keys between runs.
//! `ZOLANA_E2E_PRIORITY_FEE` gives every transaction that priority fee, in
//! lamports.
//! `ZOLANA_E2E_SENDER_SEED` / `ZOLANA_E2E_RECIPIENT_SEED` (32-byte hex) reuse
//! funded accounts, such as the example's demo accounts, where airdrops are
//! rate limited; otherwise fresh accounts are airdropped.

use std::{
    env,
    str::FromStr,
    sync::{Arc, Mutex},
    time::Duration,
};

use solana_address::Address;
use solana_instruction::{AccountMeta, Instruction};
use solana_keypair::Keypair;
use solana_pubkey::Pubkey;
use solana_signer::Signer;
use zolana_client::{ComputeBudgetConfig, Rpc, SolanaRpc};
use zolana_interface::{
    pda, SPL_TOKEN_INITIALIZE_MINT2_DISCRIMINATOR, SPL_TOKEN_MINT_ACCOUNT_LEN,
    SPL_TOKEN_MINT_TO_DISCRIMINATOR, SPL_TOKEN_PROGRAM_ID,
};
use zolana_mobile::{
    derivation_message, ActivityKind, MintConfig, MobileWallet, Operation, Output,
    PreparedTransaction, Proving, RegistrationStatus, TransactionLookupStatus, Transport,
    TransportFailure, TransportOutcome, TransportRequest, TransportResponse, WalletConfig,
    WalletError,
};
use zolana_program::instruction::CreateAssociatedTokenAccount;

const FUNDING: u64 = 1_000_000_000;
const DEPOSIT: u64 = 50_000_000;
const TRANSFER: u64 = 20_000_000;
const WITHDRAWAL: u64 = 10_000_000;
/// The application's fee on the withdrawal: a private output to the
/// recipient, as to a registered collector.
const WITHDRAWAL_FEE: u64 = 500_000;
/// Each of two transfers prepared together, and the deposit that makes sure
/// the sender has a second note for them.
const SPLIT: u64 = 1_000_000;
const FEES: u64 = 20_000_000;
/// A prover the test's transport answers itself, with a real proof of another
/// transaction. A loopback host, so the wallet sends it plaintext.
const REPLAYING_PROVER: &str = "http://replaying-prover.localhost";

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
        prover_url: Some(required("ZOLANA_E2E_PROVER_URL")),
        require_prover_tee: env::var("ZOLANA_E2E_REQUIRE_TEE").ok().map(|v| v == "true"),
        prover_tee_policy: None,
        prover_sync_only: env::var("ZOLANA_E2E_SYNC_ONLY").ok().map(|v| v == "true"),
        max_spend_inputs: None,
        reuse_notes_ms: None,
        mints: None,
        priority_fee_lamports: env::var("ZOLANA_E2E_PRIORITY_FEE")
            .ok()
            .map(|fee| fee.parse().expect("priority fee in lamports")),
    }
}

/// An application's transport: a blocking HTTP client, answering on its own
/// thread as the application's event loop does. The Solana RPC client calls
/// the transport inside its own async runtime, where a blocking client on the
/// same thread would panic. Proof requests to [`REPLAYING_PROVER`] get the
/// fixture proof.
fn transport() -> Transport {
    transport_traced(None)
}

fn transport_traced(calls: Option<Arc<Mutex<Vec<String>>>>) -> Transport {
    let http = reqwest::blocking::Client::builder()
        .timeout(None)
        .build()
        .expect("HTTP client");
    Transport::new(move |request| {
        if let Some(calls) = &calls {
            if let Ok(body) = serde_json::from_slice::<serde_json::Value>(&request.body) {
                if let Some(method) = body["method"].as_str() {
                    calls.lock().unwrap().push(method.to_string());
                }
            }
        }
        let http = http.clone();
        let outcome = match std::thread::spawn(move || send(&http, request))
            .join()
            .expect("transport thread")
        {
            Ok(response) => TransportOutcome {
                response: Some(response),
                failure: None,
            },
            Err(failure) => TransportOutcome {
                response: None,
                failure: Some(failure),
            },
        };
        Box::pin(async move { outcome })
    })
}

fn send(
    http: &reqwest::blocking::Client,
    request: TransportRequest,
) -> Result<TransportResponse, TransportFailure> {
    if request.url.starts_with(REPLAYING_PROVER) {
        return Ok(TransportResponse {
            status: 200,
            headers: Default::default(),
            body: include_bytes!("../../../../../fixtures/prove-response-2x2.json").to_vec(),
        });
    }
    let failed = |error: reqwest::Error, status| TransportFailure {
        message: error.to_string(),
        status,
    };
    let mut outgoing = http
        .request(
            reqwest::Method::from_bytes(request.method.as_bytes()).expect("HTTP method"),
            &request.url,
        )
        .body(request.body);
    for (name, value) in request.headers {
        outgoing = outgoing.header(name, value);
    }
    if let (Ok(base), Ok(token)) = (
        env::var("ZOLANA_E2E_PROVER_URL"),
        env::var("ZOLANA_E2E_PROVER_AUTH"),
    ) {
        if request
            .url
            .starts_with(&format!("{}/", base.trim_end_matches('/')))
        {
            outgoing = outgoing.header("x-sf-auth", token);
        }
    }
    if let Some(timeout) = request.timeout_ms {
        outgoing = outgoing.timeout(Duration::from_millis(timeout.into()));
    }
    let mut response = outgoing.send().map_err(|error| failed(error, None))?;
    let status = response.status().as_u16();
    let headers = response
        .headers()
        .iter()
        .filter_map(|(name, value)| Some((name.to_string(), value.to_str().ok()?.to_string())))
        .collect();
    // A proving key goes to the file the wallet names, never into memory.
    if let Some(path) = request.download_path {
        let mut file = std::fs::File::create(path).expect("key file");
        response
            .copy_to(&mut file)
            .map_err(|error| failed(error, Some(status)))?;
        return Ok(TransportResponse {
            status,
            headers,
            body: Vec::new(),
        });
    }
    Ok(TransportResponse {
        status,
        headers,
        body: response
            .bytes()
            .map_err(|error| failed(error, Some(status)))?
            .to_vec(),
    })
}

fn open(signer: &Keypair) -> MobileWallet {
    open_with(signer, config())
}

fn open_with(signer: &Keypair, config: WalletConfig) -> MobileWallet {
    let pubkey = signer.pubkey().to_string();
    let signature = signer.sign_message(&derivation_message(pubkey.clone()).unwrap());
    MobileWallet::open(config, pubkey, signature.as_ref().to_vec(), transport())
        .expect("open wallet")
}

/// Read-only recovery check for an old successful shielded transaction.
/// RPC providers differ in how long they retain recent statuses; unit tests
/// cover the cache miss independently of the provider's retention.
#[test]
#[ignore = "needs ZOLANA_E2E_HISTORICAL_SIGNATURE and a live RPC/indexer"]
fn recovers_a_historical_transaction() {
    let signature = required("ZOLANA_E2E_HISTORICAL_SIGNATURE").parse().unwrap();
    let rpc = SolanaRpc::new(required("ZOLANA_E2E_RPC_URL"));
    let recent = rpc.client().get_signature_statuses(&[signature]).unwrap();
    let history = rpc
        .client()
        .get_signature_statuses_with_history(&[signature])
        .unwrap();
    let status = history.value[0].as_ref().expect("historical status");
    assert!(status.err.is_none());
    assert!(status.satisfies_commitment(solana_commitment_config::CommitmentConfig::confirmed()));

    // An intentionally past height must not override evidence of success.
    let wallet = open(&account("ZOLANA_E2E_SENDER_SEED"));
    wallet
        .wait_for_transaction(signature.to_string(), Some(0))
        .unwrap();
    println!(
        "Recovered historical transaction {signature} at slot {}; recent lookup present: {}",
        status.slot,
        recent.value[0].is_some()
    );
}

/// Sign as `signers`, which must be the required signers in order.
fn submit(wallet: &MobileWallet, signers: &[&Keypair], pending: PreparedTransaction) -> String {
    let expected: Vec<String> = signers.iter().map(|s| s.pubkey().to_string()).collect();
    assert_eq!(pending.signers(), expected);
    let signatures = signers
        .iter()
        .map(|s| s.sign_message(&pending.message()).as_ref().to_vec())
        .collect();
    let operation = pending.operation();
    println!("{operation:?}");
    let signature = wallet.submit(&pending, signatures).expect("submit");
    if matches!(
        operation,
        Operation::Deposit { .. } | Operation::Spend { .. } | Operation::Merge { .. }
    ) {
        let calls = Arc::new(Mutex::new(Vec::new()));
        let owner = zolana_keypair::ShieldedAddress::from_str(&wallet.shielded_address())
            .unwrap()
            .solana_address()
            .unwrap();
        let mut cold = MobileWallet::open_with_keys(
            config(),
            owner.to_string(),
            wallet.export_keys(),
            transport_traced(Some(Arc::clone(&calls))),
        )
        .unwrap();
        let lookup = cold
            .transaction(signature.clone())
            .expect("cold signature lookup");
        assert_eq!(
            lookup.status,
            TransactionLookupStatus::Ready,
            "lookup after {operation:?}"
        );
        assert!(
            calls.lock().unwrap().iter().all(|method| ![
                "getShieldedTransactionsByTags",
                "getShieldedTransactionsByNullifiers",
                "getEncryptedUtxosByTags",
            ]
            .contains(&method.as_str())),
            "a lookup must not scan history"
        );
        let expected = cold
            .activity()
            .unwrap()
            .into_iter()
            .filter(|entry| entry.signature == signature)
            .collect::<Vec<_>>();
        assert_eq!(
            lookup.entries, expected,
            "signature lookup agrees with history"
        );
        println!(
            "cold lookup returned {} rows without history pages",
            lookup.entries.len()
        );
    }
    signature
}

/// The private SOL balance, read from the indexer now.
fn private_sol(wallet: &mut MobileWallet) -> u64 {
    wallet.private_balance(None).unwrap()
}

/// The newest history entry is the transaction `signature`, moving `amount`
/// SOL as `kind`.
fn newest(wallet: &mut MobileWallet, signature: &str, kind: ActivityKind, amount: u64) {
    newest_of(wallet, signature, kind, None, amount);
}

/// As [`newest`], of `mint` (`None` for SOL).
fn newest_of(
    wallet: &mut MobileWallet,
    signature: &str,
    kind: ActivityKind,
    mint: Option<&str>,
    amount: u64,
) {
    let activity = wallet.activity().unwrap();
    let entry = activity.first().expect("an activity entry");
    assert_eq!(
        (
            entry.signature.as_str(),
            entry.kind,
            entry.mint.as_deref(),
            entry.amount
        ),
        (signature, kind, mint, amount)
    );
}

fn private(recipient: &Keypair, amount: u64) -> Output {
    Output::Private {
        recipient: recipient.pubkey().to_string(),
        amount,
    }
}

fn public(recipient: &Keypair, amount: u64) -> Output {
    Output::Public {
        recipient: recipient.pubkey().to_string(),
        amount,
    }
}

/// A private transfer of `amount` to `recipient`, proved on this machine.
fn private_transfer(
    wallet: &mut MobileWallet,
    recipient: &Keypair,
    amount: u64,
) -> PreparedTransaction {
    wallet
        .prepare_spend(vec![private(recipient, amount)], None, None, None, None)
        .expect("prove transfer")
}

fn register(wallet: &mut MobileWallet, signer: &Keypair) {
    if let Some(pending) = wallet
        .prepare_registration(None, None)
        .expect("prepare registration")
    {
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

    let pending = sender_wallet.prepare_deposit(None, DEPOSIT, None).unwrap();
    let deposit = submit(&sender_wallet, &[&sender], pending);
    assert_eq!(private_sol(&mut sender_wallet), sender_before + DEPOSIT);
    newest(&mut sender_wallet, &deposit, ActivityKind::Deposit, DEPOSIT);

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
    let pending = private_transfer(&mut sender_wallet, &recipient, TRANSFER);
    println!("built and proved on device in {:?}", started.elapsed());
    // A slow approval: the same proof under a new blockhash.
    let refreshed = (0..40)
        .find_map(|_| {
            std::thread::sleep(std::time::Duration::from_millis(500));
            let refreshed = sender_wallet.refresh(&pending).expect("refresh");
            (refreshed.message() != pending.message()).then_some(refreshed)
        })
        .expect("a new blockhash");
    assert!(refreshed.last_valid_block_height() >= pending.last_valid_block_height());
    assert_eq!(refreshed.signers(), pending.signers());
    assert_eq!(refreshed.operation(), pending.operation());
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
    // After a restart the signature alone is enough to wait for one that
    // landed.
    recipient_wallet
        .wait_for_transaction(transfer.clone(), None)
        .expect("wait for a landed transaction");
    newest(
        &mut recipient_wallet,
        &transfer,
        ActivityKind::Received,
        TRANSFER,
    );

    // The other session must not pick a note the transfer already spent.
    // A withdrawal to the sender's public account and the application's fee
    // to the recipient's wallet, in one proof. The recipient pays the
    // network fee, so the sender receives the full amount.
    let public_before = rpc.get_balance(sender.pubkey()).unwrap();
    let recipient_before = private_sol(&mut recipient_wallet);
    let outputs = vec![
        public(&sender, WITHDRAWAL),
        private(&recipient, WITHDRAWAL_FEE),
    ];
    let pending = stale
        .prepare_spend(
            outputs.clone(),
            None,
            Some(recipient.pubkey().to_string()),
            None,
            None,
        )
        .expect("another session still builds a spendable withdrawal");
    assert_eq!(
        pending.operation(),
        Operation::Spend {
            mint: None,
            outputs,
            created_token_accounts: Vec::new(),
        }
    );
    let withdrawal = submit(&stale, &[&recipient, &sender], pending);
    assert_eq!(
        rpc.get_balance(sender.pubkey()).unwrap(),
        public_before + WITHDRAWAL
    );
    assert_eq!(
        private_sol(&mut recipient_wallet),
        recipient_before + WITHDRAWAL_FEE
    );
    // A spend with a note for another wallet reads as sent, for the whole
    // total.
    newest(
        &mut stale,
        &withdrawal,
        ActivityKind::Sent,
        WITHDRAWAL + WITHDRAWAL_FEE,
    );
    assert_eq!(
        stale.private_balance(None).unwrap(),
        sender_before + DEPOSIT - TRANSFER - WITHDRAWAL - WITHDRAWAL_FEE
    );

    // Two transfers prepared before either is sent take different notes, so
    // both land: the first reserves the notes it spends.
    let pending = sender_wallet.prepare_deposit(None, SPLIT, None).unwrap();
    submit(&sender_wallet, &[&sender], pending);
    let recipient_before = private_sol(&mut recipient_wallet);
    let first = private_transfer(&mut sender_wallet, &recipient, SPLIT);
    let second = private_transfer(&mut sender_wallet, &recipient, SPLIT);
    submit(&sender_wallet, &[&sender], first);
    submit(&sender_wallet, &[&sender], second);
    assert_eq!(
        private_sol(&mut recipient_wallet),
        recipient_before + 2 * SPLIT
    );
    assert_eq!(
        private_sol(&mut sender_wallet),
        sender_before + DEPOSIT - TRANSFER - WITHDRAWAL - WITHDRAWAL_FEE - SPLIT
    );

    // A remote proof the pinned verifying key rejects fails the spend before
    // a message is built: here a real proof of another transaction.
    let pubkey = sender.pubkey().to_string();
    let signature = sender.sign_message(&derivation_message(pubkey.clone()).unwrap());
    let replaying = WalletConfig {
        prover_url: Some(REPLAYING_PROVER.to_string()),
        ..config()
    };
    let error = MobileWallet::open(replaying, pubkey, signature.as_ref().to_vec(), transport())
        .expect("open wallet")
        .prepare_spend(
            vec![private(&recipient, SPLIT)],
            None,
            None,
            Some(Proving::Remote),
            None,
        )
        .err();
    assert_eq!(error, Some(WalletError::ProofInvalid));
    // A transfer the remote prover proves lands like one proved on the device.
    let started = std::time::Instant::now();
    let pending = sender_wallet
        .prepare_spend(
            vec![private(&recipient, SPLIT)],
            None,
            None,
            Some(Proving::Remote),
            None,
        )
        .expect("prove transfer remotely");
    println!("built and proved remotely in {:?}", started.elapsed());
    let transfer = submit(&sender_wallet, &[&sender], pending);
    assert_eq!(
        private_sol(&mut recipient_wallet),
        recipient_before + 3 * SPLIT
    );
    newest(&mut sender_wallet, &transfer, ActivityKind::Sent, SPLIT);
}

/// The recipient's notes merged twice, proved by the remote prover and on this
/// machine. A merge needs no signature of its owner: the sender pays alone.
const TOKEN_DEPOSIT: u64 = 5_000_000;
const TOKEN_TRANSFER: u64 = 2_000_000;
const TOKEN_WITHDRAWAL: u64 = 1_000_000;
/// The application's fee on the token withdrawal: a private output to the
/// sender's wallet, as to a registered collector.
const TOKEN_FEE: u64 = 100_000;
/// Tokens sent publicly to an account without a token account, which the
/// spend creates.
const TOKEN_PUBLIC_SEND: u64 = 500_000;
/// The rent of an SPL Token account, which the fee payer of a spend that
/// creates one pays.
const TOKEN_ACCOUNT_RENT: u64 = 2_039_280;

fn send_instructions(
    rpc: &SolanaRpc,
    payer: &Keypair,
    signers: &[&dyn Signer],
    instructions: &[Instruction],
) {
    rpc.create_and_send_transaction(
        instructions,
        Address::new_from_array(payer.pubkey().to_bytes()),
        signers,
        ComputeBudgetConfig::for_instruction_count(instructions.len()),
    )
    .expect("send");
}

/// `ZOLANA_E2E_MINT`, an SPL Token mint `authority` mints, or a new one the
/// shielded pool has not registered yet: its first deposit registers it.
fn test_mint(rpc: &SolanaRpc, authority: &Keypair) -> Pubkey {
    if let Ok(mint) = env::var("ZOLANA_E2E_MINT") {
        return mint.parse().expect("ZOLANA_E2E_MINT is a base58 key");
    }
    let token_program = Pubkey::new_from_array(SPL_TOKEN_PROGRAM_ID);
    let mint = Keypair::new();
    let rent = rpc
        .get_minimum_balance_for_rent_exemption(SPL_TOKEN_MINT_ACCOUNT_LEN)
        .unwrap();
    // SystemInstruction::CreateAccount: tag, lamports, space, owner.
    let mut create = vec![0u8; 4];
    create.extend_from_slice(&rent.to_le_bytes());
    create.extend_from_slice(&(SPL_TOKEN_MINT_ACCOUNT_LEN as u64).to_le_bytes());
    create.extend_from_slice(&token_program.to_bytes());
    // InitializeMint2: six decimals, `authority` mints, no freeze authority.
    let mut initialize = vec![SPL_TOKEN_INITIALIZE_MINT2_DISCRIMINATOR, 6];
    initialize.extend_from_slice(&authority.pubkey().to_bytes());
    initialize.push(0);
    send_instructions(
        rpc,
        authority,
        &[authority, &mint],
        &[
            Instruction {
                program_id: Pubkey::default(),
                accounts: vec![
                    AccountMeta::new(authority.pubkey(), true),
                    AccountMeta::new(mint.pubkey(), true),
                ],
                data: create,
            },
            Instruction {
                program_id: token_program,
                accounts: vec![AccountMeta::new(mint.pubkey(), false)],
                data: initialize,
            },
        ],
    );
    println!("test mint {}, reuse it with ZOLANA_E2E_MINT", mint.pubkey());
    mint.pubkey()
}

/// `owner`'s associated token account of `mint`, created by `payer` if it
/// is missing.
fn token_account(rpc: &SolanaRpc, payer: &Keypair, owner: Pubkey, mint: Pubkey) -> Pubkey {
    let create = CreateAssociatedTokenAccount {
        payer: payer.pubkey(),
        owner,
        mint,
        token_program: Pubkey::new_from_array(SPL_TOKEN_PROGRAM_ID),
    };
    let address = create.address();
    send_instructions(rpc, payer, &[payer], &[create.instruction()]);
    address
}

fn mint_to(rpc: &SolanaRpc, authority: &Keypair, mint: Pubkey, account: Pubkey, amount: u64) {
    let mut data = vec![SPL_TOKEN_MINT_TO_DISCRIMINATOR];
    data.extend_from_slice(&amount.to_le_bytes());
    send_instructions(
        rpc,
        authority,
        &[authority],
        &[Instruction {
            program_id: Pubkey::new_from_array(SPL_TOKEN_PROGRAM_ID),
            accounts: vec![
                AccountMeta::new(mint, false),
                AccountMeta::new(account, false),
                AccountMeta::new_readonly(authority.pubkey(), true),
            ],
            data,
        }],
    );
}

/// The amount a token account holds: bytes 64..72 of its data.
fn public_tokens(rpc: &SolanaRpc, account: Pubkey) -> u64 {
    let data = rpc
        .get_account(Address::new_from_array(account.to_bytes()))
        .unwrap()
        .expect("token account")
        .data;
    u64::from_le_bytes(data[64..72].try_into().unwrap())
}

#[test]
#[ignore = "needs a Zolana cluster and indexer"]
fn deposits_transfers_and_withdraws_an_spl_token() {
    let sender = account("ZOLANA_E2E_SENDER_SEED");
    let recipient = account("ZOLANA_E2E_RECIPIENT_SEED");
    let mut rpc = SolanaRpc::new(required("ZOLANA_E2E_RPC_URL"));
    for (signer, needed) in [(&sender, FEES + TOKEN_ACCOUNT_RENT), (&recipient, FEES)] {
        if rpc.get_balance(signer.pubkey()).expect("balance") < needed {
            rpc.airdrop(&signer.pubkey(), FUNDING).expect("airdrop");
        }
    }
    let mint = test_mint(&rpc, &sender);
    // A mint the pool has not registered yet, a new one or the demo token on
    // a fresh pool, is registered by its first deposit.
    let new_mint = rpc
        .get_account(pda::spl_asset_registry(&mint))
        .unwrap()
        .is_none();
    let sender_tokens = token_account(&rpc, &sender, sender.pubkey(), mint);
    mint_to(&rpc, &sender, mint, sender_tokens, TOKEN_DEPOSIT);
    let recipient_tokens = token_account(&rpc, &sender, recipient.pubkey(), mint);

    let with_mint = || WalletConfig {
        mints: Some(vec![MintConfig {
            mint: mint.to_string(),
            token_program: Pubkey::new_from_array(SPL_TOKEN_PROGRAM_ID).to_string(),
        }]),
        ..config()
    };
    let mut sender_wallet = open_with(&sender, with_mint());
    let mut recipient_wallet = open_with(&recipient, with_mint());
    register(&mut sender_wallet, &sender);
    register(&mut recipient_wallet, &recipient);
    let name = mint.to_string();
    let private_tokens =
        |wallet: &mut MobileWallet| wallet.private_balance(Some(name.clone())).unwrap();
    let sender_before = private_tokens(&mut sender_wallet);
    let recipient_before = private_tokens(&mut recipient_wallet);

    let pending = sender_wallet
        .prepare_deposit(Some(name.clone()), TOKEN_DEPOSIT, None)
        .unwrap();
    assert_eq!(
        pending.operation(),
        Operation::Deposit {
            mint: Some(name.clone()),
            amount: TOKEN_DEPOSIT,
            creates_spl_interface: new_mint,
        }
    );
    let deposit = submit(&sender_wallet, &[&sender], pending);
    assert_eq!(
        private_tokens(&mut sender_wallet),
        sender_before + TOKEN_DEPOSIT
    );
    newest_of(
        &mut sender_wallet,
        &deposit,
        ActivityKind::Deposit,
        Some(&name),
        TOKEN_DEPOSIT,
    );

    let pending = sender_wallet
        .prepare_spend(
            vec![private(&recipient, TOKEN_TRANSFER)],
            Some(name.clone()),
            None,
            None,
            None,
        )
        .expect("prove a token transfer");
    let transfer = submit(&sender_wallet, &[&sender], pending);
    assert_eq!(
        private_tokens(&mut sender_wallet),
        sender_before + TOKEN_DEPOSIT - TOKEN_TRANSFER
    );
    assert_eq!(
        private_tokens(&mut recipient_wallet),
        recipient_before + TOKEN_TRANSFER
    );
    newest_of(
        &mut recipient_wallet,
        &transfer,
        ActivityKind::Received,
        Some(&name),
        TOKEN_TRANSFER,
    );
    // A wallet given no mints learns the token from the notes it reads.
    let discovered = open_with(&recipient, config())
        .balances()
        .unwrap()
        .into_iter()
        .find(|balance| balance.mint.as_deref() == Some(name.as_str()))
        .expect("the received token is discovered");
    assert_eq!(discovered.amount, recipient_before + TOKEN_TRANSFER);

    // Tokens to the recipient's own token account and the application's fee
    // to the sender's wallet, in one proof.
    let public_before = public_tokens(&rpc, recipient_tokens);
    let pending = recipient_wallet
        .prepare_spend(
            vec![
                public(&recipient, TOKEN_WITHDRAWAL),
                private(&sender, TOKEN_FEE),
            ],
            Some(name.clone()),
            None,
            None,
            None,
        )
        .expect("prove a token withdrawal");
    let withdrawal = submit(&recipient_wallet, &[&recipient], pending);
    assert_eq!(
        public_tokens(&rpc, recipient_tokens),
        public_before + TOKEN_WITHDRAWAL
    );
    assert_eq!(
        private_tokens(&mut recipient_wallet),
        recipient_before + TOKEN_TRANSFER - TOKEN_WITHDRAWAL - TOKEN_FEE
    );
    assert_eq!(
        private_tokens(&mut sender_wallet),
        sender_before + TOKEN_DEPOSIT - TOKEN_TRANSFER + TOKEN_FEE
    );
    newest_of(
        &mut recipient_wallet,
        &withdrawal,
        ActivityKind::Sent,
        Some(&name),
        TOKEN_WITHDRAWAL + TOKEN_FEE,
    );
    // Tokens to an account that has never held this mint: the spend creates
    // its token account, the sender paying the rent.
    let stranger = Keypair::new();
    let outputs = vec![public(&stranger, TOKEN_PUBLIC_SEND)];
    let pending = sender_wallet
        .prepare_spend(outputs.clone(), Some(name.clone()), None, None, None)
        .expect("prove a public token send to an account without a token account");
    assert_eq!(
        pending.operation(),
        Operation::Spend {
            mint: Some(name.clone()),
            outputs,
            created_token_accounts: vec![stranger.pubkey().to_string()],
        }
    );
    submit(&sender_wallet, &[&sender], pending);
    let stranger_tokens = pda::associated_token_address_with_program(
        &stranger.pubkey(),
        &mint,
        &Pubkey::new_from_array(SPL_TOKEN_PROGRAM_ID),
    );
    assert_eq!(public_tokens(&rpc, stranger_tokens), TOKEN_PUBLIC_SEND);
    assert_eq!(
        private_tokens(&mut sender_wallet),
        sender_before + TOKEN_DEPOSIT - TOKEN_TRANSFER + TOKEN_FEE - TOKEN_PUBLIC_SEND
    );

    // The SOL balance is not touched by token flows.
    assert!(sender_wallet
        .balances()
        .unwrap()
        .iter()
        .any(|balance| balance.mint.as_deref() == Some(name.as_str())));
}

/// Two private outputs in one proof, as an application pays a recipient and
/// its own fee collector: here both go to the recipient, which is registered.
#[test]
#[ignore = "needs a Zolana cluster and indexer"]
fn pays_two_recipients_in_one_transfer() {
    let sender = account("ZOLANA_E2E_SENDER_SEED");
    let recipient = account("ZOLANA_E2E_RECIPIENT_SEED");
    let mut sender_wallet = open(&sender);
    let mut recipient_wallet = open(&recipient);
    register(&mut sender_wallet, &sender);
    register(&mut recipient_wallet, &recipient);
    if private_sol(&mut sender_wallet) < SPLIT * 3 {
        let pending = sender_wallet.prepare_deposit(None, DEPOSIT, None).unwrap();
        submit(&sender_wallet, &[&sender], pending);
    }
    let sender_before = private_sol(&mut sender_wallet);
    let recipient_before = private_sol(&mut recipient_wallet);

    let outputs = vec![private(&recipient, SPLIT), private(&recipient, SPLIT / 4)];
    let pending = sender_wallet
        .prepare_spend(outputs.clone(), None, None, None, None)
        .expect("prove a transfer with two outputs");
    assert_eq!(
        pending.operation(),
        Operation::Spend {
            mint: None,
            outputs,
            created_token_accounts: Vec::new(),
        }
    );
    let transfer = submit(&sender_wallet, &[&sender], pending);

    assert_eq!(
        private_sol(&mut sender_wallet),
        sender_before - SPLIT - SPLIT / 4
    );
    assert_eq!(
        private_sol(&mut recipient_wallet),
        recipient_before + SPLIT + SPLIT / 4
    );
    newest(
        &mut sender_wallet,
        &transfer,
        ActivityKind::Sent,
        SPLIT + SPLIT / 4,
    );
}

#[test]
#[ignore = "needs a Zolana cluster and indexer"]
fn merges_notes_proved_remotely_and_on_the_device() {
    let owner = account("ZOLANA_E2E_RECIPIENT_SEED");
    let payer = account("ZOLANA_E2E_SENDER_SEED");
    let mut rpc = SolanaRpc::new(required("ZOLANA_E2E_RPC_URL"));
    for signer in [&owner, &payer] {
        if rpc.get_balance(signer.pubkey()).expect("balance") < FEES + 2 * SPLIT {
            rpc.airdrop(&signer.pubkey(), FUNDING).expect("airdrop");
        }
    }
    let mut wallet = open(&owner);
    register(&mut wallet, &owner);
    if let Some(pending) = wallet.prepare_merging(true, None).expect("prepare merging") {
        submit(&wallet, &[&owner], pending);
    }
    assert!(wallet.merging_enabled().unwrap());
    assert_eq!(
        wallet
            .prepare_merging(true, None)
            .unwrap()
            .map(|p| p.operation()),
        None
    );
    for proving in [Proving::Remote, Proving::Local] {
        // Notes to merge on a fresh account too, and after the last merge
        // took them all.
        for _ in 0..2 {
            let pending = wallet.prepare_deposit(None, SPLIT, None).unwrap();
            submit(&wallet, &[&owner], pending);
        }
        let before = private_sol(&mut wallet);
        let started = std::time::Instant::now();
        let pending = wallet
            .prepare_merge(
                None,
                Some(8),
                Some(payer.pubkey().to_string()),
                Some(proving),
                None,
            )
            .expect("prepare merge");
        println!("merge proved {proving:?} in {:?}", started.elapsed());
        let Operation::Merge {
            mint: None,
            amount: merged,
        } = pending.operation()
        else {
            panic!("a SOL merge: {:?}", pending.operation());
        };
        let signature = submit(&wallet, &[&payer], pending);
        assert_eq!(private_sol(&mut wallet), before);
        newest(&mut wallet, &signature, ActivityKind::SelfTransfer, merged);
    }
}

/// The example backend, real remote merge proofs and local final proofs.
#[test]
#[ignore = "needs funded devnet demo accounts, indexer, TEE and example backend"]
fn sponsored_cache_rounds_restore_and_pay_locally() {
    use base64::{engine::general_purpose::STANDARD, Engine};
    use serde_json::json;
    use zolana_mobile::CacheTarget;
    let backend = required("ZOLANA_E2E_BACKEND_URL");
    let token = required("ZOLANA_E2E_PROVER_AUTH");
    let http = reqwest::blocking::Client::new();
    let owner = account("ZOLANA_E2E_SENDER_SEED");
    let recipient = account("ZOLANA_E2E_RECIPIENT_SEED");
    let mut wallet = open(&owner);
    register(&mut wallet, &owner);
    let mut receiver = open(&recipient);
    register(&mut receiver, &recipient);
    if let Some(pending) = wallet.prepare_merging(true, None).unwrap() {
        submit(&wallet, &[&owner], pending);
    }
    let mut mints = vec![None];
    if let Ok(mint) = env::var("ZOLANA_E2E_MINT") {
        mints.push(Some(mint));
    }
    for mint in mints {
        for _ in 0..4 {
            let pending = wallet.prepare_deposit(mint.clone(), SPLIT, None).unwrap();
            submit(&wallet, &[&owner], pending);
        }
        let run: serde_json::Value = http
            .post(format!("{backend}/runs"))
            .header("x-sf-auth", &token)
            .json(&json!({"owner":owner.pubkey().to_string(),"treeId":0,"slots":2}))
            .send()
            .unwrap()
            .error_for_status()
            .unwrap()
            .json()
            .unwrap();
        let target: CacheTarget = serde_json::from_value(run["target"].clone()).unwrap();
        let id = run["id"].as_str().unwrap();
        let mut snapshots = Vec::new();
        for slot in 0..2 {
            let target = CacheTarget {
                slot,
                ..target.clone()
            };
            let pending = wallet
                .prepare_cached_merge(
                    target,
                    mint.clone(),
                    Some(2),
                    Some(Proving::Remote),
                    Some(5_000),
                )
                .expect("prepare cached merge through authenticated TEE proxy");
            assert_eq!(pending.signers(), vec![pending.fee_payer()]);
            assert_eq!(pending.priority_fee_lamports(), 5_000);
            let snapshot = wallet.export_prepared_transaction(&pending).unwrap();
            let body = json!({"slot":slot,"message":STANDARD.encode(pending.message())});
            let relay = || {
                http.post(format!("{backend}/runs/{id}/merges"))
                    .header("x-sf-auth", &token)
                    .json(&body)
                    .send()
                    .unwrap()
                    .error_for_status()
                    .unwrap()
                    .json::<serde_json::Value>()
                    .unwrap()
            };
            let submitted = relay();
            assert_eq!(
                submitted["signature"],
                relay()["signature"],
                "relayer retries preserve signed bytes"
            );
            let output = pending.cached_output().unwrap();
            let mut ready = false;
            for _ in 0..60 {
                if wallet.verify_cached_outputs(vec![output.clone()]).is_ok() {
                    ready = true;
                    break;
                }
                std::thread::sleep(Duration::from_millis(500));
            }
            assert!(ready, "expected cache commitment should land");
            snapshots.push(snapshot);
            // Reopen between rounds too: restore original-input and predicted
            // output reservations before discovering the next round's inputs.
            drop(wallet);
            wallet = open(&owner);
            for snapshot in &snapshots {
                wallet
                    .restore_prepared_transaction(snapshot.clone())
                    .unwrap();
            }
        }
        drop(wallet);
        wallet = open(&owner);
        let restored: Vec<_> = snapshots
            .into_iter()
            .map(|snapshot| wallet.restore_prepared_transaction(snapshot).unwrap())
            .collect();
        let cached: Vec<_> = restored
            .iter()
            .map(|pending| pending.cached_output().unwrap())
            .collect();
        let total: u64 = cached.iter().map(|output| output.amount()).sum();
        let amount = total / 4;
        let outputs = vec![
            Output::Private {
                recipient: recipient.pubkey().to_string(),
                amount,
            },
            Output::Public {
                recipient: owner.pubkey().to_string(),
                amount,
            },
        ];
        let payment = wallet
            .prepare_cached_spend(outputs, cached, mint.clone(), None, Some(5_000))
            .expect("cached send and withdrawal proved locally");
        assert_eq!(payment.signers(), vec![owner.pubkey().to_string()]);
        assert!(!payment.outputs().is_empty());
        let saved = wallet.export_prepared_transaction(&payment).unwrap();
        drop(wallet);
        wallet = open(&owner);
        let payment = wallet.restore_prepared_transaction(saved).unwrap();
        let signature = submit(&wallet, &[&owner], payment);
        println!(
            "cached send + withdrawal confirmed for {}: {signature}",
            mint.as_deref().unwrap_or("SOL")
        );
    }
}
