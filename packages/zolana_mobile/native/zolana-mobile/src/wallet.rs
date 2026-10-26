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
//! 2. each [`PreparedTransaction::message_bytes`], the v1 Solana message this
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
    cmp::Reverse,
    collections::{HashMap, HashSet},
    str::FromStr,
    sync::{Arc, Mutex},
    thread::{self, sleep},
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use solana_commitment_config::CommitmentConfig;
use solana_instruction::{AccountMeta, Instruction};
use solana_message::{v1, VersionedMessage};
use solana_pubkey::Pubkey;
use solana_signature::Signature;
use solana_transaction::versioned::VersionedTransaction;
use solana_transaction_error::TransactionError as ChainError;
use zeroize::Zeroizing;
use zolana_client::{
    compile_message,
    user_registry::{
        build_registration_transaction_sync, fetch_user_record_optional_checked,
        resolved_address_from_record, set_merge_encryption_instruction,
        try_resolve_registered_address,
    },
    ClientError, ComputeBudgetConfig, IndexerPollConfig, IndexerRpcConfig, MergeSubmission,
    ProverClient, Rpc, SignedPrivateTransaction, SolanaRpc, Submission, UnsignedMessage, UtxoScan,
    UtxoScanResult, ZolanaClient,
};
use zolana_interface::{pda, state::ProtocolConfig, MAX_INPUT_TREES, PROGRAM_ID_PUBKEY};
use zolana_keypair::{derivation, Curve, NullifierKey, PublicKey, ShieldedAddress, ViewingKey};
use zolana_program::instruction::{
    AssetDeposit, CreateAssociatedTokenAccount, CreateSplInterface, Deposit, DepositAsset,
    DepositSplAccounts, TransactInterfaceTransferAccounts,
};
use zolana_transaction::{
    instructions::{
        merge::{MergeTransaction, MAX_MERGE_INPUTS, MERGE_DEFAULT_INPUT_COUNT},
        transact::{ConfidentialTransaction, SppProofInputs, SppProofOutputUtxo, MAX_SPEND_INPUTS},
    },
    select_merge, select_spend_excluding, AssetBalance, LocalShieldedKeys,
    SpendableDecryptionResult, TransactionError, WalletUtxo,
};
use zolana_user_registry_interface::MergeEncryptionScheme;

use crate::{
    activity::{self, ActivityEntry},
    asset::{mint_name, Asset, Assets, MintConfig},
    error::{client_message, settles_the_send, WalletError},
    keys::{KeyStore, ProvingKeys},
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
    /// application's backend as a proxy of the prover's routes, or a Zolana
    /// prover. It receives each spend's witness, the wallet's nullifier
    /// secret included, encrypted to the enclave the Zolana SDK pins once the
    /// prover attests to it; only a loopback prover gets it in plaintext.
    pub prover_url: Option<String>,
    /// The most notes a spend takes on the device, 1 to 40; 40 when `None`.
    /// A spend that needs more fails with [`WalletError::MergeRequired`]
    /// naming this cap, before proving. Proof time and memory grow with the
    /// notes a spend takes: a low cap, with merges that keep the notes few,
    /// keeps spends fast.
    pub max_spend_inputs: Option<u32>,
    /// For how many milliseconds a spend may take its notes from the last
    /// read ([`MobileWallet::balances`], [`MobileWallet::overview`],
    /// [`MobileWallet::private_balance`] or [`MobileWallet::activity`])
    /// instead of reading the indexer again; `None` reads before every
    /// spend. A send or a confirmation ends the reuse. A note spent
    /// elsewhere since the read fails the spend when it is sent, with
    /// [`WalletError::NotesAlreadySpent`], as a note spent right after a
    /// read would.
    pub reuse_notes_ms: Option<u64>,
    /// The SPL mints the wallet holds, each with its token program. A call
    /// that names another mint fails with [`WalletError::MintNotConfigured`],
    /// and [`MobileWallet::balances`] reports SOL and these only; notes in
    /// other mints are left out. `None` holds every mint the shielded pool
    /// registered: the wallet looks up a mint a call names, and the mints of
    /// notes a read meets.
    pub mints: Option<Vec<MintConfig>>,
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

/// What a prepared transaction does, to show the user before signing.
/// Amounts are in base units; `mint` is `None` for SOL.
#[derive(Clone, Debug, PartialEq, Eq)]
#[flutter_rust_bridge::frb(non_opaque)]
pub enum Operation {
    /// Publishes the wallet's shielded address so others can pay it.
    Registration,
    /// Moves public funds into the private balance. Public: sender, asset,
    /// amount. `creates_spl_interface` when it is the mint's first deposit
    /// into the pool: the transaction first creates the mint's registry
    /// entry and vault, and this account pays their rent.
    Deposit {
        mint: Option<String>,
        amount: u64,
        creates_spl_interface: bool,
    },
    /// Spends private funds of one asset to `outputs`, in the order given.
    /// Public: this account, the asset, each public output's recipient and
    /// amount, and the Solana address each private output's note is tagged
    /// with. Hidden: the private amounts and the change.
    ///
    /// `created_token_accounts` are the public token recipients that had no
    /// associated token account for the mint: the transaction creates each,
    /// and the fee payer pays its rent.
    Spend {
        mint: Option<String>,
        outputs: Vec<Output>,
        created_token_accounts: Vec<String>,
    },
    /// Turns merges of this account's notes on or off. Public: the account
    /// and the setting.
    Merging { enabled: bool },
    /// Combines notes of one asset, `amount` in total, into one for this
    /// account. Public: the account and roughly how many notes; not the asset
    /// or any amount.
    Merge { mint: Option<String>, amount: u64 },
    /// A transaction the application composed itself, such as a call to
    /// another program that spends this wallet's notes; `programs` are its
    /// top-level programs. This wallet does not prepare one yet.
    Custom { programs: Vec<String> },
}

/// One output of a spend, in the spend's asset. `recipient` is a base58
/// Solana account, `amount` in base units.
#[derive(Clone, Debug, PartialEq, Eq)]
#[flutter_rust_bridge::frb(non_opaque)]
pub enum Output {
    /// A private note for a registered wallet. The amount is hidden; the
    /// note is tagged with the recipient's Solana address on chain.
    Private { recipient: String, amount: u64 },
    /// Public funds to a Solana account, tokens to its associated token
    /// account, which the transaction creates when it does not exist.
    /// Recipient and amount are public.
    Public { recipient: String, amount: u64 },
}

impl Output {
    fn recipient(&self) -> &str {
        match self {
            Output::Private { recipient, .. } | Output::Public { recipient, .. } => recipient,
        }
    }

    fn amount(&self) -> u64 {
        match self {
            Output::Private { amount, .. } | Output::Public { amount, .. } => *amount,
        }
    }
}

/// One output slot of a prepared spend, in proof order, including padding.
/// Recompute its commitment and compare it with the corresponding output of
/// the shielded-pool instruction in the message before signing. Do not log this.
///
/// A dummy has no owner. A compact slot is left out of the transaction; its
/// commitment is 32 zero bytes. The list position is the proof slot index.
#[derive(Clone, Debug, PartialEq, Eq)]
#[flutter_rust_bridge::frb(non_opaque)]
pub struct OutputOpening {
    /// 34-byte signing key: scheme, 32-byte key, zero padding. Absent for a dummy.
    pub owner_signing_key: Option<Vec<u8>>,
    /// Owner nullifier public key. Absent for a dummy.
    pub nullifier_key: Option<Vec<u8>>,
    /// `None` for SOL.
    pub mint: Option<String>,
    pub amount: u64,
    /// Final 32-byte big-endian output blinding.
    pub blinding: Vec<u8>,
    /// Raw id of the output tree, shared by every slot.
    pub tree_id: u16,
    pub data_hash: Option<Vec<u8>>,
    pub ring_data_hash: Option<Vec<u8>>,
    pub ring_program_id: Option<String>,
    /// Compact padding is not written into the transaction.
    pub compact: bool,
}

impl OutputOpening {
    fn from_output(output: &SppProofOutputUtxo, tree_id: u16) -> Self {
        let (owner_signing_key, nullifier_key) = match &output.owner_address {
            Some(address) => (
                Some(address.signing_pubkey.as_bytes().to_vec()),
                Some(address.nullifier_pubkey.to_vec()),
            ),
            None => (None, None),
        };
        Self {
            owner_signing_key,
            nullifier_key,
            mint: mint_name(&Pubkey::new_from_array(*output.asset.asset.as_array())),
            amount: output.amount,
            blinding: output.blinding.to_vec(),
            tree_id,
            data_hash: output.data_hash.map(|hash| hash.to_vec()),
            ring_data_hash: output.ring_data_hash.map(|hash| hash.to_vec()),
            ring_program_id: output.ring_program_id.map(|id| id.to_string()),
            compact: output.is_compact(),
        }
    }
}

fn output_openings(inputs: &SppProofInputs) -> Vec<OutputOpening> {
    inputs
        .output_utxos
        .iter()
        .map(|output| OutputOpening::from_output(output, inputs.output_tree_id))
        .collect()
}

/// An output whose recipient was checked on chain.
enum Leg {
    Private {
        address: ShieldedAddress,
        amount: u64,
    },
    Public {
        recipient: Pubkey,
        amount: u64,
        /// A token recipient without its associated token account, which the
        /// transaction creates.
        creates_token_account: bool,
    },
}

/// A transaction the wallet built and, where needed, proved, awaiting
/// signatures: each of [`Self::signers`] signs [`Self::message`] with
/// Ed25519, in order. [`Self::operation`] and [`Self::fee_payer`] are what to
/// show the user before signing.
pub struct PreparedTransaction {
    /// Unique per wallet: the key of the notes a spend reserves.
    id: u64,
    operation: Operation,
    message: VersionedMessage,
    last_valid_block_height: u64,
    /// Nullifiers of the notes it spends; empty unless it is a spend.
    spends: Vec<[u8; 32]>,
    /// Output slots of a spend, in proof order. Empty for any other transaction.
    outputs: Vec<OutputOpening>,
}

impl PreparedTransaction {
    /// What it does, with what it moves and to whom.
    #[flutter_rust_bridge::frb(sync, getter)]
    pub fn operation(&self) -> Operation {
        self.operation.clone()
    }

    /// The base58 account that pays the network fee: the first signer.
    #[flutter_rust_bridge::frb(sync, getter)]
    pub fn fee_payer(&self) -> String {
        self.message.static_account_keys()[0].to_string()
    }

    /// The last block height at which the message can still land. Past it,
    /// `refresh` gives it a new blockhash without a new proof.
    #[flutter_rust_bridge::frb(sync, getter)]
    pub fn last_valid_block_height(&self) -> u64 {
        self.last_valid_block_height
    }

    /// The serialized Solana message every signer signs with Ed25519.
    #[flutter_rust_bridge::frb(sync, getter)]
    pub fn message(&self) -> Vec<u8> {
        self.message.serialize()
    }

    /// Base58 public keys that must sign, in signature order. The first is
    /// the fee payer, whose signature is the transaction signature.
    #[flutter_rust_bridge::frb(sync, getter)]
    pub fn signers(&self) -> Vec<String> {
        let required = usize::from(self.message.header().num_required_signatures);
        self.message.static_account_keys()[..required]
            .iter()
            .map(ToString::to_string)
            .collect()
    }

    /// Output slots of a spend, in proof order, including padding. Empty
    /// unless this is a spend. Do not log them.
    #[flutter_rust_bridge::frb(sync, getter)]
    pub fn outputs(&self) -> Vec<OutputOpening> {
        self.outputs.clone()
    }
}

/// The wallet's derived keys, for the application's secure storage. They open
/// the wallet with [`MobileWallet::open_with_keys`] without a derivation
/// signature. They cannot move funds, but they show the wallet's balances and
/// history and link its spends.
///
/// - `viewing_private_key`: 32 bytes, the clamped X25519 secret.
/// - `viewing_public_key`: 33 bytes, `0x04` and the X25519 u-coordinate.
/// - `nullifier_private_key`: 31 bytes.
/// - `nullifier_public_key`: 32 bytes, the Poseidon hash of the private key.
pub struct WalletKeys {
    pub viewing_private_key: Vec<u8>,
    pub viewing_public_key: Vec<u8>,
    pub nullifier_private_key: Vec<u8>,
    pub nullifier_public_key: Vec<u8>,
}

/// The private balances, one per asset held, and the history, newest first.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WalletOverview {
    pub balances: Vec<TokenBalance>,
    pub activity: Vec<ActivityEntry>,
}

