//! A private wallet for SOL and SPL tokens whose Solana key never enters
//! this library.
//!
//! The application's signer (a platform keystore, a wallet adapter, a remote
//! custodian) signs two kinds of bytes:
//!
//! 1. once, the Zolana derivation message from [`derivation_message`]. The
//!    signature is the seed of the wallet's nullifier and viewing keys, which
//!    stay in this process for reading notes and proving. An application that
//!    saved them with [`MobileWallet::export_keys`] opens the wallet from them
//!    instead, without a signature.
//! 2. each [`PendingTransaction::message_bytes`], the v1 Solana message this
//!    library built, proved and returned unsigned.
//!
//! The wallet keeps no chain state between calls. Balances, spends and history
//! read the wallet's notes from the indexer when they run, so they see a
//! spend by another client, or by the last transaction, as soon as the
//! indexer has it.
//!
//! Proofs are generated on the device with the pinned key for their shape, or,
//! for a spend that asks for [`Proving::Remote`], by the application's backend
//! through [`MobileWallet::set_remote_prover`]. The client verifies a
//! backend's proof against the pinned verifying key before the wallet builds
//! the message.

use std::{
    cmp::Reverse,
    collections::{HashMap, HashSet},
    str::FromStr,
    sync::{Arc, Mutex, PoisonError},
    thread::sleep,
};

use flutter_rust_bridge::DartFnFuture;
use solana_instruction::Instruction;
use solana_message::VersionedMessage;
use solana_pubkey::Pubkey;
use solana_signature::Signature;
use solana_transaction::versioned::VersionedTransaction;
use zeroize::Zeroizing;
use zolana_client::{
    compile_message,
    user_registry::{
        build_registration_transaction_sync, fetch_user_record_optional_checked,
        resolved_address_from_record, try_resolve_registered_address,
    },
    ClientError, ComputeBudgetConfig, IndexerPollConfig, Rpc, SignedPrivateTransaction, SolanaRpc,
    SpendableUtxos, Submission, ZolanaClient,
};
use zolana_interface::pda;
use zolana_keypair::{derivation, Curve, NullifierKey, PublicKey, ShieldedAddress, ViewingKey};
use zolana_program::instruction::{
    AssetDeposit, CreateAssociatedTokenAccount, Deposit, DepositAsset, DepositSplAccounts,
    TransactInterfaceTransferAccounts, TransactSolTransferAccounts, TransactSplWithdrawalAccounts,
};
use zolana_transaction::{
    instructions::transact::{auto_shapes, ConfidentialTransaction},
    AssetRegistry, LocalShieldedKeys, SpendableDecryptionResult, WalletUtxo,
};

use crate::{
    activity::{self, ActivityEntry},
    asset::{mint_name, token_account_amount, Asset},
    keys::KeyStore,
    prover::{proving_error, NativeProver, Proving, RemoteProver, SpendProver, WalletProver},
    transport::Transport,
};

/// Where the wallet reads chain state and stores proving keys.
pub struct WalletConfig {
    pub rpc_url: String,
    pub indexer_url: String,
    /// Directory for downloaded proving keys; keep it across launches.
    pub proving_key_dir: String,
    /// Overrides [`crate::DEFAULT_PROVING_KEYS_URL`].
    pub proving_key_url: Option<String>,
    /// Where spends are proved unless a call says otherwise: on the device
    /// when `None`. [`Proving::Remote`] needs
    /// [`MobileWallet::set_remote_prover`].
    pub proving: Option<Proving>,
    /// SPL mints [`MobileWallet::balances`] reports. SOL is always included,
    /// and a mint named in any call is added for the rest of the session.
    /// Notes in other mints are left out, as the Zolana SDK leaves out assets
    /// its registry does not hold.
    pub mints: Vec<String>,
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
    /// Unique per wallet: the key of the notes a spend reserves.
    id: u64,
    kind: PendingTransactionKind,
    message: VersionedMessage,
    summary: String,
    last_valid_block_height: u64,
    /// Nullifiers of the notes it spends; empty unless it is a spend.
    spends: Vec<[u8; 32]>,
}

impl PendingTransaction {
    pub fn kind(&self) -> PendingTransactionKind {
        self.kind
    }

