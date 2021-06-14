use super::{errors, Env, PromisesAPI, IO};
use crate::prelude::{vec, Vec, H256};
use crate::types::PromiseResult;
use borsh::{BorshDeserialize, BorshSerialize};

const READ_STORAGE_REGISTER_ID: u64 = 0;
const INPUT_REGISTER_ID: u64 = 0;
const REMOVE_REGISTER_ID: u64 = 0;
const GAS_FOR_STATE_MIGRATION: u64 = 100_000_000_000_000;

mod exports {
    #[allow(unused)]
    extern "C" {
        // #############
        // # Registers #
        // #############
        pub(crate) fn read_register(register_id: u64, ptr: u64);
        pub(crate) fn register_len(register_id: u64) -> u64;
        // ###############
        // # Context API #
        // ###############
        pub(crate) fn current_account_id(register_id: u64);
        pub(crate) fn signer_account_id(register_id: u64);
        pub(crate) fn signer_account_pk(register_id: u64);
        pub(crate) fn predecessor_account_id(register_id: u64);
        pub(crate) fn input(register_id: u64);
        // TODO #1903 fn block_height() -> u64;
        pub(crate) fn block_index() -> u64;
        pub(crate) fn block_timestamp() -> u64;
        fn epoch_height() -> u64;
        pub(crate) fn storage_usage() -> u64;
        // #################
        // # Economics API #
        // #################
        fn account_balance(balance_ptr: u64);
        pub(crate) fn attached_deposit(balance_ptr: u64);
        pub(crate) fn prepaid_gas() -> u64;
        fn used_gas() -> u64;
        // ############
        // # Math API #
        // ############
        fn random_seed(register_id: u64);
        pub(crate) fn sha256(value_len: u64, value_ptr: u64, register_id: u64);
        pub(crate) fn keccak256(value_len: u64, value_ptr: u64, register_id: u64);
        // #####################
        // # Miscellaneous API #
        // #####################
        pub(crate) fn value_return(value_len: u64, value_ptr: u64);
        pub(crate) fn panic();
        pub(crate) fn panic_utf8(len: u64, ptr: u64);
        pub(crate) fn log_utf8(len: u64, ptr: u64);
        fn log_utf16(len: u64, ptr: u64);
        fn abort(msg_ptr: u32, filename_ptr: u32, line: u32, col: u32);
        // ################
        // # Promises API #
        // ################
        pub(crate) fn promise_create(
            account_id_len: u64,
            account_id_ptr: u64,
            method_name_len: u64,
            method_name_ptr: u64,
            arguments_len: u64,
            arguments_ptr: u64,
            amount_ptr: u64,
            gas: u64,
        ) -> u64;
        pub(crate) fn promise_then(
            promise_index: u64,
            account_id_len: u64,
            account_id_ptr: u64,
            method_name_len: u64,
            method_name_ptr: u64,
            arguments_len: u64,
            arguments_ptr: u64,
            amount_ptr: u64,
            gas: u64,
        ) -> u64;
        fn promise_and(promise_idx_ptr: u64, promise_idx_count: u64) -> u64;
        pub(crate) fn promise_batch_create(account_id_len: u64, account_id_ptr: u64) -> u64;
        fn promise_batch_then(promise_index: u64, account_id_len: u64, account_id_ptr: u64) -> u64;
        // #######################
        // # Promise API actions #
        // #######################
        fn promise_batch_action_create_account(promise_index: u64);
        pub(crate) fn promise_batch_action_deploy_contract(
            promise_index: u64,
            code_len: u64,
            code_ptr: u64,
        );
        pub(crate) fn promise_batch_action_function_call(
            promise_index: u64,
            method_name_len: u64,
            method_name_ptr: u64,
            arguments_len: u64,
            arguments_ptr: u64,
            amount_ptr: u64,
            gas: u64,
        );
        pub(crate) fn promise_batch_action_transfer(promise_index: u64, amount_ptr: u64);
        fn promise_batch_action_stake(
            promise_index: u64,
            amount_ptr: u64,
            public_key_len: u64,
            public_key_ptr: u64,
        );
        fn promise_batch_action_add_key_with_full_access(
            promise_index: u64,
            public_key_len: u64,
            public_key_ptr: u64,
            nonce: u64,
        );
        fn promise_batch_action_add_key_with_function_call(
            promise_index: u64,
            public_key_len: u64,
            public_key_ptr: u64,
            nonce: u64,
            allowance_ptr: u64,
            receiver_id_len: u64,
            receiver_id_ptr: u64,
            method_names_len: u64,
            method_names_ptr: u64,
        );
        fn promise_batch_action_delete_key(
            promise_index: u64,
            public_key_len: u64,
            public_key_ptr: u64,
        );
        fn promise_batch_action_delete_account(
            promise_index: u64,
            beneficiary_id_len: u64,
            beneficiary_id_ptr: u64,
        );
        // #######################
        // # Promise API results #
        // #######################
        pub(crate) fn promise_results_count() -> u64;
        pub(crate) fn promise_result(result_idx: u64, register_id: u64) -> u64;
        pub(crate) fn promise_return(promise_id: u64);
        // ###############
        // # Storage API #
        // ###############
        pub(crate) fn storage_write(
            key_len: u64,
            key_ptr: u64,
            value_len: u64,
            value_ptr: u64,
            register_id: u64,
        ) -> u64;
        pub(crate) fn storage_read(key_len: u64, key_ptr: u64, register_id: u64) -> u64;
        pub(crate) fn storage_remove(key_len: u64, key_ptr: u64, register_id: u64) -> u64;
        pub(crate) fn storage_has_key(key_len: u64, key_ptr: u64) -> u64;
        fn storage_iter_prefix(prefix_len: u64, prefix_ptr: u64) -> u64;
        fn storage_iter_range(start_len: u64, start_ptr: u64, end_len: u64, end_ptr: u64) -> u64;
        fn storage_iter_next(iterator_id: u64, key_register_id: u64, value_register_id: u64)
            -> u64;
        // ###############
        // # Validator API #
        // ###############
        fn validator_stake(account_id_len: u64, account_id_ptr: u64, stake_ptr: u64);
        fn validator_total_stake(stake_ptr: u64);
    }
}