/// A spendable private balance. `mint` is `None` for SOL.
///
/// `amount` is what the indexer has confirmed for this wallet, `reserved`
/// included: a payment or change on its way appears once its transaction is
/// confirmed and indexed. Reservations live in this open wallet's memory
/// only.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TokenBalance {
    pub mint: Option<String>,
    pub amount: u64,
    /// How many spendable notes the amount is in: what a spend of all of it
    /// takes, and what a merge reduces.
    pub notes: u64,
    /// The part of `amount` in notes this wallet's prepared spends hold.
    pub reserved: u64,
    /// One entry per proof width the pinned shapes offer (2, 4, 8 and 16
    /// outputs), ascending. A spend with `k` private outputs proves with
    /// `k + 1` outputs, its change included, and public outputs take no
    /// slot: a withdrawal, a withdrawal with one private fee, or a send to
    /// one wallet reads the entry with `outputs: 2`, a send to one wallet
    /// with a fee the entry with `outputs: 4`. The limits include any
    /// application fee the spend pays.
    pub limits: Vec<SpendLimit>,
}

/// The most one spend of up to `outputs` outputs, change included, can take
/// now without a merge: from the unreserved notes, largest first, within the
/// note cap of that width and two trees.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SpendLimit {
    pub outputs: u64,
    pub amount: u64,
}

/// `balance` with what prepared spends reserve of it and its limit per proof
/// width, each capped at `configured_cap` notes.
fn token_balance(
    balance: &AssetBalance,
    reserved: &HashSet<[u8; 32]>,
    configured_cap: usize,
) -> TokenBalance {
    TokenBalance {
        mint: mint_name(&balance.mint),
        amount: balance.amount,
        notes: balance.utxos.len() as u64,
        reserved: balance
            .utxos
            .iter()
            .filter(|note| reserved.contains(&note.nullifier))
            .map(|note| note.utxo.amount)
            .sum(),
        limits: proof_widths()
            .into_iter()
            .map(|outputs| SpendLimit {
                outputs: outputs as u64,
                amount: spend_limit(
                    &balance.utxos,
                    reserved,
                    configured_cap.min(max_spend_inputs(outputs)),
                ),
            })
            .collect(),
    }
}

/// The distinct output counts of the pinned shapes, ascending.
fn proof_widths() -> Vec<usize> {
    let mut widths: Vec<usize> = zolana_client::SPP_SUPPORTED_SHAPES
        .iter()
        .map(|shape| shape.n_outputs())
        .collect();
    widths.sort_unstable();
    widths.dedup();
    widths
}

/// The most a spend of up to `cap` notes can take from `notes`, those in
/// `reserved` left out, as selection would take them: the largest first,
/// stopping before a note past the cap or on a third tree. Every amount up
/// to it selects; one more fails. The candidates are the Zolana SDK's, whose
/// filter is not public: positive notes the default-ring circuit spends.
fn spend_limit(notes: &[WalletUtxo], reserved: &HashSet<[u8; 32]>, cap: usize) -> u64 {
    let mut candidates: Vec<&WalletUtxo> = notes
        .iter()
        .filter(|note| {
            note.utxo.amount > 0
                && note.is_default_ring_spendable()
                && !reserved.contains(&note.nullifier)
        })
        .collect();
    candidates.sort_by_key(|note| Reverse(note.utxo.amount));
    let mut trees: Vec<u16> = Vec::with_capacity(MAX_INPUT_TREES);
    let mut limit = 0u64;
    for note in candidates.into_iter().take(cap) {
        if !trees.contains(&note.tree_id()) {
            if trees.len() == MAX_INPUT_TREES {
                break;
            }
            trees.push(note.tree_id());
        }
        limit = limit.saturating_add(note.utxo.amount);
    }
    limit
}

/// A read of the wallet's notes and history, and when it finished.
struct RecentRead {
    at: Instant,
    scan: UtxoScanResult,
}

