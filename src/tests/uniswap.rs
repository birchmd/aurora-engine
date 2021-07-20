use crate::prelude::Address;
use crate::test_utils::{
    self,
    erc20::{ERC20Constructor, ERC20},
    uniswap::{
        Factory, FactoryConstructor, MintParams, Pool, PositionManager, PositionManagerConstructor,
    },
    AuroraRunner, Signer,
};
use crate::types::Wei;
use primitive_types::U256;
use secp256k1::SecretKey;

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

    let _pool = create_pool(
        token_a.0.address,
        token_b.0.address,
        &factory,
        &mut runner,
        &mut signer,
    );

    approve_erc20(
        &token_a,
        manager.0.address,
        U256::MAX,
        &mut signer,
        &mut runner,
    );
    approve_erc20(
        &token_b,
        manager.0.address,
        U256::MAX,
        &mut signer,
        &mut runner,
    );

    let token0 = std::cmp::min(token_a.0.address, token_b.0.address);
    let token1 = std::cmp::max(token_a.0.address, token_b.0.address);

    let nonce = signer.use_nonce();
    let add_liquidity_tx = manager.mint(
        MintParams {
            token0,
            token1,
            fee: POOL_FEE.into(),
            tick_lower: -1000,
            tick_upper: 1000,
            amount0_desired: 10_00.into(),
            amount1_desired: 10_00.into(),
            amount0_min: U256::one(),
            amount1_min: U256::one(),
            recipient: test_utils::address_from_secret_key(&signer.secret_key),
            deadline: U256::MAX, // no deadline
        },
        nonce.into(),
    );
    let result = runner
        .submit_transaction(&signer.secret_key, add_liquidity_tx)
        .unwrap();

    panic!("{:?}", result);
}

fn approve_erc20(
    token: &ERC20,
    spender: Address,
    amount: U256,
    signer: &mut Signer,
    runner: &mut AuroraRunner,
) {
    let result = runner
        .submit_with_signer(signer, |nonce| token.approve(spender, amount, nonce))
        .unwrap();
    assert!(result.status, "Failed to approve ERC-20");
}

fn create_pool(
    token_a: Address,
    token_b: Address,
    factory: &Factory,
    runner: &mut test_utils::AuroraRunner,
    signer: &mut Signer,
) -> Pool {
    let result = runner
        .submit_with_signer(signer, |nonce| {
            factory.create_pool(token_a, token_b, POOL_FEE.into(), nonce)
        })
        .unwrap();
    assert!(result.status, "Failed to create pool");

    let mut address = [0u8; 20];
    address.copy_from_slice(&result.result[12..]);
    let pool = Pool::from_address(Address(address));

    // 2^96 corresponds to a price ratio of 1
    let result = runner
        .submit_with_signer(signer, |nonce| {
            pool.initialize(U256::from(2).pow(U256::from(96)), nonce)
        })
        .unwrap();
    assert!(result.status, "Failed to initialize pool");

    pool
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

    let wrapped_eth = create_token("Wrapped Ether", "WETH", &mut runner, &mut signer);

    let nonce = signer.use_nonce();
    let manager_constructor = PositionManagerConstructor::load();
    let manager = PositionManager(runner.deploy_contract(
        &signer.secret_key,
        |c| {
            c.deploy(
                factory.0.address,
                wrapped_eth.0.address,
                Address([0; 20]),
                nonce.into(),
            )
        },
        manager_constructor,
    ));

    (runner, signer, factory, manager)
}
