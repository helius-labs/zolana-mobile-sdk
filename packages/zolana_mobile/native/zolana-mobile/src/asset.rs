//! SOL and the SPL Token / Token-2022 mints the shielded pool has registered.

use std::str::FromStr;

use solana_pubkey::Pubkey;
use zolana_client::Rpc;
use zolana_interface::{pda, state::SplAssetRegistry};
use zolana_transaction::{AssetRegistry, SOL_MINT};

use crate::wallet::error;

/// An asset the wallet can hold. `token_program` is `None` for SOL.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Asset {
    pub mint: Pubkey,
    pub token_program: Option<Pubkey>,
}

impl Asset {
    pub const SOL: Self = Self {
        mint: SOL_MINT,
        token_program: None,
    };

    /// SOL for `None`, otherwise the mint, added to `registry` from its
    /// on-chain pool registration when the wallet has not seen it yet.
    pub fn resolve(
        rpc: &impl Rpc,
        registry: &mut AssetRegistry,
        mint: Option<&str>,
    ) -> Result<Self, String> {
        let Some(mint) = mint else {
            return Ok(Self::SOL);
        };
        let mint = parse_mint(mint)?;
        let token_program = rpc
            .get_account(mint)
            .map_err(error)?
            .ok_or("mint_not_found")?
            .owner;
        if token_program != pda::spl_token_program_id()
            && token_program != pda::spl_token_2022_program_id()
        {
            return Err("mint_invalid".to_string());
        }
        if registry.asset_id(&mint).is_err() {
            let asset_id = registered_asset_id(rpc, mint)?;
            registry.insert(asset_id, mint).map_err(error)?;
        }
        Ok(Self {
            mint,
            token_program: Some(token_program),
        })
    }

    /// `owner`'s associated token account; `None` for SOL.
    pub fn token_account(&self, owner: &Pubkey) -> Option<Pubkey> {
        self.token_program
            .map(|program| pda::associated_token_address_with_program(owner, &self.mint, &program))
    }

    /// `amount` in base units, for transaction summaries.
    pub fn describe(&self, amount: u64) -> String {
        if self.token_program.is_none() {
            format!("{amount} lamports")
        } else {
            format!("{amount} base units of {}", self.mint)
        }
    }
}

/// `None` for SOL, the base58 mint otherwise: how assets cross to Dart.
pub(crate) fn mint_name(mint: &Pubkey) -> Option<String> {
    (*mint != SOL_MINT).then(|| mint.to_string())
}

pub(crate) fn parse_mint(mint: &str) -> Result<Pubkey, String> {
    Pubkey::from_str(mint).map_err(|_| "mint_invalid".to_string())
}

/// The token amount of an SPL Token or Token-2022 account; both put it at
/// bytes 64..72.
pub(crate) fn token_account_amount(data: &[u8]) -> Result<u64, String> {
    data.get(64..72)
        .and_then(|bytes| bytes.try_into().ok())
        .map(u64::from_le_bytes)
        .ok_or_else(|| "token_account_invalid".to_string())
}

/// The asset id the shielded pool assigned to `mint`, read from its registry
/// account. A mint without one cannot enter the pool.
fn registered_asset_id(rpc: &impl Rpc, mint: Pubkey) -> Result<u64, String> {
    let unsupported = || "asset_not_supported".to_string();
    let account = rpc
        .get_account(pda::spl_asset_registry(&mint))
        .map_err(error)?
        .ok_or_else(unsupported)?;
    if account.owner != pda::shielded_pool_program_id() {
        return Err(unsupported());
    }
    let record = SplAssetRegistry::from_account_bytes(&account.data).map_err(|_| unsupported())?;
    if record.mint != mint {
        return Err(unsupported());
    }
    Ok(record.asset_id)
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use solana_account::Account;
    use zolana_client::ClientError;

    use super::*;

    #[derive(Default)]
    struct Accounts(HashMap<Pubkey, Account>);

    impl Accounts {
        fn with(mut self, address: Pubkey, owner: Pubkey, data: Vec<u8>) -> Self {
            self.0.insert(
                address,
                Account {
                    lamports: 1,
                    data,
                    owner,
                    executable: false,
                    rent_epoch: 0,
                },
            );
            self
        }
    }

    impl Rpc for Accounts {
        fn get_account(&self, address: Pubkey) -> Result<Option<Account>, ClientError> {
            Ok(self.0.get(&address).cloned())
        }
    }

    const MINT: Pubkey = Pubkey::new_from_array([7; 32]);

    fn registered(asset_id: u64) -> Accounts {
        Accounts::default()
            .with(MINT, pda::spl_token_program_id(), vec![0; 82])
            .with(
                pda::spl_asset_registry(&MINT),
                pda::shielded_pool_program_id(),
                SplAssetRegistry::account_bytes(MINT, asset_id).to_vec(),
            )
    }

    #[test]
    fn resolves_a_registered_mint_into_the_wallet_registry() {
        let mut registry = AssetRegistry::default();
        let mint = MINT.to_string();
        let asset = Asset::resolve(&registered(2), &mut registry, Some(&mint)).unwrap();
        assert_eq!(asset.token_program, Some(pda::spl_token_program_id()));
        assert_eq!(registry.asset_id(&MINT).unwrap(), 2);
        assert_eq!(
            Asset::resolve(&registered(2), &mut registry, None).unwrap(),
            Asset::SOL
        );
    }

    #[test]
    fn refuses_mints_the_pool_has_not_registered() {
        let mint = MINT.to_string();
        let resolve = |rpc: &Accounts| {
            Asset::resolve(rpc, &mut AssetRegistry::default(), Some(&mint)).unwrap_err()
        };
        assert_eq!(resolve(&Accounts::default()), "mint_not_found");
        let not_a_mint = Accounts::default().with(MINT, Pubkey::new_from_array([9; 32]), vec![]);
        assert_eq!(resolve(&not_a_mint), "mint_invalid");
        let unregistered =
            Accounts::default().with(MINT, pda::spl_token_2022_program_id(), vec![0; 82]);
        assert_eq!(resolve(&unregistered), "asset_not_supported");
        let forged = registered(2).with(
            pda::spl_asset_registry(&MINT),
            Pubkey::new_from_array([9; 32]),
            SplAssetRegistry::account_bytes(MINT, 2).to_vec(),
        );
        assert_eq!(resolve(&forged), "asset_not_supported");
    }

    #[test]
    fn reads_token_account_amounts() {
        let mut data = vec![0; 165];
        data[64..72].copy_from_slice(&42u64.to_le_bytes());
        assert_eq!(token_account_amount(&data).unwrap(), 42);
        assert_eq!(
            token_account_amount(&data[..70]).unwrap_err(),
            "token_account_invalid"
        );
    }
}
