//! SOL and the SPL Token / Token-2022 mints the application configured, each
//! with its token program. The wallet reads nothing about a mint but the asset
//! id the shielded pool registered for it.

use std::{collections::HashMap, str::FromStr};

use solana_pubkey::Pubkey;
use zolana_client::{fetch_asset_id, ClientError, Rpc};
use zolana_interface::pda;
use zolana_transaction::{AssetRegistry, SOL_MINT};

use crate::error::WalletError;

/// An SPL mint the wallet holds, and the token program that owns it: SPL
/// Token or Token-2022. Both base58.
pub struct MintConfig {
    pub mint: String,
    pub token_program: String,
}

/// An asset the wallet can hold. `token_program` is `None` for SOL.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Asset {
    pub mint: Pubkey,
    pub token_program: Option<Pubkey>,
}

impl Asset {
    /// `owner`'s associated token account; `None` for SOL.
    pub fn token_account(&self, owner: &Pubkey) -> Option<Pubkey> {
        self.token_program
            .map(|program| pda::associated_token_address_with_program(owner, &self.mint, &program))
    }
}

/// The configured mints, and the asset ids read for them so far.
pub(crate) struct Assets {
    token_programs: HashMap<Pubkey, Pubkey>,
    registry: AssetRegistry,
}

impl Assets {
    pub fn new(mints: Vec<MintConfig>) -> Result<Self, WalletError> {
        let token_programs = mints
            .into_iter()
            .map(|config| {
                let mint = parse_mint(&config.mint)?;
                let token_program = Pubkey::from_str(&config.token_program)
                    .ok()
                    .filter(|program| {
                        *program == pda::spl_token_program_id()
                            || *program == pda::spl_token_2022_program_id()
                    })
                    .ok_or(WalletError::InvalidTokenProgram {
                        mint: config.mint,
                        token_program: config.token_program,
                    })?;
                Ok((mint, token_program))
            })
            .collect::<Result<_, WalletError>>()?;
        Ok(Self {
            token_programs,
            registry: AssetRegistry::default(),
        })
    }

    /// SOL for `None`, otherwise a configured mint, its asset id read the
    /// first time it is used.
    pub fn resolve(&mut self, rpc: &impl Rpc, mint: Option<&str>) -> Result<Asset, WalletError> {
        let Some(name) = mint else {
            return Ok(Asset {
                mint: SOL_MINT,
                token_program: None,
            });
        };
        let mint = parse_mint(name)?;
        let token_program = *self
            .token_programs
            .get(&mint)
            .ok_or_else(|| WalletError::MintNotConfigured { mint: name.into() })?;
        self.register(rpc, mint)?;
        Ok(Asset {
            mint,
            token_program: Some(token_program),
        })
    }

    /// SOL and every configured mint, each asset id read once.
    pub fn registry(&mut self, rpc: &impl Rpc) -> Result<&AssetRegistry, WalletError> {
        let mints: Vec<Pubkey> = self.token_programs.keys().copied().collect();
        for mint in mints {
            self.register(rpc, mint)?;
        }
        Ok(&self.registry)
    }

    fn register(&mut self, rpc: &impl Rpc, mint: Pubkey) -> Result<(), WalletError> {
        if self.registry.asset_id(&mint).is_ok() {
            return Ok(());
        }
        let asset_id = fetch_asset_id(rpc, mint).map_err(|failure| match failure {
            ClientError::SplAssetNotRegistered { .. }
            | ClientError::InvalidSplAssetRegistry { .. } => WalletError::AssetNotSupported {
                mint: mint.to_string(),
            },
            failure => failure.into(),
        })?;
        Ok(self.registry.insert(asset_id, mint)?)
    }
}

/// `None` for SOL, the base58 mint otherwise: how assets cross to Dart.
pub(crate) fn mint_name(mint: &Pubkey) -> Option<String> {
    (*mint != SOL_MINT).then(|| mint.to_string())
}