pub struct External;

impl IO for External {
    fn read_input(&self) -> Vec<u8> {
        unsafe {
            exports::input(INPUT_REGISTER_ID);
            let bytes: Vec<u8> = vec![0; exports::register_len(INPUT_REGISTER_ID) as usize];
            exports::read_register(INPUT_REGISTER_ID, bytes.as_ptr() as *const u64 as u64);
            bytes
        }
    }

    fn return_output(&mut self, value: &[u8]) {
        unsafe {
            exports::value_return(value.len() as u64, value.as_ptr() as u64);
        }
    }

    fn read_storage(&self, key: &[u8]) -> Option<Vec<u8>> {
        self.read_storage_len(key).map(|value_size| unsafe {
            let bytes = vec![0u8; value_size];
            exports::read_register(
                READ_STORAGE_REGISTER_ID,
                bytes.as_ptr() as *const u64 as u64,
            );
            bytes
        })
    }

    fn read_storage_len(&self, key: &[u8]) -> Option<usize> {
        unsafe {
            if exports::storage_read(
                key.len() as u64,
                key.as_ptr() as u64,
                READ_STORAGE_REGISTER_ID,
            ) == 1
            {
                Some(exports::register_len(READ_STORAGE_REGISTER_ID) as usize)
            } else {
                None
            }
        }
    }

    fn storage_has_key(&self, key: &[u8]) -> bool {
        unsafe { exports::storage_has_key(key.len() as _, key.as_ptr() as _) == 1 }
    }

    fn write_storage(&mut self, key: &[u8], value: &[u8]) {
        unsafe {
            exports::storage_write(
                key.len() as u64,
                key.as_ptr() as u64,
                value.len() as u64,
                value.as_ptr() as u64,
                0,
            );
        }
    }

