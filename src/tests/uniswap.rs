use crate::prelude::Address;
use crate::test_utils::{
    self,
    erc20::{ERC20Constructor, ERC20},
    uniswap::{Factory, FactoryConstructor},
    Signer,
};
use crate::types::Wei;
use secp256k1::SecretKey;

const INITIAL_BALANCE: u64 = 1000;
const INITIAL_NONCE: u64 = 0;

#[test]
fn it_works() {
    let (mut runner, mut signer, factory) = initialize_uniswap_factory();

    let token_a = create_token("token_a", "A", &mut runner, &mut signer);
    let token_b = create_token("token_b", "B", &mut runner, &mut signer);

    let result = create_pool(
        token_a.0.address,
        token_b.0.address,
        &factory,
        &mut runner,
        &mut signer,
    );

    panic!("{:?}", result);
}

fn create_pool(
    token_a: Address,
    token_b: Address,
    factory: &Factory,
    runner: &mut test_utils::AuroraRunner,
    signer: &mut Signer,
) -> Address {
    let nonce = signer.use_nonce();
    // The "fee" can only be specific values, see
    // https://github.com/Uniswap/uniswap-v3-core/blob/main/contracts/UniswapV3Factory.sol#L26
    let create_pool_tx = factory.create_pool(token_a, token_b, 500.into(), nonce.into());
    let result = runner
        .submit_transaction(&signer.secret_key, create_pool_tx)
        .unwrap();
    assert!(result.status, "Failed to create pool");

    let mut address = [0u8; 20];
    address.copy_from_slice(&result.result[12..]);

    Address(address)
}

fn create_token(
    name: &str,
    symbol: &str,
    runner: &mut test_utils::AuroraRunner,
    signer: &mut Signer,
) -> ERC20 {
    let constructor = ERC20Constructor::load();
    let nonce = signer.use_nonce();
    let contract = ERC20(runner.deploy_contract(
        &signer.secret_key,
        |c| c.deploy(name, symbol, nonce.into()),
        constructor,
    ));

    let nonce = signer.use_nonce();
    let mint_tx = contract.mint(
        test_utils::address_from_secret_key(&signer.secret_key),
        1_000_000.into(),
        nonce.into(),
    );
    let result = runner
        .submit_transaction(&signer.secret_key, mint_tx)
        .unwrap();
    assert!(result.status, "Minting ERC-20 tokens failed");

    contract
}

fn initialize_uniswap_factory() -> (test_utils::AuroraRunner, Signer, Factory) {
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

    let constructor = FactoryConstructor::load();
    let contract = Factory(runner.deploy_contract(
        &source_account,
        |c| c.deploy(INITIAL_NONCE.into()),
        constructor,
    ));
    let signer = Signer {
        nonce: INITIAL_NONCE + 1,
        secret_key: source_account,
    };

    (runner, signer, contract)
}
