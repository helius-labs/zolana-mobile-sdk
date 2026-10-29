//! Dart-facing payment descriptions and output openings.
use crate::asset::mint_name;
use solana_pubkey::Pubkey;
use zolana_transaction::instructions::transact::{SppProofInputs, SppProofOutputUtxo};

/// What a prepared transaction does, to show the user before signing.
/// Amounts are in base units; `mint` is `None` for SOL.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
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
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
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
    pub(super) fn recipient(&self) -> &str {
        match self {
            Output::Private { recipient, .. } | Output::Public { recipient, .. } => recipient,
        }
    }

    pub(super) fn amount(&self) -> u64 {
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
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
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
    pub(super) fn from_output(output: &SppProofOutputUtxo, tree_id: u16) -> Self {
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

pub(super) fn output_openings(inputs: &SppProofInputs) -> Vec<OutputOpening> {
    inputs
        .output_utxos
        .iter()
        .map(|output| OutputOpening::from_output(output, inputs.output_tree_id))
        .collect()
}
