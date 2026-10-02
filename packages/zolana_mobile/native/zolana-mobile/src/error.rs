//! What a wallet call reports when it fails. The bindings carry it as data,
//! so the application reads the fields instead of parsing a message.

use zolana_client::ClientError;
use zolana_keypair::KeypairError;
use zolana_program::instruction::DepositBuildError;
use zolana_transaction::TransactionError;

/// Every failure a wallet call reports. Amounts are in base units, accounts
/// and mints base58. No variant carries key material.
#[derive(Clone, Debug, PartialEq, Eq)]
#[flutter_rust_bridge::frb(non_opaque)]
pub enum WalletError {
    /// The spendable notes of the asset do not cover `requested`.
    InsufficientPrivateBalance { requested: u64, available: u64 },
    /// `amount` needs more than `max_inputs` notes, the most one spend
    /// takes. Merge first.
    MergeRequired { amount: u64, max_inputs: u64 },
    /// The notes that cover the amount are on `trees` trees, and a spend
    /// takes notes from at most `max_trees`. Merge first.
    TooManyInputTrees { trees: u64, max_trees: u64 },
    /// A spend of zero.
    AmountZero,
    /// Only notes a prepared spend reserves would cover `amount`. Submit,
    /// confirm or release that spend first.
    NotesReserved { amount: u64 },
    /// `recipient` has not registered a shielded address. A withdrawal pays
    /// it publicly.
    RecipientNotRegistered { recipient: String },
    /// `recipient` has no associated token account for `mint`.
    /// `prepare_token_account` creates it.
    RecipientTokenAccountMissing { recipient: String, mint: String },
    /// The user registry holds other keys for `owner`. The wallet never
    /// replaces them.
    RegistrationConflict { owner: String },
    /// The shielded pool has not registered `mint`.
    AssetNotSupported { mint: String },
    /// No account exists at `mint`.
    MintNotFound { mint: String },
    /// `mint` is not a base58 key, or its account is not owned by SPL Token
    /// or Token-2022.
    InvalidMint { mint: String },
    /// `value` is not a base58 public key.
    InvalidPubkey { value: String },
    /// The data of `account` is not a token account's.
    InvalidTokenAccount { account: String },
    /// The derivation signature is not the account's signature over the
    /// derivation message.
    InvalidDerivationSignature,
    /// The saved keys are malformed or do not match their public keys.
    InvalidWalletKeys,
    /// The Solana RPC client could not be built.
    TransportFailed { message: String },
    /// A signature is not 64 bytes, not base58, or not the signer's signature
    /// over the message.
    SignatureInvalid,
    /// `submit` received `got` signatures for `expected` signers.
    SignatureCountMismatch { expected: u64, got: u64 },
    /// Solana did not confirm the transaction within the wait.
    TransactionNotConfirmed { signature: String },
    /// The spend asked for remote proving and no remote prover is set.
    RemoteProverMissing,
    /// The remote prover returned no proof.
    RemoteProverFailed,
    /// The remote prover's response is not a gnark proof.
    ProofMalformed,
    /// The proof does not verify against the pinned verifying key.
    ProofInvalid,
    /// The device prover could not prove the request.
    ProofFailed,
    /// Another proof is running, or a `LocalProver` holds the prepared slot.
    ProverBusy,
    /// The prepared prover was released.
    ProverClosed,
    /// The prover state of this process is unusable.
    ProverUnavailable,
    /// gnark could not be initialized.
    ProverInitFailed,
    /// The proving key could not be loaded into gnark.
    ProverLoadFailed,
    /// The prepared circuit's shape is not one the wallet proves.
    UnsupportedCircuit,
    /// With the default transport, `open` refuses a plaintext Solana RPC URL
    /// off loopback unless `allowInsecureHttp` is set.
    RpcUrlInsecure { url: String },
    /// As [`Self::RpcUrlInsecure`], for the indexer URL.
    IndexerUrlInsecure { url: String },
    /// As [`Self::RpcUrlInsecure`], for the proving key host.
    ProvingKeyUrlInsecure { url: String },
    /// The lockfile pins no key named `name`.
    ProvingKeyUnknown { name: String },
    /// The lockfile pins another sha256 for `name` than the verifying key
    /// expects.
    ProvingKeyMismatch { name: String },
    /// The key `name` could not be downloaded.
    ProvingKeyDownloadFailed { name: String },
    /// The downloaded key `name` does not hash to the pinned value.
    ProvingKeyCorrupt { name: String },
    /// The key store could not read or write `path`.
    ProvingKeyStoreFailed { path: String },
    /// Poseidon takes 1 to 12 inputs.
    PoseidonInputCountInvalid { count: u64 },
    /// Poseidon input `index` has `length` bytes; each input is 32.
    PoseidonInputLengthInvalid { index: u64, length: u64 },
    /// The wallet was opened without a signer. The `prepare` methods work.
    SignerMissing,
    /// The signer signs for `signer`, not for `wallet`.
    SignerMismatch { wallet: String, signer: String },
    /// The transaction needs `signers`, not the wallet's account alone. Sign
    /// it in the application.
    UnexpectedSigners { signers: Vec<String> },
    /// The wallet was closed.
    WalletClosed,
    /// Any other failure of the Zolana client: the error and its causes,
    /// without key material, with `api-key` values masked.
    Client { message: String },
}

