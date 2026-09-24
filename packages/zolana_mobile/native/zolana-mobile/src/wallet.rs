//! A private wallet for SOL and SPL tokens whose Solana key never enters
//! this library.
//!
//! The application's signer (a platform keystore, a wallet adapter, a remote
//! custodian) signs two kinds of bytes:
//!
//! 1. once, the Zolana derivation message from [`derivation_message`]. The
//!    signature is the seed of the wallet's nullifier and viewing keys, which
//!    stay in this process for syncing and proving.
//! 2. each [`PendingTransaction::message_bytes`], the v1 Solana message this
//!    library built, proved and returned unsigned.
//!
//! Proofs are generated on the device with the pinned key for their shape; no
//! witness is sent to a prover server.

use std::{collections::HashMap, str::FromStr, thread::sleep, time::Duration};

use reqwest::header::{HeaderName, HeaderValue};
use solana_commitment_config::CommitmentConfig;

use solana_message::VersionedMessage;
use solana_pubkey::Pubkey;
use solana_rpc_client::{
    http_sender::HttpSender,
    rpc_client::{RpcClient, RpcClientConfig},
};
use solana_signature::Signature;
use solana_transaction::versioned::VersionedTransaction;
use zolana_client::{
    compile_message, ClientError, ComputeBudgetConfig, IndexerPollConfig, Rpc, SolanaRpc,
    ZolanaClient,
};
use zolana_interface::{instruction::CreateAssociatedTokenAccount, pda};
use zolana_keypair::{derivation, PublicKey, ShieldedAddress};
use zolana_transaction::AssetRegistry;
use zolana_wallet::{
    build_deposit_transaction_sync, build_private_transaction_sync,
    build_registration_transaction_sync, create_transfer_sync, create_withdrawal,
    fetch_user_record_optional_checked, get_private_token_balances, get_private_transactions,
    is_wallet_registered_sync, resolved_address_from_record, sync_wallet,
    ClientEd25519WalletAuthority, Deposit, DepositParams, PrivateTransactionDirection,
    PrivateTransactionKind, SyncWalletAuthority, TransferParams, Wallet, WithdrawalLeg,
    WithdrawalParams,
};

use crate::{
    asset::{mint_name, parse_mint, token_account_amount, Asset},
    keys::KeyStore,
    prover::NativeProver,
};

/// The prover URL `ZolanaClient` requires at construction. `with_prover`
/// replaces it before any request is made, so it is never contacted.
const UNUSED_PROVER_URL: &str = "https://prover.invalid";

/// Where the wallet reads chain state and stores proving keys.
pub struct WalletConfig {
    pub rpc_url: String,
    /// Extra HTTP headers on every Solana RPC request, such as an auth token.
    /// Their values are kept out of logs.
    pub rpc_headers: Option<HashMap<String, String>>,
    pub indexer_url: String,
    /// Directory for downloaded proving keys; keep it across launches.
    pub proving_key_dir: String,
    /// Overrides [`crate::DEFAULT_PROVING_KEYS_URL`].
    pub proving_key_url: Option<String>,
    /// Allow a plaintext indexer off loopback (an emulator reaching its host).
    /// The indexer sees the wallet's view tags, so never set this for funds
    /// that matter.
    pub allow_insecure_http: bool,
}

/// The Zolana derivation message the wallet's signer signs once to open it.
pub fn derivation_message(solana_pubkey: String) -> Result<Vec<u8>, String> {
    let owner = parse_pubkey(&solana_pubkey)?;
    Ok(derivation::ed25519_derivation_message(&owner.to_bytes()))
}

/// Whether the user registry publishes this wallet's shielded address.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RegistrationStatus {
    /// No record: others cannot pay this account privately yet.
    NotRegistered,
    /// The record holds this wallet's keys.
    Registered,
    /// The record holds other keys: registered by another client, rotated, or
    /// derived by a signer whose signatures differ between calls. Payments to
    /// this account go to those keys, not to this wallet.
    Conflict,
}

