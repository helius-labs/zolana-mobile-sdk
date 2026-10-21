//! SOL and the SPL Token / Token-2022 mints the wallet holds, each with its
//! token program. The application lists the mints, or the wallet discovers
//! the ones the shielded pool registered as it meets them. Of a mint the
//! wallet reads only the asset id the pool registered for it and, when it
//! discovers the mint, the token program that owns it.

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

/// The mints the wallet may use, with their token programs, and the asset ids
/// read for them so far.
pub(crate) struct Assets {
    /// The configured mints, or the ones discovered so far.
    token_programs: HashMap<Pubkey, Pubkey>,
    /// Whether a mint outside `token_programs` is looked up rather than
    /// refused.
    discover: bool,
    registry: AssetRegistry,
}

impl Assets {
    /// The listed mints only, or, for `None`, every mint the pool registered.
    pub fn new(mints: Option<Vec<MintConfig>>) -> Result<Self, WalletError> {
        let discover = mints.is_none();
        let token_programs = mints
            .unwrap_or_default()
            .into_iter()
            .map(|config| {
                let mint = parse_mint(&config.mint)?;
                let token_program = Pubkey::from_str(&config.token_program)
                    .ok()
                    .filter(is_token_program)
                    .ok_or(WalletError::InvalidTokenProgram {
                        mint: config.mint,
                        token_program: config.token_program,
                    })?;
                Ok((mint, token_program))
            })
            .collect::<Result<_, WalletError>>()?;
        Ok(Self {
            token_programs,
            discover,
            registry: AssetRegistry::default(),
        })
    }

    pub fn discovers(&self) -> bool {
        self.discover
    }

    /// SOL for `None`, otherwise a known mint, its asset id read the first
    /// time it is used. A mint the pool has not registered fails with
    /// [`WalletError::AssetNotSupported`].
    pub fn resolve(&mut self, rpc: &impl Rpc, mint: Option<&str>) -> Result<Asset, WalletError> {
        let asset = self.asset(rpc, mint)?;
        if asset.token_program.is_some() && !self.registered(rpc, asset.mint)? {
            return Err(WalletError::AssetNotSupported {
                mint: asset.mint.to_string(),
            });
        }
        Ok(asset)
    }

    /// SOL for `None`, otherwise a known mint, with whether the pool has
    /// registered it: a deposit of a mint it has not registered creates the
    /// mint's registry entry and vault first.
    pub fn resolve_for_deposit(
        &mut self,
        rpc: &impl Rpc,
        mint: Option<&str>,
    ) -> Result<(Asset, bool), WalletError> {
        let asset = self.asset(rpc, mint)?;
        let registered = asset.token_program.is_none() || self.registered(rpc, asset.mint)?;
        Ok((asset, registered))
    }

    /// SOL for `None`, otherwise a known mint and its token program, without
    /// a request. When the wallet discovers, it reads an unknown mint's token
    /// program once: the program that owns the mint account, which must be
    /// SPL Token or Token-2022.
    pub fn asset(&mut self, rpc: &impl Rpc, mint: Option<&str>) -> Result<Asset, WalletError> {
        let Some(name) = mint else {
            return Ok(Asset {
                mint: SOL_MINT,
                token_program: None,
            });
        };
        let mint = parse_mint(name)?;
        let token_program = match self.token_programs.get(&mint) {
            Some(program) => *program,
            None if self.discover => {
                let program = rpc
                    .get_account(mint)?
                    .map(|account| account.owner)
                    .filter(is_token_program)
                    .ok_or_else(|| WalletError::AssetNotSupported { mint: name.into() })?;
                self.token_programs.insert(mint, program);
                program
            }
            None => return Err(WalletError::MintNotConfigured { mint: name.into() }),
        };
        Ok(Asset {
            mint,
            token_program: Some(token_program),
        })
    }

    /// SOL and the known mints the pool has registered, each asset id read
    /// once. A mint without a usable registry entry has no notes, so it is
    /// left out rather than failing the read, and asked again next time,
    /// since a deposit may register it.
    pub fn registry(&mut self, rpc: &impl Rpc) -> Result<&AssetRegistry, WalletError> {
        let mints: Vec<Pubkey> = self.token_programs.keys().copied().collect();
        for mint in mints {
            match self.registered(rpc, mint) {
                Ok(_) | Err(WalletError::AssetNotSupported { .. }) => {}
                Err(failure) => return Err(failure),
            }
        }
        Ok(&self.registry)
    }

    /// Keep the validated mappings returned by the SDK's successful scan.
    /// Token programs remain lazy: `asset` reads one when a call names a mint.
    pub fn remember_registry(&mut self, registry: AssetRegistry) {
        self.registry = registry;
    }

    /// Whether the pool has registered `mint`, reading its asset id the
    /// first time. `false` when there is no registry entry, including an
    /// account at the registry address that only holds lamports, which the
    /// pool's create path accepts. An entry the pool owns but that does not
    /// decode as this mint's fails with [`WalletError::AssetNotSupported`].
    fn registered(&mut self, rpc: &impl Rpc, mint: Pubkey) -> Result<bool, WalletError> {
        if self.registry.asset_id(&mint).is_ok() {
            return Ok(true);
        }
        let asset_id = match fetch_asset_id(rpc, mint) {
            Ok(asset_id) => asset_id,
            Err(ClientError::SplAssetNotRegistered { .. }) => return Ok(false),
            Err(ClientError::InvalidSplAssetRegistry { .. }) => {
                return Err(WalletError::AssetNotSupported {
                    mint: mint.to_string(),
                })
            }
            Err(failure) => return Err(failure.into()),
        };
        self.registry.insert(asset_id, mint)?;
        Ok(true)
    }
}

