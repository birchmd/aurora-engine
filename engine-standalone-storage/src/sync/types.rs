use aurora_engine::parameters;
use aurora_engine::transaction::EthTransactionKind;
use aurora_engine_types::account_id::AccountId;
use aurora_engine_types::TryFrom;
use aurora_engine_types::H256;
use borsh::BorshDeserialize;
use near_primitives::views::ActionView;

/// Type describing the format of messages sent to the storage layer for keeping
/// it in sync with the blockchain.
#[derive(Debug, Clone)]
pub enum Message {
    Block(BlockMessage),
    Transaction(Box<TransactionMessage>),
}

#[derive(Debug, Clone)]
pub struct BlockMessage {
    pub height: u64,
    pub hash: H256,
    pub metadata: crate::BlockMetadata,
}

#[derive(Debug, Clone)]
pub struct TransactionMessage {
    /// Hash of the block which included this transaction
    pub block_hash: H256,
    /// Hash of the transaction on NEAR
    pub near_tx_hash: H256,
    /// If multiple Aurora transactions are included in the same block,
    /// this index gives the order in which they should be executed.
    pub position: u16,
    /// True if the transaction executed successfully on the blockchain, false otherwise.
    pub succeeded: bool,
    /// NEAR account that signed the transaction
    pub signer: AccountId,
    /// NEAR account that called the Aurora engine contract
    pub caller: AccountId,
    /// Amount of NEAR token attached to the transaction
    pub attached_near: u128,
    /// Details of the transaction that was executed
    pub transaction: TransactionKind,
}

#[derive(Debug, Clone)]
pub enum TransactionKind {
    /// Raw Ethereum transaction submitted to the engine
    Submit(EthTransactionKind),
    /// Ethereum transaction triggered by a NEAR account
    Call(parameters::CallArgs),
    /// Input here represents the EVM code used to create the new contract
    Deploy(Vec<u8>),
    /// New bridged token
    DeployErc20(parameters::DeployErc20TokenArgs),
    /// This type of transaction can impact the aurora state because of the bridge
    FtOnTransfer(parameters::NEP141FtOnTransferArgs),
    /// Bytes here will be parsed into `aurora_engine::proof::Proof`
    Deposit(Vec<u8>),
}

impl TransactionKind {
    pub fn parse_action(action: &ActionView) -> Option<(Self, u128)> {
        if let ActionView::FunctionCall {
            method_name,
            args,
            deposit,
            ..
        } = action
        {
            let bytes = base64::decode(&args).ok()?;
            let transaction_kind = match method_name.as_str() {
                "submit" => {
                    let eth_tx =
                        aurora_engine::transaction::EthTransactionKind::try_from(bytes.as_slice())
                            .ok()?;
                    Self::Submit(eth_tx)
                }
                "call" => {
                    let call_args = parameters::CallArgs::deserialize(&bytes)?;
                    Self::Call(call_args)
                }
                "deploy_code" => Self::Deploy(bytes),
                "deploy_erc20_token" => {
                    let deploy_args =
                        parameters::DeployErc20TokenArgs::try_from_slice(&bytes).ok()?;
                    Self::DeployErc20(deploy_args)
                }
                "ft_on_transfer" => {
                    let json_args = aurora_engine::json::parse_json(bytes.as_slice())?;
                    let transfer_args =
                        parameters::NEP141FtOnTransferArgs::try_from(json_args).ok()?;
                    Self::FtOnTransfer(transfer_args)
                }
                "deposit" => Self::Deposit(bytes),
                _ => return None,
            };
            return Some((transaction_kind, *deposit));
        }

        None
    }
}