    fn remove_storage(&mut self, key: &[u8]) -> bool {
        unsafe {
            exports::storage_remove(key.len() as u64, key.as_ptr() as u64, REMOVE_REGISTER_ID) == 1
        }
    }

    fn remove_storage_value(&mut self, key: &[u8]) -> Option<Vec<u8>> {
        if self.remove_storage(key) {
            unsafe {
                let bytes = vec![0u8; exports::register_len(REMOVE_REGISTER_ID) as usize];
                exports::read_register(REMOVE_REGISTER_ID, bytes.as_ptr() as *const u64 as u64);
                Some(bytes)
            }
        } else {
            None
        }
    }

    fn read_input_arr20(&self) -> Result<[u8; 20], errors::IncorrectInputLength> {
        unsafe {
            exports::input(INPUT_REGISTER_ID);
            if exports::register_len(INPUT_REGISTER_ID) == 20 {
                let bytes = [0u8; 20];
                exports::read_register(INPUT_REGISTER_ID, bytes.as_ptr() as *const u64 as u64);
                Ok(bytes)
            } else {
                Err(errors::IncorrectInputLength)
            }
        }
    }

    fn read_input_and_store(&mut self, key: &[u8]) {
        unsafe {
            exports::input(0);
            // Store register 0 into key, store the previous value in register 1.
            exports::storage_write(key.len() as _, key.as_ptr() as _, u64::MAX, 0, 1);
        }
    }

    fn read_u64(&self, key: &[u8]) -> Result<u64, errors::ReadU64Error> {
        self.read_storage_len(key)
            .ok_or(errors::ReadU64Error::MissingValue)
            .and_then(|value_size| unsafe {
                if value_size == 8 {
                    let result = [0u8; 8];
                    exports::read_register(READ_STORAGE_REGISTER_ID, result.as_ptr() as _);
                    Ok(u64::from_le_bytes(result))
                } else {
                    Err(errors::ReadU64Error::InvalidU64)
                }
            })
    }
}

impl Env for External {
    fn block_timestamp(&self) -> u64 {
        unsafe { exports::block_timestamp() }
    }

    fn block_index(&self) -> u64 {
        unsafe { exports::block_index() }
    }

    fn attached_deposit(&self) -> u128 {
        unsafe {
            let data = [0u8; core::mem::size_of::<u128>()];
            exports::attached_deposit(data.as_ptr() as u64);
            u128::from_le_bytes(data)
        }
    }

    fn prepaid_gas(&self) -> u64 {
        unsafe { exports::prepaid_gas() }
    }

    fn predecessor_account_id(&self) -> Vec<u8> {
        unsafe {
            exports::predecessor_account_id(1);
            let bytes: Vec<u8> = vec![0u8; exports::register_len(1) as usize];
            exports::read_register(1, bytes.as_ptr() as *const u64 as u64);
            bytes
        }
    }

    fn current_account_id(&self) -> Vec<u8> {
        unsafe {
            exports::current_account_id(1);
            let bytes: Vec<u8> = vec![0u8; exports::register_len(1) as usize];
            exports::read_register(1, bytes.as_ptr() as *const u64 as u64);
            bytes
        }
    }

    fn panic_utf8(bytes: &[u8]) -> ! {
        unsafe {
            exports::panic_utf8(bytes.len() as u64, bytes.as_ptr() as u64);
        }
        unreachable!()
    }

    fn log(&mut self, text: &str) {
        let bytes = text.as_bytes();
        unsafe {
            exports::log_utf8(bytes.len() as u64, bytes.as_ptr() as u64);
        }
    }
}

