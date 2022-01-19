use crate::test_utils::standalone;
use aurora_engine_types::types::Address;
use aurora_engine_types::{H160, U256, TryFrom};
use engine_standalone_storage::json_snapshot;
use aurora_engine::parameters::SubmitResult;

const NONCE_PREFIX: [u8; 2] = [0x07, 0x01];
const BALANCE_PREFIX: [u8; 2] = [0x07, 0x02];
const CODE_PREFIX: [u8; 2] = [0x07, 0x03];

fn compare_results(x: &SubmitResult, y: &SubmitResult) -> bool {
    x.status == y.status && x.logs == y.logs //&& x.gas_used == y.gas_used
}

#[test]
fn test_show_snapshots_are_poststates() {
    // test executing transaction 5uihgByR13EC2vfp3jAgyoQMEGJ4jmK814s6ueNGMLeS
    // Its receipt was executed in block 57652394 but our state snapshots are post-states,
    // so we need to use the snapshot from 57652393.
    use borsh::BorshDeserialize;
    let snapshot = json_snapshot::types::JsonSnapshot::load_from_file(
        "/home/birchmd/aurora-engine/contract.aurora.block57652393.json",
    )
    .unwrap();
    let my_address = Address::try_from_slice(&hex::decode("e0ebd9f22027730dae8e6ad5f649564bb1684aca").unwrap()).unwrap();
    let my_nonce_key = aurora_engine_types::storage::address_to_key(aurora_engine_types::storage::KeyPrefix::Nonce, &my_address);
    for entry in snapshot.result.values.iter() {
        let key = base64::decode(&entry.key).unwrap();
        let value = base64::decode(&entry.value).unwrap();
        if &key == &my_nonce_key {
            let nonce_value = aurora_engine_types::U256::from_big_endian(&value);
            println!("{:?}", nonce_value);
            break;
        }
    }
    let tx_b64 = "+QFtgwIAg4CDVu2/lCy0XttFF9WUev3jvqv5WlglBoWLgLkBBOjjNwAAAAAAAAAAAAAAAADELDCsbMFfrJvZOGGLyqGh+uhQHQAAAAAAAAAAAAAAALEr/KWlWAaq9k6ZUhkYpL8PxAgCAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAASXQEsj/SLzoAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAWwAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAEi3+mjJbEAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAFoAAAAAAAAAAAAAAADg69nyICdzDa6OatX2SVZLsWhKygAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAABh5y+1hJyKgsig/IvH1narM/LJGFFNvEHXevNDk6r8lhx2YzbCx4RDfgagepk2ln8JpDQxRzmmd8uSsil9IH9MBLxH9O6Nd/iMThw=";
    let tx_bytes = base64::decode(tx_b64).unwrap();
    let tx = aurora_engine::transaction::EthTransactionKind::try_from(tx_bytes.as_slice()).unwrap();
    match tx {
        aurora_engine::transaction::EthTransactionKind::Legacy(tx) => println!("{:?}", tx.transaction.nonce),
        _ => panic!("Err, what?"),
    }

    let mut runner = crate::test_utils::AuroraRunner::default();
    runner.wasm_config.limit_config.max_gas_burnt = 5_000_000_000_000_000;
    runner.context.storage_usage = 500_000_000;
    runner.consume_json_snapshot(snapshot);

    runner.context.block_index = 57652393;
    runner.context.block_timestamp = 1642530984685706638;
    let (outcome, error) = runner.call_with_signer_and_maybe_update("submit", "relay.aurora", "relay.aurora", tx_bytes, false);
    let outcome = outcome.unwrap();
    let profile = crate::test_utils::ExecutionProfile::new(&outcome);
    println!("{:?}\n{:?}\n{:?}", profile.host_breakdown, profile.wasm_gas(), profile.all_gas());
    if let Some(error) = error {
        panic!("{:?}", error);
    }
    let mainnet_result_b64 = "BwGEAAAACMN5oAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAgAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAACZVbmlzd2FwVjJSb3V0ZXI6IElOU1VGRklDSUVOVF9CX0FNT1VOVAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAaI0AAAAAAAAAAAAA";
    let success_bytes = base64::decode(mainnet_result_b64).unwrap();
    let mainnet_result = SubmitResult::try_from_slice(&success_bytes).unwrap();
    let submit_result = SubmitResult::try_from_slice(&outcome.return_data.as_value().unwrap()).unwrap();
    assert!(compare_results(&mainnet_result, &submit_result));
}