impl From<ClientError> for WalletError {
    fn from(failure: ClientError) -> Self {
        match failure {
            ClientError::Transaction(failure) => failure.into(),
            ClientError::UserRegistryKeysMismatch { owner } => Self::RegistrationConflict {
                owner: owner.to_string(),
            },
            ClientError::ProofVerification(_) => Self::ProofInvalid,
            failure => Self::Client {
                message: client_message(&failure),
            },
        }
    }
}

impl From<KeypairError> for WalletError {
    fn from(failure: KeypairError) -> Self {
        ClientError::from(failure).into()
    }
}

impl From<DepositBuildError> for WalletError {
    fn from(failure: DepositBuildError) -> Self {
        ClientError::from(failure).into()
    }
}

impl From<TransactionError> for WalletError {
    fn from(failure: TransactionError) -> Self {
        match failure {
            TransactionError::InsufficientBalance {
                requested,
                available,
            } => Self::InsufficientPrivateBalance {
                requested,
                available,
            },
            TransactionError::SpendNeedsMerge { amount, max_inputs } => Self::MergeRequired {
                amount,
                max_inputs: max_inputs as u64,
            },
            TransactionError::TooManyInputTrees { got, max } => Self::TooManyInputTrees {
                trees: got as u64,
                max_trees: max as u64,
            },
            TransactionError::SpendNeedsExcludedUtxos { amount } => Self::NotesReserved { amount },
            TransactionError::ZeroSpendAmount => Self::AmountZero,
            failure => Self::Client {
                message: client_message(&ClientError::from(failure)),
            },
        }
    }
}

/// The error and its causes, so a failed request says why it failed (DNS,
/// connection, TLS). Client and transaction errors describe what failed
/// without key material; keep it that way when adding variants. Request
/// errors name their URL, so `api-key` values are masked.
fn client_message(error: &ClientError) -> String {
    let mut message = error.to_string();
    let mut source = std::error::Error::source(error);
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
    use super::*;

    #[test]
    fn masks_every_api_key_in_a_message() {
        assert_eq!(
            redact_api_keys("url (https://h/?a=1&api-key=k&b=2) and api-key=k"),
            "url (https://h/?a=1&api-key=redacted&b=2) and api-key=redacted"
        );
    }

    #[test]
    fn sdk_errors_keep_their_data() {
        assert_eq!(
            WalletError::from(ClientError::Transaction(
                TransactionError::SpendNeedsMerge {
                    amount: 7,
                    max_inputs: 5
                }
            )),
            WalletError::MergeRequired {
                amount: 7,
                max_inputs: 5
            }
        );
        assert_eq!(
            WalletError::from(ClientError::ProofVerification("x".into())),
            WalletError::ProofInvalid
        );
        assert!(matches!(
            WalletError::from(ClientError::Rpc("api-key=k".into())),
            WalletError::Client { message } if message == "rpc error: api-key=redacted"
                || message.contains("api-key=redacted")
        ));
    }
}
