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
//! 2. each [`PreparedTransaction::message`], the v1 Solana message this
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

use crate::{
    asset::Assets,
    keys::KeyStore,
    prover::{NativeProver, Proving},
};
use balance::RecentRead;
use prepared::Reservations;
use solana_pubkey::Pubkey;
use std::{
    sync::{Arc, Mutex},
    time::Duration,
};
use zolana_client::{IndexerPollConfig, ProverClient, SolanaRpc, ZolanaClient};
use zolana_keypair::{NullifierKey, ShieldedAddress, ViewingKey};
use zolana_transaction::LocalShieldedKeys;

mod account;
mod balance;
mod cache;
mod deposit;
mod lookup;
mod merge;
mod notes;
mod prepared;
mod spend;
mod submission;
mod types;
mod util;
pub use account::{derivation_message, RegistrationStatus, WalletConfig, WalletKeys};
pub use balance::{SpendLimit, TokenBalance, UnresolvedAssets, WalletOverview};
pub use cache::{CacheTarget, CachedOutput};
pub use notes::SpendableNote;
pub use prepared::PreparedTransaction;
pub use types::{Operation, Output, OutputOpening};

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
    /// How a send whose response was lost is looked up: [`submission::SEND_LOOKUP`].
    send_lookup: IndexerPollConfig,
    /// Default whole-transaction priority fee in lamports.
    priority_fee: Option<u64>,
    recovery_domain: String,
    proving_keys: Arc<KeyStore>,
    /// The device prover, the client's too: [`Self::proving_keys`] proves
    /// with it so that a key stays loaded between the two.
    native_prover: NativeProver,
    /// The SDK's prover client of [`WalletConfig::prover_url`].
    remote_prover: Option<Arc<ProverClient>>,
    /// Proves nothing itself: each spend names its prover.
    client: ZolanaClient<SolanaRpc>,
}

#[cfg(test)]
mod tests;