#[test]
fn test_replay_mainnet_transactions() {
    use borsh::BorshDeserialize;
    let snapshot = json_snapshot::types::JsonSnapshot::load_from_file(
        "/home/birchmd/aurora-engine/contract.aurora.block56838299.json",
    )
    .unwrap();
    let mut runner = crate::test_utils::AuroraRunner::default();
    runner.wasm_config.limit_config.max_gas_burnt = 5_000_000_000_000_000;
    runner.context.storage_usage = 500_000_000;
    runner.consume_json_snapshot(snapshot);

    let txs = std::fs::read_to_string("/home/birchmd/aurora-engine/mainnet_txs").unwrap();

    // Idea: include only successful transactions and gas failures. Tag each transaction with status.
    // For gas failures run in one-shot to check for success with higher gas limit, record result and continue.
    // This works because on mainnet the state changes from the failed transaction would not have been committed, hence
    // why we run those failures using a one-shot runner. This way we should also be able to exactly reproduce the output
    // from all the successful transactions to ensure accuracy of the run.

    for (i, tx_spec) in txs.split('\n').enumerate() {
        if tx_spec.is_empty() {
            continue;
        }
        let mut tokens = tx_spec.split(' ');
        let block_height: u64 = tokens.next().unwrap().parse().unwrap();
        let timestamp_ns: u64 = tokens.next().unwrap().parse().unwrap();
        let tx_hex = tokens.next().unwrap();
        let tx_status = tokens.next().unwrap();
        let tx_bytes = hex::decode(tx_hex).unwrap();
        runner.context.block_index = block_height;
        runner.context.block_timestamp = timestamp_ns;
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
            let success_bytes = hex::decode(tokens.next().unwrap()).unwrap();
            let (outcome, error) = runner.call_with_signer_and_maybe_update("submit", "relay.aurora", "relay.aurora", tx_bytes, false);
            let outcome = outcome.unwrap();
            let profile = crate::test_utils::ExecutionProfile::new(&outcome);
            if let Some(error) = error {
                println!("{:?}", i);
                println!("{:?}\n{:?}\n{:?}", profile.host_breakdown, profile.wasm_gas(), profile.all_gas());
                panic!("{:?}", error);
            }
            let submit_result = SubmitResult::try_from_slice(&outcome.return_data.as_value().unwrap()).unwrap();
            let mainnet_result = SubmitResult::try_from_slice(&success_bytes).unwrap();
            if !compare_results(&mainnet_result, &submit_result) {
                println!("{:?}", i);
                println!("MAINNET: {:?}", mainnet_result);
                println!("REPLAY: {:?}", submit_result);
                panic!("SubmitResult mismatch!");
            }
        } else if tx_status == "GAS_FAIL" {
            let one_shot = runner.one_shot();
            let (outcome, error) = one_shot.call_without_update("submit", "relay.aurora", tx_bytes);
            let outcome = outcome.unwrap();
            let profile = crate::test_utils::ExecutionProfile::new(&outcome);
            if let Some(error) = error {
                println!("FAILED {:?}", i);
                println!("Transaction failed after gas increase: {:?}", error);
                println!("{:?}\nFAILED_WASM {:?}\nFAILED_WASM {:?}\n#####################", profile.host_breakdown, profile.wasm_gas(), profile.all_gas());
            } else if profile.all_gas() > 300_000_000_000_000 {
                println!("FAILED {:?}", i);
                println!("Transaction failed after gas increase: FunctionCallError(HostError(GasLimitExceeded))");
                println!("{:?}\nFAILED_WASM {:?}\nFAILED_TOTAL {:?}\n#####################", profile.host_breakdown, profile.wasm_gas(), profile.all_gas());
            } else if profile.all_gas() < 200_000_000_000_000 {
                println!("FAILED {:?}", i);
                println!("Transaction failed after gas increase: Gas usage now too low!");
                println!("{:?}\nFAILED_WASM {:?}\nFAILED_TOTAL {:?}\n#####################", profile.host_breakdown, profile.wasm_gas(), profile.all_gas());
                panic!("Too low gas estimate!");
            } else {
                println!("PASSED {:?}", i);
                println!("Previously failed transaction passed after gas increase");
                println!("{:?}\nPASSED_WASM {:?}\nPASSED_TOTAL {:?}\n#####################", profile.host_breakdown, profile.wasm_gas(), profile.all_gas());
            }
        } else {
            panic!("Unknown status: {}", tx_status);
        }
    }
    let final_state: String = runner.ext.underlying.fake_trie.iter().map(|(k, v)| {
        format!("{} {}\n", hex::encode(k), hex::encode(v))
    }).collect();
    std::fs::write("final_state.txt", final_state).unwrap();
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