/// What a submitted transaction waits for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PendingTransactionKind {
    /// Publishes the wallet's shielded address so others can pay it.
    Registration,
    /// Moves public funds into the private balance. Public: sender, asset,
    /// amount.
    Deposit,
    /// Private transfer to a registered wallet. Reveals neither amount nor
    /// recipient.
    Transfer,
    /// Moves private funds to a public account. Public: recipient, asset,
    /// amount.
    Withdrawal,
    /// Creates an associated token account so it can receive a withdrawal.
    TokenAccount,
}

/// A built, and where needed proved, v1 transaction awaiting signatures.
pub struct PendingTransaction {
    kind: PendingTransactionKind,
    message: VersionedMessage,
    summary: String,
}

impl PendingTransaction {
    pub fn kind(&self) -> PendingTransactionKind {
        self.kind
    }

    /// Human-readable description to show before asking for a signature.
    pub fn summary(&self) -> String {
        self.summary.clone()
    }

    /// The bytes every signer signs with Ed25519.
    pub fn message_bytes(&self) -> Vec<u8> {
        self.message.serialize()
    }

    /// Base58 public keys that must sign, in signature order.
    pub fn signers(&self) -> Vec<String> {
        let required = usize::from(self.message.header().num_required_signatures);
        self.message.static_account_keys()[..required]
            .iter()
            .map(ToString::to_string)
            .collect()
    }
}

/// What a row of the wallet's history did, from this wallet's side.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ActivityKind {
    /// Public funds moved into the private balance.
    Shielded,
    /// Private funds moved to a public account.
    Unshielded,
    Sent,
    Received,
    /// Notes rearranged within this wallet (change, merges, splits).
    Internal,
}

/// Amounts are in base units: lamports for SOL, the mint's smallest unit
/// otherwise. `mint` is `None` for SOL.
#[derive(Clone, Debug)]
pub struct ActivityEntry {
    pub kind: ActivityKind,
    pub mint: Option<String>,
    pub amount: u64,
    pub signature: String,
    pub slot: u64,
}

/// A spendable private balance. `mint` is `None` for SOL.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TokenBalance {
    pub mint: Option<String>,
    pub amount: u64,
}

#[derive(Clone, Debug)]
pub struct SyncSummary {
    pub stored_utxos: u64,
    pub balances: Vec<TokenBalance>,
}

/// A synced private wallet. State is in memory only: reopen and sync after a
/// restart.
pub struct MobileWallet {
    owner: Pubkey,
    authority: ClientEd25519WalletAuthority,
    wallet: Wallet,
    client: ZolanaClient<SolanaRpc>,
}

impl MobileWallet {
    /// Open the wallet of `solana_pubkey` from its signature over
    /// [`derivation_message`].
    pub fn open(
        config: WalletConfig,
        solana_pubkey: String,
        derivation_signature: Vec<u8>,
    ) -> Result<MobileWallet, String> {
        let owner = parse_pubkey(&solana_pubkey)?;
        let seed: [u8; derivation::ED25519_SEED_LEN] =
            derivation_signature
                .as_slice()
                .try_into()
                .map_err(|_| "derivation_signature_invalid".to_string())?;
        let authority = ClientEd25519WalletAuthority::from_derivation_seed(owner, &seed)
            .map_err(|_| "derivation_signature_invalid".to_string())?;
        let identity = authority.shielded_address().map_err(error)?;
        let wallet = Wallet::new(identity, AssetRegistry::default()).map_err(error)?;
        let keys = KeyStore::new(config.proving_key_dir, config.proving_key_url)?;
        let rpc = solana_rpc(config.rpc_url, config.rpc_headers.unwrap_or_default())?;
        let client = if config.allow_insecure_http {
            ZolanaClient::from_urls_allowing_insecure_http(
                rpc,
                &config.indexer_url,
                UNUSED_PROVER_URL,
            )
        } else {
            ZolanaClient::from_urls(rpc, &config.indexer_url, UNUSED_PROVER_URL).map_err(error)?
        }
        .with_prover(NativeProver::new(keys));
        Ok(MobileWallet {
            owner,
            authority,
            wallet,
            client,
        })
    }

    pub fn solana_pubkey(&self) -> String {
        self.owner.to_string()
    }

    pub fn shielded_address(&self) -> String {
        self.wallet.identity.to_string()
    }

