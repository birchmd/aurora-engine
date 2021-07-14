use crate::prelude::U256;
use crate::test_utils::solidity;
use crate::transaction::LegacyEthTransaction;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Once;

pub(crate) struct FactoryConstructor(pub solidity::ContractConstructor);

pub(crate) struct Factory(pub solidity::DeployedContract);

impl From<FactoryConstructor> for solidity::ContractConstructor {
    fn from(c: FactoryConstructor) -> Self {
        c.0
    }
}

static DOWNLOAD_ONCE: Once = Once::new();

impl FactoryConstructor {
    pub fn load() -> Self {
        let artifact_path = Self::download_artifact();
        let constructor = solidity::ContractConstructor::compile_from_extended_json(artifact_path);
        Self(constructor)
    }

    pub fn deploy(&self, nonce: U256) -> LegacyEthTransaction {
        self.0.deploy_without_args(nonce)
    }

    fn download_artifact() -> PathBuf {
        let uniswap_root_path = Path::new("etc").join("uniswap");
        let artifact_path = uniswap_root_path.join(
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

        if !artifact_path.exists() {
            DOWNLOAD_ONCE.call_once(|| {
                let output = Command::new("/usr/bin/env")
                    .current_dir(&uniswap_root_path)
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

        artifact_path
    }
}