    /// The last block height at which the message can still land. Past it,
    /// [`MobileWallet::refresh`] gives it a new blockhash.
    pub fn last_valid_block_height(&self) -> u64 {
        self.last_valid_block_height
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

/// The wallet's derived keys, for the application's secure storage. They open
/// the wallet with [`MobileWallet::open_with_keys`] without a derivation
/// signature. They cannot move funds, but they show the wallet's balances and
/// history and link its spends.
///
/// - `viewing_private_key`: 32 bytes, the P-256 scalar, big-endian.
/// - `viewing_public_key`: 33 bytes, its compressed SEC1 point.
/// - `nullifier_private_key`: 31 bytes.
/// - `nullifier_public_key`: 32 bytes, the Poseidon hash of the private key.
pub struct WalletKeys {
    pub viewing_private_key: Vec<u8>,
    pub viewing_public_key: Vec<u8>,
    pub nullifier_private_key: Vec<u8>,
    pub nullifier_public_key: Vec<u8>,
}

/// A spendable private balance. `mint` is `None` for SOL.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TokenBalance {
    pub mint: Option<String>,
    pub amount: u64,
}

/// Notes reserved by prepared spends. The next spend selects other notes
/// until a spend is submitted or confirmed, released, or past its
/// `last_valid_block_height`, when it can no longer land.
#[derive(Default)]
struct Reservations {
    next_id: u64,
    /// Nullifiers and last valid block height, by transaction id.
    reserved: HashMap<u64, (Vec<[u8; 32]>, u64)>,
}

/// A private wallet. It holds its keys, the asset ids of the mints it has
/// named and the notes its prepared spends reserve, and reads everything else
/// from the chain and the indexer when asked.
pub struct MobileWallet {
    owner: Pubkey,
    address: ShieldedAddress,
    keys: LocalShieldedKeys,
    /// Completes the spent inputs' nullifiers when proving.
    nullifier_key: NullifierKey,
    viewing_key: ViewingKey,
    /// SOL and the mints resolved so far.
    assets: AssetRegistry,
    /// Configured mints not resolved into `assets` yet.
    mints: Vec<String>,
    reservations: Mutex<Reservations>,
    /// Where spends are proved unless a call says otherwise.
    proving: Proving,
    remote_prover: Option<Arc<RemoteProver>>,
    /// Shared with the client's prover.
    spend_prover: SpendProver,
    client: ZolanaClient<SolanaRpc>,
}

impl MobileWallet {
    /// Open the wallet of `solana_pubkey` from its signature over
    /// [`derivation_message`]. Every request of the wallet goes through
    /// `transport`; it opens no connection itself.
    pub fn open(
        config: WalletConfig,
        solana_pubkey: String,
        derivation_signature: Vec<u8>,
        transport: Transport,
    ) -> Result<MobileWallet, String> {
        let owner = parse_pubkey(&solana_pubkey)?;
        let seed: [u8; derivation::ED25519_SEED_LEN] = derivation_signature
            .as_slice()
            .try_into()
            .map_err(derivation_invalid)?;
        let signing_pubkey = PublicKey::from_ed25519(&owner.to_bytes());
        let message = derivation::ed25519_derivation_message(&owner.to_bytes());
        if !signing_pubkey.verify_message(&message, &seed) {
            return Err("derivation_signature_invalid".to_string());
        }
        let (nullifier_key, viewing_key) =
            derivation::expand_roles(&seed, Curve::Ed25519).map_err(derivation_invalid)?;
        Self::from_keys(config, owner, nullifier_key, viewing_key, transport)
    }

    /// Open the wallet of `solana_pubkey` from keys [`Self::export_keys`]
    /// returned. Fails with `wallet_keys_invalid` unless each private key
    /// yields its public key. `transport` works as in [`Self::open`].
    pub fn open_with_keys(
        config: WalletConfig,
        solana_pubkey: String,
        keys: WalletKeys,
        transport: Transport,
    ) -> Result<MobileWallet, String> {
        let owner = parse_pubkey(&solana_pubkey)?;
        let viewing_secret = Zeroizing::new(keys.viewing_private_key);
        let nullifier_secret = Zeroizing::new(keys.nullifier_private_key);
        let viewing_key =
            ViewingKey::from_bytes(viewing_secret.as_slice().try_into().map_err(keys_invalid)?)
                .map_err(keys_invalid)?;
        let nullifier_key = NullifierKey::from_secret(
            nullifier_secret
                .as_slice()
                .try_into()
                .map_err(keys_invalid)?,
        );
        if viewing_key.pubkey().as_bytes().as_slice() != keys.viewing_public_key
            || nullifier_key.pubkey().map_err(keys_invalid)?.as_slice() != keys.nullifier_public_key
        {
            return Err(keys_invalid(()));
        }
        Self::from_keys(config, owner, nullifier_key, viewing_key, transport)
    }

    fn from_keys(
        config: WalletConfig,
        owner: Pubkey,
        nullifier_key: NullifierKey,
        viewing_key: ViewingKey,
        transport: Transport,
    ) -> Result<MobileWallet, String> {
        let address = ShieldedAddress {
            signing_pubkey: PublicKey::from_ed25519(&owner.to_bytes()),
            nullifier_pubkey: nullifier_key.pubkey().map_err(keys_invalid)?,
            viewing_pubkey: viewing_key.pubkey(),
        };
        let keys =
            LocalShieldedKeys::new(address, vec![viewing_key.clone()], nullifier_key.clone())
                .map_err(error)?;
        let proving_keys = KeyStore::new(
            config.proving_key_dir,
            config.proving_key_url,
            transport.clone(),
        );
        let spend_prover = SpendProver::default();
        // With its own prover, the client fetches every Merkle proof from the
        // indexer itself: the wallet's provers prove only complete witnesses.
        let client = ZolanaClient::with_prover(
            transport.solana_rpc(config.rpc_url)?,
            transport.indexer(&config.indexer_url),
            WalletProver {
                native: NativeProver::new(proving_keys),
                remote: Arc::clone(&spend_prover),
            },
        );
        Ok(MobileWallet {
            owner,
            address,
            keys,
            nullifier_key,
            viewing_key,
            assets: AssetRegistry::default(),
            mints: config.mints,
            reservations: Mutex::default(),
            proving: config.proving.unwrap_or(Proving::Local),
            remote_prover: None,
            spend_prover,
            client,
        })
    }