    /// Whether the user registry publishes this wallet's shielded address.
    /// Others can only send to a registered wallet.
    pub fn registration_status(&self) -> Result<RegistrationStatus, String> {
        let record = fetch_user_record_optional_checked(&self.client, self.owner).map_err(error)?;
        let published = record.map(|record| {
            resolved_address_from_record(self.owner, &record).map(|resolved| resolved.address)
        });
        Ok(registration_status_of(published, &self.wallet.identity))
    }

    /// `None` when the registry already holds this wallet's address. Fails
    /// with `registration_conflict` when it holds other keys: this wallet
    /// never replaces them.
    pub fn prepare_registration(&self) -> Result<Option<PendingTransaction>, String> {
        match self.registration_status()? {
            RegistrationStatus::Registered => return Ok(None),
            RegistrationStatus::Conflict => return Err("registration_conflict".to_string()),
            RegistrationStatus::NotRegistered => {}
        }
        let message = build_registration_transaction_sync(
            &self.client,
            self.owner,
            &self.wallet.identity,
            None,
        )
        .map_err(error)?;
        Ok(message.map(|message| PendingTransaction {
            kind: PendingTransactionKind::Registration,
            message,
            summary: format!("Register {} for private payments", self.shielded_address()),
        }))
    }

    /// Deposit public SOL (`mint` `None`) or tokens from this account into
    /// its own private balance. Deposits carry no proof.
    pub fn prepare_deposit(
        &mut self,
        mint: Option<String>,
        amount: u64,
    ) -> Result<PendingTransaction, String> {
        let asset = self.asset(mint)?;
        let deposit = Deposit::new(DepositParams {
            recipient: &self.wallet.identity,
            asset: asset.mint,
            amount,
            spl_token_account: asset.token_account(&self.owner),
            spl_token_program: asset.token_program,
            memo: None,
        })
        .map_err(error)?;
        let message = build_deposit_transaction_sync(
            &self.client,
            self.owner,
            pda::tree(0),
            self.owner,
            &deposit,
        )
        .map_err(error)?;
        Ok(PendingTransaction {
            kind: PendingTransactionKind::Deposit,
            message,
            summary: format!("Deposit {} (public)", asset.describe(amount)),
        })
    }

    /// Fetch and decrypt this wallet's notes from the indexer.
    pub fn sync(&mut self) -> Result<SyncSummary, String> {
        let report = sync_wallet(&mut self.wallet, &self.authority, &self.client).map_err(error)?;
        Ok(SyncSummary {
            stored_utxos: report.stored_utxos as u64,
            balances: self.balances()?,
        })
    }

    /// Public balance of this account, read from the RPC now: lamports, or
    /// the amount in its associated token account for `mint` (0 without one).
    pub fn public_balance(&mut self, mint: Option<String>) -> Result<u64, String> {
        let asset = self.asset(mint)?;
        let Some(token_account) = asset.token_account(&self.owner) else {
            return self.client.get_balance(self.owner).map_err(error);
        };
        match self.client.get_account(token_account).map_err(error)? {
            Some(account) => token_account_amount(&account.data),
            None => Ok(0),
        }
    }

    /// History found by the last [`Self::sync`], newest first.
    pub fn activity(&self) -> Vec<ActivityEntry> {
        let mut entries: Vec<ActivityEntry> = get_private_transactions(&self.wallet)
            .iter()
            .map(|transaction| ActivityEntry {
                kind: match (transaction.kind, transaction.direction) {
                    (PrivateTransactionKind::Deposit, _) => ActivityKind::Shielded,
                    (PrivateTransactionKind::PublicWithdrawal, _) => ActivityKind::Unshielded,
                    (_, PrivateTransactionDirection::SelfTransfer)
                    | (PrivateTransactionKind::Merge | PrivateTransactionKind::Split, _) => {
                        ActivityKind::Internal
                    }
                    (_, PrivateTransactionDirection::Inbound) => ActivityKind::Received,
                    (_, PrivateTransactionDirection::Outbound) => ActivityKind::Sent,
                },
                mint: mint_name(&transaction.asset),
                amount: transaction.amount,
                signature: transaction.id.signature.clone(),
                slot: transaction.id.slot,
            })
            .collect();
        entries.sort_by_key(|entry| std::cmp::Reverse(entry.slot));
        entries
    }

