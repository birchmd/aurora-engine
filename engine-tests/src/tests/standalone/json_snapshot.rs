use crate::test_utils::standalone;
use aurora_engine_types::types::Address;
use aurora_engine_types::{H160, U256, TryFrom};
use engine_standalone_storage::json_snapshot;

const NONCE_PREFIX: [u8; 2] = [0x07, 0x01];
const BALANCE_PREFIX: [u8; 2] = [0x07, 0x02];
const CODE_PREFIX: [u8; 2] = [0x07, 0x03];

#[test]
fn test_replay_mainnet_transactions() {
    use borsh::BorshDeserialize;
    let snapshot = json_snapshot::types::JsonSnapshot::load_from_file(
        "/home/birchmd/aurora-engine/contract.aurora.block56838299.json",
    )
    .unwrap();
    let mut runner = crate::test_utils::AuroraRunner::default();
    runner.wasm_config.limit_config.max_gas_burnt = 300_000_000_000_000;
    runner.context.storage_usage = 500_000_000;
    runner.consume_json_snapshot(snapshot);

    let txs = std::fs::read_to_string("/home/birchmd/aurora-engine/mainnet_txs").unwrap();

    // Idea: include only successful transactions and gas failures. Tag each transaction with status.
    // For gas failures run in one-shot to check for success with higher gas limit, record result and continue.
    // This works because on mainnet the state changes from the failed transaction would not have been committed, hence
    // why we run those failures using a one-shot runner. This way we should also be able to exactly reproduce the output
    // from all the successful transactions to ensure accuracy of the run.

    for (i, tx_spec) in txs.split('\n').enumerate() {
        let mut parser = tx_spec.split(' ');
        let tx_hex = parser.next().unwrap();
        let tx_status = parser.next().unwrap();
        let tx_bytes = hex::decode(tx_hex).unwrap();
        /*let tx = aurora_engine::transaction::EthTransactionKind::try_from(tx_bytes.as_slice()).unwrap();
        if let aurora_engine::transaction::EthTransactionKind::Legacy(legacy) = tx {
            let address = legacy.sender().unwrap();
            let nonce_key = crate::prelude::storage::address_to_key(
                crate::prelude::storage::KeyPrefix::Nonce,
                &address,
            );
            let nonce_value = crate::prelude::u256_to_arr(&legacy.transaction.nonce);
            runner.ext.underlying.fake_trie.insert(nonce_key.to_vec(), nonce_value.to_vec());
        }*/
        if tx_status == "SUCCESS" {
            let (outcome, error) = runner.call("submit", "relay.aurora", tx_bytes);
            let outcome = outcome.unwrap();
            let profile = crate::test_utils::ExecutionProfile::new(&outcome);
            if let Some(error) = error {
                println!("{:?}", i);
                println!("{:?}\n{:?}\n{:?}", profile.host_breakdown, profile.wasm_gas(), profile.all_gas());
                panic!("{:?}", error);
            }
            // TODO: validate execution against mainnet?
            //let submit_result = aurora_engine::parameters::SubmitResult::try_from_slice(&outcome.return_data.as_value().unwrap()).unwrap();
            //assert!(submit_result.status.is_ok(), "{:?}", submit_result);
        } else if tx_status == "GAS_FAIL" {
            let one_shot = runner.one_shot();
            let (outcome, error) = one_shot.call("submit", "relay.aurora", tx_bytes);
            let outcome = outcome.unwrap();
            let profile = crate::test_utils::ExecutionProfile::new(&outcome);
            if let Some(error) = error {
                println!("{:?}", i);
                println!("Transaction failed after gas increase: {:?}", error);
                println!("{:?}\n{:?}\n{:?}\n#####################", profile.host_breakdown, profile.wasm_gas(), profile.all_gas());
            }
        } else {
            panic!("Unknown status: {}", tx_status);
        }
    }
}

#[test]
fn test_consume_snapshot() {
    let snapshot = json_snapshot::types::JsonSnapshot::load_from_file(
        "src/tests/res/contract.aurora.block51077328.json",
    )
    .unwrap();
    let mut runner = standalone::StandaloneRunner::default();
    json_snapshot::initialize_engine_state(&mut runner.storage, snapshot.clone()).unwrap();

    // check accounts to see they were written properly
    runner.env.block_height = snapshot.result.block_height + 1;
    for entry in snapshot.result.values {
        let key = base64::decode(entry.key).unwrap();
        let value = base64::decode(entry.value).unwrap();
        if key.as_slice().starts_with(&NONCE_PREFIX) {
            let address = address_from_key(&key);
            let nonce = U256::from_big_endian(&value);
            assert_eq!(nonce, runner.get_nonce(&address))
        } else if key.as_slice().starts_with(&BALANCE_PREFIX) {
            let address = address_from_key(&key);
            let balance = U256::from_big_endian(&value);
            assert_eq!(balance, runner.get_balance(&address).raw())
        } else if key.as_slice().starts_with(&CODE_PREFIX) {
            let address = address_from_key(&key);
            assert_eq!(value, runner.get_code(&address))
        }
    }

    runner.close();
}

fn address_from_key(key: &[u8]) -> Address {
    let mut result = [0u8; 20];
    result.copy_from_slice(&key[2..22]);
    Address::new(H160(result))
}