    /// Prove the spends that ask for [`Proving::Remote`] with `prove`, the
    /// application's backend. It receives the `/prove` request body the Zolana
    /// SDK's prover client sends and returns its prover's proof: the gnark
    /// proof JSON, alone or as the `proof` of the prover's response. `None`
    /// fails the spend with `remote_prover_failed`.
    ///
    /// The client verifies the proof against the pinned verifying key and the
    /// public input it computed itself, before the message is built. A proof
    /// that does not parse fails with `proof_malformed`, one that does not
    /// verify with `proof_invalid`.
    pub fn set_remote_prover(
        &mut self,
        prove: impl Fn(Vec<u8>) -> DartFnFuture<Option<Vec<u8>>> + Send + Sync + 'static,
    ) {
        self.remote_prover = Some(Arc::new(RemoteProver::new(prove)));
    }

    pub fn shielded_address(&self) -> String {
        self.address.to_string()
    }

    /// The keys [`Self::open_with_keys`] opens this wallet from.
    pub fn export_keys(&self) -> WalletKeys {
        WalletKeys {
            viewing_private_key: self.viewing_key.secret_bytes().to_vec(),
            viewing_public_key: self.address.viewing_pubkey.as_bytes().to_vec(),
            nullifier_private_key: self.nullifier_key.secret().to_vec(),
            nullifier_public_key: self.address.nullifier_pubkey.to_vec(),
        }
    }

    /// Whether the user registry publishes this wallet's shielded address.
    /// Others can only send to a registered wallet.
    pub fn registration_status(&self) -> Result<RegistrationStatus, String> {
        let record = fetch_user_record_optional_checked(&self.client, self.owner).map_err(error)?;
        let published = record.map(|record| {
            resolved_address_from_record(self.owner, &record).map(|resolved| resolved.address)
        });
        Ok(registration_status_of(published, &self.address))
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
            &self.address,
            None,
            None,
        )
        .map_err(error)?;
        message
            .map(|message| {
                self.pending(
                    PendingTransactionKind::Registration,
                    message,
                    format!("Register {} for private payments", self.shielded_address()),
                )
            })
            .transpose()
    }