    /// Spendable private balances as of the last [`Self::sync`], one per
    /// asset held.
    pub fn balances(&self) -> Result<Vec<TokenBalance>, String> {
        Ok(get_private_token_balances(&self.wallet)
            .map_err(error)?
            .iter()
            .map(|balance| TokenBalance {
                mint: mint_name(&balance.mint),
                amount: balance.amount,
            })
            .collect())
    }

    /// Spendable private balance of SOL (`mint` `None`) or `mint` as of the
    /// last [`Self::sync`].
    pub fn private_balance(&self, mint: Option<String>) -> Result<u64, String> {
        let mint = mint.as_deref().map(parse_mint).transpose()?;
        let mint = mint.as_ref().and_then(mint_name);
        Ok(self
            .balances()?
            .into_iter()
            .filter(|balance| balance.mint == mint)
            .map(|balance| balance.amount)
            .sum())
    }

    /// Build and prove a private transfer to a registered wallet.
    ///
    /// Refuses an unregistered recipient: the wallet SDK turns a transfer to
    /// one into a public withdrawal, which [`Self::prepare_withdrawal`] makes
    /// explicit instead.
    pub fn prepare_transfer(
        &mut self,
        recipient: String,
        mint: Option<String>,
        amount: u64,
    ) -> Result<PendingTransaction, String> {
        let recipient = parse_pubkey(&recipient)?;
        let asset = self.asset(mint)?;
        if !is_wallet_registered_sync(&self.client, recipient).map_err(error)? {
            return Err("recipient_not_registered".to_string());
        }
        self.sync_before_spending()?;
        let created = create_transfer_sync(TransferParams {
            rpc: &self.client,
            wallet: &self.wallet,
            payer: self.owner,
            recipient,
            asset: asset.mint,
            amount,
        })
        .map_err(error)?;
        if created.recipient.is_public_withdrawal() {
            return Err("recipient_not_registered".to_string());
        }
        let message = build_private_transaction_sync(
            created.transaction,
            &self.wallet,
            &self.authority,
            &self.client,
            self.owner,
        )
        .map_err(error)?;
        Ok(PendingTransaction {
            kind: PendingTransactionKind::Transfer,
            message,
            summary: format!("Send {} privately to {recipient}", asset.describe(amount)),
        })
    }

    /// Build and prove a withdrawal of private funds to the public account
    /// `recipient`. Tokens go to its associated token account, which must
    /// exist: [`Self::prepare_token_account`] creates it.
    pub fn prepare_withdrawal(
        &mut self,
        recipient: String,
        mint: Option<String>,
        amount: u64,
    ) -> Result<PendingTransaction, String> {
        let recipient = parse_pubkey(&recipient)?;
        let asset = self.asset(mint)?;
        if let Some(token_account) = asset.token_account(&recipient) {
            if self
                .client
                .get_account(token_account)
                .map_err(error)?
                .is_none()
            {
                return Err("recipient_token_account_missing".to_string());
            }
        }
        self.sync_before_spending()?;
        let created = create_withdrawal(WithdrawalParams {
            wallet: &self.wallet,
            payer: self.owner,
            legs: vec![WithdrawalLeg {
                recipient,
                asset: asset.mint,
                amount,
                spl_token_program: asset.token_program,
            }],
        })
        .map_err(error)?;
        let message = build_private_transaction_sync(
            created.transaction,
            &self.wallet,
            &self.authority,
            &self.client,
            self.owner,
        )
        .map_err(error)?;
        Ok(PendingTransaction {
            kind: PendingTransactionKind::Withdrawal,
            message,
            summary: format!(
                "Withdraw {} to {recipient} (public)",
                asset.describe(amount)
            ),
        })
    }

    /// Create `owner`'s associated token account for `mint`, paid by this
    /// account, so a withdrawal can reach it. `None` when it already exists.
    pub fn prepare_token_account(
        &mut self,
        owner: String,
        mint: String,
    ) -> Result<Option<PendingTransaction>, String> {
        let owner = parse_pubkey(&owner)?;
        let asset = self.asset(Some(mint))?;
        let token_program = asset.token_program.ok_or("mint_invalid")?;
        let create = CreateAssociatedTokenAccount {
            payer: self.owner,
            owner,
            mint: asset.mint,
            token_program,
        };
        if self
            .client
            .get_account(create.address())
            .map_err(error)?
            .is_some()
        {
            return Ok(None);
        }
        let (blockhash, _) = self.client.get_latest_blockhash().map_err(error)?;
        let message = compile_message(
            &self.owner,
            &[create.instruction()],
            blockhash,
            ComputeBudgetConfig::for_instruction_count(1),
        )
        .map_err(error)?;
        Ok(Some(PendingTransaction {
            kind: PendingTransactionKind::TokenAccount,
            message,
            summary: format!("Create a {} token account for {owner}", asset.mint),
        }))
    }

