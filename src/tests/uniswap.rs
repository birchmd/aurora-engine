use crate::prelude::Address;
use crate::test_utils::{
    self,
    erc20::{ERC20Constructor, ERC20},
    uniswap::{Factory, FactoryConstructor, MintParams, PositionManager, PositionManagerConstructor},
    Signer,
};
use crate::types::Wei;
use secp256k1::SecretKey;
use primitive_types::U256;

const INITIAL_BALANCE: u64 = 1000;
const INITIAL_NONCE: u64 = 0;
// The "fee" can only be specific values, see
// https://github.com/Uniswap/uniswap-v3-core/blob/main/contracts/UniswapV3Factory.sol#L26
const POOL_FEE: u64 = 500;

#[test]
fn it_works() {
    let (mut runner, mut signer, factory, manager) = initialize_uniswap_factory();

    let token_a = create_token("token_a", "A", &mut runner, &mut signer);
    let token_b = create_token("token_b", "B", &mut runner, &mut signer);

    create_pool(
        token_a.0.address,
        token_b.0.address,
        &factory,
        &mut runner,
        &mut signer,
    );

    let token0 = std::cmp::min(token_a.0.address, token_b.0.address);
    let token1 = std::cmp::max(token_a.0.address, token_b.0.address);

    let nonce = signer.use_nonce();
    let add_liquidity_tx = manager.mint(MintParams {
        token0,
        token1,
        fee: POOL_FEE.into(),
        // https://github.com/Uniswap/uniswap-v3-core/blob/main/contracts/libraries/TickMath.sol#L9
        tick_lower: -887272,
        tick_upper: 887272,
        amount0_desired: 100.into(),
        amount1_desired: 100.into(),
        amount0_min: U256::one(),
        amount1_min: U256::one(),
        recipient: test_utils::address_from_secret_key(&signer.secret_key),
        deadline: U256::MAX, // no deadline
    }, nonce.into());
    let result = runner.submit_transaction(&signer.secret_key, add_liquidity_tx).unwrap();

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

    let create_pool_tx = factory.create_pool(token_a, token_b, POOL_FEE.into(), nonce.into());
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

fn initialize_uniswap_factory() -> (test_utils::AuroraRunner, Signer, Factory, PositionManager) {
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

    let mut signer = Signer {
        nonce: INITIAL_NONCE,
        secret_key: source_account,
    };

    let nonce = signer.use_nonce();
    let factory_constructor = FactoryConstructor::load();
    let factory = Factory(runner.deploy_contract(
        &signer.secret_key,
        |c| c.deploy(nonce.into()),
        factory_constructor,
    ));

    let nonce = signer.use_nonce();
    let manager_constructor = PositionManagerConstructor::load();
    let manager = PositionManager(runner.deploy_contract(
        &signer.secret_key,
        |c| c.deploy(nonce.into()),
        manager_constructor,
    ));

    (runner, signer, factory, manager)
}