impl PromisesAPI for External {
    fn promise_create(
        &mut self,
        account_id: &[u8],
        method_name: &[u8],
        arguments: &[u8],
        amount: u128,
        gas: u64,
    ) -> u64 {
        unsafe {
            exports::promise_create(
                account_id.len() as _,
                account_id.as_ptr() as _,
                method_name.len() as _,
                method_name.as_ptr() as _,
                arguments.len() as _,
                arguments.as_ptr() as _,
                &amount as *const u128 as _,
                gas,
            )
        }
    }

    fn promise_then(
        &mut self,
        promise_idx: u64,
        account_id: &[u8],
        method_name: &[u8],
        arguments: &[u8],
        amount: u128,
        gas: u64,
    ) -> u64 {
        unsafe {
            exports::promise_then(
                promise_idx,
                account_id.len() as _,
                account_id.as_ptr() as _,
                method_name.len() as _,
                method_name.as_ptr() as _,
                arguments.len() as _,
                arguments.as_ptr() as _,
                &amount as *const u128 as _,
                gas,
            )
        }
    }

    fn promise_return(&mut self, promise_idx: u64) {
        unsafe {
            exports::promise_return(promise_idx);
        }
    }

    fn promise_results_count(&self) -> u64 {
        unsafe { exports::promise_results_count() }
    }

    fn promise_result(&self, result_idx: u64) -> PromiseResult {
        unsafe {
            match exports::promise_result(result_idx, 0) {
                0 => PromiseResult::NotReady,
                1 => {
                    let bytes: Vec<u8> = vec![0; exports::register_len(0) as usize];
                    exports::read_register(0, bytes.as_ptr() as *const u64 as u64);
                    PromiseResult::Successful(bytes)
                }
                2 => PromiseResult::Failed,
                _ => panic_utf8(b"ERR_PROMISE_RETURN_CODE"),
            }
        }
    }

    fn promise_batch_create(&mut self, account_id: &[u8]) -> u64 {
        unsafe { exports::promise_batch_create(account_id.len() as _, account_id.as_ptr() as _) }
    }

    fn promise_batch_action_transfer(&mut self, promise_index: u64, amount: u128) {
        unsafe {
            exports::promise_batch_action_transfer(promise_index, &amount as *const u128 as _);
        }
    }

    fn promise_batch_action_function_call(
        &mut self,
        promise_idx: u64,
        method_name: &[u8],
        arguments: &[u8],
        amount: u128,
        gas: u64,
    ) {
        unsafe {
            exports::promise_batch_action_function_call(
                promise_idx,
                method_name.len() as _,
                method_name.as_ptr() as _,
                arguments.len() as _,
                arguments.as_ptr() as _,
                &amount as *const u128 as _,
                gas,
            )
        }
    }
}

pub fn read_input() -> Vec<u8> {
    External.read_input()
}

pub(crate) fn read_input_borsh<T: BorshDeserialize>() -> Result<T, errors::ArgParseErr> {
    External.read_input_borsh()
}

pub(crate) fn read_input_arr20() -> Result<[u8; 20], errors::IncorrectInputLength> {
    External.read_input_arr20()
}

/// Reads current input and stores in the given key keeping data in the runtime.
pub fn read_input_and_store(key: &[u8]) {
    External.read_input_and_store(key)
}

pub fn return_output(value: &[u8]) {
    External.return_output(value)
}

#[allow(dead_code)]
pub fn read_storage(key: &[u8]) -> Option<Vec<u8>> {
    External.read_storage(key)
}

pub fn read_storage_len(key: &[u8]) -> Option<usize> {
    External.read_storage_len(key)
}

/// Read u64 from storage at given key.
pub(crate) fn read_u64(key: &[u8]) -> Result<u64, errors::ReadU64Error> {
    External.read_u64(key)
}

pub fn write_storage(key: &[u8], value: &[u8]) {
    External.write_storage(key, value)
}

pub fn remove_storage(key: &[u8]) {
    External.remove_storage(key);
}

pub fn block_timestamp() -> u64 {
    External.block_timestamp()
}

pub fn block_index() -> u64 {
    External.block_index()
}

pub fn panic_utf8(bytes: &[u8]) -> ! {
    External::panic_utf8(bytes)
}

