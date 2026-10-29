//! Payment orchestration over the Rust SDK transaction builders.
use super::{
    cache::CachedOutput,
    prepared::PreparedTransaction,
    types::{output_openings, Operation, Output, OutputOpening},
    util::{beside, invalid, nullifiers, parse_pubkey},
    MobileWallet,
};
use crate::{
    asset::{mint_name, Asset},
    error::WalletError,
    prover::Proving,
};
use solana_instruction::{AccountMeta, Instruction};
use solana_message::{v1, VersionedMessage};
use solana_pubkey::Pubkey;
use zolana_client::{ClientError, Rpc, SignedPrivateTransaction, Submission, UnsignedMessage};
use zolana_keypair::ShieldedAddress;
use zolana_program::instruction::{
    CreateAssociatedTokenAccount, TransactInterfaceTransferAccounts,
};
use zolana_transaction::{
    instructions::transact::{ConfidentialTransaction, MAX_SPEND_INPUTS},
    TransactionError, WalletUtxo,
};

/// An output whose recipient was checked on chain.
pub(super) enum Leg {
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

impl MobileWallet {
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
    /// [`super::WalletConfig::max_spend_inputs`]; past that it fails with
    /// [`WalletError::MergeRequired`]. [`super::TokenBalance::limits`] says what
    /// each width can take now.
    ///
    /// `fee_payer` pays the network fee, this account when `None`. The proof
    /// binds it, so it cannot change after this call. Another fee payer signs
    /// first, before this account.
    ///
    /// `proving` says where it is proved, [`super::WalletConfig::proving`] when
    /// `None`. Remote proving without [`super::WalletConfig::prover_url`] fails with
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
        priority_fee_lamports: Option<u64>,
    ) -> Result<PreparedTransaction, WalletError> {
        self.prepare_spend_inner(
            outputs,
            mint,
            fee_payer,
            proving,
            priority_fee_lamports,
            Vec::new(),
        )
    }
    pub(super) fn prepare_spend_inner(
        &mut self,
        outputs: Vec<Output>,
        mint: Option<String>,
        fee_payer: Option<String>,
        proving: Option<Proving>,
        priority_fee_lamports: Option<u64>,
        cached: Vec<CachedOutput>,
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
            || {
                if cached.is_empty() {
                    self.select_notes(asset, total, proof_outputs(&outputs))
                } else {
                    self.select_cached_notes(&cached, asset, total, proof_outputs(&outputs))
                }
            },
        );
        let (legs, inputs) = (legs?, inputs?);
        let spends = nullifiers(&inputs);
        let mut transaction = ConfidentialTransaction::new(inputs, payer)?;
        if let Some(first) = cached.first() {
            transaction = transaction.with_output_tree_id(first.target.tree_id)?;
        }
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
        let (unsigned, openings) =
            self.prove_with_cache(transaction, settlements, payer, proving, &cached)?;
        let mut unsigned = with_setup(unsigned, &payer, &setup)?;
        unsigned.message = self.with_priority_fee(unsigned.message, priority_fee_lamports)?;
        let mut pending = self.proved(
            Operation::Spend {
                mint: mint_name(&asset.mint),
                outputs,
                created_token_accounts: created.iter().map(ToString::to_string).collect(),
            },
            unsigned,
        );
        pending.outputs = openings;
        pending.cache_inputs = cached;
        self.check_cached_transaction(&pending)?;
        self.reserve(pending, spends)
    }
    /// `outputs` with each recipient checked on chain: a private recipient
    /// is registered, and a public recipient of a token has its associated
    /// token account or gets one. SOL needs no account.
    pub(super) fn legs(
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
    pub(super) fn fee_payer(&self, fee_payer: Option<String>) -> Result<Pubkey, WalletError> {
        fee_payer.as_deref().map_or(Ok(self.owner), parse_pubkey)
    }
    /// Where a spend is proved: where `proving` says, or where the config
    /// does. Remote proving without a prover fails here, before the network.
    pub(super) fn proving(&self, proving: Option<Proving>) -> Result<Proving, WalletError> {
        let proving = proving.unwrap_or(self.proving);
        match (proving, &self.remote_prover) {
            (Proving::Remote, None) => Err(WalletError::RemoteProverMissing),
            _ => Ok(proving),
        }
    }
    /// Encrypt and prove `transaction` where `proving` says, and build the
    /// Solana message `payer` pays for.
    pub(super) fn prove_with_cache(
        &self,
        transaction: ConfidentialTransaction,
        settlement_transfers: Vec<TransactInterfaceTransferAccounts>,
        payer: Pubkey,
        proving: Proving,
        cached: &[CachedOutput],
    ) -> Result<(UnsignedMessage, Vec<OutputOpening>), WalletError> {
        let mut inputs = transaction.encrypt(&self.keys)?;
        if let Some(first) = cached.first() {
            inputs = inputs.with_read_cache(first.target.address()?);
            for input in inputs
                .input_utxos
                .iter_mut()
                .filter(|input| !input.is_dummy())
            {
                let output = cached
                    .iter()
                    .find(|c| c.commitment == input.utxo_hash)
                    .ok_or_else(|| invalid("cached input commitment mismatch"))?;
                *input = input.clone().with_cache_slot(output.target.slot)?;
            }
        }
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
}

/// A spend of `amount` the free notes cannot cover. The SDK reports an asset
/// without spendable notes by its mint alone; the wallet reports the amounts.
pub(super) fn spend_failure(amount: u64, failure: TransactionError) -> WalletError {
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
pub(super) fn proof_outputs(outputs: &[Output]) -> usize {
    let private = outputs
        .iter()
        .filter(|output| matches!(output, Output::Private { .. }))
        .count();
    (private + 1).max(2)
}

/// The most notes a spend with `outputs` outputs takes: the widest pinned
/// shape for them, within the Zolana SDK's selection cap.
pub(super) fn max_spend_inputs(outputs: usize) -> usize {
    zolana_client::SPP_SUPPORTED_SHAPES
        .iter()
        .filter(|shape| shape.n_outputs() >= outputs)
        .map(|shape| shape.n_inputs())
        .max()
        .unwrap_or(0)
        .min(MAX_SPEND_INPUTS)
}

/// `inputs` when a spend may take that many notes, else what to merge for.
pub(super) fn within_cap(
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

/// Compute units each created associated token account adds to a message's
/// limit: SPL's `CreateIdempotent` stays well under it, for SPL Token and
/// Token-2022 alike.
pub(super) const CREATE_TOKEN_ACCOUNT_UNITS: u32 = 40_000;

/// The most compute units a transaction can ask for.
pub(super) const MAX_COMPUTE_UNITS: u32 = 1_400_000;

/// `unsigned` with `setup` ahead of its instructions and its compute limit
/// raised for them. The client builds a v1 message of static keys, so its
/// instructions decompile exactly and compile again behind `setup`; the
/// blockhash, and with it the last valid block height, stay. The program
/// checks the accounts of its own instruction, not its position.
pub(super) fn with_setup(
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