    /// SOL for `None`; a mint is added to the wallet's registry the first
    /// time it is used.
    fn asset(&mut self, mint: Option<String>) -> Result<Asset, String> {
        Asset::resolve(&self.client, &mut self.wallet.registry, mint.as_deref())
    }

    /// Attach the signatures, send, and wait as [`Self::confirm`] does.
    /// Returns the signature.
    ///
    /// `signatures` follows [`PendingTransaction::signers`]; each is checked
    /// before anything is sent.
    pub fn submit(
        &mut self,
        pending: &PendingTransaction,
        signatures: Vec<Vec<u8>>,
    ) -> Result<String, String> {
        let message_bytes = pending.message_bytes();
        let required = usize::from(pending.message.header().num_required_signatures);
        if signatures.len() != required {
            return Err("signature_count_mismatch".to_string());
        }
        let mut transaction_signatures = Vec::with_capacity(required);
        for (signer, signature) in pending.message.static_account_keys()[..required]
            .iter()
            .zip(&signatures)
        {
            let signature: [u8; 64] = signature
                .as_slice()
                .try_into()
                .map_err(|_| "signature_invalid".to_string())?;
            if !PublicKey::from_ed25519(&signer.to_bytes())
                .verify_message(&message_bytes, &signature)
            {
                return Err("signature_invalid".to_string());
            }
            transaction_signatures.push(Signature::from(signature));
        }
        let transaction = VersionedTransaction {
            signatures: transaction_signatures,
            message: pending.message.clone(),
        };
        let signature = self
            .client
            .process_transaction(transaction)
            .map_err(error)?;
        self.settle(pending.kind, signature)?;
        Ok(signature.to_string())
    }

    /// Wait for a transaction the application sent itself: until Solana
    /// confirms it and, for shielded-pool transactions, the indexer has it.
    /// A confirmed shielded-pool transaction is synced before this returns,
    /// so the notes it spent are no longer offered.
    ///
    /// `signature` is the transaction signature, the fee payer's; it is
    /// checked against `pending` before anything is asked.
    pub fn confirm(
        &mut self,
        pending: &PendingTransaction,
        signature: String,
    ) -> Result<(), String> {
        let signature =
            Signature::from_str(&signature).map_err(|_| "signature_invalid".to_string())?;
        let fee_payer = pending.message.static_account_keys()[0];
        if !PublicKey::from_ed25519(&fee_payer.to_bytes())
            .verify_message(&pending.message_bytes(), signature.as_array())
        {
            return Err("signature_invalid".to_string());
        }
        self.settle(pending.kind, signature)
    }

    fn settle(&mut self, kind: PendingTransactionKind, signature: Signature) -> Result<(), String> {
        if matches!(
            kind,
            PendingTransactionKind::Registration | PendingTransactionKind::TokenAccount
        ) {
            return wait_for_confirmation(&self.client, signature);
        }
        self.client
            .confirm_private_transaction_sync(signature)
            .map_err(error)?;
        // Best effort: the transaction has landed, so a failed sync must not
        // report it as failed. The next spend syncs again first.
        let _ = sync_wallet(&mut self.wallet, &self.authority, &self.client);
        Ok(())
    }

    /// The wallet learns of spends only by syncing, and a note it still
    /// holds may have been spent since: by the last transaction if its sync
    /// failed, or by the same account on another device. Selecting one builds
    /// a transaction the indexer and the program reject.
    fn sync_before_spending(&mut self) -> Result<(), String> {
        sync_wallet(&mut self.wallet, &self.authority, &self.client)
            .map(|_| ())
            .map_err(error)
    }
}

