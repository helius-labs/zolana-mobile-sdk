//! A private SOL wallet whose Solana key never enters this library.
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

use std::str::FromStr;

use solana_message::VersionedMessage;
use solana_pubkey::Pubkey;
use solana_signature::Signature;
use solana_transaction::versioned::VersionedTransaction;
use zolana_client::{ClientError, Rpc, SolanaRpc, ZolanaClient};
use zolana_interface::pda;
use zolana_keypair::{derivation, PublicKey};
use zolana_transaction::{AssetRegistry, SOL_MINT};
use zolana_wallet::{
    build_deposit_transaction_sync, build_private_transaction_sync,
    build_registration_transaction_sync, create_transfer_sync, create_withdrawal,
    get_private_token_balances, is_wallet_registered_sync, sync_wallet,
    ClientEd25519WalletAuthority, Deposit, DepositParams, SyncWalletAuthority, TransferParams,
    Wallet, WithdrawalLeg, WithdrawalParams,
};

use crate::{keys::KeyStore, prover::NativeProver};

/// The prover URL `ZolanaClient` requires at construction. `with_prover`
/// replaces it before any request is made, so it is never contacted.
const UNUSED_PROVER_URL: &str = "https://prover.invalid";

/// Where the wallet reads chain state and stores proving keys.
pub struct WalletConfig {
    pub rpc_url: String,
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

/// What a submitted transaction waits for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PendingTransactionKind {
    /// Publishes the wallet's shielded address so others can pay it.
    Registration,
    /// Moves public SOL into the private balance. Public: sender, amount.
    Deposit,
    /// Private transfer to a registered wallet. Reveals neither amount nor
    /// recipient.
    Transfer,
    /// Moves private SOL to a public account. Public: recipient, amount.
    Withdrawal,
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

#[derive(Clone, Debug)]
pub struct SyncSummary {
    pub stored_utxos: u64,
    pub private_lamports: u64,
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
        let rpc = SolanaRpc::new(config.rpc_url);
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

    /// Whether this wallet has published its shielded address. Others can
    /// only send to a registered wallet.
    pub fn is_registered(&self) -> Result<bool, String> {
        is_wallet_registered_sync(&self.client, self.owner).map_err(error)
    }

    /// `None` when the registry already holds this wallet's address.
    pub fn prepare_registration(&self) -> Result<Option<PendingTransaction>, String> {
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

    /// Deposit public SOL from this wallet into its own private balance.
    /// Deposits carry no proof.
    pub fn prepare_deposit(&self, lamports: u64) -> Result<PendingTransaction, String> {
        let deposit = Deposit::new(DepositParams {
            recipient: &self.wallet.identity,
            asset: SOL_MINT,
            amount: lamports,
            spl_token_account: None,
            spl_token_program: None,
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
            summary: format!("Deposit {lamports} lamports (public)"),
        })
    }

    /// Fetch and decrypt this wallet's notes from the indexer.
    pub fn sync(&mut self) -> Result<SyncSummary, String> {
        let report = sync_wallet(&mut self.wallet, &self.authority, &self.client).map_err(error)?;
        Ok(SyncSummary {
            stored_utxos: report.stored_utxos as u64,
            private_lamports: self.private_lamports()?,
        })
    }

    /// Spendable private SOL as of the last [`Self::sync`].
    pub fn private_lamports(&self) -> Result<u64, String> {
        Ok(get_private_token_balances(&self.wallet)
            .map_err(error)?
            .iter()
            .filter(|balance| balance.mint == SOL_MINT)
            .map(|balance| balance.amount)
            .sum())
    }

    /// Build and prove a private transfer to a registered wallet.
    ///
    /// Refuses an unregistered recipient: the wallet SDK turns a transfer to
    /// one into a public withdrawal, which [`Self::prepare_withdrawal`] makes
    /// explicit instead.
    pub fn prepare_transfer(
        &self,
        recipient: String,
        lamports: u64,
    ) -> Result<PendingTransaction, String> {
        let recipient = parse_pubkey(&recipient)?;
        if !is_wallet_registered_sync(&self.client, recipient).map_err(error)? {
            return Err("recipient_not_registered".to_string());
        }
        let created = create_transfer_sync(TransferParams {
            rpc: &self.client,
            wallet: &self.wallet,
            payer: self.owner,
            recipient,
            asset: SOL_MINT,
            amount: lamports,
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
            summary: format!("Send {lamports} lamports privately to {recipient}"),
        })
    }

    /// Build and prove a withdrawal of private SOL to a public account.
    pub fn prepare_withdrawal(
        &self,
        recipient: String,
        lamports: u64,
    ) -> Result<PendingTransaction, String> {
        let recipient = parse_pubkey(&recipient)?;
        let created = create_withdrawal(WithdrawalParams {
            wallet: &self.wallet,
            payer: self.owner,
            legs: vec![WithdrawalLeg {
                recipient,
                asset: SOL_MINT,
                amount: lamports,
                spl_token_program: None,
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
            summary: format!("Withdraw {lamports} lamports to {recipient} (public)"),
        })
    }

    /// Attach the signatures, send, and wait for confirmation and, for
    /// shielded-pool transactions, for the indexer. Returns the signature.
    ///
    /// `signatures` follows [`PendingTransaction::signers`]; each is checked
    /// before anything is sent.
    pub fn submit(
        &self,
        pending: PendingTransaction,
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
            message: pending.message,
        };
        let signature = self
            .client
            .process_transaction(transaction)
            .map_err(error)?;
        if pending.kind != PendingTransactionKind::Registration {
            self.client
                .confirm_private_transaction_sync(signature)
                .map_err(error)?;
        }
        Ok(signature.to_string())
    }
}

fn parse_pubkey(value: &str) -> Result<Pubkey, String> {
    Pubkey::from_str(value).map_err(|_| "pubkey_invalid".to_string())
}

/// Errors reach Dart as text. Wallet and client errors describe what failed
/// without key material; keep it that way when adding variants.
fn error(error: impl Into<ClientError>) -> String {
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

    #[test]
    fn submit_checks_every_signature_before_sending() {
        let signer = Keypair::new();
        let wallet = open(&signer).unwrap();
        let pending = || {
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
        };
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
                wallet.submit(pending(), signatures).err().as_deref(),
                Some(expected)
            );
        }
        // A valid signature gets as far as the unroutable RPC.
        let error = wallet
            .submit(pending(), vec![valid.as_ref().to_vec()])
            .unwrap_err();
        assert!(!error.starts_with("signature_"), "{error}");
    }

    #[test]
    fn rejects_malformed_recipients_before_the_network() {
        let wallet = open(&Keypair::new()).unwrap();
        assert_eq!(
            wallet
                .prepare_transfer("not-a-pubkey".into(), 1)
                .err()
                .as_deref(),
            Some("pubkey_invalid")
        );
    }
}
