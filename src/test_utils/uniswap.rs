use crate::prelude::{Address, U256};
use crate::test_utils::solidity;
use crate::transaction::LegacyEthTransaction;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Once;

pub(crate) struct FactoryConstructor(pub solidity::ContractConstructor);

pub(crate) struct Factory(pub solidity::DeployedContract);

pub(crate) struct PositionManagerConstructor(pub solidity::ContractConstructor);

pub(crate) struct PositionManager(pub solidity::DeployedContract);

pub(crate) struct MintParams {
    pub token0: Address,
    pub token1: Address,
    pub fee: u64,
    pub tick_lower: i64,
    pub tick_upper: i64,
    pub amount0_desired: U256,
    pub amount1_desired: U256,
    pub amount0_min: U256,
    pub amount1_min: U256,
    pub recipient: Address,
    pub deadline: U256,
}

impl From<FactoryConstructor> for solidity::ContractConstructor {
    fn from(c: FactoryConstructor) -> Self {
        c.0
    }
}

impl From<PositionManagerConstructor> for solidity::ContractConstructor {
    fn from(c: PositionManagerConstructor) -> Self {
        c.0
    }
}

static DOWNLOAD_ONCE: Once = Once::new();

impl FactoryConstructor {
    pub fn load() -> Self {
        let artifact_path = uniswap_root_path().join(
            [
                "node_modules",
                "@uniswap",
                "v3-core",
                "artifacts",
                "contracts",
                "UniswapV3Factory.sol",
                "UniswapV3Factory.json",
            ]
            .iter()
            .collect::<PathBuf>(),
        );

        Self(load_constructor(artifact_path))
    }

    pub fn deploy(&self, nonce: U256) -> LegacyEthTransaction {
        self.0.deploy_without_args(nonce)
    }
}

impl PositionManagerConstructor {
    pub fn load() -> Self {
        let artifact_path = uniswap_root_path().join(
            [
                "node_modules",
                "@uniswap",
                "v3-periphery",
                "artifacts",
                "contracts",
                "NonfungiblePositionManager.sol",
                "NonfungiblePositionManager.json",
            ]
            .iter()
            .collect::<PathBuf>(),
        );

        Self(load_constructor(artifact_path))
    }

    pub fn deploy(
        &self,
        factory: Address,
        wrapped_eth: Address,
        token_descriptor: Address,
        nonce: U256,
    ) -> LegacyEthTransaction {
        let data = self
            .0
            .abi
            .constructor()
            .unwrap()
            .encode_input(
                self.0.code.clone(),
                &[
                    ethabi::Token::Address(factory),
                    ethabi::Token::Address(wrapped_eth),
                    ethabi::Token::Address(token_descriptor),
                ],
            )
            .unwrap();
        LegacyEthTransaction {
            nonce,
            gas_price: Default::default(),
            gas: u64::MAX.into(),
            to: None,
            value: Default::default(),
            data,
        }
    }
}

impl Factory {
    pub fn create_pool(
        &self,
        token_a: Address,
        token_b: Address,
        fee: U256,
        nonce: U256,
    ) -> LegacyEthTransaction {
        let data = self
            .0
            .abi
            .function("createPool")
            .unwrap()
            .encode_input(&[
                ethabi::Token::Address(token_a),
                ethabi::Token::Address(token_b),
                ethabi::Token::Uint(fee),
            ])
            .unwrap();

        LegacyEthTransaction {
            nonce,
            gas_price: Default::default(),
            gas: u64::MAX.into(),
            to: Some(self.0.address),
            value: Default::default(),
            data,
        }
    }
}

impl PositionManager {
    pub fn mint(&self, params: MintParams, nonce: U256) -> LegacyEthTransaction {
        let tick_lower = U256::from(u64::from_le_bytes(params.tick_lower.to_le_bytes()));
        let tick_upper = U256::from(u64::from_le_bytes(params.tick_upper.to_le_bytes()));
        let data = self
            .0
            .abi
            .function("mint")
            .unwrap()
            .encode_input(&[ethabi::Token::Tuple(vec![
                ethabi::Token::Address(params.token0),
                ethabi::Token::Address(params.token1),
                ethabi::Token::Uint(params.fee.into()),
                ethabi::Token::Int(tick_lower),
                ethabi::Token::Int(tick_upper),
                ethabi::Token::Uint(params.amount0_desired),
                ethabi::Token::Uint(params.amount1_desired),
                ethabi::Token::Uint(params.amount0_min),
                ethabi::Token::Uint(params.amount1_min),
                ethabi::Token::Address(params.recipient),
                ethabi::Token::Uint(params.deadline),
            ])])
            .unwrap();

        LegacyEthTransaction {
            nonce,
            gas_price: Default::default(),
            gas: u64::MAX.into(),
            to: Some(self.0.address),
            value: Default::default(),
            data,
        }
    }
}

fn load_constructor(artifact_path: PathBuf) -> solidity::ContractConstructor {
    if !artifact_path.exists() {
        download_uniswap_artifacts();
    }

    solidity::ContractConstructor::compile_from_extended_json(artifact_path)
}

fn uniswap_root_path() -> PathBuf {
    Path::new("etc").join("uniswap")
}

fn download_uniswap_artifacts() {
    DOWNLOAD_ONCE.call_once(|| {
        let output = Command::new("/usr/bin/env")
            .current_dir(&uniswap_root_path())
            .args(&["yarn", "install"])
            .output()
            .unwrap();

        if !output.status.success() {
            panic!(
                "Downloading uniswap npm package failed.\n{}",
                String::from_utf8(output.stderr).unwrap()
            );
        }
    });
}
