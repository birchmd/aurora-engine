use crate::prelude::{vec, Address, BTreeMap, Cow, String, Vec, U256};
use crate::{Precompile, PrecompileOutput};
use alloc::boxed::Box;
use alloc::collections::BTreeSet;
use borsh::{BorshDeserialize, BorshSerialize};

pub type PromiseId = u64;

#[derive(Debug, BorshSerialize, BorshDeserialize)]
struct CallNearArgs {
    account_id: String,
    method: String,
    args: Vec<u8>,
    near_amount: u128,
    gas_limit: u64,
}

impl CallNearArgs {
    fn decode_eth_abi(bytes: &[u8]) -> Option<Self> {
        let mut tokens = ethabi::decode(
            &[
                ethabi::ParamType::String,
                ethabi::ParamType::String,
                ethabi::ParamType::Bytes,
                ethabi::ParamType::Uint(256),
                ethabi::ParamType::Uint(256),
            ],
            bytes,
        )
        .ok()?;

        let gas_limit = tokens.pop()?.into_uint()?.low_u64();
        let near_amount = tokens.pop()?.into_uint()?.low_u128();
        let args = tokens.pop()?.into_bytes()?;
        let method = tokens.pop()?.into_string()?;
        let account_id = tokens.pop()?.into_string()?;

        Some(Self {
            account_id,
            method,
            args,
            near_amount,
            gas_limit,
        })
    }

    fn encode_eth_abi(self) -> Vec<u8> {
        ethabi::encode(&[
            ethabi::Token::String(self.account_id),
            ethabi::Token::String(self.method),
            ethabi::Token::Bytes(self.args),
            ethabi::Token::Uint(self.near_amount.into()),
            ethabi::Token::Uint(self.gas_limit.into()),
        ])
    }
}

#[derive(Debug, BorshSerialize, BorshDeserialize)]
struct CallAuroraArgs {
    address: [u8; 20], // can't use Address and derive BorshSerialize
    args: Vec<u8>,
    eth_amount: [u8; 32], // can't use U256 and derive BorshSerialize
    gas_limit: u64,
}

/// Different than `CallAuroraArgs` since the `eth_amount` not provided
/// as part of the precompile input; it is obtained from the EVM context.
struct CallAuroraInput {
    address: Address,
    args: Vec<u8>,
    gas_limit: u64,
}

impl CallAuroraInput {
    fn decode_eth_abi(bytes: &[u8]) -> Option<Self> {
        let mut tokens = ethabi::decode(
            &[
                ethabi::ParamType::Address,
                ethabi::ParamType::Bytes,
                ethabi::ParamType::Uint(256),
            ],
            bytes,
        )
        .ok()?;

        let gas_limit = tokens.pop()?.into_uint()?.low_u64();
        let args = tokens.pop()?.into_bytes()?;
        let address = tokens.pop()?.into_address()?;

        Some(Self {
            address,
            args,
            gas_limit,
        })
    }

    fn encode_eth_abi(self) -> Vec<u8> {
        ethabi::encode(&[
            ethabi::Token::Address(self.address),
            ethabi::Token::Bytes(self.args),
            ethabi::Token::Uint(self.gas_limit.into()),
        ])
    }
}

enum Multiplicity<'a, T> {
    None,
    One(&'a T),
    Many(&'a [T]),
}

impl<'a, T> core::iter::IntoIterator for Multiplicity<'a, T> {
    type Item = &'a T;

    type IntoIter = MultiplicityIter<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        MultiplicityIter::new(self)
    }
}

struct MultiplicityIter<'a, T> {
    next_item: Option<&'a T>,
    other_items: &'a [T],
    index: usize,
}

impl<'a, T> MultiplicityIter<'a, T> {
    fn new(m: Multiplicity<'a, T>) -> Self {
        match m {
            Multiplicity::None => Self {
                next_item: None,
                other_items: &[],
                index: 0,
            },
            Multiplicity::One(x) => Self {
                next_item: Some(x),
                other_items: &[],
                index: 0,
            },
            Multiplicity::Many(xs) => Self {
                next_item: xs.get(0),
                other_items: xs,
                index: 1,
            },
        }
    }
}

