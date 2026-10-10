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
//! for a spend that asks for [`Proving::Remote`], by the prover at
//! [`WalletConfig::prover_url`], which the Zolana SDK's prover client asks
//! through the application's transport. The client verifies a remote proof
//! against the pinned verifying key before the wallet builds the message.

use std::{
    collections::{HashMap, HashSet},
    str::FromStr,
    sync::{Arc, Mutex},
    thread::sleep,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

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
        resolved_address_from_record, set_merging_enabled_instruction,
        try_resolve_registered_address,
    },
    ClientError, ComputeBudgetConfig, IndexerPollConfig, MergeSubmission, ProverClient, Rpc,
    SignedPrivateTransaction, SolanaRpc, SpendableUtxos, Submission, ZolanaClient,
};
use zolana_interface::pda;
use zolana_keypair::{derivation, Curve, NullifierKey, PublicKey, ShieldedAddress, ViewingKey};
use zolana_program::instruction::{
    AssetDeposit, Deposit, DepositAsset, DepositSplAccounts, TransactInterfaceTransferAccounts,
};
use zolana_transaction::{
    instructions::{
        merge::{MergeTransaction, MAX_MERGE_INPUTS, MERGE_DEFAULT_INPUT_COUNT},
        transact::ConfidentialTransaction,
    },
    select_merge, select_spend_excluding, LocalShieldedKeys, SpendableDecryptionResult,
    TransactionError, WalletUtxo,
};

use crate::{
    activity::{self, ActivityEntry},
    asset::{mint_name, Asset, Assets, MintConfig},
    error::{rejected_by_chain, WalletError},
    keys::KeyStore,
    prover::{NativeProver, Proving},
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
    /// when `None`. [`Proving::Remote`] needs [`Self::prover_url`].
    pub proving: Option<Proving>,
    /// The prover [`Proving::Remote`] asks, through the transport: the
    /// application's backend as a proxy of the prover's `/prove/<key>` routes,
    /// or a Zolana prover. It receives each spend's witness, the wallet's
    /// nullifier secret included.
    pub prover_url: Option<String>,
    /// The SPL mints the wallet holds, each with its token program. A call
    /// that names another mint fails with [`WalletError::MintNotConfigured`].
    /// [`MobileWallet::balances`] reports SOL and these. Notes in other mints
    /// are left out, as the Zolana SDK leaves out assets its registry does
    /// not hold.
    pub mints: Vec<MintConfig>,
}

/// The Zolana derivation message the wallet's signer signs once to open it.
pub fn derivation_message(solana_pubkey: String) -> Result<Vec<u8>, WalletError> {
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
    /// Turns merges of this account's notes on or off. Public: the account
    /// and the setting.
    Merging,
    /// Combines notes of one asset into one, for this account. Public: the
    /// account and roughly how many notes; not the asset or any amount.
    Merge,
}

/// A built, and where needed proved, v1 transaction awaiting signatures. The
/// application shows what it does, from [`Self::kind`], [`Self::amount`],
/// [`Self::mint`], [`Self::recipient`] and [`Self::fee_payer`], before it asks
/// for them.
pub struct PendingTransaction {
    /// Unique per wallet: the key of the notes a spend reserves.
    id: u64,
    kind: PendingTransactionKind,
    message: VersionedMessage,
    moved: Moved,
    last_valid_block_height: u64,
    /// Nullifiers of the notes it spends; empty unless it is a spend.
    spends: Vec<[u8; 32]>,
}

/// What a transaction moves and to whom; nothing for a registration.
#[derive(Clone, Default)]
struct Moved {
    amount: Option<u64>,
    /// `None` for SOL.
    mint: Option<String>,
    recipient: Option<String>,
}

impl PendingTransaction {
    pub fn kind(&self) -> PendingTransactionKind {
        self.kind
    }

    /// Base units a deposit, transfer or withdrawal moves, or the sum a merge
    /// combines.
    pub fn amount(&self) -> Option<u64> {
        self.moved.amount
    }

    /// The mint of the asset moved, or of the token account created; `None`
    /// for SOL.
    pub fn mint(&self) -> Option<String> {
        self.moved.mint.clone()
    }

    /// The account a transfer or withdrawal pays, or whose token account is
    /// created.
    pub fn recipient(&self) -> Option<String> {
        self.moved.recipient.clone()
    }

    /// The base58 account that pays the network fee: the first signer.
    pub fn fee_payer(&self) -> String {
        self.message.static_account_keys()[0].to_string()
    }

