use super::{
    account::{
        derivation_message, registration_status_of, RegistrationStatus, WalletConfig, WalletKeys,
    },
    balance::{
        proof_widths, spend_limit, token_balance, SpendLimit, TokenBalance, UnresolvedAssets,
        WalletOverview, INDEXER_CATCH_UP,
    },
    merge::merge_expiry,
    prepared::PreparedTransaction,
    spend::{
        max_spend_inputs, proof_outputs, spend_failure, with_setup, within_cap, MAX_COMPUTE_UNITS,
    },
    submission::reconcile,
    types::{Operation, Output, OutputOpening},
    util::nullifiers,
    *,
};
use crate::{
    asset::{Asset, MintConfig},
    error::{settles_the_send, WalletError},
    prover::Proving,
    transport::tests::unreachable,
};
use solana_instruction::{AccountMeta, Instruction};
use solana_keypair::Keypair;
use solana_message::{v1, VersionedMessage};
use solana_pubkey::Pubkey;
use solana_signature::Signature;
use solana_signer::Signer;
use std::{
    collections::HashSet,
    sync::{Arc, Mutex},
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};
use zolana_client::{
    compile_message, user_registry::set_merge_encryption_instruction, ClientError,
    ComputeBudgetConfig, IndexerPollConfig, UnsignedMessage,
};
use zolana_interface::{pda, state::ProtocolConfig, PROGRAM_ID_PUBKEY};
use zolana_keypair::{PublicKey, ShieldedAddress};
use zolana_program::instruction::{
    CreateAssociatedTokenAccount, TransactInterfaceTransferAccounts,
};
use zolana_transaction::{
    instructions::transact::{canonical_shape, SppProofOutputUtxo, MAX_SPEND_INPUTS},
    select_spend_excluding, AssetBalance, SpendableDecryptionResult, WalletUtxo,
};
use zolana_user_registry_interface::{user_record_pda, MergeEncryptionScheme};

mod account;
mod balance;
mod deposit;
mod openings;
mod spend;
mod submission;
pub(super) fn config() -> WalletConfig {
    WalletConfig {
        rpc_url: "https://rpc.example".to_string(),
        indexer_url: "https://indexer.example/v1/zolana".to_string(),
        proving_key_dir: std::env::temp_dir().display().to_string(),
        proving_key_url: None,
        proving_key_catalog: None,
        proving: None,
        prover_url: None,
        require_prover_tee: None,
        prover_tee_policy: None,
        prover_sync_only: None,
        max_spend_inputs: None,
        reuse_notes_ms: None,
        mints: None,
        priority_fee_lamports: None,
    }
}

pub(super) fn open(signer: &Keypair) -> Result<MobileWallet, WalletError> {
    open_with(signer, config())
}

fn private(recipient: &str, amount: u64) -> Output {
    Output::Private {
        recipient: recipient.to_string(),
        amount,
    }
}

fn public(recipient: &str, amount: u64) -> Output {
    Output::Public {
        recipient: recipient.to_string(),
        amount,
    }
}

/// A wallet whose transport reaches nothing: a request fails as a
/// refused connection does.
fn open_with(signer: &Keypair, config: WalletConfig) -> Result<MobileWallet, WalletError> {
    let pubkey = signer.pubkey().to_string();
    let message = derivation_message(pubkey.clone())?;
    let signature = signer.sign_message(&message);
    MobileWallet::open(config, pubkey, signature.as_ref().to_vec(), unreachable())
}

/// A deposit-kind message `signer` pays for, with no instructions.
pub(super) fn unsigned(signer: &Keypair) -> PreparedTransaction {
    let message = compile_message(
        &signer.pubkey(),
        &[],
        Default::default(),
        ComputeBudgetConfig {
            cu_limit: 200_000,
            priority_fee_lamports: None,
        },
    )
    .unwrap();
    PreparedTransaction {
        id: 0,
        operation: Operation::Registration,
        message,
        last_valid_block_height: 0,
        spends: Vec::new(),
        outputs: Vec::new(),
        cached_output: None,
        cache_inputs: Vec::new(),
    }
}

/// A wallet whose Solana RPC answers each JSON-RPC method with
/// `answer(method)`: `{"result": ..}` or `{"error": ..}`. Returns the
/// methods it was asked, in order.
pub(super) fn rpc_wallet(
    signer: &Keypair,
    answer: impl Fn(&str) -> serde_json::Value + Send + Sync + 'static,
) -> (MobileWallet, Arc<Mutex<Vec<String>>>) {
    rpc_wallet_with(signer, config(), answer)
}