impl<'a, T> Iterator for MultiplicityIter<'a, T> {
    type Item = &'a T;

    fn next(&mut self) -> Option<Self::Item> {
        match self.next_item {
            Some(x) => {
                let new_next_item = if self.index >= self.other_items.len() {
                    None
                } else {
                    Some(&self.other_items[self.index])
                };
                self.index += 1;
                self.next_item = new_next_item;

                Some(x)
            }
            None => None,
        }
    }
}

#[derive(Debug, BorshSerialize, BorshDeserialize)]
enum Promise {
    Create {
        id: PromiseId,
        call_data: CallNearArgs,
    },
    CallbackNear {
        id: PromiseId,
        base_id: PromiseId,
        call_data: CallNearArgs,
    },
    CallbackAurora {
        id: PromiseId,
        base_id: PromiseId,
        call_data: CallAuroraArgs,
    },
    And {
        id: PromiseId,
        composed_ids: Vec<PromiseId>,
    },
}

impl Promise {
    fn id(&self) -> PromiseId {
        match self {
            Self::Create { id, .. } => *id,
            Self::CallbackNear { id, .. } => *id,
            Self::CallbackAurora { id, .. } => *id,
            Self::And { id, .. } => *id,
        }
    }

    fn dependencies(&self) -> Multiplicity<PromiseId> {
        match self {
            Self::Create { .. } => Multiplicity::None,
            Self::CallbackNear { base_id, .. } => Multiplicity::One(base_id),
            Self::CallbackAurora { base_id, .. } => Multiplicity::One(base_id),
            Self::And { composed_ids, .. } => Multiplicity::Many(composed_ids),
        }
    }

    fn to_precompile_output(&self, address: Address, cost: u64) -> PrecompileOutput {
        let promise_id = {
            let mut buf = Vec::with_capacity(32);
            U256::from(self.id()).to_big_endian(&mut buf);
            buf
        };
        let promise_log = evm::backend::Log {
            address,
            topics: Vec::new(),
            data: self.try_to_vec().unwrap(),
        };

        PrecompileOutput {
            cost,
            output: promise_id,
            logs: vec![promise_log],
        }
    }