    /// The last block height at which the message can still land. Past it,
    /// [`MobileWallet::refresh`] gives it a new blockhash.
    pub fn last_valid_block_height(&self) -> u64 {
        self.last_valid_block_height
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
    /// SOL and the configured mints.
    assets: Assets,
    reservations: Mutex<Reservations>,
    /// Where spends are proved unless a call says otherwise.
    proving: Proving,
    native_prover: NativeProver,
    /// The SDK's prover client of [`WalletConfig::prover_url`].
    remote_prover: Option<Arc<ProverClient>>,
    /// Proves nothing itself: each spend names its prover.
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
    ) -> Result<MobileWallet, WalletError> {
        let owner = parse_pubkey(&solana_pubkey)?;
        let seed: [u8; derivation::ED25519_SEED_LEN] =
            derivation_signature
                .as_slice()
                .try_into()
                .map_err(|_| WalletError::InvalidDerivationSignature)?;
        let signing_pubkey = PublicKey::from_ed25519(&owner.to_bytes());
        let message = derivation::ed25519_derivation_message(&owner.to_bytes());
        if !signing_pubkey.verify_message(&message, &seed) {
            return Err(WalletError::InvalidDerivationSignature);
        }
        let (nullifier_key, viewing_key) = derivation::expand_roles(&seed, Curve::Ed25519)
            .map_err(|_| WalletError::InvalidDerivationSignature)?;
        Self::from_keys(config, owner, nullifier_key, viewing_key, transport)
    }

    /// Open the wallet of `solana_pubkey` from keys [`Self::export_keys`]
    /// returned. Fails with [`WalletError::InvalidWalletKeys`] unless each
    /// private key yields its public key. `transport` works as in
    /// [`Self::open`].
    pub fn open_with_keys(
        config: WalletConfig,
        solana_pubkey: String,
        keys: WalletKeys,
        transport: Transport,
    ) -> Result<MobileWallet, WalletError> {
        let owner = parse_pubkey(&solana_pubkey)?;
        let viewing_secret = Zeroizing::new(keys.viewing_private_key);
        let nullifier_secret = Zeroizing::new(keys.nullifier_private_key);
        let viewing_key =
            ViewingKey::from_bytes(viewing_secret.as_slice().try_into().map_err(invalid_keys)?)
                .map_err(invalid_keys)?;
        let nullifier_key = NullifierKey::from_secret(
            nullifier_secret
                .as_slice()
                .try_into()
                .map_err(invalid_keys)?,
        );
        if viewing_key.pubkey().as_bytes().as_slice() != keys.viewing_public_key
            || nullifier_key.pubkey().map_err(invalid_keys)?.as_slice() != keys.nullifier_public_key
        {
            return Err(WalletError::InvalidWalletKeys);
        }
        Self::from_keys(config, owner, nullifier_key, viewing_key, transport)
    }

