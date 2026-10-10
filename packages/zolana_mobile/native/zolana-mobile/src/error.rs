//! What a wallet call reports when it fails. The bindings carry it as data,
//! so the application reads the fields instead of parsing a message.

use solana_instruction::error::InstructionError;
use solana_transaction_error::TransactionError as ChainError;
use zolana_client::ClientError;
use zolana_interface::error::ShieldedPoolError;
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
    /// Fewer than two notes of `mint` (`None` for SOL) are free to merge on
    /// any one tree.
    NothingToMerge { mint: Option<String> },
    /// This account's user record does not enable merging:
    /// `prepare_merging(true)` turns it on.
    MergingDisabled,
    /// The indexer has persisted only `indexer_slot`, behind the RPC's
    /// confirmed `rpc_slot`, so the wallet's notes could include one already
    /// spent. Try again once it catches up.
    IndexerBehind { indexer_slot: u64, rpc_slot: u64 },
    /// The chain already spent a note this transaction spends: the indexer
    /// was behind when it was prepared, or another session spent it. Prepare
    /// it again once the indexer has the spend.
    NotesAlreadySpent,
    /// Only notes a prepared spend reserves would cover `amount`. Submit,
    /// confirm or release that spend first.
    NotesReserved { amount: u64 },
    /// `recipient` has not registered a shielded address. A withdrawal pays
    /// it publicly.
    RecipientNotRegistered { recipient: String },
    /// `recipient` has no associated token account for `mint`. The
    /// application's Solana client creates it.
    RecipientTokenAccountMissing { recipient: String, mint: String },
    /// The user registry holds other keys for `owner`. The wallet never
    /// replaces them.
    RegistrationConflict { owner: String },
    /// The shielded pool has not registered `mint`.
    AssetNotSupported { mint: String },
    /// `mint` is not in [`WalletConfig::mints`](crate::WalletConfig::mints),
    /// so the wallet does not know its token program.
    MintNotConfigured { mint: String },
    /// `mint` is not a base58 key.
    InvalidMint { mint: String },
    /// The configured `token_program` of `mint` is neither SPL Token nor
    /// Token-2022.
    InvalidTokenProgram { mint: String, token_program: String },
    /// `value` is not a base58 public key.
    InvalidPubkey { value: String },
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
    /// The spend asked for remote proving and `WalletConfig.prover_url` is
    /// not set.
    RemoteProverMissing,
    /// [`Proving::Auto`](crate::Proving::Auto) would prove a circuit of
    /// `inputs` notes remotely, past `max_local_inputs`, and
    /// `WalletConfig.prover_url` is not set. Merge the notes first, or set a
    /// prover.
    ProofTooLargeForDevice { inputs: u32, max_local_inputs: u32 },
    /// The remote prover's response is not a proof of the pinned proving key.
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
    /// off loopback unless `allowInsecureHttp` is set. `api-key` values in
    /// `url` are masked.
    RpcUrlInsecure { url: String },
    /// As [`Self::RpcUrlInsecure`], for the indexer URL.
    IndexerUrlInsecure { url: String },
    /// As [`Self::RpcUrlInsecure`], for the proving key host.
    ProvingKeyUrlInsecure { url: String },
    /// As [`Self::RpcUrlInsecure`], for the remote prover.
    ProverUrlInsecure { url: String },
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
            ClientError::UserRegistryKeysMismatch { owner }
            | ClientError::MergeViewingKeyMismatch { owner } => Self::RegistrationConflict {
                owner: owner.to_string(),
            },
            ClientError::MergeDisabled { .. } => Self::MergingDisabled,
            ClientError::IndexerNotCaughtUp {
                required, indexed, ..
            } => Self::IndexerBehind {
                indexer_slot: indexed,
                rpc_slot: required,
            },
            ClientError::SolanaRpcTransaction { ref source, .. }
                if program_error(source.get_transaction_error().as_ref())
                    == Some(ShieldedPoolError::NullifierAlreadyQueued as u32) =>
            {
                Self::NotesAlreadySpent
            }
            ClientError::ProofVerification(_) | ClientError::ProvingKeyMismatch { .. } => {
                Self::ProofInvalid
            }
            ClientError::ProofParse(_) | ClientError::MissingProvingKeySha256 { .. } => {
                Self::ProofMalformed
            }
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
            TransactionError::NothingToMerge { asset } => Self::NothingToMerge {
                mint: crate::asset::mint_name(&asset),
            },
            failure => Self::Client {
                message: client_message(&ClientError::from(failure)),
            },
        }
    }
}

/// The custom error code of a failed instruction, such as the shielded pool's.
fn program_error(failure: Option<&ChainError>) -> Option<u32> {
    match failure? {
        ChainError::InstructionError(_, InstructionError::Custom(code)) => Some(*code),
        _ => None,
    }
}

/// Whether `failure` is the chain's verdict on a transaction: it ran, or a
/// node refused it, so it did not land and will not. Any other failure of a
/// send may have come after the transaction went out.
pub(crate) fn rejected_by_chain(failure: &ClientError) -> bool {
    matches!(failure, ClientError::SolanaRpcTransaction { source, .. }
        if source.get_transaction_error().is_some())
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
        assert_eq!(
            WalletError::from(ClientError::IndexerNotCaughtUp {
                required: 100,
                indexed: 90,
                attempts: 6
            }),
            WalletError::IndexerBehind {
                indexer_slot: 90,
                rpc_slot: 100
            }
        );
        let owner = solana_address::Address::new_unique();
        assert_eq!(
            WalletError::from(ClientError::MergeDisabled { owner }),
            WalletError::MergingDisabled
        );
        assert_eq!(
            WalletError::from(ClientError::MergeViewingKeyMismatch { owner }),
            WalletError::RegistrationConflict {
                owner: owner.to_string()
            }
        );
        assert_eq!(
            WalletError::from(TransactionError::NothingToMerge {
                asset: zolana_transaction::SOL_MINT
            }),
            WalletError::NothingToMerge { mint: None }
        );
        assert!(matches!(
            WalletError::from(ClientError::Rpc("api-key=k".into())),
            WalletError::Client { message } if message == "rpc error: api-key=redacted"
                || message.contains("api-key=redacted")
        ));
    }
}