    fn schedule(promises: Vec<Self>) {
        // Sanity check that all the IDs listed as dependencies are present.
        // We can exit early if this is not the case.
        let all_ids: BTreeSet<PromiseId> = promises.iter().map(|p| p.id()).collect();
        let dep_ids: BTreeSet<PromiseId> = promises
            .iter()
            .flat_map(|p| p.dependencies())
            .copied()
            .collect();
        let num_missing_deps = dep_ids.difference(&all_ids).count();
        if num_missing_deps > 0 {
            aurora_engine_sdk::panic_utf8(b"ERR_MISSING_PROMISE_ID");
        }

        let mut remaining_promises = Vec::with_capacity(promises.len());
        let mut id_mapping = BTreeMap::new();

        // All `Promise::Crate` instances can be scheduled right away because they
        // do not have dependencies. The remaining promises will depend on these.
        for p in promises {
            match p {
                Self::Create {
                    id: internal_id,
                    call_data,
                } => {
                    let external_id = aurora_engine_sdk::promise_create(
                        call_data.account_id.as_bytes(),
                        call_data.method.as_bytes(),
                        &call_data.args,
                        call_data.near_amount,
                        call_data.gas_limit,
                    );
                    id_mapping.insert(internal_id, external_id);
                }
                other => {
                    remaining_promises.push(other);
                }
            }
        }

        // The reason we are not simply tracking `remaining_promises.len()` directly
        // is because the check at the beginning only looks to see all the dependant IDs
        // are present, but does not check for cyclic dependencies (which a malicious dev
        // could construct). It would not be a big deal if we did use `remaining_promises`
        // directly and the loop was infinite because there is still the gas limit to prevent
        // the computation for actually going on forever. That said, it is better if we can give
        // good error messages instead of the error being an opaque "out of gas", so by tracking
        // that `remaining_promises` shrinks during each iteration we can be clear about what
        // has gone wrong.
        let mut num_remaining_promises = remaining_promises.len();
        while num_remaining_promises > 0 {
            remaining_promises = remaining_promises
                .into_iter()
                .filter(|p| match p {
                    Self::Create { .. } => unreachable!(), // Self::Create filtered out at the beginning
                    Self::CallbackNear {
                        id: internal_id,
                        base_id: internal_base_id,
                        call_data,
                    } => {
                        if let Some(external_base_id) = id_mapping.get(internal_base_id) {
                            let external_id = aurora_engine_sdk::promise_then(
                                *external_base_id,
                                call_data.account_id.as_bytes(),
                                call_data.method.as_bytes(),
                                &call_data.args,
                                call_data.near_amount,
                                call_data.gas_limit,
                            );
                            id_mapping.insert(*internal_id, external_id);

                            // promise scheduled, can be removed from collection
                            false
                        } else {
                            // dependency has not been scheduled yet, leave for next round
                            true
                        }
                    }
                    Self::CallbackAurora {
                        id: internal_id,
                        base_id: internal_base_id,
                        call_data,
                    } => {
                        if let Some(external_base_id) = id_mapping.get(internal_base_id) {
                            let external_id = aurora_engine_sdk::promise_then(
                                *external_base_id,
                                &aurora_engine_sdk::current_account_id(),
                                b"callback_aurora", // TODO: need to define this function in engine
                                &call_data.try_to_vec().unwrap(),
                                0,
                                0, // TODO: need to derive this from call_data.gas_limit
                            );
                            id_mapping.insert(*internal_id, external_id);

                            // promise scheduled, can be removed from collection
                            false
                        } else {
                            // dependency has not been scheduled yet, leave for next round
                            true
                        }
                    }
                    Self::And {
                        id: internal_id,
                        composed_ids,
                    } => {
                        let external_ids: Vec<_> = composed_ids
                            .iter()
                            .filter_map(|id| id_mapping.get(id))
                            .copied()
                            .collect();
                        if external_ids.len() == composed_ids.len() {
                            let external_id = aurora_engine_sdk::promise_and(&external_ids);
                            id_mapping.insert(*internal_id, external_id);

                            false
                        } else {
                            true
                        }
                    }
                })
                .collect();

            if num_remaining_promises == remaining_promises.len() {
                // No promises were filtered out. This means no new promises were scheduled.
                // Which means the remaining promises somehow have unmet dependencies.
                // This can only happen if incorrect promise IDs were given.
                aurora_engine_sdk::panic_utf8(b"ERR_INCORRECT_PROMISE_ID");
            }

            num_remaining_promises = remaining_promises.len();
        }
    }
}

fn get_next_id() -> PromiseId {
    // TODO: probably just need a counter that persists for the duration of the transaction
    todo!();
}

fn call_near(call_data: CallNearArgs) -> Promise {
    let id = get_next_id();

    Promise::Create { id, call_data }
}

fn callback_near(base_id: PromiseId, call_data: CallNearArgs) -> Promise {
    let id = get_next_id();

    Promise::CallbackNear {
        id,
        base_id,
        call_data,
    }
}

fn callback_aurora(base_id: PromiseId, call_data: CallAuroraArgs) -> Promise {
    let id = get_next_id();

    Promise::CallbackAurora {
        id,
        base_id,
        call_data,
    }
}

fn promise_and(ids: &[PromiseId]) -> Promise {
    let id = get_next_id();

    Promise::And {
        id,
        composed_ids: ids.iter().copied().collect(),
    }
}

fn promise_results_count() -> usize {
    aurora_engine_sdk::promise_results_count() as usize
}

fn promise_result(index: usize) -> Option<Vec<u8>> {
    match aurora_engine_sdk::promise_result(index as u64) {
        aurora_engine_types::types::PromiseResult::Failed => None,
        aurora_engine_types::types::PromiseResult::Successful(x) => Some(x),
        // Promise results must always be ready because
        // `promise_or` has not been implemented
        aurora_engine_types::types::PromiseResult::NotReady => unreachable!(),
    }
}

struct CallNear;

impl CallNear {
    // TODO
    const ADDRESS: Address = super::make_address(0, 0);

