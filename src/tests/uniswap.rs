use crate::prelude::Address;
use crate::test_utils::{
    self,
    uniswap::{Factory, FactoryConstructor},
};
use crate::types::Wei;
use secp256k1::SecretKey;

const INITIAL_BALANCE: u64 = 1000;
const INITIAL_NONCE: u64 = 0;

#[test]
fn it_works() {
    initialize_uniswap_factory();
}

fn initialize_uniswap_factory() -> (test_utils::AuroraRunner, SecretKey, Address, Factory) {
    // set up Aurora runner and accounts
    let mut runner = test_utils::deploy_evm();
    let mut rng = rand::thread_rng();
    let source_account = SecretKey::random(&mut rng);
    let source_address = test_utils::address_from_secret_key(&source_account);
    runner.create_address(
        source_address,
        Wei::new_u64(INITIAL_BALANCE),
        INITIAL_NONCE.into(),
    );
    let dest_address = test_utils::address_from_secret_key(&SecretKey::random(&mut rng));

    let constructor = FactoryConstructor::load();
    let contract = Factory(runner.deploy_contract(
        &source_account,
        |c| c.deploy(INITIAL_NONCE.into()),
        constructor,
    ));

    (runner, source_account, dest_address, contract)
}