/// A Solana RPC client as `SolanaRpc::new` builds it, with `headers` added.
fn solana_rpc(url: String, headers: HashMap<String, String>) -> Result<SolanaRpc, String> {
    let mut header_map = HttpSender::default_headers();
    for (name, value) in headers {
        let name = HeaderName::from_bytes(name.as_bytes())
            .map_err(|_| "rpc_header_invalid".to_string())?;
        let mut value =
            HeaderValue::from_str(&value).map_err(|_| "rpc_header_invalid".to_string())?;
        value.set_sensitive(true);
        header_map.insert(name, value);
    }
    let timeout = Duration::from_secs(30);
    let client = reqwest::Client::builder()
        .default_headers(header_map)
        .timeout(timeout)
        .pool_idle_timeout(timeout)
        .build()
        .map_err(|_| "rpc_client_unavailable".to_string())?;
    Ok(SolanaRpc::with_client(RpcClient::new_sender(
        HttpSender::new_with_client(url, client),
        RpcClientConfig::with_commitment(CommitmentConfig::confirmed()),
    )))
}

/// Poll until Solana confirms `signature`, with the indexer's backoff.
fn wait_for_confirmation(rpc: &impl Rpc, signature: Signature) -> Result<(), String> {
    let poll = IndexerPollConfig::default();
    for delay in std::iter::once(Default::default()).chain(poll.backoff()) {
        sleep(delay);
        if rpc.confirm_transaction(signature).map_err(error)? {
            return Ok(());
        }
    }
    Err("transaction_not_confirmed".to_string())
}

/// A record that does not parse is someone else's keys as much as a record
/// that differs: both are a conflict.
fn registration_status_of<E>(
    published: Option<Result<ShieldedAddress, E>>,
    identity: &ShieldedAddress,
) -> RegistrationStatus {
    match published {
        None => RegistrationStatus::NotRegistered,
        Some(Ok(address)) if address == *identity => RegistrationStatus::Registered,
        Some(_) => RegistrationStatus::Conflict,
    }
}

fn parse_pubkey(value: &str) -> Result<Pubkey, String> {
    Pubkey::from_str(value).map_err(|_| "pubkey_invalid".to_string())
}

/// Errors reach Dart as text. Wallet and client errors describe what failed
/// without key material; keep it that way when adding variants.
pub(crate) fn error(error: impl Into<ClientError>) -> String {
    error.into().to_string()
}

#[cfg(test)]
mod tests {
    use solana_keypair::Keypair;
    use solana_signer::Signer;
    use zolana_client::{compile_message, ComputeBudgetConfig};

    use super::*;

    fn config() -> WalletConfig {
        WalletConfig {
            // Unroutable: a test that reaches the network fails differently.
            rpc_url: "http://127.0.0.1:1".to_string(),
            rpc_headers: None,
            indexer_url: "http://127.0.0.1:1".to_string(),
            proving_key_dir: std::env::temp_dir().display().to_string(),
            proving_key_url: None,
            allow_insecure_http: false,
        }
    }

    fn open(signer: &Keypair) -> Result<MobileWallet, String> {
        let pubkey = signer.pubkey().to_string();
        let message = derivation_message(pubkey.clone())?;
        let signature = signer.sign_message(&message);
        MobileWallet::open(config(), pubkey, signature.as_ref().to_vec())
    }

    #[test]
    fn opens_only_from_a_signature_over_the_derivation_message() {
        let signer = Keypair::new();
        let wallet = open(&signer).expect("derivation signature opens the wallet");
        assert_eq!(wallet.solana_pubkey(), signer.pubkey().to_string());
        assert_eq!(
            wallet.shielded_address(),
            open(&signer).unwrap().shielded_address(),
            "the shielded identity is deterministic in the signer"
        );

        let pubkey = signer.pubkey().to_string();
        let other_message = signer.sign_message(b"not the derivation message");
        let other_signer =
            Keypair::new().sign_message(&derivation_message(pubkey.clone()).unwrap());
        for signature in [
            other_message.as_ref().to_vec(),
            other_signer.as_ref().to_vec(),
            vec![0; 63],
        ] {
            assert_eq!(
                MobileWallet::open(config(), pubkey.clone(), signature)
                    .err()
                    .as_deref(),
                Some("derivation_signature_invalid")
            );
        }
    }