/// `None` for SOL, the base58 mint otherwise: how assets cross to Dart.
pub(crate) fn mint_name(mint: &Pubkey) -> Option<String> {
    (*mint != SOL_MINT).then(|| mint.to_string())
}

fn is_token_program(program: &Pubkey) -> bool {
    *program == pda::spl_token_program_id() || *program == pda::spl_token_2022_program_id()
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
            self.accounts.insert(address, account(owner, data));
            self
        }
    }

    impl Rpc for Accounts {
        fn get_account(&self, address: Pubkey) -> Result<Option<Account>, ClientError> {
            self.reads.set(self.reads.get() + 1);
            Ok(self.accounts.get(&address).cloned())
        }
    }

    fn account(owner: Pubkey, data: Vec<u8>) -> Account {
        Account {
            lamports: 1,
            data,
            owner,
            executable: false,
            rent_epoch: 0,
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
        Assets::new(Some(vec![MintConfig {
            mint: MINT.to_string(),
            token_program: token_program.to_string(),
        }]))
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
            Assets::new(Some(vec![config()])).err(),
            Some(WalletError::InvalidTokenProgram {
                mint: MINT.to_string(),
                token_program: system.clone(),
            })
        );
    }

    /// A configured mint without a usable registry entry has no notes: the
    /// registry leaves it out instead of failing, and asks again next time.
    /// A deposit learns whether it must register the mint first.
    #[test]
    fn leaves_out_a_mint_the_pool_has_not_registered() {
        let mint = MINT.to_string();
        let lamports_only = Accounts::default().with(
            pda::spl_asset_registry(&MINT),
            Pubkey::default(),
            Vec::new(),
        );
        let invalid = Accounts::default().with(
            pda::spl_asset_registry(&MINT),
            pda::shielded_pool_program_id(),
            vec![1, 2, 3],
        );
        for (rpc, registers) in [
            (Accounts::default(), Some(false)),
            (lamports_only, Some(false)),
            (invalid, None),
        ] {
            let mut assets = configured(pda::spl_token_program_id());
            assert!(assets.registry(&rpc).unwrap().asset_id(&MINT).is_err());
            assets.registry(&rpc).unwrap();
            assert_eq!(rpc.reads.get(), 2, "asked again on the next read");
            let deposit = assets.resolve_for_deposit(&rpc, Some(&mint));
            match registers {
                Some(registered) => assert_eq!(deposit.unwrap().1, registered),
                None => assert_eq!(
                    deposit.unwrap_err(),
                    WalletError::AssetNotSupported { mint: mint.clone() }
                ),
            }
        }
        let mut assets = configured(pda::spl_token_program_id());
        let rpc = registered(2);
        assert!(assets.resolve_for_deposit(&rpc, Some(&mint)).unwrap().1);
        assert_eq!(assets.registry(&rpc).unwrap().asset_id(&MINT).unwrap(), 2);
        assert_eq!(rpc.reads.get(), 1);
        let (sol, registered) = assets.resolve_for_deposit(&rpc, None).unwrap();
        assert!(sol.token_program.is_none() && registered);
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

    #[test]
    fn discovers_a_mint_a_call_names() {
        let mut assets = Assets::new(None).unwrap();
        assert!(assets.discovers());
        let rpc = registered(2).with(MINT, pda::spl_token_2022_program_id(), Vec::new());
        let mint = MINT.to_string();
        let asset = assets.resolve(&rpc, Some(&mint)).unwrap();
        assert_eq!(asset.token_program, Some(pda::spl_token_2022_program_id()));
        assert_eq!(assets.resolve(&rpc, Some(&mint)).unwrap(), asset);
        assert_eq!(assets.registry(&rpc).unwrap().asset_id(&MINT).unwrap(), 2);
        assert_eq!(
            rpc.reads.get(),
            2,
            "the token program and the asset id, once"
        );

        let unsupported = WalletError::AssetNotSupported { mint: mint.clone() };
        let discover = |rpc: &Accounts| Assets::new(None).unwrap().resolve(rpc, Some(&mint));
        assert_eq!(discover(&Accounts::default()), Err(unsupported.clone()));
        let not_a_token = registered(2).with(MINT, Pubkey::new_from_array([9; 32]), Vec::new());
        assert_eq!(discover(&not_a_token), Err(unsupported));
        let unregistered = Accounts::default().with(MINT, pda::spl_token_program_id(), Vec::new());
        let (asset, registered) = Assets::new(None)
            .unwrap()
            .resolve_for_deposit(&unregistered, Some(&mint))
            .unwrap();
        assert_eq!(asset.token_program, Some(pda::spl_token_program_id()));
        assert!(!registered, "its first deposit registers it");
        assert!(!configured(pda::spl_token_program_id()).discovers());
    }

    #[test]
    fn a_discovered_registry_keeps_token_program_lookup_lazy() {
        let mut assets = Assets::new(None).unwrap();
        assets.remember_registry(AssetRegistry::new([(2, MINT)]).unwrap());
        // There is no registry account in this RPC: the scan already read it.
        let rpc = Accounts::default().with(MINT, pda::spl_token_2022_program_id(), Vec::new());
        let asset = assets.resolve(&rpc, Some(&MINT.to_string())).unwrap();
        assert_eq!(asset.token_program, Some(pda::spl_token_2022_program_id()));
        assert_eq!(assets.registry(&rpc).unwrap().asset_id(&MINT).unwrap(), 2);
        assert_eq!(rpc.reads.get(), 1, "only the mint's token program is read");
    }
}
