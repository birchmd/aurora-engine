use borsh::BorshDeserialize;
use near_crypto::{PublicKey, SecretKey};
use near_jsonrpc_client::JsonRpcClient;
use near_jsonrpc_primitives::types::query::{QueryResponseKind, RpcQueryRequest};
use near_primitives::borsh::BorshSerialize;
use near_primitives::hash::CryptoHash;
use near_primitives::transaction::{self, Action, SignedTransaction, Transaction};
use near_primitives::views::{FinalExecutionOutcomeView, FinalExecutionStatus};
use near_sdk_sim::{DEFAULT_GAS, STORAGE_AMOUNT};
use std::str::FromStr;
use serde_json;
use serde::Deserialize;

use crate::prelude::Address;

#[test]
fn test_testnet() {
    actix::System::new().block_on(async move {
        let client = create_testnet_client();

        let reader = std::fs::File::open("/home/birchmd/.near-credentials/testnet/birchmd.testnet.json").unwrap();
        let json_key: JsonKey = serde_json::from_reader(reader).unwrap();
        let near_signing_key = SigningKey::parse_key(&json_key.account_id, &json_key.private_key);
        let eth_signing_key = secp256k1::SecretKey::parse_slice(&hex::decode("7d640019603753a2449e6b57201ea1e7ee642b287cba417e8e9ba253c7cfa442").unwrap()).unwrap();

        //deploy_erc20(&client, &near_signing_key, &eth_signing_key).await;
        //mint_erc20(&client, &near_signing_key, &eth_signing_key).await;
        println!("{:?}", crate::test_utils::address_from_secret_key(&eth_signing_key));
        println!("{:?}", near_signing_key);
    });
}

async fn deploy_erc20(client: &JsonRpcClient, near_signing_key: &SigningKey, eth_signing_key: &secp256k1::SecretKey) {
    let (block_hash, _, nonce) = latest_block_and_nonce(client, near_signing_key).await;

    let constructor = crate::test_utils::erc20::ERC20Constructor::load();
    let deploy_tx = constructor.deploy("birchmd_test_token", "BIRCH", 0.into());
    let signed_eth_tx = crate::test_utils::sign_transaction(deploy_tx, Some(1313161555), eth_signing_key);
    let near_tx = create_submit_tx(signed_eth_tx, block_hash, nonce, near_signing_key);

    let outcome = send_tx(near_tx, client, near_signing_key).await;
    let return_value = get_return_value(&outcome);
    let submit_result = crate::parameters::SubmitResult::try_from_slice(&return_value).unwrap();

    println!("{:?}", submit_result);
    println!("{:?}", hex::encode(submit_result.result));
}

async fn mint_erc20(client: &JsonRpcClient, near_signing_key: &SigningKey, eth_signing_key: &secp256k1::SecretKey) {
    let (block_hash, _, nonce) = latest_block_and_nonce(client, near_signing_key).await;

    let constructor = crate::test_utils::erc20::ERC20Constructor::load();
    let contract = crate::test_utils::erc20::ERC20(crate::test_utils::solidity::DeployedContract {
        abi: constructor.0.abi,
        address: Address::from_slice(&hex::decode("196c016bf03bbcbed2c0082ded8315bddc89f59a").unwrap()),
    });
    let address = crate::test_utils::address_from_secret_key(&eth_signing_key);
    let mint_tx = contract.mint(address, 1000000000000000u64.into(), 1.into());
    let signed_eth_tx = crate::test_utils::sign_transaction(mint_tx, Some(1313161555), eth_signing_key);
    let near_tx = create_submit_tx(signed_eth_tx, block_hash, nonce, near_signing_key);

    let outcome = send_tx(near_tx, client, near_signing_key).await;
    let return_value = get_return_value(&outcome);
    let submit_result = crate::parameters::SubmitResult::try_from_slice(&return_value).unwrap();

    println!("{:?}", submit_result);
    println!("{:?}", hex::encode(submit_result.result));
}

fn get_return_value(outcome: &FinalExecutionOutcomeView) -> Vec<u8> {
    match &outcome.status {
        FinalExecutionStatus::SuccessValue(s) => {
            near_primitives::serialize::from_base64(&s).unwrap()
        }
        _ => unreachable!(), // status is validated in `send_tx`
    }
}

fn create_submit_tx(
    signed_eth_tx: crate::transaction::EthSignedTransaction,
    block_hash: CryptoHash,
    nonce: u64,
    account: &SigningKey,
) -> Transaction {
    Transaction {
        signer_id: account.account_id.clone(),
        public_key: account.public_key.clone(),
        receiver_id: "aurora".to_string(),
        actions: vec![Action::FunctionCall(transaction::FunctionCallAction {
            method_name: "submit".to_string(),
            args: rlp::encode(&signed_eth_tx).to_vec(),
            gas: DEFAULT_GAS,
            deposit: 0,
        })],
        block_hash,
        nonce,
    }
}

fn create_testnet_client() -> JsonRpcClient {
    near_jsonrpc_client::new_client("http://rpc.testnet.near.org")
}

async fn latest_block_and_nonce(
    client: &JsonRpcClient,
    account: &SigningKey,
) -> (CryptoHash, u64, u64) {
    let latest_block_response = client
        .query(RpcQueryRequest {
            block_reference: near_primitives::types::Finality::Final.into(),
            request: near_primitives::views::QueryRequest::ViewAccessKey {
                account_id: account.account_id.clone(),
                public_key: account.public_key.clone(),
            },
        })
        .await
        .unwrap();
    let current_nonce = match latest_block_response.kind {
        QueryResponseKind::AccessKey(key) => key.nonce,
        _ => panic!("Didn't get access key information?"),
    };
    (
        latest_block_response.block_hash,
        latest_block_response.block_height,
        current_nonce + 1,
    )
}

async fn send_tx(
    tx: Transaction,
    client: &JsonRpcClient,
    account: &SigningKey,
) -> FinalExecutionOutcomeView {
    let signature = account.secret_key.sign(tx.get_hash_and_size().0.as_ref());
    let signed_tx = SignedTransaction::new(signature, tx);

    let outcome = client
        .broadcast_tx_commit(near_primitives::serialize::to_base64(
            &signed_tx.try_to_vec().unwrap(),
        ))
        .await
        .unwrap();

    println!("{:?}", outcome.transaction_outcome.outcome);
    // assert is success
    match outcome.status {
        FinalExecutionStatus::SuccessValue(ref v) => println!("VALUE: {:?}", v),
        other => panic!("{:?}", other),
    }

    outcome
}

#[derive(Deserialize)]
struct JsonKey {
    account_id: String,
    public_key: String,
    private_key: String,
}

#[derive(Debug)]
struct SigningKey {
    account_id: String,
    public_key: PublicKey,
    secret_key: SecretKey,
}

impl SigningKey {
    fn parse_key(account_id: &str, secret_key: &str) -> Self {
        let secret_key = SecretKey::from_str(secret_key).unwrap();
        let public_key = secret_key.public_key();
        Self {
            account_id: account_id.to_string(),
            public_key,
            secret_key,
        }
    }
}