fn rpc_wallet_with(
    signer: &Keypair,
    config: WalletConfig,
    answer: impl Fn(&str) -> serde_json::Value + Send + Sync + 'static,
) -> (MobileWallet, Arc<Mutex<Vec<String>>>) {
    rpc_wallet_asked(signer, config, move |method, _| answer(method))
}

/// [`rpc_wallet_with`], answering each method by its params too.
pub(super) fn rpc_wallet_asked(
    signer: &Keypair,
    config: WalletConfig,
    answer: impl Fn(&str, &serde_json::Value) -> serde_json::Value + Send + Sync + 'static,
) -> (MobileWallet, Arc<Mutex<Vec<String>>>) {
    let asked = Arc::new(Mutex::new(Vec::new()));
    let log = Arc::clone(&asked);
    let (transport, _) = crate::transport::tests::fake(move |request| {
        let body: serde_json::Value = serde_json::from_slice(&request.body).unwrap();
        let method = body["method"].as_str().unwrap_or_default().to_string();
        let mut reply = answer(&method, &body["params"]);
        reply["jsonrpc"] = "2.0".into();
        reply["id"] = body["id"].clone();
        log.lock().unwrap().push(method);
        crate::transport::tests::ok(&reply.to_string())
    });
    let pubkey = signer.pubkey().to_string();
    let signature = signer.sign_message(&derivation_message(pubkey.clone()).unwrap());
    let wallet =
        MobileWallet::open(config, pubkey, signature.as_ref().to_vec(), transport).unwrap();
    (wallet, asked)
}

fn status(landed: bool) -> serde_json::Value {
    let value = if landed {
        serde_json::json!([{
            "slot": 5, "confirmations": null, "err": null,
            "status": {"Ok": null}, "confirmationStatus": "finalized"
        }])
    } else {
        serde_json::json!([null])
    };
    serde_json::json!({"result": {"context": {"slot": 5}, "value": value}})
}

const QUICK: IndexerPollConfig = IndexerPollConfig {
    num_retries: 2,
    delay_ms: 1,
    max_delay_ms: 1,
};

fn recovery_wallet(
    statuses: Vec<serde_json::Value>,
    block_height: u64,
) -> (MobileWallet, Arc<Mutex<Vec<String>>>) {
    let statuses = Mutex::new(std::collections::VecDeque::from(statuses));
    let (mut wallet, asked) =
        rpc_wallet_asked(
            &Keypair::new(),
            config(),
            move |method, params| match method {
                "getSignatureStatuses" => {
                    assert_eq!(params[1]["searchTransactionHistory"], true);
                    let mut statuses = statuses.lock().unwrap();
                    if statuses.len() > 1 {
                        statuses.pop_front().unwrap()
                    } else {
                        statuses.front().unwrap().clone()
                    }
                }
                "getBlockHeight" => serde_json::json!({"result": block_height}),
                "getShieldedTransactionsBySignature" => indexed(None),
                _ => panic!("unexpected request: {method}"),
            },
        );
    wallet.send_lookup = QUICK;
    (wallet, asked)
}

/// The send error of a note the chain already spent, refused in
/// simulation.
fn nullifier_queued() -> serde_json::Value {
    serde_json::json!({"error": {
        "code": -32002,
        "message": "Transaction simulation failed: custom program error: 0x1b83",
        "data": {
            "err": {"InstructionError": [0, {"Custom": 7043}]},
            "logs": [], "accounts": null, "unitsConsumed": 0,
            "returnData": null, "innerInstructions": null
        }
    }})
}

/// A SOL note of `amount` on `tree_id` whose nullifier is
/// `[nullifier; 32]`.
pub(super) fn note(amount: u64, nullifier: u8, tree_id: u16) -> WalletUtxo {
    WalletUtxo {
        utxo: zolana_transaction::Utxo {
            owner: PublicKey::from_ed25519(&[1; 32]),
            asset: zolana_transaction::Mint {
                asset: zolana_transaction::SOL_MINT,
                asset_id: zolana_transaction::SOL_ASSET_ID,
            },
            amount,
            blinding: [0; 32],
            ring_program_id: None,
            data: Default::default(),
        },
        nullifier_pubkey: [0; 32],
        utxo_hash: [nullifier; 32],
        nullifier: [nullifier; 32],
        tx_viewing_key: None,
        data_hash: None,
        ring_data_hash: None,
        tree_id,
        leaf_index: 0,
        latest_tree_id: None,
        slot: 0,
        tx_signature: Signature::default(),
        slot_index: 0,
    }
}