    fn gas_cost(input: &CallNearArgs) -> u64 {
        // TODO: conversion from attached NEAR gas to EVM gas cost
        input.gas_limit
    }
}

impl Precompile for CallNear {
    fn required_gas(input: &[u8]) -> Result<u64, evm::ExitError> {
        let call_data = unwrap_input(CallNearArgs::decode_eth_abi(input))?;
        Ok(Self::gas_cost(&call_data))
    }

    fn run(
        input: &[u8],
        target_gas: Option<u64>,
        _context: &evm::Context,
        _is_static: bool,
    ) -> crate::EvmPrecompileResult {
        let call_data = unwrap_input(CallNearArgs::decode_eth_abi(input))?;
        let cost = Self::gas_cost(&call_data);

        if let Some(target_gas) = target_gas {
            if cost > target_gas {
                return Err(evm::ExitError::OutOfGas);
            }
        }

        let promise = call_near(call_data);
        let output = promise.to_precompile_output(Self::ADDRESS, cost);

        Ok(output.into())
    }
}

struct CallbackNear;

impl CallbackNear {
    // TODO
    const ADDRESS: Address = super::make_address(0, 0);

    fn read_input(input: &[u8]) -> Option<(PromiseId, CallNearArgs)> {
        if input.len() < 5 {
            return None;
        }
        let base_id = U256::from_big_endian(&input[..4]).low_u64();
        let args = CallNearArgs::decode_eth_abi(&input[4..])?;
        Some((base_id, args))
    }

    fn gas_cost(input: &CallNearArgs) -> u64 {
        // Can use the same function as `CallNear` since it only depends on the
        // gas cost conversion.
        CallNear::gas_cost(input)
    }
}

impl Precompile for CallbackNear {
    fn required_gas(input: &[u8]) -> Result<u64, evm::ExitError> {
        let (_, call_data) = unwrap_input(Self::read_input(input))?;
        Ok(Self::gas_cost(&call_data))
    }

    fn run(
        input: &[u8],
        target_gas: Option<u64>,
        _context: &evm::Context,
        _is_static: bool,
    ) -> crate::EvmPrecompileResult {
        let (base_id, call_data) = unwrap_input(Self::read_input(input))?;
        let cost = Self::gas_cost(&call_data);

        if let Some(target_gas) = target_gas {
            if cost > target_gas {
                return Err(evm::ExitError::OutOfGas);
            }
        }

        let promise = callback_near(base_id, call_data);
        let output = promise.to_precompile_output(Self::ADDRESS, cost);

        Ok(output.into())
    }
}

struct CallbackAurora;

impl CallbackAurora {
    // TODO
    const ADDRESS: Address = super::make_address(0, 0);

    fn read_input(input: &[u8]) -> Option<(PromiseId, CallAuroraInput)> {
        if input.len() < 5 {
            return None;
        }
        let base_id = U256::from_big_endian(&input[..4]).low_u64();
        let args = CallAuroraInput::decode_eth_abi(&input[4..])?;
        Some((base_id, args))
    }

    fn gas_cost(input: &CallAuroraInput) -> u64 {
        input.gas_limit
    }
}

impl Precompile for CallbackAurora {
    fn required_gas(input: &[u8]) -> Result<u64, evm::ExitError> {
        let (_, call_data) = unwrap_input(Self::read_input(input))?;
        Ok(Self::gas_cost(&call_data))
    }

    fn run(
        input: &[u8],
        target_gas: Option<u64>,
        context: &evm::Context,
        is_static: bool,
    ) -> crate::EvmPrecompileResult {
        if is_static {
            return Err(evm::ExitError::Other(Cow::from("ERR_INVALID_IN_STATIC")));
        }

        let (base_id, call_data_input) = unwrap_input(Self::read_input(input))?;
        let cost = Self::gas_cost(&call_data_input);
        let eth_amount = {
            let mut buf = [0u8; 32];
            context.apparent_value.to_big_endian(&mut buf);
            buf
        };
        let call_data = CallAuroraArgs {
            address: call_data_input.address.0,
            args: call_data_input.args,
            eth_amount,
            gas_limit: call_data_input.gas_limit,
        };

        if let Some(target_gas) = target_gas {
            if cost > target_gas {
                return Err(evm::ExitError::OutOfGas);
            }
        }

        let promise = callback_aurora(base_id, call_data);
        let output = promise.to_precompile_output(Self::ADDRESS, cost);

        Ok(output.into())
    }
}