    /// Deposit public SOL (`mint` `None`) or tokens from this account into
    /// its own private balance. Deposits carry no proof.
    pub fn prepare_deposit(
        &mut self,
        mint: Option<String>,
        amount: u64,
    ) -> Result<PendingTransaction, String> {
        let asset = self.asset(mint)?;
        let deposit_asset = match asset.token_program {
            None => DepositAsset::Sol,
            Some(token_program) => DepositAsset::Spl(DepositSplAccounts {
                mint: asset.mint,
                user_token: asset.token_account(&self.owner).ok_or("mint_invalid")?,
                token_program,
            }),
        };
        // The wallet's viewing key tags a deposit: it is how the wallet finds
        // the output without knowing who sent it.
        let deposit = Deposit {
            tree: pda::tree(0),
            depositor: self.owner,
            deposits: vec![AssetDeposit {
                asset: deposit_asset,
                view_tag: self.address.viewing_pubkey.x(),
                owner: self.address.owner_hash().map_err(error)?,
                amount,
                memo: None,
            }],
        }
        .instruction()
        .map_err(error)?;
        self.pending(
            PendingTransactionKind::Deposit,
            self.message(deposit)?,
            format!("Deposit {} (public)", asset.describe(amount)),
        )
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

    /// Spendable private balances, read from the indexer now: one per asset
    /// held in SOL, the configured mints and the mints named so far.
    pub fn balances(&mut self) -> Result<Vec<TokenBalance>, String> {
        Ok(self
            .spendable()?
            .balances
            .assets
            .iter()
            .map(|balance| TokenBalance {
                mint: mint_name(&balance.mint),
                amount: balance.amount,
            })
            .collect())
    }

    /// Spendable private balance of SOL (`mint` `None`) or `mint`, read from
    /// the indexer now.
    pub fn private_balance(&mut self, mint: Option<String>) -> Result<u64, String> {
        let asset = self.asset(mint)?;
        Ok(self
            .spendable()?
            .balances
            .get_balance(asset.mint)
            .map_or(0, |balance| balance.amount))
    }

    /// Transaction history, read from the indexer now, newest first: one
    /// entry per asset each transaction moved, in SOL and every mint
    /// [`Self::balances`] reports.
    ///
    /// The indexer does not say which spends were withdrawals: a spend whose
    /// outputs are all this wallet's own is listed as unshielded, one with
    /// another wallet's output as sent.
    pub fn activity(&mut self) -> Result<Vec<ActivityEntry>, String> {
        self.resolve_mints()?;
        activity::fetch(&self.keys, &self.assets, &self.client).map_err(error)
    }

    /// Build and prove a private transfer to a registered wallet.
    ///
    /// Refuses an unregistered recipient instead of paying it publicly:
    /// [`Self::prepare_withdrawal`] makes a public payment explicit.
    ///
    /// `fee_payer` pays the network fee, this account when `None`. The proof
    /// binds it, so it cannot change after this call. Another fee payer signs
    /// first, before this account.
    ///
    /// `proving` says where it is proved, [`WalletConfig::proving`] when
    /// `None`. Remote proving without [`Self::set_remote_prover`] fails with
    /// `remote_prover_missing`.
    pub fn prepare_transfer(
        &mut self,
        recipient: String,
        mint: Option<String>,
        amount: u64,
        fee_payer: Option<String>,
        proving: Option<Proving>,
    ) -> Result<PendingTransaction, String> {
        let recipient = parse_pubkey(&recipient)?;
        let payer = self.fee_payer(fee_payer)?;
        let remote = self.remote(proving)?;
        let asset = self.asset(mint)?;
        let Some(registered) =
            try_resolve_registered_address(&self.client, recipient).map_err(error)?
        else {
            return Err("recipient_not_registered".to_string());
        };
        let inputs = self.select_notes(asset, amount)?;
        let spends = nullifiers(&inputs);
        let mut transaction = ConfidentialTransaction::new(inputs, payer).map_err(error)?;
        match asset.token_program {
            None => transaction.transfer_sol(&registered.address, amount),
            Some(_) => transaction.transfer(&registered.address, asset.mint, amount),
        }
        .map_err(error)?;
        let pending = self.pending(
            PendingTransactionKind::Transfer,
            self.prove(transaction, Vec::new(), payer, remote)?,
            format!("Send {} privately to {recipient}", asset.describe(amount)),
        )?;
        Ok(self.reserve(pending, spends))
    }

    /// Build and prove a withdrawal of private funds to the public account
    /// `recipient`. Tokens go to its associated token account, which must
    /// exist: [`Self::prepare_token_account`] creates it.
    ///
    /// `fee_payer` and `proving` work as in [`Self::prepare_transfer`].
    pub fn prepare_withdrawal(
        &mut self,
        recipient: String,
        mint: Option<String>,
        amount: u64,
        fee_payer: Option<String>,
        proving: Option<Proving>,
    ) -> Result<PendingTransaction, String> {
        let recipient = parse_pubkey(&recipient)?;
        let payer = self.fee_payer(fee_payer)?;
        let remote = self.remote(proving)?;
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
        let inputs = self.select_notes(asset, amount)?;
        let spends = nullifiers(&inputs);
        let mut transaction = ConfidentialTransaction::new(inputs, payer).map_err(error)?;
        let settlement = withdraw_to(&mut transaction, recipient, asset, amount)?;
        let pending = self.pending(
            PendingTransactionKind::Withdrawal,
            self.prove(transaction, vec![settlement], payer, remote)?,
            format!(
                "Withdraw {} to {recipient} (public)",
                asset.describe(amount)
            ),
        )?;
        Ok(self.reserve(pending, spends))
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
        self.pending(
            PendingTransactionKind::TokenAccount,
            self.message(create.instruction())?,
            format!("Create a {} token account for {owner}", asset.mint),
        )
        .map(Some)
    }

    fn fee_payer(&self, fee_payer: Option<String>) -> Result<Pubkey, String> {
        fee_payer.as_deref().map_or(Ok(self.owner), parse_pubkey)
    }

    /// The backend that proves a spend, or `None` to prove it on the device.
    fn remote(&self, proving: Option<Proving>) -> Result<Option<Arc<RemoteProver>>, String> {
        match proving.unwrap_or(self.proving) {
            Proving::Local => Ok(None),
            Proving::Remote => self
                .remote_prover
                .clone()
                .map(Some)
                .ok_or_else(|| "remote_prover_missing".to_string()),
        }
    }

    /// A message with `instruction` alone, paid by this account. Its blockhash
    /// is set by [`Self::pending`].
    fn message(&self, instruction: Instruction) -> Result<VersionedMessage, String> {
        compile_message(
            &self.owner,
            &[instruction],
            Default::default(),
            ComputeBudgetConfig::for_instruction_count(1),
        )
        .map_err(error)
    }

    /// `message` with the latest blockhash, set last so that it is as young
    /// as it can be when the application gets it.
    fn pending(
        &self,
        kind: PendingTransactionKind,
        mut message: VersionedMessage,
        summary: String,
    ) -> Result<PendingTransaction, String> {
        let (blockhash, last_valid_block_height) =
            self.client.get_latest_blockhash().map_err(error)?;
        message.set_recent_blockhash(blockhash);
        let mut reservations = self.reservations();
        let id = reservations.next_id;
        reservations.next_id += 1;
        Ok(PendingTransaction {
            id,
            kind,
            message,
            summary,
            last_valid_block_height,
            spends: Vec::new(),
        })
    }

    /// `pending`, reserving the notes with `spends` as nullifiers until it
    /// can no longer land.
    fn reserve(
        &self,
        mut pending: PendingTransaction,
        spends: Vec<[u8; 32]>,
    ) -> PendingTransaction {
        self.reservations().reserved.insert(
            pending.id,
            (spends.clone(), pending.last_valid_block_height),
        );
        pending.spends = spends;
        pending
    }

    /// Release the notes `pending` reserves, so the next spend can select
    /// them: for a prepared spend that will not be sent, such as one the user
    /// declined. Submitting or confirming it does this too.
    pub fn release(&self, pending: &PendingTransaction) {
        self.reservations().reserved.remove(&pending.id);
    }

    fn reservations(&self) -> std::sync::MutexGuard<'_, Reservations> {
        self.reservations
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    /// The notes a spend of `amount` takes, other than those prepared spends
    /// reserve. Reservations past their last valid block height are dropped:
    /// those spends can no longer land.
    fn select_notes(&mut self, asset: Asset, amount: u64) -> Result<Vec<WalletUtxo>, String> {
        let spendable = self.spendable()?;
        let reserved = if self.reservations().reserved.is_empty() {
            HashSet::new()
        } else {
            let height = self.client.get_block_height().map_err(error)?;
            let mut reservations = self.reservations();
            reservations
                .reserved
                .retain(|_, (_, last_valid_block_height)| *last_valid_block_height >= height);
            reservations
                .reserved
                .values()
                .flat_map(|(spends, _)| spends.iter().copied())
                .collect()
        };
        select_notes(&spendable, asset, amount, &reserved)
    }

    /// `pending` with a new blockhash and the same proof, for an approval that
    /// outlived [`PendingTransaction::last_valid_block_height`]. Signatures
    /// over the old message do not apply to the new one.
    ///
    /// The proof stays valid while the trees still hold the roots it was built
    /// on: a state tree keeps its last 500 roots, about its last 500
    /// transactions, and a nullifier tree its last 100 batch roots. After that
    /// the program rejects the proof as stale, and the spend is prepared again.
    ///
    /// A refreshed spend reserves its notes again until its new last valid
    /// block height.
    pub fn refresh(&self, pending: &PendingTransaction) -> Result<PendingTransaction, String> {
        let refreshed = self.pending(
            pending.kind,
            pending.message.clone(),
            pending.summary.clone(),
        )?;
        self.release(pending);
        Ok(if pending.spends.is_empty() {
            refreshed
        } else {
            self.reserve(refreshed, pending.spends.clone())
        })
    }

    /// SOL for `None`; a mint is added to the wallet's registry the first
    /// time it is used.
    fn asset(&mut self, mint: Option<String>) -> Result<Asset, String> {
        Asset::resolve(&self.client, &mut self.assets, mint.as_deref())
    }

    /// The wallet's spendable notes as the indexer has them now, in SOL and
    /// every mint in the registry.
    fn spendable(&mut self) -> Result<SpendableDecryptionResult, String> {
        self.resolve_mints()?;
        SpendableUtxos::new(&self.keys, &self.assets)
            .fetch(&self.client)
            .map_err(error)
    }

    /// Resolve the configured mints once each; one that fails stays for the
    /// next call.
    fn resolve_mints(&mut self) -> Result<(), String> {
        while let Some(mint) = self.mints.last().cloned() {
            self.asset(Some(mint))?;
            self.mints.pop();
        }
        Ok(())
    }

    /// Encrypt and prove `transaction`, on the device or by `remote`, and build
    /// the Solana message `payer` pays for.
    fn prove(
        &self,
        transaction: ConfidentialTransaction,
        settlement_transfers: Vec<TransactInterfaceTransferAccounts>,
        payer: Pubkey,
        remote: Option<Arc<RemoteProver>>,
    ) -> Result<VersionedMessage, String> {
        let signed = SignedPrivateTransaction {
            transaction: transaction.encrypt(&self.keys).map_err(error)?,
            settlement_transfers,
        };
        *self
            .spend_prover
            .lock()
            .unwrap_or_else(PoisonError::into_inner) = remote;
        Submission::new(&signed, payer, &self.nullifier_key)
            .finish_unsigned_sync(&self.client)
            .map_err(proving_error)
    }

    /// Attach the signatures, send, and wait as [`Self::confirm`] does.
    /// Returns the signature.
    ///
    /// `signatures` follows [`PendingTransaction::signers`]; each is checked
    /// before anything is sent.
    pub fn submit(
        &self,
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
            if !signed_by(signer, &message_bytes, &signature) {
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
        self.settle(pending, signature)?;
        Ok(signature.to_string())
    }

    /// Wait for a transaction the application sent itself: until Solana
    /// confirms it and, for shielded-pool transactions, the indexer has it,
    /// so the next balance or spend reads the notes it created and not the
    /// ones it spent.
    ///
    /// `signature` is the transaction signature, the fee payer's; it is
    /// checked against `pending` before anything is asked.
    pub fn confirm(&self, pending: &PendingTransaction, signature: String) -> Result<(), String> {
        let signature =
            Signature::from_str(&signature).map_err(|_| "signature_invalid".to_string())?;
        let fee_payer = &pending.message.static_account_keys()[0];
        if !signed_by(fee_payer, &pending.message_bytes(), signature.as_array()) {
            return Err("signature_invalid".to_string());
        }
        self.settle(pending, signature)
    }

    /// Wait for a shielded-pool transaction by its signature alone, such as one
    /// sent before the application restarted: until Solana confirms it and the
    /// indexer has it, so the next balance reads its notes. A transaction that
    /// failed on chain fails here with the chain's error.
    pub fn wait_for_transaction(&self, signature: String) -> Result<(), String> {
        let signature =
            Signature::from_str(&signature).map_err(|_| "signature_invalid".to_string())?;
        self.client
            .confirm_private_transaction_sync(signature)
            .map_err(error)
    }

    /// Once it settles, a spend's notes are spent and no longer reserved.
    fn settle(&self, pending: &PendingTransaction, signature: Signature) -> Result<(), String> {
        if matches!(
            pending.kind,
            PendingTransactionKind::Registration | PendingTransactionKind::TokenAccount
        ) {
            return wait_for_confirmation(&self.client, signature);
        }
        self.client
            .confirm_private_transaction_sync(signature)
            .map_err(error)?;
        self.release(pending);
        Ok(())
    }
}

/// A ring-bound note's commitment covers its ring; the default-ring circuit
/// does not.
fn is_default_ring_spendable(entry: &WalletUtxo) -> bool {
    entry.utxo.ring_program_id.is_none() && entry.ring_data_hash.is_none()
}

/// The notes a spend of `amount` takes, as the Zolana CLI selects them:
/// largest first, all on one tree, at most as many as the widest automatic
/// shape has inputs. A balance spread over trees, or needing more notes, has
/// to be merged first.
fn select_notes(
    spendable: &SpendableDecryptionResult,
    asset: Asset,
    amount: u64,
    reserved: &HashSet<[u8; 32]>,
) -> Result<Vec<WalletUtxo>, String> {
    let eligible: Vec<&WalletUtxo> = spendable
        .utxos()
        .filter(|entry| entry.utxo.asset.asset == asset.mint && is_default_ring_spendable(entry))
        .collect();
    let free = eligible
        .iter()
        .copied()
        .filter(|entry| !reserved.contains(&entry.nullifier))
        .collect();
    match pick(free, amount) {
        Ok(selected) => Ok(selected),
        // The notes it needs are reserved by a prepared spend.
        Err(_) if pick(eligible, amount).is_ok() => Err("notes_reserved".to_string()),
        Err(error) => Err(error.to_string()),
    }
}

fn pick(mut candidates: Vec<&WalletUtxo>, amount: u64) -> Result<Vec<WalletUtxo>, &'static str> {
    let mut trees: Vec<_> = candidates.iter().map(|entry| entry.tree_id()).collect();
    trees.sort();
    trees.dedup();
    match trees.len() {
        0 => return Err("insufficient_private_balance"),
        1 => {}
        _ => return Err("merge_required"),
    }
    let max_inputs = auto_shapes()
        .map(|shape| shape.n_inputs())
        .max()
        .unwrap_or(0);
    candidates.sort_by_key(|entry| Reverse(entry.utxo.amount));
    let total: u64 = candidates.iter().map(|entry| entry.utxo.amount).sum();
    let mut selected = Vec::new();
    let mut covered = 0u64;
    for entry in candidates.into_iter().take(max_inputs) {
        covered += entry.utxo.amount;
        selected.push(entry.clone());
        if covered >= amount {
            return Ok(selected);
        }
    }
    Err(if total >= amount {
        "merge_required"
    } else {
        "insufficient_private_balance"
    })
}

fn nullifiers(notes: &[WalletUtxo]) -> Vec<[u8; 32]> {
    notes.iter().map(|note| note.nullifier).collect()
}

/// Where a withdrawal settles: the recipient itself for SOL, its associated
/// token account for SPL, as the Zolana CLI builds it.
fn withdraw_to(
    transaction: &mut ConfidentialTransaction,
    recipient: Pubkey,
    asset: Asset,
    amount: u64,
) -> Result<TransactInterfaceTransferAccounts, String> {
    let (Some(token_program), Some(user_token_account)) =
        (asset.token_program, asset.token_account(&recipient))
    else {
        transaction.withdraw_sol(amount, recipient).map_err(error)?;
        return Ok(TransactInterfaceTransferAccounts::Sol(
            TransactSolTransferAccounts { recipient },
        ));
    };
    transaction
        .withdraw(asset.mint, amount, user_token_account)
        .map_err(error)?;
    Ok(TransactInterfaceTransferAccounts::SplWithdrawal(
        TransactSplWithdrawalAccounts {
            mint: asset.mint,
            spl_interface: pda::spl_interface(&asset.mint),
            user_token_account,
            token_program,
        },
    ))
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

/// Whether `signature` is `signer`'s Ed25519 signature over `message`.
fn signed_by(signer: &Pubkey, message: &[u8], signature: &[u8; 64]) -> bool {
    PublicKey::from_ed25519(&signer.to_bytes()).verify_message(message, signature)
}

/// Saved keys that are malformed or do not match their public keys.
fn keys_invalid<E>(_: E) -> String {
    "wallet_keys_invalid".to_string()
}

/// A derivation signature that does not yield the wallet's keys.
fn derivation_invalid<E>(_: E) -> String {
    "derivation_signature_invalid".to_string()
}

fn parse_pubkey(value: &str) -> Result<Pubkey, String> {
    Pubkey::from_str(value).map_err(|_| "pubkey_invalid".to_string())
}

/// Errors reach Dart as text: the error and its causes, so a failed request
/// says why it failed (DNS, connection, TLS). Client and transaction errors
/// describe what failed without key material; keep it that way when adding
/// variants. Request errors name their URL, so `api-key` values are masked.
pub(crate) fn error(error: impl Into<ClientError>) -> String {
    let error = error.into();
    let mut message = error.to_string();
    let mut source = std::error::Error::source(&error);
    while let Some(cause) = source {
        let cause_message = cause.to_string();
        if !message.contains(&cause_message) {
            message.push_str(": ");
            message.push_str(&cause_message);
        }
        source = cause.source();
    }
    redact_api_keys(&message)
}

fn redact_api_keys(message: &str) -> String {
    const PARAMETER: &str = "api-key=";
    let mut redacted = String::with_capacity(message.len());
    let mut rest = message;
    while let Some(start) = rest.find(PARAMETER) {
        let (head, tail) = rest.split_at(start + PARAMETER.len());
        redacted.push_str(head);
        redacted.push_str("redacted");
        let end = tail
            .find(|c: char| c == '&' || c == ')' || c == '"' || c.is_whitespace())
            .unwrap_or(tail.len());
        rest = &tail[end..];
    }
    redacted.push_str(rest);
    redacted
}

#[cfg(test)]
mod tests {
    use solana_keypair::Keypair;
    use solana_signer::Signer;
    use zolana_client::{compile_message, ComputeBudgetConfig};

    use super::*;
    use crate::transport::tests::unreachable;

    fn config() -> WalletConfig {
        WalletConfig {
            rpc_url: "https://rpc.example".to_string(),
            indexer_url: "https://indexer.example/v1/zolana".to_string(),
            proving_key_dir: std::env::temp_dir().display().to_string(),
            proving_key_url: None,
            proving: None,
            mints: Vec::new(),
        }
    }

    fn open(signer: &Keypair) -> Result<MobileWallet, String> {
        open_with(signer, config())
    }

    /// A wallet whose transport reaches nothing: a request fails as a
    /// refused connection does.
    fn open_with(signer: &Keypair, config: WalletConfig) -> Result<MobileWallet, String> {
        let pubkey = signer.pubkey().to_string();
        let message = derivation_message(pubkey.clone())?;
        let signature = signer.sign_message(&message);
        MobileWallet::open(config, pubkey, signature.as_ref().to_vec(), unreachable())
    }

    #[test]
    fn opens_only_from_a_signature_over_the_derivation_message() {
        let signer = Keypair::new();
        let wallet = open(&signer).expect("derivation signature opens the wallet");
        assert_eq!(wallet.owner, signer.pubkey());
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
                MobileWallet::open(config(), pubkey.clone(), signature, unreachable())
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
                priority_fee_lamports: None,
            },
        )
        .unwrap();
        PendingTransaction {
            id: 0,
            kind: PendingTransactionKind::Deposit,
            message,
            summary: String::new(),
            last_valid_block_height: 0,
            spends: Vec::new(),
        }
    }