    fn from_keys(
        config: WalletConfig,
        owner: Pubkey,
        nullifier_key: NullifierKey,
        viewing_key: ViewingKey,
        transport: Transport,
    ) -> Result<MobileWallet, WalletError> {
        let address = ShieldedAddress {
            signing_pubkey: PublicKey::from_ed25519(&owner.to_bytes()),
            nullifier_pubkey: nullifier_key.pubkey().map_err(invalid_keys)?,
            viewing_pubkey: viewing_key.pubkey(),
        };
        let keys =
            LocalShieldedKeys::new(address, vec![viewing_key.clone()], nullifier_key.clone())?;
        let assets = Assets::new(config.mints)?;
        let proving_keys = KeyStore::new(
            config.proving_key_dir,
            config.proving_key_url,
            transport.clone(),
        );
        let native_prover = NativeProver::new(proving_keys);
        // With its own prover, the client fetches every Merkle proof from the
        // indexer itself: the wallet's provers prove only complete witnesses.
        let client = ZolanaClient::with_prover(
            transport.solana_rpc(config.rpc_url)?,
            transport.indexer(&config.indexer_url),
            native_prover.clone(),
        );
        Ok(MobileWallet {
            owner,
            address,
            keys,
            nullifier_key,
            viewing_key,
            assets,
            reservations: Mutex::default(),
            proving: config.proving.unwrap_or(Proving::Local),
            native_prover,
            remote_prover: config.prover_url.map(|url| Arc::new(transport.prover(url))),
            client,
        })
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
    pub fn registration_status(&self) -> Result<RegistrationStatus, WalletError> {
        let record = fetch_user_record_optional_checked(&self.client, self.owner)?;
        let published = record.map(|record| {
            resolved_address_from_record(self.owner, &record).map(|resolved| resolved.address)
        });
        Ok(registration_status_of(published, &self.address))
    }

    /// `None` when the registry already holds this wallet's address. Fails
    /// with [`WalletError::RegistrationConflict`] when it holds other keys:
    /// this wallet never replaces them.
    pub fn prepare_registration(&self) -> Result<Option<PendingTransaction>, WalletError> {
        let message = build_registration_transaction_sync(
            &self.client,
            self.owner,
            &self.address,
            None,
            None,
        )?;
        message
            .map(|message| {
                self.pending(
                    PendingTransactionKind::Registration,
                    message,
                    Moved::default(),
                )
            })
            .transpose()
    }

    /// Whether this account's user record lets merges of its notes run.
    /// `false` before registration.
    pub fn merging_enabled(&self) -> Result<bool, WalletError> {
        Ok(
            fetch_user_record_optional_checked(&self.client, self.owner)?
                .is_some_and(|record| record.merging_enabled),
        )
    }

    /// Turn merges of this account's notes on or off; this account signs.
    /// `None` when the record already says so. While merging is on, a merge
    /// proved with this wallet's nullifier secret is valid without this
    /// account's signature: whoever holds the secret and the notes, such as a
    /// prover that proved its spends, can merge them. It cannot move funds.
    pub fn prepare_merging(
        &self,
        enabled: bool,
    ) -> Result<Option<PendingTransaction>, WalletError> {
        if self.merging_enabled()? == enabled {
            return Ok(None);
        }
        let instruction = set_merging_enabled_instruction(self.owner, enabled);
        self.pending(
            PendingTransactionKind::Merging,
            self.message(instruction)?,
            Moved::default(),
        )
        .map(Some)
    }

    /// Build and prove a merge of the smallest notes of `mint` (SOL for
    /// `None`) on one tree into one: at most `max_inputs` of them, 24 when
    /// `None`, 54 at most. Notes that prepared spends reserve are left out,
    /// and the merge reserves its own. Fails with
    /// [`WalletError::NothingToMerge`] below two notes, and with
    /// [`WalletError::MergingDisabled`] unless [`Self::prepare_merging`] turned
    /// merging on.
    ///
    /// The merge needs no signature of this account: `fee_payer` pays and
    /// signs alone, this account when `None`. `proving` works as in
    /// [`Self::prepare_transfer`]; a merge is proved with this wallet's
    /// nullifier secret, which a remote prover receives. The proof expires
    /// after ten minutes, so a merge cannot be sent later by whoever holds it;
    /// [`Self::refresh`] does not extend it.
    pub fn prepare_merge(
        &mut self,
        mint: Option<String>,
        max_inputs: Option<u32>,
        fee_payer: Option<String>,
        proving: Option<Proving>,
    ) -> Result<PendingTransaction, WalletError> {
        let payer = self.fee_payer(fee_payer)?;
        let proving = self.proving(proving)?;
        let asset = self.asset(mint)?;
        let reserved = self.reserved()?;
        let max_inputs = max_inputs.map_or(MERGE_DEFAULT_INPUT_COUNT, |max| {
            usize::try_from(max).unwrap_or(MAX_MERGE_INPUTS)
        });
        let inputs = select_merge(self.spendable()?.utxos(), asset.mint, max_inputs, &reserved)?;
        let spends = nullifiers(&inputs);
        let tree_id = inputs[0].tree_id();
        let merge = MergeTransaction::new(inputs)?
            .with_output_tree_id(tree_id)
            .with_expiry(merge_expiry())
            .encrypt(&self.keys)?;
        let amount = merge.output_utxo.amount;
        let mut submission =
            MergeSubmission::new(&merge, self.owner, &self.address, &self.nullifier_key);
        if proving == Proving::Remote {
            let remote = self
                .remote_prover
                .clone()
                .ok_or(WalletError::RemoteProverMissing)?;
            submission = submission.with_prover(remote);
        }
        self.native_prover.take_failure();
        let message = submission
            .finish_unsigned_sync(&self.client, payer)
            .map_err(|failure| {
                self.native_prover
                    .take_failure()
                    .unwrap_or_else(|| failure.into())
            })?;
        let pending = self.pending(
            PendingTransactionKind::Merge,
            message,
            moved(asset, amount, None),
        )?;
        Ok(self.reserve(pending, spends))
    }

    /// Deposit public SOL (`mint` `None`) or tokens from this account into
    /// its own private balance. Deposits carry no proof.
    pub fn prepare_deposit(
        &mut self,
        mint: Option<String>,
        amount: u64,
    ) -> Result<PendingTransaction, WalletError> {
        let asset = self.asset(mint)?;
        let deposit_asset = match asset.token_program {
            None => DepositAsset::Sol,
            Some(token_program) => DepositAsset::Spl(DepositSplAccounts {
                mint: asset.mint,
                user_token: pda::associated_token_address_with_program(
                    &self.owner,
                    &asset.mint,
                    &token_program,
                ),
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
                owner: self.address.owner_hash()?,
                amount,
                memo: None,
            }],
        }
        .instruction()?;
        self.pending(
            PendingTransactionKind::Deposit,
            self.message(deposit)?,
            moved(asset, amount, None),
        )
    }

    /// Spendable private balances, read from the indexer now: one per asset
    /// held in SOL and the configured mints.
    pub fn balances(&mut self) -> Result<Vec<TokenBalance>, WalletError> {
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
    pub fn private_balance(&mut self, mint: Option<String>) -> Result<u64, WalletError> {
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
    /// outputs are all this wallet's own is listed as a withdrawal, one with
    /// another wallet's output as sent.
    pub fn activity(&mut self) -> Result<Vec<ActivityEntry>, WalletError> {
        let assets = self.assets.registry(&self.client)?;
        Ok(activity::fetch(&self.keys, assets, &self.client)?)
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
    /// `None`. Remote proving without [`WalletConfig::prover_url`] fails with
    /// [`WalletError::RemoteProverMissing`]. The client verifies a remote
    /// proof against the pinned verifying key and the public input it computed
    /// itself before the message is built: a response that is not a proof of
    /// the pinned key fails with [`WalletError::ProofMalformed`], a proof that
    /// does not verify with [`WalletError::ProofInvalid`].
    pub fn prepare_transfer(
        &mut self,
        recipient: String,
        mint: Option<String>,
        amount: u64,
        fee_payer: Option<String>,
        proving: Option<Proving>,
    ) -> Result<PendingTransaction, WalletError> {
        let recipient = parse_pubkey(&recipient)?;
        let payer = self.fee_payer(fee_payer)?;
        let proving = self.proving(proving)?;
        let asset = self.asset(mint)?;
        let Some(registered) = try_resolve_registered_address(&self.client, recipient)? else {
            return Err(WalletError::RecipientNotRegistered {
                recipient: recipient.to_string(),
            });
        };
        let inputs = self.select_notes(asset, amount)?;
        let spends = nullifiers(&inputs);
        let mut transaction = ConfidentialTransaction::new(inputs, payer)?;
        match asset.token_program {
            None => transaction.transfer_sol(&registered.address, amount),
            Some(_) => transaction.transfer(&registered.address, asset.mint, amount),
        }?;
        let pending = self.pending(
            PendingTransactionKind::Transfer,
            self.prove(transaction, Vec::new(), payer, proving)?,
            moved(asset, amount, Some(recipient)),
        )?;
        Ok(self.reserve(pending, spends))
    }

    /// Build and prove a withdrawal of private funds to the public account
    /// `recipient`. Tokens go to its associated token account. Without one
    /// the withdrawal could not settle, so it fails with
    /// `recipient_token_account_missing` before proving; the application
    /// creates the account with its own Solana client.
    ///
    /// `fee_payer` and `proving` work as in [`Self::prepare_transfer`].
    pub fn prepare_withdrawal(
        &mut self,
        recipient: String,
        mint: Option<String>,
        amount: u64,
        fee_payer: Option<String>,
        proving: Option<Proving>,
    ) -> Result<PendingTransaction, WalletError> {
        let recipient = parse_pubkey(&recipient)?;
        let payer = self.fee_payer(fee_payer)?;
        let proving = self.proving(proving)?;
        let asset = self.asset(mint)?;
        if let Some(token_account) = asset.token_account(&recipient) {
            if self.client.get_account(token_account)?.is_none() {
                return Err(WalletError::RecipientTokenAccountMissing {
                    recipient: recipient.to_string(),
                    mint: asset.mint.to_string(),
                });
            }
        }
        let inputs = self.select_notes(asset, amount)?;
        let spends = nullifiers(&inputs);
        let mut transaction = ConfidentialTransaction::new(inputs, payer)?;
        let settlement =
            transaction.withdraw_to(asset.mint, amount, recipient, asset.token_program)?;
        let pending = self.pending(
            PendingTransactionKind::Withdrawal,
            self.prove(transaction, vec![settlement], payer, proving)?,
            moved(asset, amount, Some(recipient)),
        )?;
        Ok(self.reserve(pending, spends))
    }

    fn fee_payer(&self, fee_payer: Option<String>) -> Result<Pubkey, WalletError> {
        fee_payer.as_deref().map_or(Ok(self.owner), parse_pubkey)
    }

    /// Where a spend is proved: where `proving` says, or where the config
    /// does. Remote proving without a prover fails here, before the network.
    fn proving(&self, proving: Option<Proving>) -> Result<Proving, WalletError> {
        let proving = proving.unwrap_or(self.proving);
        match (proving, &self.remote_prover) {
            (Proving::Remote, None) => Err(WalletError::RemoteProverMissing),
            _ => Ok(proving),
        }
    }

    /// A message with `instruction` alone, paid by this account. Its blockhash
    /// is set by [`Self::pending`].
    fn message(&self, instruction: Instruction) -> Result<VersionedMessage, WalletError> {
        Ok(compile_message(
            &self.owner,
            &[instruction],
            Default::default(),
            ComputeBudgetConfig::for_instruction_count(1),
        )?)
    }

    /// `message` with the latest blockhash, set last so that it is as young
    /// as it can be when the application gets it.
    fn pending(
        &self,
        kind: PendingTransactionKind,
        mut message: VersionedMessage,
        moved: Moved,
    ) -> Result<PendingTransaction, WalletError> {
        let (blockhash, last_valid_block_height) = self.client.get_latest_blockhash()?;
        message.set_recent_blockhash(blockhash);
        let mut reservations = self.reservations();
        let id = reservations.next_id;
        reservations.next_id += 1;
        Ok(PendingTransaction {
            id,
            kind,
            message,
            moved,
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
    fn select_notes(&mut self, asset: Asset, amount: u64) -> Result<Vec<WalletUtxo>, WalletError> {
        let spendable = self.spendable()?;
        let reserved = self.reserved()?;
        select_spend_excluding(spendable.utxos(), asset.mint, amount, &reserved)
            .map_err(|failure| spend_failure(amount, failure))
    }

    /// The nullifiers of the notes prepared spends and merges reserve.
    /// Reservations past their last valid block height are dropped: those
    /// transactions can no longer land.
    fn reserved(&self) -> Result<HashSet<[u8; 32]>, WalletError> {
        if self.reservations().reserved.is_empty() {
            return Ok(HashSet::new());
        }
        let height = self.client.get_block_height()?;
        let mut reservations = self.reservations();
        reservations
            .reserved
            .retain(|_, (_, last_valid_block_height)| *last_valid_block_height >= height);
        Ok(reservations
            .reserved
            .values()
            .flat_map(|(spends, _)| spends.iter().copied())
            .collect())
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
    pub fn refresh(&self, pending: &PendingTransaction) -> Result<PendingTransaction, WalletError> {
        let refreshed =
            self.pending(pending.kind, pending.message.clone(), pending.moved.clone())?;
        self.release(pending);
        Ok(if pending.spends.is_empty() {
            refreshed
        } else {
            self.reserve(refreshed, pending.spends.clone())
        })
    }

    /// SOL for `None`, otherwise a configured mint.
    fn asset(&mut self, mint: Option<String>) -> Result<Asset, WalletError> {
        self.assets.resolve(&self.client, mint.as_deref())
    }

    /// The wallet's spendable notes as the indexer has them now, in SOL and
    /// every configured mint.
    fn spendable(&mut self) -> Result<SpendableDecryptionResult, WalletError> {
        let assets = self.assets.registry(&self.client)?;
        Ok(SpendableUtxos::new(&self.keys, assets).fetch(&self.client)?)
    }

    /// Encrypt and prove `transaction` where `proving` says, and build the
    /// Solana message `payer` pays for.
    fn prove(
        &self,
        transaction: ConfidentialTransaction,
        settlement_transfers: Vec<TransactInterfaceTransferAccounts>,
        payer: Pubkey,
        proving: Proving,
    ) -> Result<VersionedMessage, WalletError> {
        let signed = SignedPrivateTransaction {
            transaction: transaction.encrypt(&self.keys)?,
            settlement_transfers,
        };
        let mut submission = Submission::new(&signed, payer, &self.nullifier_key);
        if proving == Proving::Remote {
            let remote = self
                .remote_prover
                .clone()
                .ok_or(WalletError::RemoteProverMissing)?;
            submission = submission.with_prover(remote);
        }
        // A device failure the client reports as a message; the wallet reports
        // what happened.
        self.native_prover.take_failure();
        submission
            .finish_unsigned_sync(&self.client)
            .map_err(|failure| {
                self.native_prover
                    .take_failure()
                    .unwrap_or_else(|| failure.into())
            })
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
    ) -> Result<String, WalletError> {
        let message_bytes = pending.message_bytes();
        let required = usize::from(pending.message.header().num_required_signatures);
        if signatures.len() != required {
            return Err(WalletError::SignatureCountMismatch {
                expected: required as u64,
                got: signatures.len() as u64,
            });
        }
        let mut transaction_signatures = Vec::with_capacity(required);
        for (signer, signature) in pending.message.static_account_keys()[..required]
            .iter()
            .zip(&signatures)
        {
            let signature: [u8; 64] = signature
                .as_slice()
                .try_into()
                .map_err(|_| WalletError::SignatureInvalid)?;
            if !signed_by(signer, &message_bytes, &signature) {
                return Err(WalletError::SignatureInvalid);
            }
            transaction_signatures.push(Signature::from(signature));
        }
        let signature = transaction_signatures[0];
        let transaction = VersionedTransaction {
            signatures: transaction_signatures,
            message: pending.message.clone(),
        };
        if let Err(failure) = self.client.process_transaction(transaction) {
            // A send can fail after the transaction went out, such as when
            // the RPC loses the confirmation: look for it before failing.
            if rejected_by_chain(&failure) || !self.landed(signature, pending)? {
                return Err(failure.into());
            }
        }
        self.settle(pending, signature)?;
        Ok(signature.to_string())
    }

    /// Whether `signature`, `pending` sent, landed: polled until it shows or
    /// its blockhash expires. A transaction the chain ran and refused fails
    /// with the chain's error.
    fn landed(
        &self,
        signature: Signature,
        pending: &PendingTransaction,
    ) -> Result<bool, WalletError> {
        let poll = IndexerPollConfig::default();
        for delay in std::iter::once(Default::default()).chain(poll.backoff()) {
            sleep(delay);
            let status = self
                .client
                .get_signature_statuses(vec![signature])?
                .into_iter()
                .next()
                .flatten();
            if let Some(status) = status {
                return match status.err {
                    None => Ok(true),
                    Some(failure) => Err(ClientError::SolanaRpcTransaction {
                        operation: "process_transaction",
                        source: failure.into(),
                    }
                    .into()),
                };
            }
            if self.client.get_block_height()? > pending.last_valid_block_height {
                return Ok(false);
            }
        }
        Ok(false)
    }

    /// Wait for a transaction the application sent itself: until Solana
    /// confirms it and, for shielded-pool transactions, the indexer has it,
    /// so the next balance or spend reads the notes it created and not the
    /// ones it spent.
    ///
    /// `signature` is the transaction signature, the fee payer's; it is
    /// checked against `pending` before anything is asked.
    pub fn confirm(
        &self,
        pending: &PendingTransaction,
        signature: String,
    ) -> Result<(), WalletError> {
        let signature =
            Signature::from_str(&signature).map_err(|_| WalletError::SignatureInvalid)?;
        let fee_payer = &pending.message.static_account_keys()[0];
        if !signed_by(fee_payer, &pending.message_bytes(), signature.as_array()) {
            return Err(WalletError::SignatureInvalid);
        }
        self.settle(pending, signature)
    }

    /// Wait for a shielded-pool transaction by its signature alone, such as one
    /// sent before the application restarted: until Solana confirms it and the
    /// indexer has it, so the next balance reads its notes. A transaction that
    /// failed on chain fails here with the chain's error.
    pub fn wait_for_transaction(&self, signature: String) -> Result<(), WalletError> {
        let signature =
            Signature::from_str(&signature).map_err(|_| WalletError::SignatureInvalid)?;
        Ok(self.client.confirm_private_transaction_sync(signature)?)
    }

    /// Once it settles, a spend's notes are spent and no longer reserved.
    fn settle(
        &self,
        pending: &PendingTransaction,
        signature: Signature,
    ) -> Result<(), WalletError> {
        if matches!(
            pending.kind,
            PendingTransactionKind::Registration | PendingTransactionKind::Merging
        ) {
            return wait_for_confirmation(&self.client, signature);
        }
        self.client.confirm_private_transaction_sync(signature)?;
        self.release(pending);
        Ok(())
    }
}

/// A spend of `amount` the free notes cannot cover. The SDK reports an asset
/// without spendable notes by its mint alone; the wallet reports the amounts.
fn spend_failure(amount: u64, failure: TransactionError) -> WalletError {
    match failure {
        TransactionError::NoSpendableBalance { .. } => WalletError::InsufficientPrivateBalance {
            requested: amount,
            available: 0,
        },
        failure => failure.into(),
    }
}

/// `amount` of `asset`, paid to `recipient` for a transfer or withdrawal.
fn moved(asset: Asset, amount: u64, recipient: Option<Pubkey>) -> Moved {
    Moved {
        amount: Some(amount),
        mint: mint_name(&asset.mint),
        recipient: recipient.map(|recipient| recipient.to_string()),
    }
}

fn nullifiers(notes: &[WalletUtxo]) -> Vec<[u8; 32]> {
    notes.iter().map(|note| note.nullifier).collect()
}

/// How long a merge proof can be sent. Anyone who holds it can send it, and
/// the program refuses it after this.
const MERGE_LIFETIME: Duration = Duration::from_secs(600);

/// The unix time a merge prepared now expires at.
fn merge_expiry() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .saturating_add(MERGE_LIFETIME)
        .as_secs()
}

/// Poll until Solana confirms `signature`, with the indexer's backoff.
fn wait_for_confirmation(rpc: &impl Rpc, signature: Signature) -> Result<(), WalletError> {
    let poll = IndexerPollConfig::default();
    for delay in std::iter::once(Default::default()).chain(poll.backoff()) {
        sleep(delay);
        if rpc.confirm_transaction(signature)? {
            return Ok(());
        }
    }
    Err(WalletError::TransactionNotConfirmed {
        signature: signature.to_string(),
    })
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
fn invalid_keys<E>(_: E) -> WalletError {
    WalletError::InvalidWalletKeys
}

fn parse_pubkey(value: &str) -> Result<Pubkey, WalletError> {
    Pubkey::from_str(value).map_err(|_| WalletError::InvalidPubkey {
        value: value.to_string(),
    })
}

#[cfg(test)]
mod tests {
    use solana_keypair::Keypair;
    use solana_signer::Signer;
    use zolana_client::{compile_message, ClientError, ComputeBudgetConfig};

    use super::*;
    use crate::transport::tests::unreachable;

    fn config() -> WalletConfig {
        WalletConfig {
            rpc_url: "https://rpc.example".to_string(),
            indexer_url: "https://indexer.example/v1/zolana".to_string(),
            proving_key_dir: std::env::temp_dir().display().to_string(),
            proving_key_url: None,
            proving: None,
            prover_url: None,
            mints: Vec::new(),
        }
    }

    fn open(signer: &Keypair) -> Result<MobileWallet, WalletError> {
        open_with(signer, config())
    }

    /// A wallet whose transport reaches nothing: a request fails as a
    /// refused connection does.
    fn open_with(signer: &Keypair, config: WalletConfig) -> Result<MobileWallet, WalletError> {
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
                MobileWallet::open(config(), pubkey.clone(), signature, unreachable()).err(),
                Some(WalletError::InvalidDerivationSignature)
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
            moved: Moved::default(),
            last_valid_block_height: 0,
            spends: Vec::new(),
        }
    }

    /// A wallet whose Solana RPC answers each JSON-RPC method with
    /// `answer(method)`: `{"result": ..}` or `{"error": ..}`. Returns the
    /// methods it was asked, in order.
    fn rpc_wallet(
        signer: &Keypair,
        answer: impl Fn(&str) -> serde_json::Value + Send + Sync + 'static,
    ) -> (MobileWallet, Arc<Mutex<Vec<String>>>) {
        let asked = Arc::new(Mutex::new(Vec::new()));
        let log = Arc::clone(&asked);
        let (transport, _) = crate::transport::tests::fake(move |request| {
            let body: serde_json::Value = serde_json::from_slice(&request.body).unwrap();
            let method = body["method"].as_str().unwrap_or_default().to_string();
            let mut reply = answer(&method);
            reply["jsonrpc"] = "2.0".into();
            reply["id"] = body["id"].clone();
            log.lock().unwrap().push(method);
            crate::transport::tests::ok(&reply.to_string())
        });
        let pubkey = signer.pubkey().to_string();
        let signature = signer.sign_message(&derivation_message(pubkey.clone()).unwrap());
        let wallet =
            MobileWallet::open(config(), pubkey, signature.as_ref().to_vec(), transport).unwrap();
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

    /// A send whose confirmation the RPC lost, for a transaction that landed
    /// (or did not): the wallet looks for it before it reports a failure.
    #[test]
    fn a_send_error_is_checked_against_the_signature_status() {
        let signer = Keypair::new();
        let internal =
            || serde_json::json!({"error": {"code": -32603, "message": "Internal error"}});
        let registration = |signer: &Keypair| PendingTransaction {
            kind: PendingTransactionKind::Registration,
            ..unsigned(signer)
        };
        let signed = |pending: &PendingTransaction| {
            vec![signer
                .sign_message(&pending.message_bytes())
                .as_ref()
                .to_vec()]
        };

        let (wallet, asked) = rpc_wallet(&signer, move |method| match method {
            "sendTransaction" => internal(),
            _ => status(true),
        });
        let pending = registration(&signer);
        let signature = wallet
            .submit(&pending, signed(&pending))
            .expect("it landed");
        assert_eq!(
            signature,
            signer.sign_message(&pending.message_bytes()).to_string()
        );
        assert!(asked
            .lock()
            .unwrap()
            .contains(&"getSignatureStatuses".to_string()));

        let (wallet, _) = rpc_wallet(&signer, move |method| match method {
            "sendTransaction" => internal(),
            "getBlockHeight" => serde_json::json!({"result": 10}),
            _ => status(false),
        });
        let error = wallet.submit(&pending, signed(&pending)).unwrap_err();
        assert!(
            matches!(&error, WalletError::Client { message } if message.contains("Internal error")),
            "{error:?}"
        );
    }

    /// A spend of a note the chain already spent fails as such, at once: the
    /// node refused it, so it cannot have landed.
    #[test]
    fn a_spent_note_is_notes_already_spent() {
        let signer = Keypair::new();
        let (wallet, asked) = rpc_wallet(&signer, |method| match method {
            "sendTransaction" => serde_json::json!({"error": {
                "code": -32002,
                "message": "Transaction simulation failed: custom program error: 0x1b83",
                "data": {
                    "err": {"InstructionError": [0, {"Custom": 7043}]},
                    "logs": [], "accounts": null, "unitsConsumed": 0,
                    "returnData": null, "innerInstructions": null
                }
            }}),
            _ => status(true),
        });
        let pending = unsigned(&signer);
        let signatures = vec![signer
            .sign_message(&pending.message_bytes())
            .as_ref()
            .to_vec()];
        assert_eq!(
            wallet.submit(&pending, signatures).unwrap_err(),
            WalletError::NotesAlreadySpent
        );
        assert_eq!(*asked.lock().unwrap(), ["sendTransaction"]);
    }

    /// A SOL note of `amount` on `tree_id` whose nullifier is
    /// `[nullifier; 32]`.
    fn note(amount: u64, nullifier: u8, tree_id: u16) -> WalletUtxo {
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
            tree_id,
            leaf_index: 0,
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

    #[test]
    fn selection_leaves_out_reserved_notes_and_reports_what_it_lacks() {
        let notes = spendable(vec![note(50, 1, 0), note(30, 2, 0), note(20, 3, 0)]);
        let select = |notes: &SpendableDecryptionResult, mint, amount, reserved: &[u8]| {
            let reserved = reserved.iter().map(|&n| [n; 32]).collect();
            select_spend_excluding(notes.utxos(), mint, amount, &reserved)
                .map(|picked| nullifiers(&picked))
                .map_err(|failure| spend_failure(amount, failure))
        };
        let sol = zolana_transaction::SOL_MINT;
        assert_eq!(select(&notes, sol, 40, &[]), Ok(vec![[1; 32]]));
        // The largest note is reserved: the others cover it.
        assert_eq!(select(&notes, sol, 40, &[1]), Ok(vec![[2; 32], [3; 32]]));
        assert_eq!(
            select(&notes, sol, 60, &[1]).unwrap_err(),
            WalletError::NotesReserved { amount: 60 }
        );
        assert_eq!(
            select(&notes, sol, 200, &[1]).unwrap_err(),
            WalletError::InsufficientPrivateBalance {
                requested: 200,
                available: 50
            }
        );
        assert_eq!(
            select(&notes, Pubkey::new_unique(), 1, &[]).unwrap_err(),
            WalletError::InsufficientPrivateBalance {
                requested: 1,
                available: 0
            }
        );
        assert_eq!(
            select(&notes, sol, 0, &[]).unwrap_err(),
            WalletError::AmountZero
        );
        // A spend takes notes from two trees, not from three.
        let two_trees = spendable(vec![note(50, 1, 0), note(30, 2, 1)]);
        assert_eq!(select(&two_trees, sol, 70, &[]), Ok(vec![[1; 32], [2; 32]]));
        let three_trees = spendable(vec![note(30, 1, 0), note(20, 2, 1), note(10, 3, 2)]);
        assert_eq!(
            select(&three_trees, sol, 55, &[]).unwrap_err(),
            WalletError::TooManyInputTrees {
                trees: 3,
                max_trees: 2
            }
        );
        let many = spendable((1..=41).map(|n| note(10, n, 0)).collect());
        assert_eq!(
            select(&many, sol, 410, &[]).unwrap_err(),
            WalletError::MergeRequired {
                amount: 410,
                max_inputs: 40
            }
        );
    }

    #[test]
    fn a_merge_expires_ten_minutes_after_it_is_prepared() {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let expiry = merge_expiry();
        assert!(
            (now + 600..=now + 601).contains(&expiry),
            "{expiry} vs {now}"
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
        let count = |got| WalletError::SignatureCountMismatch { expected: 1, got };
        for (signatures, expected) in [
            (vec![], count(0)),
            (vec![valid.as_ref().to_vec(); 2], count(2)),
            (
                vec![wrong_signer.as_ref().to_vec()],
                WalletError::SignatureInvalid,
            ),
            (
                vec![valid.as_ref()[..63].to_vec()],
                WalletError::SignatureInvalid,
            ),
        ] {
            assert_eq!(wallet.submit(&pending(), signatures).err(), Some(expected));
        }
        // A valid signature gets as far as the transport.
        let error = wallet
            .submit(&pending(), vec![valid.as_ref().to_vec()])
            .unwrap_err();
        assert!(matches!(error, WalletError::Client { .. }), "{error:?}");
    }

    #[test]
    fn confirm_accepts_only_the_fee_payer_signature() {
        let signer = Keypair::new();
        let wallet = open(&signer).unwrap();
        let pending = unsigned(&signer);
        let other = Keypair::new().sign_message(&pending.message_bytes());
        for signature in ["not-a-signature".to_string(), other.to_string()] {
            assert_eq!(
                wallet.confirm(&pending, signature).err(),
                Some(WalletError::SignatureInvalid)
            );
        }
        // The fee payer's signature gets as far as the transport.
        let signature = signer.sign_message(&pending.message_bytes()).to_string();
        let error = wallet.confirm(&pending, signature).unwrap_err();
        assert_ne!(error, WalletError::SignatureInvalid);
    }

    #[test]
    fn waits_for_a_transaction_by_its_signature() {
        let wallet = open(&Keypair::new()).unwrap();
        assert_eq!(
            wallet
                .wait_for_transaction("not-a-signature".to_string())
                .unwrap_err(),
            WalletError::SignatureInvalid
        );
        // A real signature gets as far as the transport.
        let error = wallet
            .wait_for_transaction(Signature::default().to_string())
            .unwrap_err();
        assert_ne!(error, WalletError::SignatureInvalid);
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
            wallet.registration_status().unwrap_err(),
            wallet.private_balance(None).unwrap_err(),
        ] {
            let WalletError::Client { message } = &error else {
                panic!("{error:?}");
            };
            assert!(message.contains("api-key=redacted"), "{message}");
            assert!(!message.contains("secret"), "{message}");
        }
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
                reopen(signer.pubkey().to_string(), keys).err(),
                Some(WalletError::InvalidWalletKeys)
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
        let owner = Pubkey::new_unique();
        assert_eq!(
            WalletError::from(ClientError::UserRegistryKeysMismatch { owner }),
            WalletError::RegistrationConflict {
                owner: owner.to_string()
            }
        );
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
            .map(|result| result.err() == Some(WalletError::RemoteProverMissing))
        };

        let mut local = open(&signer).unwrap();
        assert_eq!(missing(&mut local, Some(Proving::Remote)), [true; 2]);
        // The others get as far as the transport.
        assert_eq!(missing(&mut local, None), [false; 2]);
        let config = |prover_url: Option<&str>| WalletConfig {
            proving: Some(Proving::Remote),
            prover_url: prover_url.map(str::to_string),
            ..config()
        };
        let mut remote = open_with(&signer, config(None)).unwrap();
        assert_eq!(missing(&mut remote, None), [true; 2]);
        assert_eq!(missing(&mut remote, Some(Proving::Local)), [false; 2]);
        let mut remote = open_with(&signer, config(Some("https://prover.example"))).unwrap();
        assert_eq!(missing(&mut remote, None), [false; 2]);
    }

    #[test]
    fn rejects_malformed_keys_before_the_network() {
        let mut wallet = open(&Keypair::new()).unwrap();
        let recipient = Keypair::new().pubkey().to_string();
        let bad = "not-a-pubkey".to_string();
        let invalid = Some(WalletError::InvalidPubkey { value: bad.clone() });
        let error = |result: Result<PendingTransaction, WalletError>| result.err();
        assert_eq!(
            error(wallet.prepare_transfer(bad.clone(), None, 1, None, None)),
            invalid
        );
        assert_eq!(
            error(wallet.prepare_transfer(recipient.clone(), None, 1, Some(bad.clone()), None)),
            invalid
        );
        assert_eq!(
            error(wallet.prepare_withdrawal(recipient, None, 1, Some(bad), None)),
            invalid
        );
    }
}
