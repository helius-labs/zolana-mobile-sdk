//! The application's public fee on a deposit or a withdrawal: a plain
//! transfer of the same asset from the wallet's account to the application's
//! collector, in the same transaction as the shielded-pool instruction, so
//! both land or neither does.
//!
//! The transfer rides after the pool instruction. The pool's proof binds the
//! fee payer, the signers and the pool instruction's own data, not the other
//! instructions of the message, so appending one does not touch the proof.

use solana_instruction::{AccountMeta, Instruction};
use solana_message::VersionedMessage;
use solana_pubkey::Pubkey;
use zolana_client::{compile_message, ClientError, ComputeBudgetConfig};
use zolana_interface::SPL_TOKEN_TRANSFER_CHECKED_DISCRIMINATOR;

use crate::{asset::Asset, error::WalletError};

/// The System Program's `Transfer` instruction index.
const SYSTEM_TRANSFER: u32 = 2;
/// Where a mint keeps its decimals, in SPL Token and Token-2022 alike: after
/// the optional authority (36 bytes) and the supply (8).
const MINT_DECIMALS_OFFSET: usize = 44;

/// The transfer of `amount` from `owner` to `collector`, who receives it in
/// the same asset: lamports for SOL, otherwise tokens from `owner`'s
/// associated token account to the collector's. `decimals` is the mint's, and
/// only a token transfer reads it.
pub(crate) fn transfer(
    asset: &Asset,
    owner: &Pubkey,
    collector: &Pubkey,
    amount: u64,
    decimals: u8,
) -> Instruction {
    let (Some(program), Some(from), Some(to)) = (
        asset.token_program,
        asset.token_account(owner),
        asset.token_account(collector),
    ) else {
        let mut data = SYSTEM_TRANSFER.to_le_bytes().to_vec();
        data.extend_from_slice(&amount.to_le_bytes());
        return Instruction {
            program_id: Pubkey::default(),
            accounts: vec![
                AccountMeta::new(*owner, true),
                AccountMeta::new(*collector, false),
            ],
            data,
        };
    };
    let mut data = vec![SPL_TOKEN_TRANSFER_CHECKED_DISCRIMINATOR];
    data.extend_from_slice(&amount.to_le_bytes());
    data.push(decimals);
    Instruction {
        program_id: program,
        accounts: vec![
            AccountMeta::new(from, false),
            AccountMeta::new_readonly(asset.mint, false),
            AccountMeta::new(to, false),
            AccountMeta::new_readonly(*owner, true),
        ],
        data,
    }
}

/// What a fee transfer may add to a transaction's compute units: a System
/// transfer takes a few hundred, a Token-2022 `TransferChecked` with
/// extensions tens of thousands.
const TRANSFER_COMPUTE_UNITS: u32 = 50_000;
const MAX_COMPUTE_UNITS: u32 = 1_400_000;

/// `budget` with room for the fee transfer.
pub(crate) fn budget(budget: ComputeBudgetConfig) -> ComputeBudgetConfig {
    ComputeBudgetConfig {
        cu_limit: budget
            .cu_limit
            .saturating_add(TRANSFER_COMPUTE_UNITS)
            .min(MAX_COMPUTE_UNITS),
        ..budget
    }
}

/// The decimals recorded in a mint account's data.
pub(crate) fn mint_decimals(mint: &Pubkey, data: &[u8]) -> Result<u8, WalletError> {
    data.get(MINT_DECIMALS_OFFSET)
        .copied()
        .ok_or_else(|| WalletError::InvalidMint {
            mint: mint.to_string(),
        })
}

/// `message` with `instruction` after its own, under the same fee payer,
/// blockhash and compute budget header `budget`. Every account keeps the
/// signer and writable roles the message gave it, and `instruction` adds its
/// own.
pub(crate) fn append(
    message: &VersionedMessage,
    instruction: Instruction,
    budget: ComputeBudgetConfig,
) -> Result<VersionedMessage, ClientError> {
    let VersionedMessage::V1(v1) = message else {
        return Err(ClientError::UnsupportedTransactionVersion);
    };
    let keys = &v1.account_keys;
    let signers = usize::from(v1.header.num_required_signatures);
    let readonly_signed = usize::from(v1.header.num_readonly_signed_accounts);
    let readonly_unsigned = usize::from(v1.header.num_readonly_unsigned_accounts);
    let meta = |index: u8| {
        let index = usize::from(index);
        let writable = if index < signers {
            index < signers - readonly_signed
        } else {
            index < keys.len() - readonly_unsigned
        };
        AccountMeta {
            pubkey: keys[index],
            is_signer: index < signers,
            is_writable: writable,
        }
    };
    let mut instructions: Vec<Instruction> = v1
        .instructions
        .iter()
        .map(|compiled| Instruction {
            program_id: keys[usize::from(compiled.program_id_index)],
            accounts: compiled.accounts.iter().map(|&index| meta(index)).collect(),
            data: compiled.data.clone(),
        })
        .collect();
    instructions.push(instruction);
    compile_message(&keys[0], &instructions, v1.lifetime_specifier, budget)
}

#[cfg(test)]
mod tests {
    use solana_hash::Hash;
    use zolana_interface::pda;

    use super::*;