/// Notes reserved by prepared spends. The next spend selects other notes
/// until a spend's outcome is known, it is released, or it is past its
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
    /// SOL and the mints the wallet knows.
    assets: Mutex<Assets>,
    reservations: Mutex<Reservations>,
    /// [`WalletConfig::max_spend_inputs`].
    max_spend_inputs: usize,
    /// [`WalletConfig::reuse_notes_ms`]; zero reads before every spend.
    reuse_notes: Duration,
    /// The last read, for the spends within `reuse_notes` of it.
    recent: Mutex<Option<RecentRead>>,
    /// Where spends are proved unless a call says otherwise.
    proving: Proving,
    /// How a send whose response was lost is looked up: [`SEND_LOOKUP`].
    send_lookup: IndexerPollConfig,
    proving_keys: Arc<KeyStore>,
    /// The device prover, the client's too: [`Self::proving_keys`] proves
    /// with it so that a key stays loaded between the two.
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
        let viewing_key = ViewingKey::from_canonical_bytes(
            viewing_secret.as_slice().try_into().map_err(invalid_keys)?,
        )
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
        let assets = Mutex::new(Assets::new(config.mints)?);
        let max_spend_inputs = match config.max_spend_inputs {
            None => MAX_SPEND_INPUTS,
            Some(max) if (1..=MAX_SPEND_INPUTS as u32).contains(&max) => max as usize,
            Some(max) => {
                return Err(WalletError::InvalidMaxSpendInputs {
                    max_inputs: max.into(),
                    limit: MAX_SPEND_INPUTS as u64,
                })
            }
        };
        let proving_keys = Arc::new(KeyStore::new(
            config.proving_key_dir,
            config.proving_key_url,
            transport.clone(),
        ));
        let native_prover = NativeProver::new(Arc::clone(&proving_keys));
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
            max_spend_inputs,
            reuse_notes: Duration::from_millis(config.reuse_notes_ms.unwrap_or(0)),
            recent: Mutex::new(None),
            proving: config.proving.unwrap_or(Proving::Local),
            send_lookup: SEND_LOOKUP,
            proving_keys,
            native_prover,
            remote_prover: config.prover_url.map(|url| Arc::new(transport.prover(url))),
            client,
        })
    }

    /// The proving keys this wallet proves with on the device. Their calls
    /// run beside the wallet's own and outlive it.
    pub fn proving_keys(&self) -> ProvingKeys {
        ProvingKeys {
            store: Arc::clone(&self.proving_keys),
            prover: self.native_prover.clone(),
        }
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
    pub fn prepare_registration(&self) -> Result<Option<PreparedTransaction>, WalletError> {
        let message =
            build_registration_transaction_sync(&self.client, self.owner, &self.address, None)?;
        message
            .map(|message| self.pending(Operation::Registration, message))
            .transpose()
    }

    /// Whether this account's user record lets merges of its notes run.
    /// `false` before registration.
    pub fn merging_enabled(&self) -> Result<bool, WalletError> {
        Ok(
            fetch_user_record_optional_checked(&self.client, self.owner)?
                .is_some_and(|record| record.merge_encryption != MergeEncryptionScheme::Disabled),
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
    ) -> Result<Option<PreparedTransaction>, WalletError> {
        if self.merging_enabled()? == enabled {
            return Ok(None);
        }
        let scheme = if enabled {
            MergeEncryptionScheme::PoseidonCtrV1
        } else {
            MergeEncryptionScheme::Disabled
        };
        let instruction = set_merge_encryption_instruction(self.owner, scheme);
        self.pending(
            Operation::Merging { enabled },
            self.message(&[instruction])?,
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
    /// [`Self::prepare_spend`]; a merge is proved with this wallet's
    /// nullifier secret, which a remote prover receives. The proof expires
    /// after ten minutes, so a merge cannot be sent later by whoever holds it;
    /// [`Self::refresh`] does not extend it.
    pub fn prepare_merge(
        &mut self,
        mint: Option<String>,
        max_inputs: Option<u32>,
        fee_payer: Option<String>,
        proving: Option<Proving>,
    ) -> Result<PreparedTransaction, WalletError> {
        let payer = self.fee_payer(fee_payer)?;
        let proving = self.proving(proving)?;
        let asset = self.asset(mint)?;
        let reserved = self.reserved()?;
        let max_inputs = max_inputs.map_or(MERGE_DEFAULT_INPUT_COUNT, |max| {
            usize::try_from(max).unwrap_or(MAX_MERGE_INPUTS)
        });
        let inputs = select_merge(
            self.recent_notes()?.utxos(),
            asset.mint,
            max_inputs,
            &reserved,
        )?;
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
        let pending = self.proved(
            Operation::Merge {
                mint: mint_name(&asset.mint),
                amount,
            },
            submission.finish_unsigned_sync(&self.client, payer)?,
        );
        Ok(self.reserve(pending, spends))
    }

    /// Deposit public SOL (`mint` `None`) or tokens from this account into
    /// its own private balance. Deposits carry no proof.
    ///
    /// The first deposit of a mint into the pool creates the mint's registry
    /// entry and vault in the same transaction, signed and paid for by this
    /// account; [`Operation::Deposit`] says so. A pool whose protocol config
    /// lets only its authority create them refuses with
    /// [`WalletError::AssetNotSupported`] before the blockhash is read. Two
    /// first deposits of one mint prepared at the same time both create
    /// them, so the second to land fails.
    pub fn prepare_deposit(
        &mut self,
        mint: Option<String>,
        amount: u64,
    ) -> Result<PreparedTransaction, WalletError> {
        let (asset, registered) = self
            .assets()
            .resolve_for_deposit(&self.client, mint.as_deref())?;
        let mut instructions = Vec::with_capacity(2);
        if let (false, Some(token_program)) = (registered, asset.token_program) {
            self.check_spl_interface_creation(asset.mint)?;
            instructions.push(
                CreateSplInterface {
                    authority: self.owner,
                    mint: asset.mint,
                    token_program,
                }
                .instruction(),
            );
        }
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
                view_tag: self.address.viewing_pubkey.u(),
                owner: self.address.owner_hash()?,
                amount,
                memo: None,
            }],
        }
        .instruction()?;
        instructions.push(deposit);
        self.pending(
            Operation::Deposit {
                mint: mint_name(&asset.mint),
                amount,
                creates_spl_interface: !registered,
            },
            self.message(&instructions)?,
        )
    }

    /// Whether this account may create `mint`'s registry entry and vault, as
    /// the pool's `create_spl_interface` decides: anyone when the protocol
    /// config allows it, else only the protocol authority.
    fn check_spl_interface_creation(&self, mint: Pubkey) -> Result<(), WalletError> {
        let address = pda::protocol_config();
        let account = self
            .client
            .get_account(address)?
            .filter(|account| account.owner == PROGRAM_ID_PUBKEY)
            .ok_or(ClientError::ProtocolConfigNotFound(address))?;
        let config = ProtocolConfig::from_account_bytes(&account.data)
            .map_err(|failure| ClientError::InvalidProtocolConfig(format!("{failure:?}")))?;
        if config.allows_permissionless_spl_interface_creation()
            || config.check_protocol_authority(&self.owner).is_ok()
        {
            return Ok(());
        }
        Err(WalletError::AssetNotSupported {
            mint: mint.to_string(),
        })
    }

    /// Spendable private balances, read from the indexer now: one per asset
    /// held in SOL and the mints the wallet knows, each with what prepared
    /// spends reserve of it and the most one spend can take per proof width.
    pub fn balances(&mut self) -> Result<Vec<TokenBalance>, WalletError> {
        let spendable = self.spendable()?;
        self.token_balances(&spendable)
    }

    /// [`Self::balances`] and [`Self::activity`] from one read of the
    /// indexer, which each of them makes on its own.
    pub fn overview(&mut self) -> Result<WalletOverview, WalletError> {
        let scan = self.scan()?;
        Ok(WalletOverview {
            balances: self.token_balances(&scan.spendable)?,
            activity: activity::entries(&scan.history),
        })
    }

    /// [`TokenBalance`]s of `spendable`, against the notes prepared spends
    /// reserve now.
    fn token_balances(
        &self,
        spendable: &SpendableDecryptionResult,
    ) -> Result<Vec<TokenBalance>, WalletError> {
        let reserved = self.reserved()?;
        Ok(spendable
            .balances
            .assets
            .iter()
            .map(|balance| token_balance(balance, &reserved, self.max_spend_inputs))
            .collect())
    }

    /// Spendable private balance of SOL (`mint` `None`) or `mint`, read from
    /// the indexer now.
    pub fn private_balance(&mut self, mint: Option<String>) -> Result<u64, WalletError> {
        // A mint the pool has not registered holds nothing: zero, not a
        // failure.
        let asset = self.assets().asset(&self.client, mint.as_deref())?;
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
        Ok(activity::entries(&self.scan()?.history))
    }

    /// Build and prove a spend of `mint` (SOL for `None`) to `outputs`, in
    /// one transaction. A private output is a note for a registered wallet,
    /// its amount hidden and the note tagged with the recipient's Solana
    /// address on chain; a public output moves funds to a Solana account,
    /// tokens to its associated token account, recipient and amount public.
    /// An application's own fee is one more private output, to its
    /// registered collector.
    ///
    /// Before the network: no outputs fail with [`WalletError::NoOutputs`],
    /// a zero amount with [`WalletError::AmountZero`]. Then, beside the note
    /// selection, an unregistered private recipient fails with
    /// [`WalletError::RecipientNotRegistered`] instead of being paid
    /// publicly. A public token recipient without an associated token
    /// account for the mint gets one: the transaction creates it ahead of
    /// the spend, the fee payer pays its rent, and
    /// [`Operation::Spend`]'s `created_token_accounts` names the recipient.
    ///
    /// The private outputs and the change take the proof's output slots,
    /// public outputs none: a spend with `k` private outputs proves with
    /// `k + 1` outputs, whose keys take at most 40 notes for `k <= 1`, 24
    /// for `k <= 3`, 16 for `k <= 7` and 8 for `k <= 15`, within
    /// [`WalletConfig::max_spend_inputs`]; past that it fails with
    /// [`WalletError::MergeRequired`]. [`TokenBalance::limits`] says what
    /// each width can take now.
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
    pub fn prepare_spend(
        &mut self,
        outputs: Vec<Output>,
        mint: Option<String>,
        fee_payer: Option<String>,
        proving: Option<Proving>,
    ) -> Result<PreparedTransaction, WalletError> {
        if outputs.is_empty() {
            return Err(WalletError::NoOutputs);
        }
        let mut recipients = Vec::with_capacity(outputs.len());
        let mut total = 0u64;
        for output in &outputs {
            if output.amount() == 0 {
                return Err(WalletError::AmountZero);
            }
            total = total
                .checked_add(output.amount())
                .ok_or(TransactionError::SelectedBalanceOverflow)?;
            recipients.push(parse_pubkey(output.recipient())?);
        }
        let payer = self.fee_payer(fee_payer)?;
        let proving = self.proving(proving)?;
        let asset = self.asset(mint)?;
        let (legs, inputs) = beside(
            || self.legs(asset, &outputs, &recipients),
            || self.select_notes(asset, total, proof_outputs(&outputs)),
        );
        let (legs, inputs) = (legs?, inputs?);
        let spends = nullifiers(&inputs);
        let mut transaction = ConfidentialTransaction::new(inputs, payer)?;
        let mut settlements = Vec::new();
        let mut created = Vec::new();
        for leg in legs {
            match leg {
                Leg::Private { address, amount } => {
                    match asset.token_program {
                        None => transaction.transfer_sol(&address, amount),
                        Some(_) => transaction.transfer(&address, asset.mint, amount),
                    }?;
                }
                Leg::Public {
                    recipient,
                    amount,
                    creates_token_account,
                } => {
                    if creates_token_account && !created.contains(&recipient) {
                        created.push(recipient);
                    }
                    settlements.push(transaction.withdraw_to(
                        asset.mint,
                        amount,
                        recipient,
                        asset.token_program,
                    )?);
                }
            }
        }
        let setup: Vec<Instruction> = match asset.token_program {
            Some(token_program) => created
                .iter()
                .map(|&owner| {
                    CreateAssociatedTokenAccount {
                        payer,
                        owner,
                        mint: asset.mint,
                        token_program,
                    }
                    .instruction()
                })
                .collect(),
            None => Vec::new(),
        };
        let (unsigned, openings) = self.prove(transaction, settlements, payer, proving)?;
        let mut pending = self.proved(
            Operation::Spend {
                mint: mint_name(&asset.mint),
                outputs,
                created_token_accounts: created.iter().map(ToString::to_string).collect(),
            },
            with_setup(unsigned, &payer, &setup)?,
        );
        pending.outputs = openings;
        Ok(self.reserve(pending, spends))
    }

    /// `outputs` with each recipient checked on chain: a private recipient
    /// is registered, and a public recipient of a token has its associated
    /// token account or gets one. SOL needs no account.
    fn legs(
        &self,
        asset: Asset,
        outputs: &[Output],
        recipients: &[Pubkey],
    ) -> Result<Vec<Leg>, WalletError> {
        outputs
            .iter()
            .zip(recipients)
            .map(|(output, &recipient)| match *output {
                Output::Private { amount, .. } => Ok(Leg::Private {
                    address: self.registered_address(recipient)?,
                    amount,
                }),
                Output::Public { amount, .. } => {
                    let creates_token_account = match asset.token_account(&recipient) {
                        Some(token_account) => self.client.get_account(token_account)?.is_none(),
                        None => false,
                    };
                    Ok(Leg::Public {
                        recipient,
                        amount,
                        creates_token_account,
                    })
                }
            })
            .collect()
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

    /// A message with `instructions`, paid by this account. Its blockhash is
    /// set by [`Self::pending`].
    fn message(&self, instructions: &[Instruction]) -> Result<VersionedMessage, WalletError> {
        Ok(compile_message(
            &self.owner,
            instructions,
            Default::default(),
            ComputeBudgetConfig::for_instruction_count(instructions.len()),
        )?)
    }

    /// `message` with the latest blockhash, set last so that it is as young
    /// as it can be when the application gets it.
    fn pending(
        &self,
        operation: Operation,
        mut message: VersionedMessage,
    ) -> Result<PreparedTransaction, WalletError> {
        let (blockhash, last_valid_block_height) = self.client.get_latest_blockhash()?;
        message.set_recent_blockhash(blockhash);
        Ok(self.proved(
            operation,
            UnsignedMessage {
                message,
                last_valid_block_height,
            },
        ))
    }

    /// A message the SDK proved and built with the latest blockhash, fetched
    /// after proving.
    fn proved(&self, operation: Operation, unsigned: UnsignedMessage) -> PreparedTransaction {
        let mut reservations = self.reservations();
        let id = reservations.next_id;
        reservations.next_id += 1;
        PreparedTransaction {
            id,
            operation,
            message: unsigned.message,
            last_valid_block_height: unsigned.last_valid_block_height,
            spends: Vec::new(),
            outputs: Vec::new(),
        }
    }

    /// `pending`, reserving the notes with `spends` as nullifiers until it
    /// can no longer land.
    fn reserve(
        &self,
        mut pending: PreparedTransaction,
        spends: Vec<[u8; 32]>,
    ) -> PreparedTransaction {
        self.reservations().reserved.insert(
            pending.id,
            (spends.clone(), pending.last_valid_block_height),
        );
        pending.spends = spends;
        pending
    }

    /// Release the notes `pending` reserves, so the next spend can select
    /// them: for a prepared spend that will not be sent, such as one the user
    /// declined. [`Self::submit`] and [`Self::confirm`] do this once the
    /// outcome is known.
    pub fn release(&self, pending: &PreparedTransaction) {
        self.reservations().reserved.remove(&pending.id);
    }

    fn reservations(&self) -> std::sync::MutexGuard<'_, Reservations> {
        self.reservations
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    /// The notes a spend of `amount` with `outputs` outputs takes, other than
    /// those prepared spends reserve: at most the configured cap and what the
    /// pinned shapes allow, [`WalletError::MergeRequired`] beyond.
    /// Reservations past their last valid block height are dropped: those
    /// spends can no longer land.
    fn select_notes(
        &self,
        asset: Asset,
        amount: u64,
        outputs: usize,
    ) -> Result<Vec<WalletUtxo>, WalletError> {
        let spendable = self.recent_notes()?;
        let reserved = self.reserved()?;
        let inputs = select_spend_excluding(spendable.utxos(), asset.mint, amount, &reserved)
            .map_err(|failure| spend_failure(amount, failure))?;
        within_cap(
            inputs,
            amount,
            self.max_spend_inputs.min(max_spend_inputs(outputs)),
        )
    }

    /// The shielded address `recipient` registered.
    fn registered_address(&self, recipient: Pubkey) -> Result<ShieldedAddress, WalletError> {
        try_resolve_registered_address(&self.client, recipient)?
            .map(|registered| registered.address)
            .ok_or_else(|| WalletError::RecipientNotRegistered {
                recipient: recipient.to_string(),
            })
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
    /// outlived [`PreparedTransaction::last_valid_block_height`]. Signatures
    /// over the old message do not apply to the new one.
    ///
    /// The proof stays valid while the trees still hold the roots it was built
    /// on: a state tree keeps its last 500 roots, about its last 500
    /// transactions, and a nullifier tree its last 100 batch roots. After that
    /// the program rejects the proof as stale, and the spend is prepared again.
    ///
    /// A refreshed spend reserves its notes again until its new last valid
    /// block height.
    pub fn refresh(
        &self,
        pending: &PreparedTransaction,
    ) -> Result<PreparedTransaction, WalletError> {
        let mut refreshed = self.pending(pending.operation.clone(), pending.message.clone())?;
        refreshed.outputs = pending.outputs.clone();
        self.release(pending);
        Ok(if pending.spends.is_empty() {
            refreshed
        } else {
            self.reserve(refreshed, pending.spends.clone())
        })
    }

    /// SOL for `None`, otherwise a configured or discovered mint.
    fn asset(&self, mint: Option<String>) -> Result<Asset, WalletError> {
        self.assets().resolve(&self.client, mint.as_deref())
    }

    fn assets(&self) -> std::sync::MutexGuard<'_, Assets> {
        self.assets
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    /// The wallet's spendable notes, in SOL and every mint it knows, once
    /// the indexer has caught up with the RPC.
    fn spendable(&self) -> Result<SpendableDecryptionResult, WalletError> {
        Ok(self.scan()?.spendable)
    }

    /// The wallet's notes and history, in SOL and every mint it knows, once
    /// the indexer has caught up with the RPC. Kept for the spends
    /// [`WalletConfig::reuse_notes_ms`] lets take their notes from it. A
    /// discovering wallet delegates registry lookup and re-decryption to the
    /// SDK, retaining its resolved mappings only after a successful scan.
    fn scan(&self) -> Result<UtxoScanResult, WalletError> {
        let indexed = self.indexed()?;
        let scan = {
            let mut assets = self.assets();
            let discover = assets.discovers();
            let registry = assets.registry(&self.client)?;
            let scan = UtxoScan::new(&self.keys, registry).with_indexer_config(indexed);
            let scan = if discover {
                scan.with_asset_discovery()
            } else {
                scan
            };
            let scan = scan.fetch(&self.client)?;
            assets.remember_registry(scan.asset_registry.clone());
            scan
        };
        if !self.reuse_notes.is_zero() {
            *self.recent() = Some(RecentRead {
                at: Instant::now(),
                scan: scan.clone(),
            });
        }
        Ok(scan)
    }

    /// The notes a spend selects from: the last read's, within
    /// [`WalletConfig::reuse_notes_ms`] of it, or a read now.
    fn recent_notes(&self) -> Result<SpendableDecryptionResult, WalletError> {
        if let Some(read) = self.recent().as_ref() {
            if read.at.elapsed() <= self.reuse_notes {
                return Ok(read.scan.spendable.clone());
            }
        }
        self.spendable()
    }

    /// The last read no longer stands for the chain: a transaction of this
    /// wallet went out or landed.
    fn forget_notes(&self) {
        *self.recent() = None;
    }

    fn recent(&self) -> std::sync::MutexGuard<'_, Option<RecentRead>> {
        self.recent
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    /// Indexer reads wait for the RPC's confirmed slot, the commitment Photon
    /// indexes at: a spend that slot holds, from this session or another, is
    /// read rather than selected again. An indexer still behind after
    /// [`INDEXER_CATCH_UP`] fails with [`WalletError::IndexerBehind`].
    fn indexed(&self) -> Result<IndexerRpcConfig, WalletError> {
        Ok(IndexerRpcConfig {
            poll: INDEXER_CATCH_UP,
            require_slot: Some(self.client.get_slot()?),
        })
    }

    /// Encrypt and prove `transaction` where `proving` says, and build the
    /// Solana message `payer` pays for.
    fn prove(
        &self,
        transaction: ConfidentialTransaction,
        settlement_transfers: Vec<TransactInterfaceTransferAccounts>,
        payer: Pubkey,
        proving: Proving,
    ) -> Result<(UnsignedMessage, Vec<OutputOpening>), WalletError> {
        let inputs = transaction.encrypt(&self.keys)?;
        let openings = output_openings(&inputs);
        let signed = SignedPrivateTransaction {
            transaction: inputs,
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
        Ok((submission.finish_unsigned_sync(&self.client)?, openings))
    }

    /// Attach the signatures, send, and wait as [`Self::confirm`] does.
    /// Returns the signature.
    ///
    /// `signatures` follows [`PreparedTransaction::signers`]; each is checked
    /// before anything is sent. A spend's notes are released once the send
    /// has an outcome: spent when it landed, free again when the chain
    /// refused it, it expired or it spent a note the chain already had. They
    /// stay reserved while the outcome is unknown, since it may still land.
    pub fn submit(
        &self,
        pending: &PreparedTransaction,
        signatures: Vec<Vec<u8>>,
    ) -> Result<String, WalletError> {
        let message_bytes = pending.message();
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
        self.forget_notes();
        match self.client.process_transaction(transaction) {
            Ok(_) => {}
            // The chain refused it, in simulation or after it ran: it never
            // spent its notes.
            Err(failure) if settles_the_send(&failure) => {
                self.release(pending);
                return Err(failure.into());
            }
            // A send can fail after the transaction went out, such as when
            // the RPC loses the confirmation: look for it before failing.
            Err(_) => self.outcome(pending, signature)?,
        }
        self.settle(pending, signature, true)?;
        Ok(signature.to_string())
    }

    /// Wait for a transaction the application sent itself, as
    /// [`Self::submit`] waits for a send whose response was lost: until Solana
    /// confirms it and, for shielded-pool transactions, the indexer has it,
    /// so the next balance or spend reads the notes it created and not the
    /// ones it spent; the chain's error when the chain refused it;
    /// [`WalletError::TransactionExpired`] when it did not land before
    /// `last_valid_block_height`; and [`WalletError::TransactionOutcomeUnknown`]
    /// when that cannot be told. A spend's notes are released with every
    /// outcome but the unknown one.
    ///
    /// `signature` is the transaction signature, the fee payer's; it is
    /// checked against `pending` before anything is asked.
    pub fn confirm(
        &self,
        pending: &PreparedTransaction,
        signature: String,
    ) -> Result<(), WalletError> {
        let signature =
            Signature::from_str(&signature).map_err(|_| WalletError::SignatureInvalid)?;
        let fee_payer = &pending.message.static_account_keys()[0];
        if !signed_by(fee_payer, &pending.message(), signature.as_array()) {
            return Err(WalletError::SignatureInvalid);
        }
        self.settle(pending, signature, false)
    }

    /// Wait for a shielded-pool transaction by its signature, such as one sent
    /// before the application restarted, until its outcome is known, as
    /// [`Self::submit`] tells a send whose response was lost: done once Solana
    /// confirms it and the indexer has it, so the next balance reads its
    /// notes; the chain's error when the chain refused it;
    /// [`WalletError::TransactionExpired`] when it did not land before
    /// `last_valid_block_height`, so sending it again is safe; and
    /// [`WalletError::TransactionOutcomeUnknown`] when that cannot be told.
    /// Save the prepared transaction's last valid block height with its
    /// signature: without it, a transaction never seen cannot be told expired
    /// and ends unknown.
    pub fn wait_for_transaction(
        &self,
        signature: String,
        last_valid_block_height: Option<u64>,
    ) -> Result<(), WalletError> {
        let signature =
            Signature::from_str(&signature).map_err(|_| WalletError::SignatureInvalid)?;
        self.forget_notes();
        reconcile(
            &self.client,
            signature,
            last_valid_block_height,
            self.send_lookup,
        )?;
        Ok(self.client.confirm_private_transaction_sync(signature)?)
    }

    /// Wait for `pending`, sent as `signature`: a registration or a merging
    /// change until Solana confirms it, a shielded-pool transaction until its
    /// outcome is known and the indexer has it. `landed` when the send
    /// already confirmed it, so its status is not asked again.
    fn settle(
        &self,
        pending: &PreparedTransaction,
        signature: Signature,
        landed: bool,
    ) -> Result<(), WalletError> {
        self.forget_notes();
        if matches!(
            pending.operation,
            Operation::Registration | Operation::Merging { .. }
        ) {
            return wait_for_confirmation(&self.client, signature);
        }
        if landed {
            self.release(pending);
        } else {
            self.outcome(pending, signature)?;
        }
        Ok(self.client.confirm_private_transaction_sync(signature)?)
    }

    /// What became of `pending`, sent as `signature`, as [`reconcile`] tells
    /// it. Every outcome but the unknown one releases its notes: spent once
    /// it landed, free again when the chain refused it, it expired or it
    /// spent a note the chain already had. An unknown outcome keeps them
    /// reserved, since the transaction may still land.
    fn outcome(
        &self,
        pending: &PreparedTransaction,
        signature: Signature,
    ) -> Result<(), WalletError> {
        let outcome = reconcile(
            &self.client,
            signature,
            Some(pending.last_valid_block_height),
            self.send_lookup,
        );
        if !matches!(outcome, Err(WalletError::TransactionOutcomeUnknown { .. })) {
            self.release(pending);
        }
        outcome
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

/// The proof's output slots a spend to `outputs` takes: each private output
/// and the change, two at least, the narrowest keys' width. Public outputs
/// settle outside the proof's outputs.
fn proof_outputs(outputs: &[Output]) -> usize {
    let private = outputs
        .iter()
        .filter(|output| matches!(output, Output::Private { .. }))
        .count();
    (private + 1).max(2)
}

/// The most notes a spend with `outputs` outputs takes: the widest pinned
/// shape for them, within the Zolana SDK's selection cap.
fn max_spend_inputs(outputs: usize) -> usize {
    zolana_client::SPP_SUPPORTED_SHAPES
        .iter()
        .filter(|shape| shape.n_outputs() >= outputs)
        .map(|shape| shape.n_inputs())
        .max()
        .unwrap_or(0)
        .min(MAX_SPEND_INPUTS)
}

/// `inputs` when a spend may take that many notes, else what to merge for.
fn within_cap(
    inputs: Vec<WalletUtxo>,
    amount: u64,
    cap: usize,
) -> Result<Vec<WalletUtxo>, WalletError> {
    if inputs.len() > cap {
        return Err(WalletError::MergeRequired {
            amount,
            max_inputs: cap as u64,
        });
    }
    Ok(inputs)
}

/// Both, `first` on a thread of its own while `second` runs here: the
/// network calls of a spend that need nothing from each other.
fn beside<A: Send, B>(first: impl FnOnce() -> A + Send, second: impl FnOnce() -> B) -> (A, B) {
    thread::scope(|scope| {
        let first = scope.spawn(first);
        let second = second();
        let first = first
            .join()
            .unwrap_or_else(|panic| std::panic::resume_unwind(panic));
        (first, second)
    })
}

fn nullifiers(notes: &[WalletUtxo]) -> Vec<[u8; 32]> {
    notes.iter().map(|note| note.nullifier).collect()
}

/// How long a merge proof can be sent. Anyone who holds it can send it, and
/// the program refuses it after this.
const MERGE_LIFETIME: Duration = Duration::from_secs(600);

/// About ten seconds for the indexer to reach the RPC's slot, asked again
/// after a tenth of a second at first: the indexer is a slot or two behind
/// at most times, and a spend waits for that, not for a stall. A stalled
/// indexer fails after the schedule.
const INDEXER_CATCH_UP: IndexerPollConfig = IndexerPollConfig {
    num_retries: 7,
    delay_ms: 100,
    max_delay_ms: 4_000,
};

/// The unix time a merge prepared now expires at.
fn merge_expiry() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .saturating_add(MERGE_LIFETIME)
        .as_secs()
}

/// How a send whose response was lost is looked up: on the indexer poll's
/// schedule, then at its longest delay until the blockhash expires.
const SEND_LOOKUP: IndexerPollConfig = IndexerPollConfig {
    num_retries: 10,
    delay_ms: 400,
    max_delay_ms: 8_000,
};

/// What a status lookup says of a transaction whose send went unanswered.
enum SendStatus {
    /// Landed and confirmed, without an error.
    Confirmed,
    /// Landed and confirmed, and the chain refused it.
    Refused(ChainError),
    /// Landed but not confirmed yet: it may still be dropped with its fork.
    Processed,
    /// Not seen, and its blockhash still lets it land.
    Pending,
    /// Not seen, and its blockhash no longer lets it land.
    Expired,
}

/// What became of `signature`, whose send went unanswered: looked up until it
/// is confirmed, refused by the chain or expired. Lookups that fail one after
/// another, a landed transaction that does not confirm, and, without
/// `last_valid_block_height`, one never seen, end after `poll`'s retries as
/// [`WalletError::TransactionOutcomeUnknown`].
fn reconcile(
    rpc: &impl Rpc,
    signature: Signature,
    last_valid_block_height: Option<u64>,
    poll: IndexerPollConfig,
) -> Result<(), WalletError> {
    let unknown = |message: String| WalletError::TransactionOutcomeUnknown {
        signature: signature.to_string(),
        message,
    };
    let mut delays = poll
        .backoff()
        .chain(std::iter::repeat(Duration::from_millis(poll.max_delay_ms)));
    let (mut failed_lookups, mut processed_lookups, mut unseen_lookups) = (0, 0, 0);
    loop {
        match send_status(rpc, signature, last_valid_block_height) {
            Ok(SendStatus::Confirmed) => return Ok(()),
            Ok(SendStatus::Refused(failure)) => {
                return Err(ClientError::SolanaRpcTransaction {
                    operation: "process_transaction",
                    source: failure.into(),
                }
                .into())
            }
            Ok(SendStatus::Expired) => {
                return Err(WalletError::TransactionExpired {
                    signature: signature.to_string(),
                })
            }
            Ok(SendStatus::Pending) => {
                failed_lookups = 0;
                unseen_lookups += 1;
                if last_valid_block_height.is_none() && unseen_lookups > poll.num_retries {
                    return Err(unknown(
                        "not seen, and without its last valid block height it cannot be told expired"
                            .to_string(),
                    ));
                }
            }
            Ok(SendStatus::Processed) => {
                failed_lookups = 0;
                processed_lookups += 1;
                if processed_lookups > poll.num_retries {
                    return Err(unknown("landed but not confirmed".to_string()));
                }
            }
            Err(failure) => {
                failed_lookups += 1;
                if failed_lookups > poll.num_retries {
                    return Err(unknown(client_message(&failure)));
                }
            }
        }
        sleep(delays.next().unwrap_or_default());
    }
}

/// One look at `signature` in the RPC's status cache, which holds a landed
/// transaction for longer than its blockhash lets it land, so one missing once
/// `last_valid_block_height` passed never landed.
fn send_status(
    rpc: &impl Rpc,
    signature: Signature,
    last_valid_block_height: Option<u64>,
) -> Result<SendStatus, ClientError> {
    let status = rpc
        .get_signature_statuses(vec![signature])?
        .into_iter()
        .next()
        .flatten();
    Ok(match status {
        Some(status) if status.satisfies_commitment(CommitmentConfig::confirmed()) => {
            match status.err {
                None => SendStatus::Confirmed,
                Some(failure) => SendStatus::Refused(failure),
            }
        }
        Some(_) => SendStatus::Processed,
        None if last_valid_block_height
            .map(|height| Ok::<_, ClientError>(rpc.get_block_height()? > height))
            .transpose()?
            .unwrap_or(false) =>
        {
            SendStatus::Expired
        }
        None => SendStatus::Pending,
    })
}

/// Poll until Solana confirms `signature`, with the indexer's backoff.
/// Compute units each created associated token account adds to a message's
/// limit: SPL's `CreateIdempotent` stays well under it, for SPL Token and
/// Token-2022 alike.
const CREATE_TOKEN_ACCOUNT_UNITS: u32 = 40_000;

/// The most compute units a transaction can ask for.
const MAX_COMPUTE_UNITS: u32 = 1_400_000;

/// `unsigned` with `setup` ahead of its instructions and its compute limit
/// raised for them. The client builds a v1 message of static keys, so its
/// instructions decompile exactly and compile again behind `setup`; the
/// blockhash, and with it the last valid block height, stay. The program
/// checks the accounts of its own instruction, not its position.
fn with_setup(
    unsigned: UnsignedMessage,
    payer: &Pubkey,
    setup: &[Instruction],
) -> Result<UnsignedMessage, WalletError> {
    if setup.is_empty() {
        return Ok(unsigned);
    }
    let VersionedMessage::V1(message) = &unsigned.message else {
        return Err(ClientError::UnsupportedTransactionVersion.into());
    };
    let header = message.header;
    let signers = usize::from(header.num_required_signatures);
    let writable_signers = signers - usize::from(header.num_readonly_signed_accounts);
    let writable_accounts =
        message.account_keys.len() - usize::from(header.num_readonly_unsigned_accounts);
    let account = |index: u8| {
        let index = usize::from(index);
        AccountMeta {
            pubkey: message.account_keys[index],
            is_signer: index < signers,
            is_writable: if index < signers {
                index < writable_signers
            } else {
                index < writable_accounts
            },
        }
    };
    let instructions: Vec<Instruction> = setup
        .iter()
        .cloned()
        .chain(message.instructions.iter().map(|compiled| Instruction {
            program_id: message.account_keys[usize::from(compiled.program_id_index)],
            accounts: compiled.accounts.iter().copied().map(account).collect(),
            data: compiled.data.clone(),
        }))
        .collect();
    let mut config = message.config;
    if let Some(limit) = config.compute_unit_limit {
        let added = CREATE_TOKEN_ACCOUNT_UNITS.saturating_mul(setup.len() as u32);
        config.compute_unit_limit = Some(limit.saturating_add(added).min(MAX_COMPUTE_UNITS));
    }
    let rebuilt = v1::Message::try_compile_with_config(
        payer,
        &instructions,
        message.lifetime_specifier,
        config,
    )
    .map_err(|error| ClientError::TransactionCompile(error.to_string()))?;
    Ok(UnsignedMessage {
        message: VersionedMessage::V1(rebuilt),
        last_valid_block_height: unsigned.last_valid_block_height,
    })
}

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
    use solana_transaction_status_client_types::{
        TransactionConfirmationStatus, TransactionStatus,
    };
    use zolana_client::{compile_message, ClientError, ComputeBudgetConfig};
    use zolana_transaction::instructions::transact::canonical_shape;

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
            max_spend_inputs: None,
            reuse_notes_ms: None,
            mints: None,
        }
    }

    fn open(signer: &Keypair) -> Result<MobileWallet, WalletError> {
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
    fn unsigned(signer: &Keypair) -> PreparedTransaction {
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
        }
    }

    /// A wallet whose Solana RPC answers each JSON-RPC method with
    /// `answer(method)`: `{"result": ..}` or `{"error": ..}`. Returns the
    /// methods it was asked, in order.
    fn rpc_wallet(
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
    fn rpc_wallet_asked(
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

    /// A send whose confirmation the RPC lost, for a transaction that landed
    /// (or did not): the wallet looks for it before it reports a failure.
    #[test]
    fn a_send_error_is_checked_against_the_signature_status() {
        let signer = Keypair::new();
        let internal =
            || serde_json::json!({"error": {"code": -32603, "message": "Internal error"}});
        let registration = |signer: &Keypair| PreparedTransaction {
            operation: Operation::Registration,
            ..unsigned(signer)
        };
        let signed = |pending: &PreparedTransaction| {
            vec![signer.sign_message(&pending.message()).as_ref().to_vec()]
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
            signer.sign_message(&pending.message()).to_string()
        );
        assert!(asked
            .lock()
            .unwrap()
            .contains(&"getSignatureStatuses".to_string()));

        // Not seen, and past its blockhash: it can be sent again.
        let (wallet, _) = rpc_wallet(&signer, move |method| match method {
            "sendTransaction" => internal(),
            "getBlockHeight" => serde_json::json!({"result": 10}),
            _ => status(false),
        });
        assert_eq!(
            wallet.submit(&pending, signed(&pending)).unwrap_err(),
            WalletError::TransactionExpired {
                signature: signer.sign_message(&pending.message()).to_string()
            }
        );
    }

    /// What one status lookup answers.
    #[derive(Clone)]
    enum Lookup {
        Fails,
        Missing,
        Processed,
        Confirmed(Option<ChainError>),
    }

    /// An RPC whose status lookups answer `lookups` in turn, the last one from
    /// then on, at `block_height`. Counts the lookups.
    struct Lookups {
        lookups: Mutex<std::collections::VecDeque<Lookup>>,
        block_height: u64,
        asked: Mutex<u32>,
    }

    impl Lookups {
        fn new(lookups: &[Lookup], block_height: u64) -> Self {
            Self {
                lookups: Mutex::new(lookups.iter().cloned().collect()),
                block_height,
                asked: Mutex::new(0),
            }
        }
    }

    impl Rpc for Lookups {
        fn get_block_height(&self) -> Result<u64, ClientError> {
            Ok(self.block_height)
        }

        fn get_signature_statuses(
            &self,
            signatures: Vec<Signature>,
        ) -> Result<Vec<Option<TransactionStatus>>, ClientError> {
            *self.asked.lock().unwrap() += 1;
            let mut lookups = self.lookups.lock().unwrap();
            let lookup = if lookups.len() > 1 {
                lookups.pop_front()
            } else {
                lookups.front().cloned()
            };
            let level = |err, confirmation_status| {
                Some(TransactionStatus {
                    slot: 1,
                    confirmations: Some(1),
                    status: Ok(()),
                    err,
                    confirmation_status: Some(confirmation_status),
                })
            };
            let status = match lookup.expect("a scripted lookup") {
                Lookup::Fails => return Err(ClientError::Rpc("lookup timed out".into())),
                Lookup::Missing => None,
                Lookup::Processed => level(None, TransactionConfirmationStatus::Processed),
                Lookup::Confirmed(err) => level(err, TransactionConfirmationStatus::Confirmed),
            };
            Ok(signatures.iter().map(|_| status.clone()).collect())
        }
    }

    const QUICK: IndexerPollConfig = IndexerPollConfig {
        num_retries: 2,
        delay_ms: 1,
        max_delay_ms: 1,
    };

    fn reconciled(rpc: &Lookups) -> Result<(), WalletError> {
        reconcile(rpc, Signature::from([3; 64]), Some(100), QUICK)
    }

    /// The lookups last until the blockhash expires, however short the
    /// retry schedule, and only a confirmed status counts: a processed one
    /// can still be dropped with its fork.
    #[test]
    fn a_lost_send_is_looked_up_until_it_is_confirmed() {
        let unseen = Lookups::new(
            &[
                Lookup::Missing,
                Lookup::Missing,
                Lookup::Missing,
                Lookup::Missing,
                Lookup::Confirmed(None),
            ],
            99,
        );
        assert_eq!(reconciled(&unseen), Ok(()));

        let processed = Lookups::new(&[Lookup::Processed, Lookup::Confirmed(None)], 99);
        assert_eq!(reconciled(&processed), Ok(()));
        assert_eq!(*processed.asked.lock().unwrap(), 2);

        let flaky = Lookups::new(&[Lookup::Fails, Lookup::Confirmed(None)], 99);
        assert_eq!(reconciled(&flaky), Ok(()));
    }

    /// Each outcome says whether sending again is safe.
    #[test]
    fn a_lost_send_ends_refused_expired_or_unknown() {
        let refused = Lookups::new(
            &[Lookup::Confirmed(Some(ChainError::InsufficientFundsForFee))],
            99,
        );
        assert!(matches!(
            reconciled(&refused),
            Err(WalletError::Client { message }) if message.contains("Insufficient funds for fee")
        ));

        let expired = Lookups::new(&[Lookup::Missing], 101);
        assert!(matches!(
            reconciled(&expired),
            Err(WalletError::TransactionExpired { .. })
        ));

        let unreachable = Lookups::new(&[Lookup::Fails], 99);
        assert!(matches!(
            reconciled(&unreachable),
            Err(WalletError::TransactionOutcomeUnknown { message, .. })
                if message.contains("lookup timed out")
        ));

        let unconfirmed = Lookups::new(&[Lookup::Processed], 99);
        assert!(matches!(
            reconciled(&unconfirmed),
            Err(WalletError::TransactionOutcomeUnknown { .. })
        ));
    }

    /// A send refused in simulation was never sent and settles at once; one
    /// found already processed may have landed and is looked up.
    #[test]
    fn only_an_already_processed_simulation_is_looked_up() {
        let preflight = |err: &str| {
            let result: solana_rpc_client_api::response::RpcSimulateTransactionResult =
                serde_json::from_str(&format!(r#"{{"err":"{err}"}}"#)).unwrap();
            ClientError::SolanaRpcTransaction {
                operation: "process_transaction",
                source: solana_rpc_client_api::request::RpcError::RpcResponseError {
                    code: -32002,
                    message: "Transaction simulation failed".into(),
                    data: solana_rpc_client_api::request::RpcResponseErrorData::SendTransactionPreflightFailure(result),
                }
                .into(),
            }
        };
        assert!(settles_the_send(&preflight("InsufficientFundsForFee")));
        assert!(!settles_the_send(&preflight("AlreadyProcessed")));
        assert!(!settles_the_send(&ClientError::Rpc(
            "connection reset".into()
        )));
    }

    /// The notes and history wait for the indexer to reach the RPC's slot,
    /// and a stalled indexer fails as `IndexerBehind` instead of answering
    /// with notes another session may have spent.
    #[test]
    fn notes_are_read_once_the_indexer_has_the_rpcs_slot() {
        let signer = Keypair::new();
        let at_slot = |indexed: u64| {
            move |method: &str| match method {
                "getSlot" => serde_json::json!({"result": 100}),
                _ => serde_json::json!({"result": {
                    "context": {"blockTime": 0, "slot": indexed},
                    "transactions": [],
                    "matches": [],
                    "nextCursor": null,
                }}),
            }
        };

        let (mut wallet, asked) = rpc_wallet(&signer, at_slot(100));
        assert!(wallet.balances().expect("caught up").is_empty());
        assert!(wallet.activity().expect("caught up").is_empty());
        asked.lock().unwrap().clear();
        assert_eq!(
            wallet.overview().expect("caught up"),
            WalletOverview {
                balances: Vec::new(),
                activity: Vec::new(),
            }
        );
        // One read of each stream serves both halves.
        let tag_reads = asked
            .lock()
            .unwrap()
            .iter()
            .filter(|method| *method == "getShieldedTransactionsByTags")
            .count();
        assert_eq!(tag_reads, 1);

        let (mut wallet, asked) = rpc_wallet(&signer, at_slot(90));
        let behind = WalletError::IndexerBehind {
            indexer_slot: 90,
            rpc_slot: 100,
        };
        assert_eq!(wallet.balances().unwrap_err(), behind);
        let reads = asked
            .lock()
            .unwrap()
            .iter()
            .filter(|method| *method == "getShieldedTransactionsByTags")
            .count();
        assert_eq!(reads, INDEXER_CATCH_UP.num_retries as usize + 1);
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

    /// A spend of a note the chain already spent fails as such, at once: the
    /// node refused it, so it cannot have landed.
    #[test]
    fn a_spent_note_is_notes_already_spent() {
        let signer = Keypair::new();
        let (wallet, asked) = rpc_wallet(&signer, |method| match method {
            "sendTransaction" => nullifier_queued(),
            _ => status(true),
        });
        let pending = unsigned(&signer);
        let signatures = vec![signer.sign_message(&pending.message()).as_ref().to_vec()];
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

    #[test]
    fn balances_count_their_notes_and_what_prepared_spends_hold() {
        let notes = || vec![note(50, 1, 0), note(30, 2, 0), note(0, 3, 0)];
        assert_eq!(
            token_balance(&balance(notes()), &reserved(&[]), MAX_SPEND_INPUTS),
            TokenBalance {
                mint: None,
                amount: 80,
                notes: 3,
                reserved: 0,
                limits: [2, 4, 8, 16]
                    .map(|outputs| SpendLimit {
                        outputs,
                        amount: 80
                    })
                    .to_vec(),
            }
        );
        // A reserved note stays in the amount and leaves the limits.
        let held = token_balance(&balance(notes()), &reserved(&[1]), MAX_SPEND_INPUTS);
        assert_eq!((held.amount, held.reserved), (80, 50));
        assert_eq!(limits(&held), [(2, 30), (4, 30), (8, 30), (16, 30)]);
    }

    /// Each width's limit is what selection takes from the free notes,
    /// largest first: at most the width's cap, or the configured one, and
    /// nothing from a third tree.
    #[test]
    fn limits_stop_at_the_cap_and_before_a_third_tree() {
        let three = || vec![note(50, 1, 0), note(30, 2, 0), note(20, 3, 0)];
        assert_eq!(
            limits(&token_balance(&balance(three()), &reserved(&[]), 2)),
            [(2, 80), (4, 80), (8, 80), (16, 80)]
        );
        assert_eq!(
            limits(&token_balance(&balance(three()), &reserved(&[]), 1)),
            [(2, 50), (4, 50), (8, 50), (16, 50)]
        );
        // The note on the third tree stops the count, and the smaller note
        // on the first tree after it is not taken either.
        let trees = vec![
            note(30, 1, 0),
            note(20, 2, 1),
            note(10, 3, 2),
            note(5, 4, 0),
        ];
        assert_eq!(
            token_balance(&balance(trees), &reserved(&[]), MAX_SPEND_INPUTS).limits[0].amount,
            50
        );
        // Of 25 equal notes, the 2-output keys take all, the 4-output keys
        // 24, the 8-output keys 16 and the 16-output keys 8.
        let many = balance((1..=25).map(|n| note(10, n, 0)).collect());
        assert_eq!(
            limits(&token_balance(&many, &reserved(&[]), MAX_SPEND_INPUTS)),
            [(2, 250), (4, 240), (8, 160), (16, 80)]
        );
        assert_eq!(proof_widths(), [2, 4, 8, 16]);
    }

    /// Every amount up to a width's limit selects within that width's cap,
    /// and one more fails: short, too many notes or too many trees.
    #[test]
    fn every_amount_up_to_a_limit_selects_and_one_more_does_not() {
        let sets: [Vec<WalletUtxo>; 5] = [
            vec![note(50, 1, 0), note(30, 2, 0), note(20, 3, 0)],
            vec![
                note(30, 1, 0),
                note(20, 2, 1),
                note(10, 3, 2),
                note(5, 4, 0),
            ],
            (1..=41).map(|n| note(10, n, 0)).collect(),
            vec![note(7, 1, 0), note(7, 2, 1), note(7, 3, 2)],
            vec![note(0, 1, 0), note(9, 2, 0)],
        ];
        let free = reserved(&[]);
        for notes in sets {
            let spendable = spendable(notes);
            let balance = &spendable.balances.assets[0];
            for width in proof_widths() {
                let cap = max_spend_inputs(width);
                let limit = spend_limit(&balance.utxos, &free, cap);
                let select = |amount| {
                    select_spend_excluding(
                        spendable.utxos(),
                        zolana_transaction::SOL_MINT,
                        amount,
                        &free,
                    )
                    .map_err(|failure| spend_failure(amount, failure))
                    .and_then(|inputs| within_cap(inputs, amount, cap))
                };
                for amount in 1..=limit {
                    assert!(select(amount).is_ok(), "{amount} of {limit} at {width}");
                }
                assert!(
                    select(limit + 1).is_err(),
                    "{} past {limit} at {width}",
                    limit + 1
                );
            }
        }
    }

    #[test]
    fn a_spend_takes_at_most_the_configured_notes() {
        let notes = vec![note(50, 1, 0), note(30, 2, 0), note(20, 3, 0)];
        assert_eq!(within_cap(notes.clone(), 100, 3).unwrap().len(), 3);
        assert_eq!(
            within_cap(notes, 100, 2).unwrap_err(),
            WalletError::MergeRequired {
                amount: 100,
                max_inputs: 2
            }
        );

        let signer = Keypair::new();
        let with = |max_spend_inputs| {
            open_with(
                &signer,
                WalletConfig {
                    max_spend_inputs,
                    ..config()
                },
            )
        };
        for (max, cap) in [(None, MAX_SPEND_INPUTS), (Some(4), 4), (Some(40), 40)] {
            assert_eq!(with(max).unwrap().max_spend_inputs, cap);
        }
        for max in [0, 41] {
            assert_eq!(
                with(Some(max)).err(),
                Some(WalletError::InvalidMaxSpendInputs {
                    max_inputs: max.into(),
                    limit: 40
                })
            );
        }
    }

    /// With `reuse_notes_ms`, a spend takes its notes from a read made within
    /// the window instead of reading again; a send or a confirmation ends
    /// the reuse, and without the window every spend reads.
    #[test]
    fn a_spend_reuses_the_last_read_within_its_window() {
        let signer = Keypair::new();
        let caught_up = |method: &str| match method {
            "getSlot" => serde_json::json!({"result": 100}),
            "getBlockHeight" => serde_json::json!({"result": 5}),
            _ => serde_json::json!({"result": {
                "context": {"blockTime": 0, "slot": 100},
                "transactions": [],
                "matches": [],
                "nextCursor": null,
            }}),
        };
        let tag_reads = |asked: &Arc<Mutex<Vec<String>>>| {
            asked
                .lock()
                .unwrap()
                .iter()
                .filter(|method| *method == "getShieldedTransactionsByTags")
                .count()
        };
        let (mut wallet, asked) = rpc_wallet_with(
            &signer,
            WalletConfig {
                reuse_notes_ms: Some(60_000),
                ..config()
            },
            caught_up,
        );
        wallet.balances().unwrap();
        assert_eq!(tag_reads(&asked), 1);
        // The merge finds nothing to take, in the notes of the read.
        assert!(matches!(
            wallet.prepare_merge(None, None, None, None).err(),
            Some(WalletError::NothingToMerge { .. })
        ));
        assert_eq!(tag_reads(&asked), 1, "no read within the window");
        wallet.forget_notes();
        let _ = wallet.prepare_merge(None, None, None, None);
        assert_eq!(tag_reads(&asked), 2, "read again once forgotten");
        let stale = Instant::now().checked_sub(Duration::from_secs(61)).unwrap();
        wallet.recent().as_mut().unwrap().at = stale;
        let _ = wallet.prepare_merge(None, None, None, None);
        assert_eq!(tag_reads(&asked), 3, "a read older than the window");

        let (mut wallet, asked) = rpc_wallet_with(&signer, config(), caught_up);
        wallet.balances().unwrap();
        let _ = wallet.prepare_merge(None, None, None, None);
        assert_eq!(tag_reads(&asked), 2, "every spend reads without a window");
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
        let mut wallet = open(&signer).unwrap();
        let pending = || unsigned(&signer);
        assert_eq!(pending().signers(), vec![signer.pubkey().to_string()]);
        let valid = signer.sign_message(&pending().message());
        let wrong_signer = Keypair::new().sign_message(&pending().message());
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
        // A valid signature gets as far as the transport, which answers
        // neither the send nor the lookups after it.
        wallet.send_lookup = QUICK;
        let error = wallet
            .submit(&pending(), vec![valid.as_ref().to_vec()])
            .unwrap_err();
        assert!(
            matches!(error, WalletError::TransactionOutcomeUnknown { .. }),
            "{error:?}"
        );
    }

    #[test]
    fn confirm_accepts_only_the_fee_payer_signature() {
        let signer = Keypair::new();
        let wallet = open(&signer).unwrap();
        let pending = unsigned(&signer);
        let other = Keypair::new().sign_message(&pending.message());
        for signature in ["not-a-signature".to_string(), other.to_string()] {
            assert_eq!(
                wallet.confirm(&pending, signature).err(),
                Some(WalletError::SignatureInvalid)
            );
        }
        // The fee payer's signature gets as far as the transport.
        let signature = signer.sign_message(&pending.message()).to_string();
        let error = wallet.confirm(&pending, signature).unwrap_err();
        assert_ne!(error, WalletError::SignatureInvalid);
    }

    #[test]
    fn waits_for_a_transaction_by_its_signature() {
        let mut wallet = open(&Keypair::new()).unwrap();
        wallet.send_lookup = QUICK;
        assert_eq!(
            wallet
                .wait_for_transaction("not-a-signature".to_string(), None)
                .unwrap_err(),
            WalletError::SignatureInvalid
        );
        // A real signature gets as far as the transport, which answers no
        // lookup.
        let error = wallet
            .wait_for_transaction(Signature::default().to_string(), Some(100))
            .unwrap_err();
        assert!(
            matches!(error, WalletError::TransactionOutcomeUnknown { .. }),
            "{error:?}"
        );

        // Not seen, and past its blockhash: sending again is safe.
        let signer = Keypair::new();
        let (wallet, _) = rpc_wallet(&signer, |method| match method {
            "getBlockHeight" => serde_json::json!({"result": 10}),
            _ => status(false),
        });
        assert!(matches!(
            wallet.wait_for_transaction(Signature::default().to_string(), Some(5)),
            Err(WalletError::TransactionExpired { .. })
        ));
    }

    /// Without its last valid block height, a transaction never seen cannot
    /// be told expired, so it ends unknown rather than waiting forever.
    #[test]
    fn a_transaction_never_seen_without_its_height_is_unknown() {
        let unseen = Lookups::new(&[Lookup::Missing], 1_000);
        assert!(matches!(
            reconcile(&unseen, Signature::from([3; 64]), None, QUICK),
            Err(WalletError::TransactionOutcomeUnknown { message, .. })
                if message.contains("last valid block height")
        ));
        assert_eq!(*unseen.asked.lock().unwrap(), QUICK.num_retries + 1);
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
                wallet.prepare_spend(vec![private(&recipient, 1)], None, None, proving),
                wallet.prepare_spend(vec![public(&recipient, 1)], None, None, proving),
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

    /// Every remote prover, the Helius gateway included, gets attested,
    /// encrypted requests; only a loopback prover gets plaintext.
    #[test]
    fn only_a_loopback_prover_gets_plaintext_proof_requests() {
        let signer = Keypair::new();
        let tee = |url: &str| {
            let config = WalletConfig {
                prover_url: Some(url.to_string()),
                ..config()
            };
            let wallet = open_with(&signer, config).unwrap();
            wallet.remote_prover.as_ref().unwrap().has_tee()
        };
        assert!(tee("https://prover.example"));
        assert!(tee(
            "https://beta-devnet.helius-rpc.com/v1/zolana?api-key=k"
        ));
        assert!(!tee("http://127.0.0.1:3001"));
    }

    /// A deposit is one instruction this account pays for and signs, and the
    /// operation names what it moves.
    #[test]
    fn a_deposit_is_one_instruction_this_account_signs() {
        let signer = Keypair::new();
        let (mut wallet, _) = rpc_wallet(&signer, |method| match method {
            "getLatestBlockhash" => serde_json::json!({"result": {
                "context": {"slot": 1},
                "value": {
                    "blockhash": "11111111111111111111111111111111",
                    "lastValidBlockHeight": 9
                }
            }}),
            _ => serde_json::json!({"error": {"code": -32603, "message": "unexpected"}}),
        });
        let pending = wallet.prepare_deposit(None, 100).unwrap();
        assert_eq!(
            pending.operation(),
            Operation::Deposit {
                mint: None,
                amount: 100,
                creates_spl_interface: false,
            }
        );
        assert_eq!(pending.message.instructions().len(), 1);
        assert_eq!(pending.signers(), [signer.pubkey().to_string()]);
        assert_eq!(pending.last_valid_block_height(), 9);
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
                ("getAccountInfo", Some(address)) if address == registry_address => {
                    answers.0.clone()
                }
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

    /// The first deposit of a mint the pool has not registered creates its
    /// registry entry and vault first, in the same transaction this account
    /// signs; once registered, a deposit is one instruction again.
    #[test]
    fn a_first_token_deposit_creates_the_mints_registry_entry_and_vault() {
        let signer = Keypair::new();
        let other = Keypair::new().pubkey();
        let deposit = |registry, config| {
            let (mut wallet, _) = token_wallet(&signer, registry, config);
            wallet.prepare_deposit(Some(MINT.to_string()), 100)
        };
        let registered = account(
            PROGRAM_ID_PUBKEY,
            &zolana_interface::state::SplAssetRegistry::account_bytes(MINT, 2),
        );
        // Lamports sent to the registry address make no registry entry.
        let lamports_only = account(Pubkey::default(), &[]);
        for (registry, config, creates) in [
            (serde_json::Value::Null, protocol_config(true, other), true),
            (lamports_only, protocol_config(true, other), true),
            (
                serde_json::Value::Null,
                protocol_config(false, signer.pubkey()),
                true,
            ),
            (registered, serde_json::Value::Null, false),
        ] {
            let pending = deposit(registry, config).unwrap();
            assert_eq!(
                pending.operation(),
                Operation::Deposit {
                    mint: Some(MINT.to_string()),
                    amount: 100,
                    creates_spl_interface: creates,
                }
            );
            let instructions = pending.message.instructions();
            assert_eq!(instructions.len(), if creates { 2 } else { 1 });
            let keys = pending.message.static_account_keys();
            if creates {
                let create = &instructions[0];
                assert_eq!(
                    keys[usize::from(create.program_id_index)],
                    PROGRAM_ID_PUBKEY
                );
                assert_eq!(
                    create.data,
                    [zolana_interface::instruction::tag::CREATE_SPL_INTERFACE]
                );
            }
            assert_eq!(pending.signers(), [signer.pubkey().to_string()]);
        }
    }

    /// A pool that lets only its authority register mints refuses another
    /// account's first deposit before the blockhash is read.
    #[test]
    fn a_first_token_deposit_the_pool_restricts_is_refused_before_the_blockhash() {
        let signer = Keypair::new();
        let (mut wallet, asked) = token_wallet(
            &signer,
            serde_json::Value::Null,
            protocol_config(false, Keypair::new().pubkey()),
        );
        assert_eq!(
            wallet.prepare_deposit(Some(MINT.to_string()), 100).err(),
            Some(WalletError::AssetNotSupported {
                mint: MINT.to_string()
            })
        );
        assert!(!asked
            .lock()
            .unwrap()
            .iter()
            .any(|method| method == "getLatestBlockhash"));
        let (mut wallet, _) =
            token_wallet(&signer, serde_json::Value::Null, serde_json::Value::Null);
        assert!(matches!(
            wallet.prepare_deposit(Some(MINT.to_string()), 100),
            Err(WalletError::Client { message }) if message.contains("protocol config")
        ));
    }

    /// A public token recipient without its associated token account gets
    /// one in the spend; a recipient that has one, and SOL, need none.
    #[test]
    fn a_public_token_recipient_without_its_account_gets_one() {
        let mint = Pubkey::new_unique();
        let token = Asset {
            mint,
            token_program: Some(pda::spl_token_program_id()),
        };
        let (without, with) = (Pubkey::new_unique(), Pubkey::new_unique());
        let existing = token.token_account(&with).unwrap().to_string();
        let (transport, _) = crate::transport::tests::fake(move |request| {
            let body: serde_json::Value = serde_json::from_slice(&request.body).unwrap();
            assert_eq!(body["method"], "getAccountInfo");
            let value = if body["params"][0] == existing.as_str() {
                serde_json::json!({
                    "data": ["", "base64"], "executable": false, "lamports": 2_039_280,
                    "owner": pda::spl_token_program_id().to_string(), "rentEpoch": 0, "space": 0
                })
            } else {
                serde_json::Value::Null
            };
            crate::transport::tests::ok(
                &serde_json::json!({
                    "jsonrpc": "2.0", "id": body["id"],
                    "result": {"context": {"slot": 1}, "value": value}
                })
                .to_string(),
            )
        });
        let signer = Keypair::new();
        let pubkey = signer.pubkey().to_string();
        let signature = signer.sign_message(&derivation_message(pubkey.clone()).unwrap());
        let wallet =
            MobileWallet::open(config(), pubkey, signature.as_ref().to_vec(), transport).unwrap();
        let creates = |asset: Asset, recipient: Pubkey| {
            let legs = wallet
                .legs(asset, &[public(&recipient.to_string(), 1)], &[recipient])
                .unwrap();
            match legs[..] {
                [Leg::Public {
                    creates_token_account,
                    ..
                }] => creates_token_account,
                _ => panic!("one public leg"),
            }
        };
        assert!(creates(token, without));
        assert!(!creates(token, with));
        let sol = Asset {
            mint: zolana_transaction::SOL_MINT,
            token_program: None,
        };
        assert!(!creates(sol, without));
    }

    /// The created accounts go ahead of the proved message, which compiles
    /// again unchanged behind them: the same blockhash, the transact
    /// instruction's accounts and roles, and a compute limit raised for each
    /// account, at most to the ceiling.
    #[test]
    fn created_token_accounts_go_ahead_of_the_proved_message() {
        let payer = Pubkey::new_unique();
        let transact = Instruction {
            program_id: Pubkey::new_unique(),
            accounts: vec![
                AccountMeta::new(payer, true),
                AccountMeta::new_readonly(Pubkey::new_unique(), true),
                AccountMeta::new(Pubkey::new_unique(), false),
                AccountMeta::new_readonly(Pubkey::new_unique(), false),
            ],
            data: vec![1, 2, 3],
        };
        let create = CreateAssociatedTokenAccount {
            payer,
            owner: Pubkey::new_unique(),
            mint: Pubkey::new_unique(),
            token_program: pda::spl_token_2022_program_id(),
        }
        .instruction();
        let blockhash = Default::default();
        let proved = |limit: u32| UnsignedMessage {
            message: compile_message(
                &payer,
                core::slice::from_ref(&transact),
                blockhash,
                ComputeBudgetConfig::new(limit),
            )
            .unwrap(),
            last_valid_block_height: 9,
        };
        let expected = |limit: u32| {
            compile_message(
                &payer,
                &[create.clone(), transact.clone()],
                blockhash,
                ComputeBudgetConfig::new(limit),
            )
            .unwrap()
        };

        let rebuilt =
            with_setup(proved(1_000_000), &payer, core::slice::from_ref(&create)).unwrap();
        assert_eq!(rebuilt.message, expected(1_040_000));
        assert_eq!(rebuilt.last_valid_block_height, 9);
        let capped = with_setup(proved(1_390_000), &payer, core::slice::from_ref(&create)).unwrap();
        assert_eq!(capped.message, expected(MAX_COMPUTE_UNITS));
        assert_eq!(
            with_setup(proved(1_000_000), &payer, &[]).unwrap().message,
            proved(1_000_000).message
        );
    }

    /// The outputs are checked before anything is asked of the network: no
    /// outputs, a zero amount and a total past `u64` each fail at once.
    #[test]
    fn a_spend_checks_its_outputs_before_the_network() {
        let signer = Keypair::new();
        let (mut wallet, asked) = rpc_wallet(
            &signer,
            |_| serde_json::json!({"error": {"code": -32603, "message": "unexpected"}}),
        );
        let recipient = Keypair::new().pubkey().to_string();
        let spend = |wallet: &mut MobileWallet, outputs| {
            wallet
                .prepare_spend(outputs, None, None, None)
                .err()
                .expect("fails before the network")
        };
        assert_eq!(spend(&mut wallet, vec![]), WalletError::NoOutputs);
        assert_eq!(
            spend(
                &mut wallet,
                vec![private(&recipient, 1), public(&recipient, 0)]
            ),
            WalletError::AmountZero
        );
        let overflow = spend(
            &mut wallet,
            vec![private(&recipient, u64::MAX), public(&recipient, 1)],
        );
        assert!(
            matches!(&overflow, WalletError::Client { message } if message.contains("overflow")),
            "{overflow:?}"
        );
        assert!(asked.lock().unwrap().is_empty());
    }

    /// Private outputs and the change take the proof's output slots, public
    /// outputs none; the slots pick the widest keys, and with them the notes
    /// a spend can take.
    #[test]
    fn private_outputs_and_the_change_take_the_proof_slots() {
        let recipient = Pubkey::new_unique().to_string();
        let slots = |outputs: &[Output]| proof_outputs(outputs);
        assert_eq!(slots(&[public(&recipient, 1)]), 2);
        assert_eq!(slots(&[public(&recipient, 1), public(&recipient, 1)]), 2);
        assert_eq!(slots(&[private(&recipient, 1)]), 2);
        assert_eq!(slots(&[private(&recipient, 1), public(&recipient, 1)]), 2);
        assert_eq!(slots(&[private(&recipient, 1), private(&recipient, 1)]), 3);
        assert_eq!(
            slots(&[
                public(&recipient, 1),
                private(&recipient, 1),
                private(&recipient, 1),
                private(&recipient, 1),
            ]),
            4
        );
        // One private output and change: the 2-output keys, up to the SDK's
        // cap. Two or three: the 4-output keys, which stop at 24 notes.
        assert_eq!(max_spend_inputs(2), MAX_SPEND_INPUTS);
        assert_eq!(max_spend_inputs(3), 24);
        assert_eq!(max_spend_inputs(4), 24);
        assert_eq!(max_spend_inputs(5), 16);
        assert_eq!(max_spend_inputs(9), 8);
        assert!(canonical_shape(24, 3).is_ok());
        assert!(canonical_shape(25, 3).is_err());
    }

    /// A spend whose send has a verdict frees its notes at once: refused in
    /// simulation, or expired after a lost send, through `submit` or
    /// `confirm`. One whose outcome stays unknown keeps them: it may still
    /// land.
    #[test]
    fn a_failed_send_releases_its_notes_unless_the_outcome_is_unknown() {
        let signer = Keypair::new();
        let internal =
            || serde_json::json!({"error": {"code": -32603, "message": "Internal error"}});
        let spend = |wallet: &MobileWallet| {
            let pending = PreparedTransaction {
                operation: Operation::Spend {
                    mint: None,
                    outputs: Vec::new(),
                    created_token_accounts: Vec::new(),
                },
                last_valid_block_height: 5,
                ..unsigned(&signer)
            };
            wallet.reserve(pending, vec![[1; 32]])
        };
        let signed = |pending: &PreparedTransaction| {
            vec![signer.sign_message(&pending.message()).as_ref().to_vec()]
        };
        let held = |wallet: &MobileWallet| wallet.reservations().reserved.len();

        let (wallet, _) = rpc_wallet(&signer, |method| match method {
            "sendTransaction" => nullifier_queued(),
            _ => status(true),
        });
        let pending = spend(&wallet);
        assert_eq!(held(&wallet), 1);
        assert_eq!(
            wallet.submit(&pending, signed(&pending)).unwrap_err(),
            WalletError::NotesAlreadySpent
        );
        assert_eq!(held(&wallet), 0, "refused: the notes are free");

        let expired = move |method: &str| match method {
            "sendTransaction" => internal(),
            "getBlockHeight" => serde_json::json!({"result": 10}),
            _ => status(false),
        };
        let (wallet, _) = rpc_wallet(&signer, expired);
        let pending = spend(&wallet);
        assert!(matches!(
            wallet.submit(&pending, signed(&pending)),
            Err(WalletError::TransactionExpired { .. })
        ));
        assert_eq!(held(&wallet), 0, "expired: the notes are free");

        let (wallet, _) = rpc_wallet(&signer, expired);
        let pending = spend(&wallet);
        let signature = signer.sign_message(&pending.message()).to_string();
        assert!(matches!(
            wallet.confirm(&pending, signature),
            Err(WalletError::TransactionExpired { .. })
        ));
        assert_eq!(held(&wallet), 0, "expired after the app sent it");

        let (mut wallet, _) = rpc_wallet(&signer, move |_| internal());
        wallet.send_lookup = QUICK;
        let pending = spend(&wallet);
        assert!(matches!(
            wallet.submit(&pending, signed(&pending)),
            Err(WalletError::TransactionOutcomeUnknown { .. })
        ));
        assert_eq!(held(&wallet), 1, "unknown: the notes stay reserved");
    }

    #[test]
    fn rejects_malformed_keys_before_the_network() {
        let mut wallet = open(&Keypair::new()).unwrap();
        let recipient = Keypair::new().pubkey().to_string();
        let bad = "not-a-pubkey".to_string();
        let invalid = Some(WalletError::InvalidPubkey { value: bad.clone() });
        let error = |result: Result<PreparedTransaction, WalletError>| result.err();
        assert_eq!(
            error(wallet.prepare_spend(vec![private(&bad, 1)], None, None, None)),
            invalid
        );
        assert_eq!(
            error(wallet.prepare_spend(vec![public(&bad, 1)], None, None, None)),
            invalid
        );
        assert_eq!(
            error(wallet.prepare_spend(vec![private(&recipient, 1)], None, Some(bad), None)),
            invalid
        );
    }

    #[test]
    fn refreshed_spends_keep_their_openings() {
        let signer = Keypair::new();
        let (wallet, _) = rpc_wallet(&signer, |_| {
            serde_json::json!({"result": {
                "context": {"slot": 5}, "value": {
                    "blockhash": "11111111111111111111111111111111", "lastValidBlockHeight": 250
                }
            }})
        });
        let mut pending = unsigned(&signer);
        pending.outputs = vec![OutputOpening::from_output(
            &SppProofOutputUtxo::default(),
            0,
        )];
        let refreshed = wallet.refresh(&pending).unwrap();
        assert_eq!(refreshed.outputs(), pending.outputs());
        assert_eq!(refreshed.last_valid_block_height(), 250);
    }

    #[test]
    fn openings_preserve_all_commitment_fields() {
        use solana_address::Address;
        use zolana_transaction::Mint;

        let wallet = open(&Keypair::new()).unwrap();
        let mut real = SppProofOutputUtxo::new(Mint::SOL, 5, wallet.address).unwrap();
        real.data_hash = Some([7; 32]);
        real.ring_data_hash = Some([8; 32]);
        real.ring_program_id = Some(Address::new_from_array([9; 32]));
        let dummy = SppProofOutputUtxo::default();
        let compact = SppProofOutputUtxo {
            compact: true,
            ..dummy.clone()
        };
        for output in [real, dummy, compact] {
            let opening = OutputOpening::from_output(&output, 4);
            assert_eq!(
                commitment_of(&opening, wallet.address.viewing_pubkey),
                output.hash(4).unwrap()
            );
            assert_eq!(opening.owner_signing_key.is_none(), output.is_dummy());
            assert_eq!(opening.nullifier_key.is_none(), output.is_dummy());
            assert_eq!(opening.compact, output.is_compact());
        }
    }

    /// Golden v1 messages built with the actual SDK wire serializers, plus
    /// Poseidon vectors for the Dart policy tests. No real funds or proofs.
    #[test]
    fn spend_check_vectors_match_example() {
        use solana_address::Address;
        use zolana_hasher::{primitives::right_align, Hasher, Poseidon};
        use zolana_interface::instruction::{
            CircuitId, InputUtxo, InterfaceTransfer, OwnerTag, TransactIxData, TransactOutput,
            TransactProof, TreeContext,
        };
        use zolana_program::instruction::{
            Transact, TransactSolTransferAccounts, TransactSplWithdrawalAccounts,
        };
        use zolana_transaction::{
            derive_output_blinding_seed, utxo::derive_transact_output_blinding, Mint,
        };

        let signers = [1, 2, 3].map(|byte| Keypair::new_from_array([byte; 32]));
        let wallets = signers.each_ref().map(|signer| open(signer).unwrap());
        let payer = signers[0].pubkey();
        let recipient = signers[1].pubkey();
        let token_program = pda::spl_token_program_id();
        let mint = Pubkey::new_from_array([8; 32]);
        let mut hashes = serde_json::Map::new();
        let mut hash = |inputs: Vec<[u8; 32]>| {
            let result = Poseidon::hashv(
                &inputs
                    .iter()
                    .map(|field| field.as_slice())
                    .collect::<Vec<_>>(),
            )
            .unwrap();
            let key = inputs
                .iter()
                .map(|field| {
                    field
                        .iter()
                        .map(|byte| format!("{byte:02x}"))
                        .collect::<String>()
                })
                .collect::<Vec<_>>()
                .join(":");
            hashes.insert(key, serde_json::json!(result));
            result
        };
        let mut cases = Vec::new();
        for spl in [false, true] {
            for withdraw in [false, true] {
                let asset = if spl {
                    Mint::new(Address::new_from_array(mint.to_bytes()), 7)
                } else {
                    Mint::SOL
                };
                let first = [1; 32];
                let seed = derive_output_blinding_seed(&first, &[2; 32]).unwrap();
                let mut outputs = vec![
                    SppProofOutputUtxo::new(asset, 7, wallets[0].address).unwrap(),
                    SppProofOutputUtxo::new(asset, 5, wallets[2].address).unwrap(),
                ];
                if !withdraw {
                    outputs.push(SppProofOutputUtxo::new(asset, 100, wallets[1].address).unwrap());
                }
                outputs.push(SppProofOutputUtxo::default());
                while outputs.len() < 4 {
                    outputs.push(SppProofOutputUtxo {
                        compact: true,
                        ..Default::default()
                    });
                }
                for (index, output) in outputs.iter_mut().enumerate() {
                    output.blinding =
                        derive_transact_output_blinding(&first, &seed, index as u32).unwrap();
                    if output.is_compact() {
                        continue;
                    }
                    let dummy = output.is_dummy();
                    let owner = match output.owner_address {
                        None => [0; 32],
                        Some(address) => {
                            let mut tagged = vec![0x53];
                            tagged.extend_from_slice(&address.signing_pubkey.as_ed25519().unwrap());
                            let identity = hash(vec![
                                right_align::<31>(tagged[..31].try_into().unwrap()),
                                right_align::<2>(tagged[31..].try_into().unwrap()),
                            ]);
                            hash(vec![identity, address.nullifier_pubkey])
                        }
                    };
                    let asset_hash = if dummy {
                        [0; 32]
                    } else {
                        hash(vec![
                            right_align::<31>(asset.asset.as_array()[..31].try_into().unwrap()),
                            right_align::<1>(asset.asset.as_array()[31..].try_into().unwrap()),
                        ])
                    };
                    let ring = hash(vec![[0; 32], [0; 32]]);
                    let owner_utxo = hash(vec![owner, output.blinding]);
                    let commitment = hash(vec![
                        right_align(&(if dummy { 1u16 } else { 3u16 }).to_be_bytes()),
                        [0; 32],
                        asset_hash,
                        right_align(&(if dummy { 0 } else { output.amount }).to_be_bytes()),
                        [0; 32],
                        ring,
                        owner_utxo,
                    ]);
                    assert_eq!(commitment, output.hash(0).unwrap());
                }
                let user_token =
                    pda::associated_token_address_with_program(&recipient, &mint, &token_program);
                let (transfers, settlement) = if !withdraw {
                    (vec![], vec![])
                } else if spl {
                    let (spl_interface, bump) = pda::spl_interface_with_bump(&mint);
                    (
                        vec![InterfaceTransfer::SplWithdrawal {
                            amount: 100,
                            spl_interface_bump: bump,
                        }],
                        vec![TransactInterfaceTransferAccounts::SplWithdrawal(
                            TransactSplWithdrawalAccounts {
                                mint,
                                spl_interface,
                                user_token_account: user_token,
                                token_program,
                            },
                        )],
                    )
                } else {
                    (
                        vec![InterfaceTransfer::SolWithdrawal { amount: 100 }],
                        vec![TransactInterfaceTransferAccounts::Sol(
                            TransactSolTransferAccounts { recipient },
                        )],
                    )
                };
                let builder = Transact {
                    payer,
                    input_trees: vec![pda::tree(0)],
                    output_tree: pda::tree(0),
                    owner_signers: vec![payer],
                    interface_transfer_accounts: settlement,
                    data: TransactIxData {
                        expiry_unix_ts: u64::MAX,
                        tx_viewing_pk: [0; 33],
                        salt: [0; 16],
                        interface_transfers: transfers,
                        data_hash: None,
                        ring_data_hash: None,
                        outputs: outputs
                            .iter()
                            .filter(|output| !output.is_compact())
                            .map(|output| TransactOutput {
                                utxo_hash: output.hash(0).unwrap(),
                                owner_tag: match output.owner_address {
                                    Some(address) if address == wallets[0].address => {
                                        OwnerTag::Account(0)
                                    }
                                    Some(address) => OwnerTag::Inline(
                                        address.signing_pubkey.confidential_view_tag().unwrap(),
                                    ),
                                    None => OwnerTag::Inline([9; 32]),
                                },
                                data: Some(vec![42; 8]),
                            })
                            .collect(),
                        messages: vec![],
                        private_tx_hash: [0; 32],
                        circuit: CircuitId::ConfidentialEddsa(2, 4, 3),
                        proof: TransactProof::zeroed(),
                        inputs: vec![
                            InputUtxo {
                                nullifier_hash: first,
                                tree_index: 0,
                            },
                            InputUtxo {
                                nullifier_hash: [3; 32],
                                tree_index: 0,
                            },
                        ],
                        tree_contexts: vec![TreeContext {
                            utxo_tree_root_index: 0,
                            nullifier_tree_root_index: 0,
                        }],
                    },
                };
                for create in [false, true] {
                    if create && !(spl && withdraw) {
                        continue;
                    }
                    let mut instructions = Vec::new();
                    if create {
                        instructions.push(
                            CreateAssociatedTokenAccount {
                                payer,
                                owner: recipient,
                                mint,
                                token_program,
                            }
                            .instruction(),
                        );
                    }
                    instructions.push(builder.instruction());
                    let message = compile_message(
                        &payer,
                        &instructions,
                        Default::default(),
                        ComputeBudgetConfig::new(1_000_000),
                    )
                    .unwrap();
                    let openings = outputs.iter().map(|output| {
                        let o = OutputOpening::from_output(output, 0);
                        serde_json::json!({
                            "ownerSigningKey": o.owner_signing_key, "nullifierKey": o.nullifier_key,
                            "mint": o.mint, "amount": o.amount.to_string(), "blinding": o.blinding,
                            "treeId": o.tree_id, "compact": o.compact,
                        })
                    }).collect::<Vec<_>>();
                    cases.push(serde_json::json!({
                        "name": format!("{}-{}{}", if spl { "spl" } else { "sol" }, if withdraw { "withdraw" } else { "private" }, if create { "-create" } else { "" }),
                        "message": message.serialize(), "openings": openings,
                        "mint": spl.then(|| mint.to_string()), "withdraw": withdraw,
                        "tokenAccount": user_token.to_string(), "tokenProgram": token_program.to_string(),
                    }));
                }
            }
        }
        let fixture = serde_json::json!({
            "sender": payer.to_string(), "recipient": recipient.to_string(),
            "collector": signers[2].pubkey().to_string(), "tree": pda::tree(0).to_string(),
            "nullifiers": signers.iter().zip(&wallets).map(|(signer, wallet)| (signer.pubkey().to_string(), wallet.address.nullifier_pubkey.to_vec())).collect::<std::collections::BTreeMap<_, _>>(),
            "hashes": hashes, "cases": cases,
        });
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../example/test/fixtures/spend_checks.json");
        if std::env::var_os("UPDATE_SPEND_CHECK_FIXTURE").is_some() {
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(&path, serde_json::to_string(&fixture).unwrap() + "\n").unwrap();
        }
        let expected: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
        assert_eq!(
            fixture, expected,
            "regenerate with UPDATE_SPEND_CHECK_FIXTURE=1"
        );
    }

    fn commitment_of(opening: &OutputOpening, viewing: zolana_keypair::X25519Pubkey) -> [u8; 32] {
        use std::str::FromStr;

        use solana_address::Address;
        use zolana_keypair::PublicKey;
        use zolana_transaction::{Mint, SppProofOutputUtxo};

        let owner_address = match (&opening.owner_signing_key, &opening.nullifier_key) {
            (Some(signing), Some(nullifier)) => Some(zolana_keypair::ShieldedAddress {
                signing_pubkey: PublicKey::from_bytes(signing.as_slice().try_into().unwrap())
                    .unwrap(),
                nullifier_pubkey: nullifier.as_slice().try_into().unwrap(),
                viewing_pubkey: viewing,
            }),
            _ => None,
        };
        let asset = match &opening.mint {
            None => Mint::SOL,
            Some(mint) => Mint::new(Address::from_str(mint).unwrap(), 0),
        };
        let output = SppProofOutputUtxo {
            asset,
            amount: opening.amount,
            blinding: opening.blinding.as_slice().try_into().unwrap(),
            ring_program_id: opening
                .ring_program_id
                .as_deref()
                .map(|id| Address::from_str(id).unwrap()),
            ring_data_hash: opening
                .ring_data_hash
                .as_deref()
                .map(|hash| hash.try_into().unwrap()),
            data_hash: opening
                .data_hash
                .as_deref()
                .map(|hash| hash.try_into().unwrap()),
            owner_address,
            compact: opening.compact,
            ..SppProofOutputUtxo::default()
        };
        output.hash(opening.tree_id).unwrap()
    }
}