pub fn predecessor_account_id() -> Vec<u8> {
    External.predecessor_account_id()
}

/// Calls environment sha256 on given input.
pub fn sha256(input: &[u8]) -> H256 {
    unsafe {
        exports::sha256(input.len() as u64, input.as_ptr() as u64, 1);
        let bytes = H256::zero();
        exports::read_register(1, bytes.0.as_ptr() as *const u64 as u64);
        bytes
    }
}

/// Calls environment keccak256 on given input.
pub fn keccak(input: &[u8]) -> H256 {
    unsafe {
        exports::keccak256(input.len() as u64, input.as_ptr() as u64, 1);
        let bytes = H256::zero();
        exports::read_register(1, bytes.0.as_ptr() as *const u64 as u64);
        bytes
    }
}

/// Returns account id of the current account.
pub fn current_account_id() -> Vec<u8> {
    External.current_account_id()
}

/// Deploy code from given key in place of the current key.
pub fn self_deploy(code_key: &[u8]) {
    unsafe {
        // Load current account id into register 0.
        exports::current_account_id(0);
        // Use register 0 as the destination for the promise.
        let promise_id = exports::promise_batch_create(u64::MAX as _, 0);
        // Remove code from storage and store it in register 1.
        exports::storage_remove(code_key.len() as _, code_key.as_ptr() as _, 1);
        exports::promise_batch_action_deploy_contract(promise_id, u64::MAX, 1);
        promise_batch_action_function_call(
            promise_id,
            b"state_migration",
            &[],
            0,
            GAS_FOR_STATE_MIGRATION,
        )
    }
}

pub fn save_contract<T: BorshSerialize>(key: &[u8], data: &T) {
    write_storage(key, &data.try_to_vec().unwrap()[..]);
}

#[allow(dead_code)]
pub fn log(data: &str) {
    External.log(data)
}

#[allow(unused)]
pub fn prepaid_gas() -> u64 {
    External.prepaid_gas()
}

pub fn promise_create(
    account_id: &[u8],
    method_name: &[u8],
    arguments: &[u8],
    amount: u128,
    gas: u64,
) -> u64 {
    External.promise_create(account_id, method_name, arguments, amount, gas)
}

pub fn promise_then(
    promise_idx: u64,
    account_id: &[u8],
    method_name: &[u8],
    arguments: &[u8],
    amount: u128,
    gas: u64,
) -> u64 {
    External.promise_then(promise_idx, account_id, method_name, arguments, amount, gas)
}

pub fn promise_return(promise_idx: u64) {
    External.promise_return(promise_idx)
}

pub fn promise_results_count() -> u64 {
    External.promise_results_count()
}

pub fn promise_result(result_idx: u64) -> PromiseResult {
    External.promise_result(result_idx)
}

pub fn assert_private_call() {
    assert_eq!(
        predecessor_account_id(),
        current_account_id(),
        "ERR_PRIVATE_CALL"
    );
}

pub fn attached_deposit() -> u128 {
    External.attached_deposit()
}

pub fn assert_one_yocto() {
    assert_eq!(attached_deposit(), 1, "ERR_1YOCTO_ATTACH")
}

pub fn promise_batch_action_transfer(promise_index: u64, amount: u128) {
    External.promise_batch_action_transfer(promise_index, amount)
}

pub fn storage_byte_cost() -> u128 {
    External::storage_byte_cost()
}

pub fn promise_batch_create(account_id: &[u8]) -> u64 {
    External.promise_batch_create(account_id)
}

pub fn promise_batch_action_function_call(
    promise_idx: u64,
    method_name: &[u8],
    arguments: &[u8],
    amount: u128,
    gas: u64,
) {
    External.promise_batch_action_function_call(promise_idx, method_name, arguments, amount, gas)
}

#[allow(dead_code)]
pub fn storage_has_key(key: &[u8]) -> bool {
    External.storage_has_key(key)
}