fn spendable(utxos: Vec<WalletUtxo>) -> SpendableDecryptionResult {
    SpendableDecryptionResult {
        balances: zolana_transaction::Balances {
            assets: vec![zolana_transaction::AssetBalance {
                asset_id: zolana_transaction::SOL_ASSET_ID,
                mint: zolana_transaction::SOL_MINT,
                amount: utxos.iter().map(|note| note.utxo.amount).sum(),
                utxos,
            }],
        },
        ..Default::default()
    }
}

/// The SOL balance of `notes`.
fn balance(notes: Vec<WalletUtxo>) -> AssetBalance {
    spendable(notes).balances.assets.remove(0)
}

fn reserved(nullifiers: &[u8]) -> HashSet<[u8; 32]> {
    nullifiers.iter().map(|&n| [n; 32]).collect()
}

fn limits(balance: &TokenBalance) -> Vec<(u64, u64)> {
    balance
        .limits
        .iter()
        .map(|limit| (limit.outputs, limit.amount))
        .collect()
}

fn indexed(signature: Option<Signature>) -> serde_json::Value {
    let transactions: Vec<_> = signature
        .into_iter()
        .map(|signature| {
            serde_json::json!({
                "eventIndex": 0, "transaction": {
                    "slot": 5, "txSignature": signature.to_string(), "eventIndex": 0,
                    "txViewingPk": null, "salt": null, "proofless": true,
                    "nullifiers": [], "messages": [], "outputSlots": []
                }
            })
        })
        .collect();
    serde_json::json!({"result": {
        "context": {"slot": 10, "blockTime": 0}, "transactions": transactions
    }})
}

const MINT: Pubkey = Pubkey::new_from_array([7; 32]);

/// An account for `getAccountInfo`, its data in base64.
fn account(owner: Pubkey, data: &[u8]) -> serde_json::Value {
    use base64::Engine;
    serde_json::json!({
        "lamports": 1_000_000,
        "data": [base64::engine::general_purpose::STANDARD.encode(data), "base64"],
        "owner": owner.to_string(),
        "executable": false,
        "rentEpoch": 0,
        "space": data.len(),
    })
}

/// The pool's protocol config: `permissionless` mint registration, and
/// `authority` its protocol authority.
fn protocol_config(permissionless: bool, authority: Pubkey) -> serde_json::Value {
    let config = ProtocolConfig {
        discriminator: zolana_interface::state::discriminator::PROTOCOL_CONFIG,
        protocol_authority: authority,
        tree_creation_authority: authority,
        forester_authority: authority,
        ring_creation_authority: authority,
        fee_authority: authority,
        tree_creation_is_permissionless: 0,
        ring_activation_is_permissionless: 0,
        spl_interface_creation_is_permissionless: u8::from(permissionless),
        next_tree_id: 1,
    };
    account(PROGRAM_ID_PUBKEY, bytemuck::bytes_of(&config))
}

/// A wallet holding [`MINT`], its registry entry `registry` (`null` for
/// none), and the pool's `config`; the methods it was asked.
fn token_wallet(
    signer: &Keypair,
    registry: serde_json::Value,
    pool_config: serde_json::Value,
) -> (MobileWallet, Arc<Mutex<Vec<String>>>) {
    let wallet_config = WalletConfig {
        mints: Some(vec![MintConfig {
            mint: MINT.to_string(),
            token_program: pda::spl_token_program_id().to_string(),
        }]),
        ..config()
    };
    let registry_address = pda::spl_asset_registry(&MINT).to_string();
    let config_address = pda::protocol_config().to_string();
    let answers = (registry, pool_config);
    rpc_wallet_asked(signer, wallet_config, move |method, params| {
        let value = match (method, params[0].as_str()) {
            ("getAccountInfo", Some(address)) if address == registry_address => answers.0.clone(),
            ("getAccountInfo", Some(address)) if address == config_address => answers.1.clone(),
            ("getLatestBlockhash", _) => serde_json::json!({
                "blockhash": "11111111111111111111111111111111",
                "lastValidBlockHeight": 9
            }),
            _ => return serde_json::json!({"error": {"code": -32603, "message": "unexpected"}}),
        };
        serde_json::json!({"result": {"context": {"slot": 1}, "value": value}})
    })
}