fn parse_mint(mint: &str) -> Result<Pubkey, WalletError> {
    Pubkey::from_str(mint).map_err(|_| WalletError::InvalidMint { mint: mint.into() })
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;

    use solana_account::Account;
    use zolana_interface::state::SplAssetRegistry;

    use super::*;

    /// The accounts it holds; counts the reads.
    #[derive(Default)]
    struct Accounts {
        accounts: HashMap<Pubkey, Account>,
        reads: Cell<usize>,
    }

    impl Accounts {
        fn with(mut self, address: Pubkey, owner: Pubkey, data: Vec<u8>) -> Self {
            self.accounts.insert(
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
            self.reads.set(self.reads.get() + 1);
            Ok(self.accounts.get(&address).cloned())
        }
    }

    const MINT: Pubkey = Pubkey::new_from_array([7; 32]);

    fn registered(asset_id: u64) -> Accounts {
        Accounts::default().with(
            pda::spl_asset_registry(&MINT),
            pda::shielded_pool_program_id(),
            SplAssetRegistry::account_bytes(MINT, asset_id).to_vec(),
        )
    }

    fn configured(token_program: Pubkey) -> Assets {
        Assets::new(vec![MintConfig {
            mint: MINT.to_string(),
            token_program: token_program.to_string(),
        }])
        .unwrap()
    }

    #[test]
    fn resolves_a_configured_mint_with_one_read() {
        let mut assets = configured(pda::spl_token_2022_program_id());
        let rpc = registered(2);
        let mint = MINT.to_string();
        let asset = assets.resolve(&rpc, Some(&mint)).unwrap();
        assert_eq!(asset.token_program, Some(pda::spl_token_2022_program_id()));
        assert_eq!(assets.resolve(&rpc, Some(&mint)).unwrap(), asset);
        assert_eq!(assets.registry(&rpc).unwrap().asset_id(&MINT).unwrap(), 2);
        assert_eq!(rpc.reads.get(), 1, "only the asset id is read, once");
        assert_eq!(assets.resolve(&rpc, None).unwrap().mint, SOL_MINT);
    }

    #[test]
    fn refuses_a_mint_it_was_not_given() {
        let mut assets = configured(pda::spl_token_program_id());
        let rpc = Accounts::default();
        let other = Pubkey::new_from_array([8; 32]).to_string();
        assert_eq!(
            assets.resolve(&rpc, Some(&other)).unwrap_err(),
            WalletError::MintNotConfigured { mint: other }
        );
        assert_eq!(
            assets.resolve(&rpc, Some("not-a-mint")).unwrap_err(),
            WalletError::InvalidMint {
                mint: "not-a-mint".into()
            }
        );
        assert_eq!(rpc.reads.get(), 0);
        let system = Pubkey::new_from_array([9; 32]).to_string();
        let config = || MintConfig {
            mint: MINT.to_string(),
            token_program: system.clone(),
        };
        assert_eq!(
            Assets::new(vec![config()]).err(),
            Some(WalletError::InvalidTokenProgram {
                mint: MINT.to_string(),
                token_program: system.clone(),
            })
        );
    }

    #[test]
    fn refuses_mints_the_pool_has_not_registered() {
        let mint = MINT.to_string();
        let resolve = |rpc: &Accounts| {
            configured(pda::spl_token_program_id())
                .resolve(rpc, Some(&mint))
                .unwrap_err()
        };
        let unsupported = WalletError::AssetNotSupported { mint: mint.clone() };
        assert_eq!(resolve(&Accounts::default()), unsupported);
        let forged = registered(2).with(
            pda::spl_asset_registry(&MINT),
            Pubkey::new_from_array([9; 32]),
            SplAssetRegistry::account_bytes(MINT, 2).to_vec(),
        );
        assert_eq!(resolve(&forged), unsupported);
        let invalid = Accounts::default().with(
            pda::spl_asset_registry(&MINT),
            pda::shielded_pool_program_id(),
            vec![1, 2, 3],
        );
        assert_eq!(resolve(&invalid), unsupported);
    }
}