    /// A deposit-kind message `signer` pays for, with no instructions.
    fn unsigned(signer: &Keypair) -> PendingTransaction {
        let message = compile_message(
            &signer.pubkey(),
            &[],
            Default::default(),
            ComputeBudgetConfig {
                cu_limit: 200_000,
                cu_price_micro_lamports: None,
            },
        )
        .unwrap();
        PendingTransaction {
            kind: PendingTransactionKind::Deposit,
            message,
            summary: String::new(),
        }
    }

    #[test]
    fn submit_checks_every_signature_before_sending() {
        let signer = Keypair::new();
        let mut wallet = open(&signer).unwrap();
        let pending = || unsigned(&signer);
        assert_eq!(pending().signers(), vec![signer.pubkey().to_string()]);
        let valid = signer.sign_message(&pending().message_bytes());
        let wrong_signer = Keypair::new().sign_message(&pending().message_bytes());
        for (signatures, expected) in [
            (vec![], "signature_count_mismatch"),
            (vec![valid.as_ref().to_vec(); 2], "signature_count_mismatch"),
            (vec![wrong_signer.as_ref().to_vec()], "signature_invalid"),
            (vec![valid.as_ref()[..63].to_vec()], "signature_invalid"),
        ] {
            assert_eq!(
                wallet.submit(&pending(), signatures).err().as_deref(),
                Some(expected)
            );
        }
        // A valid signature gets as far as the unroutable RPC.
        let error = wallet
            .submit(&pending(), vec![valid.as_ref().to_vec()])
            .unwrap_err();
        assert!(!error.starts_with("signature_"), "{error}");
    }

    #[test]
    fn confirm_accepts_only_the_fee_payer_signature() {
        let signer = Keypair::new();
        let mut wallet = open(&signer).unwrap();
        let pending = unsigned(&signer);
        let other = Keypair::new().sign_message(&pending.message_bytes());
        for signature in ["not-a-signature".to_string(), other.to_string()] {
            assert_eq!(
                wallet.confirm(&pending, signature).err().as_deref(),
                Some("signature_invalid")
            );
        }
        // The fee payer's signature gets as far as the unroutable RPC.
        let signature = signer.sign_message(&pending.message_bytes()).to_string();
        let error = wallet.confirm(&pending, signature).unwrap_err();
        assert_ne!(error, "signature_invalid");
    }

    #[test]
    fn rejects_rpc_headers_that_are_not_http() {
        let signer = Keypair::new();
        let pubkey = signer.pubkey().to_string();
        let signature = signer.sign_message(&derivation_message(pubkey.clone()).unwrap());
        for header in [("bad name", "value"), ("x-token", "line\nbreak")] {
            let config = WalletConfig {
                rpc_headers: Some(HashMap::from([(header.0.into(), header.1.into())])),
                ..config()
            };
            assert_eq!(
                MobileWallet::open(config, pubkey.clone(), signature.as_ref().to_vec())
                    .err()
                    .as_deref(),
                Some("rpc_header_invalid")
            );
        }
        let config = WalletConfig {
            rpc_headers: Some(HashMap::from([("x-token".into(), "secret".into())])),
            ..config()
        };
        assert!(MobileWallet::open(config, pubkey, signature.as_ref().to_vec()).is_ok());
    }

    #[test]
    fn a_record_with_other_keys_is_a_conflict() {
        let identity = open(&Keypair::new()).unwrap().wallet.identity;
        let other = open(&Keypair::new()).unwrap().wallet.identity;
        let status = |published: Option<Result<ShieldedAddress, ()>>| {
            registration_status_of(published, &identity)
        };
        assert_eq!(status(None), RegistrationStatus::NotRegistered);
        assert_eq!(status(Some(Ok(identity))), RegistrationStatus::Registered);
        assert_eq!(status(Some(Ok(other))), RegistrationStatus::Conflict);
        assert_eq!(status(Some(Err(()))), RegistrationStatus::Conflict);
    }

    #[test]
    fn rejects_malformed_recipients_before_the_network() {
        let mut wallet = open(&Keypair::new()).unwrap();
        assert_eq!(
            wallet
                .prepare_transfer("not-a-pubkey".into(), None, 1)
                .err()
                .as_deref(),
            Some("pubkey_invalid")
        );
    }
}
