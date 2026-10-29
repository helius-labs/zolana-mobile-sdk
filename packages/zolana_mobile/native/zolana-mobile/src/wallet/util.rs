//! Small shared boundary conversions and independent reads.
use crate::error::WalletError;
use solana_pubkey::Pubkey;
use std::{str::FromStr, thread};
use zolana_transaction::WalletUtxo;

/// Both, `first` on a thread of its own while `second` runs here: the
/// network calls of a spend that need nothing from each other.
pub(super) fn beside<A: Send, B>(
    first: impl FnOnce() -> A + Send,
    second: impl FnOnce() -> B,
) -> (A, B) {
    thread::scope(|scope| {
        let first = scope.spawn(first);
        let second = second();
        let first = first
            .join()
            .unwrap_or_else(|panic| std::panic::resume_unwind(panic));
        (first, second)
    })
}

pub(super) fn nullifiers(notes: &[WalletUtxo]) -> Vec<[u8; 32]> {
    notes.iter().map(|note| note.nullifier).collect()
}

pub(super) fn parse_pubkey(value: &str) -> Result<Pubkey, WalletError> {
    Pubkey::from_str(value).map_err(|_| WalletError::InvalidPubkey {
        value: value.to_string(),
    })
}

pub(super) fn invalid(message: &str) -> WalletError {
    WalletError::Client {
        message: message.to_owned(),
    }
}
