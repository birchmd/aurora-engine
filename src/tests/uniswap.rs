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

    let result = add_equal_liquidity(
        &manager,
        10_000.into(),
        &token_a,
        &token_b,
        &mut runner,
        &mut signer,
    );

    panic!("{:?}", result);
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
struct LiquidityResult {
    token_id: U256,
    liquidity: U256,
    amount0: U256,
    amount1: U256,
}

fn add_equal_liquidity(
    manager: &PositionManager,
    amount: U256,
    token_a: &ERC20,
    token_b: &ERC20,
    runner: &mut AuroraRunner,
    signer: &mut Signer,
) -> LiquidityResult {
    approve_erc20(token_a, manager.0.address, U256::MAX, signer, runner);
    approve_erc20(token_b, manager.0.address, U256::MAX, signer, runner);

    let token0 = std::cmp::min(token_a.0.address, token_b.0.address);
    let token1 = std::cmp::max(token_a.0.address, token_b.0.address);

    let params = MintParams {
        token0,
        token1,
        fee: POOL_FEE.into(),
        tick_lower: -1000,
        tick_upper: 1000,
        amount0_desired: amount,
        amount1_desired: amount,
        amount0_min: U256::one(),
        amount1_min: U256::one(),
        recipient: test_utils::address_from_secret_key(&signer.secret_key),
        deadline: U256::MAX, // no deadline
    };

    let result = runner
        .submit_with_signer(signer, |nonce| manager.mint(params, nonce))
        .unwrap();
    assert!(result.status);

    let result = {
        let mut values = [U256::zero(); 4];
        for i in 0..4 {
            let lower = i * 32;
            let upper = (i + 1) * 32;
            let value = U256::from_big_endian(&result.result[lower..upper]);
            values[i] = value;
        }
        LiquidityResult {
            token_id: values[0],
            liquidity: values[1],
            amount0: values[2],
            amount1: values[3],
        }
    };
    assert_eq!(result.amount0, amount);
    assert_eq!(result.amount1, amount);

    result
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