    /// A SOL note of `amount` whose nullifier is `[nullifier; 32]`.
    fn note(amount: u64, nullifier: u8) -> WalletUtxo {
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
            data_hash: None,
            ring_data_hash: None,
            tree_id: 0,
            leaf_index: 0,
            slot: 0,
            tx_signature: Signature::default(),
            slot_index: 0,
        }
    }

    #[test]
    fn spends_leave_out_the_notes_prepared_spends_reserve() {
        let utxos = vec![note(50, 1), note(30, 2), note(20, 3)];
        let notes = SpendableDecryptionResult {
            balances: zolana_transaction::Balances {
                assets: vec![zolana_transaction::AssetBalance {
                    asset_id: zolana_transaction::SOL_ASSET_ID,
                    mint: zolana_transaction::SOL_MINT,
                    amount: 100,
                    utxos,
                }],
            },
            ..Default::default()
        };
        let select = |amount, reserved: &[u8]| {
            let reserved = reserved.iter().map(|&n| [n; 32]).collect();
            select_notes(&notes, Asset::SOL, amount, &reserved).map(|picked| nullifiers(&picked))
        };
        assert_eq!(select(40, &[]), Ok(vec![[1; 32]]));
        // The largest note is reserved: the others cover it.
        assert_eq!(select(40, &[1]), Ok(vec![[2; 32], [3; 32]]));
        assert_eq!(select(60, &[1]).unwrap_err(), "notes_reserved");
        assert_eq!(
            select(200, &[1]).unwrap_err(),
            "insufficient_private_balance"
        );
    }

    #[test]
    fn submit_checks_every_signature_before_sending() {
        let signer = Keypair::new();
        let wallet = open(&signer).unwrap();
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
        // A valid signature gets as far as the transport.
        let error = wallet
            .submit(&pending(), vec![valid.as_ref().to_vec()])
            .unwrap_err();
        assert!(!error.starts_with("signature_"), "{error}");
    }

    #[test]
    fn confirm_accepts_only_the_fee_payer_signature() {
        let signer = Keypair::new();
        let wallet = open(&signer).unwrap();
        let pending = unsigned(&signer);
        let other = Keypair::new().sign_message(&pending.message_bytes());
        for signature in ["not-a-signature".to_string(), other.to_string()] {
            assert_eq!(
                wallet.confirm(&pending, signature).err().as_deref(),
                Some("signature_invalid")
            );
        }
        // The fee payer's signature gets as far as the transport.
        let signature = signer.sign_message(&pending.message_bytes()).to_string();
        let error = wallet.confirm(&pending, signature).unwrap_err();
        assert_ne!(error, "signature_invalid");
    }

    #[test]
    fn waits_for_a_transaction_by_its_signature() {
        let wallet = open(&Keypair::new()).unwrap();
        assert_eq!(
            wallet
                .wait_for_transaction("not-a-signature".to_string())
                .unwrap_err(),
            "signature_invalid"
        );
        // A real signature gets as far as the transport.
        let error = wallet
            .wait_for_transaction(Signature::default().to_string())
            .unwrap_err();
        assert_ne!(error, "signature_invalid");
    }

    #[test]
    fn errors_mask_api_keys_in_urls() {
        let config = WalletConfig {
            rpc_url: "https://rpc.example/?api-key=secret".to_string(),
            indexer_url: "https://indexer.example/v1/zolana?api-key=secret".to_string(),
            ..config()
        };
        let mut wallet = open_with(&Keypair::new(), config).unwrap();
        for error in [
            wallet.public_balance(None).unwrap_err(),
            wallet.private_balance(None).unwrap_err(),
        ] {
            assert!(error.contains("api-key=redacted"), "{error}");
            assert!(!error.contains("secret"), "{error}");
        }
        assert_eq!(
            redact_api_keys("url (https://h/?a=1&api-key=k&b=2) and api-key=k"),
            "url (https://h/?a=1&api-key=redacted&b=2) and api-key=redacted"
        );
    }

    #[test]
    fn reopens_from_exported_keys_and_refuses_others() {
        let signer = Keypair::new();
        let wallet = open(&signer).unwrap();
        let reopen = |solana_pubkey: String, keys| {
            MobileWallet::open_with_keys(config(), solana_pubkey, keys, unreachable())
        };
        let keys = wallet.export_keys();
        let lengths = [
            keys.viewing_private_key.len(),
            keys.viewing_public_key.len(),
            keys.nullifier_private_key.len(),
            keys.nullifier_public_key.len(),
        ];
        assert_eq!(lengths, [32, 33, 31, 32]);
        let reopened = reopen(signer.pubkey().to_string(), keys).unwrap();
        assert_eq!(reopened.shielded_address(), wallet.shielded_address());
        assert_eq!(
            reopened.export_keys().viewing_private_key,
            wallet.export_keys().viewing_private_key
        );

        // Another account's keys open, under another address: the registry
        // check reports them as a conflict.
        let other = Keypair::new();
        let foreign = reopen(other.pubkey().to_string(), wallet.export_keys()).unwrap();
        assert_ne!(foreign.address, open(&other).unwrap().address);

        fn field(keys: &mut WalletKeys, index: usize) -> &mut Vec<u8> {
            match index {
                0 => &mut keys.viewing_private_key,
                1 => &mut keys.viewing_public_key,
                2 => &mut keys.nullifier_private_key,
                _ => &mut keys.nullifier_public_key,
            }
        }
        let mut broken = Vec::new();
        for index in 0..4 {
            let mut flipped = wallet.export_keys();
            field(&mut flipped, index)[0] ^= 1;
            broken.push(flipped);
            let mut short = wallet.export_keys();
            field(&mut short, index).pop();
            broken.push(short);
        }
        let mut zero = wallet.export_keys();
        zero.viewing_private_key = vec![0; 32];
        broken.push(zero);
        for keys in broken {
            assert_eq!(
                reopen(signer.pubkey().to_string(), keys).err().as_deref(),
                Some("wallet_keys_invalid")
            );
        }
    }

    #[test]
    fn a_record_with_other_keys_is_a_conflict() {
        let identity = open(&Keypair::new()).unwrap().address;
        let other = open(&Keypair::new()).unwrap().address;
        let status = |published: Option<Result<ShieldedAddress, ()>>| {
            registration_status_of(published, &identity)
        };
        assert_eq!(status(None), RegistrationStatus::NotRegistered);
        assert_eq!(status(Some(Ok(identity))), RegistrationStatus::Registered);
        assert_eq!(status(Some(Ok(other))), RegistrationStatus::Conflict);
        assert_eq!(status(Some(Err(()))), RegistrationStatus::Conflict);
    }

    #[test]
    fn each_spend_is_proved_where_it_asks_or_the_config_says() {
        let signer = Keypair::new();
        let recipient = Keypair::new().pubkey().to_string();
        let missing = |wallet: &mut MobileWallet, proving| {
            [
                wallet.prepare_transfer(recipient.clone(), None, 1, None, proving),
                wallet.prepare_withdrawal(recipient.clone(), None, 1, None, proving),
            ]
            .map(|result| result.err().as_deref() == Some("remote_prover_missing"))
        };

        let mut local = open(&signer).unwrap();
        assert_eq!(missing(&mut local, Some(Proving::Remote)), [true; 2]);
        // The others get as far as the transport.
        assert_eq!(missing(&mut local, None), [false; 2]);
        let config = WalletConfig {
            proving: Some(Proving::Remote),
            ..config()
        };
        let mut remote = open_with(&signer, config).unwrap();
        assert_eq!(missing(&mut remote, None), [true; 2]);
        assert_eq!(missing(&mut remote, Some(Proving::Local)), [false; 2]);
        remote.set_remote_prover(|_| Box::pin(async { None }));
        assert_eq!(missing(&mut remote, None), [false; 2]);
    }

    #[test]
    fn rejects_malformed_keys_before_the_network() {
        let mut wallet = open(&Keypair::new()).unwrap();
        let recipient = Keypair::new().pubkey().to_string();
        let bad = "not-a-pubkey".to_string();
        let error = |result: Result<PendingTransaction, String>| result.err();
        assert_eq!(
            error(wallet.prepare_transfer(bad.clone(), None, 1, None, None)).as_deref(),
            Some("pubkey_invalid")
        );
        assert_eq!(
            error(wallet.prepare_transfer(recipient.clone(), None, 1, Some(bad.clone()), None))
                .as_deref(),
            Some("pubkey_invalid")
        );
        assert_eq!(
            error(wallet.prepare_withdrawal(recipient, None, 1, Some(bad), None)).as_deref(),
            Some("pubkey_invalid")
        );
    }
}
