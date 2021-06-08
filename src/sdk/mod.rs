use crate::prelude::Vec;
use borsh::BorshDeserialize;

#[cfg(feature = "engine")]
mod external;

#[cfg(feature = "engine")]
pub use external::*;

pub(crate) trait IO {
    fn read_input(&self) -> Vec<u8>;
    fn return_output(&mut self, value: &[u8]);
    fn read_storage(&self, key: &[u8]) -> Option<Vec<u8>>;
    fn read_storage_len(&self, key: &[u8]) -> Option<usize>;
    fn storage_has_key(&self, key: &[u8]) -> bool;
    fn write_storage(&mut self, key: &[u8], value: &[u8]);
    /// Remove entry from storage without capturing the value present at the given key.
    /// Returns `true` if the key had some value associated with it (now removed), and
    /// `false` otherwise.
    fn remove_storage(&mut self, key: &[u8]) -> bool;
    /// Remove entry from storage and capture the value present at the given key (if any)
    fn remove_storage_value(&mut self, key: &[u8]) -> Option<Vec<u8>>;

    fn read_input_borsh<T: BorshDeserialize>(&self) -> Result<T, errors::ArgParseErr> {
        let bytes = self.read_input();
        T::try_from_slice(&bytes).map_err(|_| errors::ArgParseErr)
    }

    /// This impl Should be overridden for maximal efficiency
    fn read_input_arr20(&self) -> Result<[u8; 20], errors::IncorrectInputLength> {
        let bytes = self.read_input();

        if bytes.len() != 20 {
            return Err(errors::IncorrectInputLength);
        }

        let mut result = [0u8; 20];
        result.copy_from_slice(&bytes);
        Ok(result)
    }

    /// This impl Should be overridden for maximal efficiency
    fn read_input_and_store(&mut self, key: &[u8]) {
        let value = self.read_input();
        self.write_storage(key, &value);
    }

    /// This impl Should be overridden for maximal efficiency
    fn read_u64(&self, key: &[u8]) -> Result<u64, errors::ReadU64Error> {
        let value = self
            .read_storage(key)
            .ok_or(errors::ReadU64Error::MissingValue)?;

        if value.len() != 8 {
            return Err(errors::ReadU64Error::InvalidU64);
        }

        let mut result = [0u8; 8];
        result.copy_from_slice(&value);
        Ok(u64::from_le_bytes(result))
    }

    fn storage_byte_cost() -> u128 {
        crate::types::STORAGE_PRICE_PER_BYTE
    }
}

pub(crate) trait Env {
    fn block_timestamp(&self) -> u64;
    fn block_index(&self) -> u64;
    fn attached_deposit(&self) -> u128;
    fn prepaid_gas(&self) -> u64;
    fn predecessor_account_id(&self) -> Vec<u8>;
    fn current_account_id(&self) -> Vec<u8>;
    fn panic_utf8(bytes: &[u8]) -> !;
    fn log_utf8(&mut self, bytes: &[u8]);

    fn is_private_call(&self) -> bool {
        self.predecessor_account_id() == self.current_account_id()
    }
}

pub(crate) trait PromisesAPI {
    fn promise_create(
        &mut self,
        account_id: &[u8],
        method_name: &[u8],
        arguments: &[u8],
        amount: u128,
        gas: u64,
    ) -> u64;
    fn promise_then(
        &mut self,
        promise_idx: u64,
        account_id: &[u8],
        method_name: &[u8],
        arguments: &[u8],
        amount: u128,
        gas: u64,
    ) -> u64;
    fn promise_return(&mut self, promise_idx: u64);
    fn promise_results_count(&self) -> u64;
    fn promise_result(&self, result_idx: u64) -> crate::types::PromiseResult;
    fn promise_batch_create(&mut self, account_id: &[u8]) -> u64;
    fn promise_batch_action_transfer(&mut self, promise_index: u64, amount: u128);
    fn promise_batch_action_function_call(
        &mut self,
        promise_idx: u64,
        method_name: &[u8],
        arguments: &[u8],
        amount: u128,
        gas: u64,
    );
}

pub(crate) mod errors {
    pub(crate) struct IncorrectInputLength;
    impl AsRef<[u8]> for IncorrectInputLength {
        fn as_ref(&self) -> &[u8] {
            b"ERR_INCORRECT_INPUT_LENGTH"
        }
    }

    pub(crate) struct ArgParseErr;
    impl AsRef<[u8]> for ArgParseErr {
        fn as_ref(&self) -> &[u8] {
            b"ERR_ARG_PARSE"
        }
    }

    pub(crate) enum ReadU64Error {
        InvalidU64,
        MissingValue,
    }
    impl AsRef<[u8]> for ReadU64Error {
        fn as_ref(&self) -> &[u8] {
            match self {
                Self::InvalidU64 => b"ERR_NOT_U64",
                Self::MissingValue => b"ERR_U64_NOT_FOUND",
            }
        }
    }
}
