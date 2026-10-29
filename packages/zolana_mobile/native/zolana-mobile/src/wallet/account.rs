//! Wallet identity, configuration and on-chain registration.
use super::{
    prepared::PreparedTransaction, submission::SEND_LOOKUP, types::Operation, util::parse_pubkey,
    MobileWallet,
};
use crate::{
    asset::{Assets, MintConfig},
    error::WalletError,
    keys::{KeyStore, ProvingKeys},
    prover::{NativeProver, Proving},
    transport::Transport,
};
use solana_pubkey::Pubkey;
use std::{
    sync::{Arc, Mutex},
    time::Duration,
};
use zeroize::Zeroizing;
use zolana_client::{
    user_registry::{
        build_registration_transaction_sync, fetch_user_record_optional_checked,
        resolved_address_from_record, set_merge_encryption_instruction,
        try_resolve_registered_address,
    },
    ZolanaClient,
};
use zolana_keypair::{derivation, Curve, NullifierKey, PublicKey, ShieldedAddress, ViewingKey};
use zolana_transaction::{instructions::transact::MAX_SPEND_INPUTS, LocalShieldedKeys};
use zolana_user_registry_interface::{
    instruction::{register, RegisterData},
    user_record_pda, MergeEncryptionScheme,
};

/// Where the wallet reads chain state and stores proving keys.
pub struct WalletConfig {
    pub rpc_url: String,
    pub indexer_url: String,
    /// Directory for downloaded proving keys; keep it across launches.
    pub proving_key_dir: String,
    /// Overrides [`crate::DEFAULT_PROVING_KEYS_URL`].
    pub proving_key_url: Option<String>,
    /// Optional /proving-keys JSON catalog. Uses its complete zopro URLs only
    /// when source and artifact digests match the bundled pins. Never fetched
    /// automatically; mutually exclusive with proving_key_url.
    pub proving_key_catalog: Option<String>,
    /// Where spends are proved unless a call says otherwise: on the device
    /// when `None`. [`Proving::Remote`] needs [`Self::prover_url`].
    pub proving: Option<Proving>,
    /// The prover [`Proving::Remote`] asks, through the transport: the
    /// application's backend as a proxy of the prover's routes, or a Zolana
    /// prover. It receives each spend's witness, the wallet's nullifier
    /// secret included, encrypted to the enclave the supplied policy pins once the
    /// prover attests to it; only a loopback prover gets it in plaintext.
    pub prover_url: Option<String>,
    /// Require attestation even for a loopback proxy. Remote URLs always attest.
    pub require_prover_tee: Option<bool>,
    /// Trusted deployment policy JSON, supplied by the app, never by the prover.
    /// Required for remote URLs and when require_prover_tee is true. Missing
    /// policy fails before HTTP; no URL selects a policy automatically.
    pub prover_tee_policy: Option<String>,
    /// Refuse queue fallback and status polling for remote proofs.
    pub prover_sync_only: Option<bool>,
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
    /// Default total priority fee per transaction, in lamports. Each prepare
    /// call can override it, including with zero. `None` adds no priority fee.
    pub priority_fee_lamports: Option<u64>,
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
    pub(super) fn from_keys(
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
        let proving_keys = Arc::new(
            KeyStore::new(
                config.proving_key_dir,
                config.proving_key_url,
                transport.clone(),
            )
            .with_catalog(config.proving_key_catalog.as_deref())?,
        );
        let native_prover = NativeProver::new(Arc::clone(&proving_keys));
        // With its own prover, the client fetches every Merkle proof from the
        // indexer itself: the wallet's provers prove only complete witnesses.
        let recovery_domain = format!("{}\n{}", config.rpc_url, config.indexer_url);
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
            priority_fee: config.priority_fee_lamports,
            recovery_domain,
            proving_keys,
            native_prover,
            remote_prover: config
                .prover_url
                .map(|url| {
                    transport
                        .configured_prover(
                            url,
                            config.require_prover_tee.unwrap_or(false),
                            config.prover_tee_policy.as_deref(),
                            config.prover_sync_only.unwrap_or(false),
                        )
                        .map(Arc::new)
                })
                .transpose()?,
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
    /// this wallet never replaces them. `priority_fee_lamports` overrides
    /// [`WalletConfig::priority_fee_lamports`], as on every prepared
    /// transaction.
    /// When `enable_merging` is `Some(true)`, a new registration also selects
    /// `PoseidonCtrV1` in the same message, after register. `None` and
    /// `Some(false)` leave merging disabled. An existing registration is
    /// still a no-op; use [`Self::prepare_merging`] to change its scheme.
    pub fn prepare_registration(
        &self,
        priority_fee_lamports: Option<u64>,
        enable_merging: Option<bool>,
    ) -> Result<Option<PreparedTransaction>, WalletError> {
        let message =
            build_registration_transaction_sync(&self.client, self.owner, &self.address, None)?;
        message
            .map(|message| {
                let message = if enable_merging.unwrap_or(false) {
                    self.message(&[
                        register(
                            user_record_pda(&self.owner).0,
                            self.owner,
                            self.owner,
                            RegisterData {
                                nullifier_pubkey: self.address.nullifier_pubkey,
                                viewing_pubkey: *self.address.viewing_pubkey.as_bytes(),
                            },
                        ),
                        set_merge_encryption_instruction(
                            self.owner,
                            MergeEncryptionScheme::PoseidonCtrV1,
                        ),
                    ])?
                } else {
                    message
                };
                let message = self.with_priority_fee(message, priority_fee_lamports)?;
                self.pending(Operation::Registration, message)
            })
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
        priority_fee_lamports: Option<u64>,
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
        let message =
            self.with_priority_fee(self.message(&[instruction])?, priority_fee_lamports)?;
        self.pending(Operation::Merging { enabled }, message)
            .map(Some)
    }
    /// The shielded address `recipient` registered.
    pub(super) fn registered_address(
        &self,
        recipient: Pubkey,
    ) -> Result<ShieldedAddress, WalletError> {
        try_resolve_registered_address(&self.client, recipient)?
            .map(|registered| registered.address)
            .ok_or_else(|| WalletError::RecipientNotRegistered {
                recipient: recipient.to_string(),
            })
    }
}

/// A record that does not parse is someone else's keys as much as a record
/// that differs: both are a conflict.
pub(super) fn registration_status_of<E>(
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
pub(super) fn signed_by(signer: &Pubkey, message: &[u8], signature: &[u8; 64]) -> bool {
    PublicKey::from_ed25519(&signer.to_bytes()).verify_message(message, signature)
}

/// Saved keys that are malformed or do not match their public keys.
pub(super) fn invalid_keys<E>(_: E) -> WalletError {
    WalletError::InvalidWalletKeys
}
