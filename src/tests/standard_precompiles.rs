use crate::parameters::SubmitResult;
use crate::test_utils::{
    self,
    standard_precompiles::{PrecompilesConstructor, PrecompilesContract},
};
use crate::types::Wei;
use borsh::BorshDeserialize;
use secp256k1::SecretKey;

const INITIAL_BALANCE: Wei = Wei::new_u64(1000);
const INITIAL_NONCE: u64 = 0;

#[test]
fn standard_precompiles() {
    let mut runner = test_utils::deploy_evm();
    let mut rng = rand::thread_rng();
    let source_account = SecretKey::random(&mut rng);
    runner.create_address(
        test_utils::address_from_secret_key(&source_account),
        INITIAL_BALANCE,
        INITIAL_NONCE.into(),
    );

    let constructor = PrecompilesConstructor::load();
    let contract = PrecompilesContract(runner.deploy_contract(
        &source_account,
        |c| c.deploy(INITIAL_NONCE.into()),
        constructor,
    ));

    let test_all_tx = contract.call_method("test_ecpair", (INITIAL_NONCE + 1).into());
    let tx = test_utils::sign_transaction(test_all_tx, Some(runner.chain_id), &source_account);
    runner.wasm_config.limit_config.max_gas_burnt = u64::MAX;
    let (outcome, error) = runner.call(test_utils::SUBMIT, "x.near", rlp::encode(&tx).to_vec());
    assert!(error.is_none());
    let outcome = outcome.unwrap();
    let profile = test_utils::ExecutionProfile::new(&outcome);
    let result: SubmitResult =
        SubmitResult::try_from_slice(&outcome.return_data.as_value().unwrap()).unwrap();

    println!("{:?}", profile.host_breakdown);
    println!("WASM_GAS {:?}", profile.wasm_gas());
    println!("ALL_GAS {:?}", profile.all_gas());
    println!("EVM_GAS {:?}", result.gas_used);
}
