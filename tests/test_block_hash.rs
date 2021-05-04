use near_crypto::{PublicKey, SecretKey};
use near_jsonrpc_client::JsonRpcClient;
use near_jsonrpc_primitives::types::query::{QueryResponseKind, RpcQueryRequest};
use near_primitives::borsh::BorshSerialize;
use near_primitives::hash::CryptoHash;
use near_primitives::transaction::{self, Action, SignedTransaction, Transaction};
use near_primitives::views::{FinalExecutionOutcomeView, FinalExecutionStatus};
use near_sdk_sim::{DEFAULT_GAS, STORAGE_AMOUNT};
use std::str::FromStr;

use aurora_engine::parameters::NewCallArgs;

near_sdk_sim::lazy_static_include::lazy_static_include_bytes! {
    EVM_WASM_BYTES => "release.wasm"
}

/// This test assumes you have a local node setup running already. The easiest way to do this
/// is probably to use the `start_cluster` functionality in `nearcore/pytest`. In the future
/// maybe this test could spin up a cluster itself.
/// The reason we use a real cluster as opposed to `near-vm-runner` (as in the benchmarks)
/// or `near-sdk-sim` (as in other tests) is because the other environments to not adequately
/// simulate blocks. Using a real cluster allows us to check the functionality works as intended.
/// TODO: test skip blocks too (probably would require purposely taking down one node in the cluster)
#[ignore]
#[test]
fn test_block_hash() {
    actix::System::new().block_on(async move {
        let client = create_local_client(3040);
        let signing_key = SigningKey::parse_key("test0", "ed25519:3KyUucjyGk1L58AJBB6Rf6EZFqmpTSSKG7KKsptMvpJLDBiZmAkU4dR1HzNS6531yZ2cR5PxnTM7NLVvSfJjZPh7");

        deploy_evm(&client, &signing_key).await;
        call_block_hash(&client, &signing_key).await;
    });
}

async fn call_block_hash(client: &JsonRpcClient, account: &SigningKey) {
    let (block_hash, block_height, nonce) = latest_block_and_nonce(client, account).await;
    let tx = create_block_hash_tx(block_height, block_hash, nonce + 1, account);
    let outcome = send_tx(tx, client, account).await;
    let return_value = get_return_value(&outcome);

    // querying at a real block height gives the block hash
    assert_eq!(&return_value, block_hash.as_ref());

    let tx = create_block_hash_tx(block_height - 257, block_hash, nonce + 2, account);
    let outcome = send_tx(tx, client, account).await;
    let return_value = get_return_value(&outcome);

    // querying at a block height more than 256 blocks ago gives zero
    assert_eq!(&return_value, &[0u8; 32]);
}

fn get_return_value(outcome: &FinalExecutionOutcomeView) -> Vec<u8> {
    match &outcome.status {
        FinalExecutionStatus::SuccessValue(s) => {
            near_primitives::serialize::from_base64(&s).unwrap()
        }
        _ => unreachable!(), // status is validated in `send_tx`
    }
}

fn create_block_hash_tx(
    block_height: u64,
    block_hash: CryptoHash,
    nonce: u64,
    account: &SigningKey,
) -> Transaction {
    let block_height_str = format!("{}", block_height);
    Transaction {
        signer_id: account.account_id.clone(),
        public_key: account.public_key.clone(),
        receiver_id: account.account_id.clone(),
        actions: vec![Action::FunctionCall(transaction::FunctionCallAction {
            method_name: "block_height".to_string(),
            args: block_height_str.as_bytes().to_vec(),
            gas: DEFAULT_GAS,
            deposit: STORAGE_AMOUNT,
        })],
        block_hash,
        nonce,
    }
}

fn create_local_client(port: usize) -> JsonRpcClient {
    let addr = format!("http://127.0.0.1:{}", port);
    near_jsonrpc_client::new_client(&addr)
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

    // assert is success
    match outcome.status {
        FinalExecutionStatus::SuccessValue(ref v) => println!("VALUE: {:?}", v),
        other => panic!("{:?}", other),
    }

    outcome
}

async fn deploy_evm(client: &JsonRpcClient, owner_account: &SigningKey) {
    let (block_hash, _, nonce) = latest_block_and_nonce(client, owner_account).await;
    let args = NewCallArgs {
        chain_id: [0u8; 32],
        owner_id: owner_account.account_id.clone(),
        bridge_prover_id: "abridged".to_string(),
        upgrade_delay_blocks: 1,
    };
    let deploy_tx = Transaction {
        signer_id: owner_account.account_id.clone(),
        public_key: owner_account.public_key.clone(),
        receiver_id: owner_account.account_id.clone(),
        actions: vec![
            Action::DeployContract(transaction::DeployContractAction {
                code: EVM_WASM_BYTES.to_vec(),
            }),
            Action::FunctionCall(transaction::FunctionCallAction {
                method_name: "new".to_string(),
                args: args.try_to_vec().unwrap(),
                gas: DEFAULT_GAS,
                deposit: STORAGE_AMOUNT,
            }),
        ],
        block_hash,
        nonce,
    };
    send_tx(deploy_tx, client, owner_account).await;
}

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