    const OWNER: Pubkey = Pubkey::new_from_array([1; 32]);
    const COLLECTOR: Pubkey = Pubkey::new_from_array([2; 32]);
    const MINT: Pubkey = Pubkey::new_from_array([3; 32]);

    fn token(program: Pubkey) -> Asset {
        Asset {
            mint: MINT,
            token_program: Some(program),
        }
    }

    #[test]
    fn a_sol_fee_is_a_system_transfer_from_the_owner() {
        let sol = Asset {
            mint: zolana_transaction::SOL_MINT,
            token_program: None,
        };
        let instruction = transfer(&sol, &OWNER, &COLLECTOR, 5, 9);
        assert_eq!(instruction.program_id, Pubkey::default());
        assert_eq!(instruction.data, [2, 0, 0, 0, 5, 0, 0, 0, 0, 0, 0, 0]);
        assert_eq!(
            instruction.accounts,
            [
                AccountMeta::new(OWNER, true),
                AccountMeta::new(COLLECTOR, false)
            ]
        );
    }

    #[test]
    fn a_token_fee_moves_between_associated_token_accounts_under_either_program() {
        for program in [
            pda::spl_token_program_id(),
            pda::spl_token_2022_program_id(),
        ] {
            let asset = token(program);
            let instruction = transfer(&asset, &OWNER, &COLLECTOR, 7, 6);
            assert_eq!(instruction.program_id, program);
            assert_eq!(instruction.data, [12, 7, 0, 0, 0, 0, 0, 0, 0, 6]);
            assert_eq!(
                instruction.accounts,
                [
                    AccountMeta::new(asset.token_account(&OWNER).unwrap(), false),
                    AccountMeta::new_readonly(MINT, false),
                    AccountMeta::new(asset.token_account(&COLLECTOR).unwrap(), false),
                    AccountMeta::new_readonly(OWNER, true),
                ]
            );
        }
    }

    #[test]
    fn reads_the_decimals_of_a_mint() {
        let mut data = vec![0; 82];
        data[MINT_DECIMALS_OFFSET] = 6;
        assert_eq!(mint_decimals(&MINT, &data), Ok(6));
        assert_eq!(
            mint_decimals(&MINT, &[0; 10]),
            Err(WalletError::InvalidMint {
                mint: MINT.to_string()
            })
        );
    }

    /// The pool instruction keeps its accounts, roles and data when a fee
    /// follows it, and the header and blockhash stay.
    #[test]
    fn appends_an_instruction_without_changing_the_first() {
        let payer = Pubkey::new_from_array([9; 32]);
        let program = Pubkey::new_from_array([8; 32]);
        let pool = Instruction {
            program_id: program,
            accounts: vec![
                AccountMeta::new(payer, true),
                AccountMeta::new_readonly(OWNER, true),
                AccountMeta::new(Pubkey::new_from_array([4; 32]), false),
                AccountMeta::new_readonly(Pubkey::new_from_array([5; 32]), false),
            ],
            data: vec![1, 2, 3],
        };
        let budget = ComputeBudgetConfig::new(400_000).with_priority_fee(7);
        let blockhash = Hash::new_from_array([6; 32]);
        let before =
            compile_message(&payer, std::slice::from_ref(&pool), blockhash, budget).unwrap();
        let sol = Asset {
            mint: zolana_transaction::SOL_MINT,
            token_program: None,
        };
        let fee = transfer(&sol, &OWNER, &COLLECTOR, 5, 9);
        let after = append(&before, fee.clone(), budget).unwrap();

        let expected = compile_message(&payer, &[pool, fee], blockhash, budget).unwrap();
        assert_eq!(after, expected);
        let (VersionedMessage::V1(before), VersionedMessage::V1(after)) = (&before, &after) else {
            panic!("v1 messages")
        };
        assert_eq!(after.instructions[0].data, before.instructions[0].data);
        assert_eq!(after.instructions.len(), 2);
        assert_eq!(after.lifetime_specifier, blockhash);
        assert_eq!(after.config, before.config);
        // The owner who signs the pool instruction also signs the fee, and the
        // fee payer stays first.
        assert_eq!(after.account_keys[0], payer);
        assert_eq!(after.header.num_required_signatures, 2);
        assert!(after.account_keys.contains(&COLLECTOR));
    }

    #[test]
    fn the_fee_adds_compute_units_up_to_the_ceiling() {
        let base = ComputeBudgetConfig::new(450_000).with_priority_fee(3);
        assert_eq!(
            budget(base),
            ComputeBudgetConfig::new(500_000).with_priority_fee(3)
        );
        assert_eq!(
            budget(ComputeBudgetConfig::new(1_390_000)).cu_limit,
            1_400_000
        );
    }

    #[test]
    fn refuses_a_message_that_is_not_v1() {
        let legacy = VersionedMessage::Legacy(Default::default());
        assert!(matches!(
            append(
                &legacy,
                transfer(
                    &Asset {
                        mint: zolana_transaction::SOL_MINT,
                        token_program: None
                    },
                    &OWNER,
                    &COLLECTOR,
                    1,
                    9
                ),
                ComputeBudgetConfig::new(1)
            ),
            Err(ClientError::UnsupportedTransactionVersion)
        ));
    }
}