struct PromiseAnd;

impl PromiseAnd {
    // TODO
    const ADDRESS: Address = super::make_address(0, 0);

    fn read_input(input: &[u8]) -> Option<Vec<PromiseId>> {
        let mut tokens = ethabi::decode(
            &[ethabi::ParamType::Array(Box::new(ethabi::ParamType::Uint(
                256,
            )))],
            input,
        )
        .ok()?;
        let ids = tokens
            .pop()?
            .into_array()?
            .into_iter()
            .filter_map(|id| Some(id.into_uint()?.low_u64()))
            .collect();
        Some(ids)
    }

    fn gas_cost(_input: &[PromiseId]) -> u64 {
        // TODO: cost?
        0
    }
}

impl Precompile for PromiseAnd {
    fn required_gas(input: &[u8]) -> Result<u64, evm::ExitError> {
        let promises = unwrap_input(Self::read_input(input))?;
        Ok(Self::gas_cost(&promises))
    }

    fn run(
        input: &[u8],
        target_gas: Option<u64>,
        _context: &evm::Context,
        _is_static: bool,
    ) -> crate::EvmPrecompileResult {
        let promises = unwrap_input(Self::read_input(input))?;
        let cost = Self::gas_cost(&promises);

        if let Some(target_gas) = target_gas {
            if cost > target_gas {
                return Err(evm::ExitError::OutOfGas);
            }
        }

        let promise = promise_and(&promises);
        let output = promise.to_precompile_output(Self::ADDRESS, cost);

        Ok(output.into())
    }
}

struct PromiseResultsCount;

impl PromiseResultsCount {
    // TODO
    const ADDRESS: Address = super::make_address(0, 0);

    // TODO: cost?
    const COST: u64 = 0;
}

impl Precompile for PromiseResultsCount {
    fn required_gas(_input: &[u8]) -> Result<u64, evm::ExitError> {
        Ok(Self::COST)
    }

    fn run(
        _input: &[u8],
        target_gas: Option<u64>,
        _context: &evm::Context,
        _is_static: bool,
    ) -> crate::EvmPrecompileResult {
        let cost = Self::COST;

        if let Some(target_gas) = target_gas {
            if cost > target_gas {
                return Err(evm::ExitError::OutOfGas);
            }
        }

        let result = promise_results_count();
        let result_bytes = {
            let mut buf = Vec::with_capacity(32);
            U256::from(result).to_big_endian(&mut buf);
            buf
        };
        let output = PrecompileOutput::without_logs(cost, result_bytes);

        Ok(output.into())
    }
}

struct PromiseResult;

impl PromiseResult {
    // TODO
    const ADDRESS: Address = super::make_address(0, 0);

    // TODO: cost?
    const COST: u64 = 0;

    fn read_input(input: &[u8]) -> Option<usize> {
        if input.len() != 32 {
            return None;
        }

        let index = U256::from_big_endian(input).low_u64();
        Some(index as usize)
    }
}

impl Precompile for PromiseResult {
    fn required_gas(_input: &[u8]) -> Result<u64, evm::ExitError> {
        Ok(Self::COST)
    }

    fn run(
        input: &[u8],
        target_gas: Option<u64>,
        _context: &evm::Context,
        _is_static: bool,
    ) -> crate::EvmPrecompileResult {
        let cost = Self::COST;

        if let Some(target_gas) = target_gas {
            if cost > target_gas {
                return Err(evm::ExitError::OutOfGas);
            }
        }

        let index = unwrap_input(Self::read_input(input))?;
        let result = promise_result(index);
        let result_bytes = result.unwrap_or_default();
        let output = PrecompileOutput::without_logs(cost, result_bytes);

        Ok(output.into())
    }
}

fn unwrap_input<T>(input: Option<T>) -> Result<T, evm::ExitError> {
    input.ok_or_else(|| evm::ExitError::Other(Cow::from("Bad input")))
}
