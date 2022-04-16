#![cfg_attr(not(feature = "std"), no_std)]
#![cfg_attr(not(feature = "std"), feature(alloc_error_handler))]

#[cfg(feature = "contract")]
use crate::prelude::{Address, U256, Vec};
use crate::prelude::{H256, STORAGE_PRICE_PER_BYTE};
pub use types::keccak;

pub mod dup_cache;
pub mod env;
pub mod error;
pub mod io;
#[cfg(feature = "contract")]
pub mod near_runtime;
mod prelude;
pub mod promise;
pub mod types;

#[cfg(feature = "contract")]
use near_runtime::exports;

#[cfg(feature = "contract")]
const ECRECOVER_MESSAGE_SIZE: u64 = 32;
#[cfg(feature = "contract")]
const ECRECOVER_SIGNATURE_LENGTH: u64 = 64;
#[cfg(feature = "contract")]
const ECRECOVER_MALLEABILITY_FLAG: u64 = 1;

#[cfg(feature = "contract")]
pub fn panic_utf8(bytes: &[u8]) -> ! {
    unsafe {
        exports::panic_utf8(bytes.len() as u64, bytes.as_ptr() as u64);
    }
    unreachable!()
}

#[cfg(feature = "contract")]
pub fn log_utf8(bytes: &[u8]) {
    unsafe {
        exports::log_utf8(bytes.len() as u64, bytes.as_ptr() as u64);
    }
}

/// Calls environment sha256 on given input.
#[cfg(feature = "contract")]
pub fn sha256(input: &[u8]) -> H256 {
    unsafe {
        exports::sha256(input.len() as u64, input.as_ptr() as u64, 1);
        let bytes = H256::zero();
        exports::read_register(1, bytes.0.as_ptr() as *const u64 as u64);
        bytes
    }
}

#[cfg(not(feature = "contract"))]
pub fn sha256(input: &[u8]) -> H256 {
    use sha2::Digest;

    let output = sha2::Sha256::digest(input);
    H256(output.into())
}

/// Calls environment ripemd160 on given input.
#[cfg(feature = "contract")]
pub fn ripemd160(input: &[u8]) -> [u8; 20] {
    unsafe {
        const REGISTER_ID: u64 = 1;
        exports::ripemd160(input.len() as u64, input.as_ptr() as u64, REGISTER_ID);
        let bytes = [0u8; 20];
        exports::read_register(REGISTER_ID, bytes.as_ptr() as u64);
        bytes
    }
}

#[cfg(feature = "contract")]
pub fn alt_bn128_g1_sum(g: (U256, U256), h: (U256, U256)) -> [u8; 64] {
    const REGISTER_ID: u64 = 1;
    let mut bytes = Vec::with_capacity(65 * 2);
    
    bytes.push(0); // positive sign
    bytes.extend_from_slice(&[0; 64]);
    g.0.to_little_endian(&mut bytes[1..33]);
    g.1.to_little_endian(&mut bytes[33..65]);
    
    bytes.push(0);
    bytes.extend_from_slice(&[0; 64]);
    h.0.to_little_endian(&mut bytes[66..98]);
    h.1.to_little_endian(&mut bytes[98..130]);

    let value_ptr = bytes.as_ptr() as u64;
    let value_len = bytes.len() as u64;

    unsafe {
        exports::alt_bn128_g1_sum(value_len, value_ptr, REGISTER_ID);
        let mut output = [0u8; 64];
        exports::read_register(REGISTER_ID, output.as_ptr() as u64);
        let x = U256::from_little_endian(&output[0..32]);
        let y = U256::from_little_endian(&output[32..64]);
        x.to_big_endian(&mut output[0..32]);
        y.to_big_endian(&mut output[32..64]);
        return output;
    }
}

#[cfg(feature = "contract")]
pub fn alt_bn128_g1_scalar_multiple(g: (U256, U256), k: U256) -> [u8; 64] {
    const REGISTER_ID: u64 = 1;
    let mut bytes = [0u8; 96];

    g.0.to_little_endian(&mut bytes[0..32]);
    g.1.to_little_endian(&mut bytes[32..64]);
    k.to_little_endian(&mut bytes[64..96]);

    let value_ptr = bytes.as_ptr() as u64;
    let value_len = bytes.len() as u64;

    unsafe {
        exports::alt_bn128_g1_multiexp(value_len, value_ptr, REGISTER_ID);
        let mut output = [0u8; 64];
        exports::read_register(REGISTER_ID, output.as_ptr() as u64);
        let x = U256::from_little_endian(&output[0..32]);
        let y = U256::from_little_endian(&output[32..64]);
        x.to_big_endian(&mut output[0..32]);
        y.to_big_endian(&mut output[32..64]);
        return output;
    }
}

#[cfg(feature = "contract")]
pub fn alt_bn128_pairing(pairs: &[((U256, U256), ((U256, U256), (U256, U256)))]) -> bool {
    let n = pairs.len();
    let mut bytes = Vec::with_capacity(n * 6 * 32);
    let mut buf = [0u8; 6*32];

    for ((x, y), ((xre, xim), (yre, yim))) in pairs {
        x.to_little_endian(&mut buf[0..32]);
        y.to_little_endian(&mut buf[32..64]);
        xre.to_little_endian(&mut buf[64..96]);
        xim.to_little_endian(&mut buf[96..128]);
        yre.to_little_endian(&mut buf[128..160]);
        yim.to_little_endian(&mut buf[160..192]);
        bytes.extend_from_slice(&buf);
    }

    let value_ptr = bytes.as_ptr() as u64;
    let value_len = bytes.len() as u64;

    let result = unsafe { exports::alt_bn128_pairing_check(value_len, value_ptr) };

    result == 1
}

/// Recover address from message hash and signature.
#[cfg(feature = "contract")]
pub fn ecrecover(hash: H256, signature: &[u8]) -> Result<Address, ECRecoverErr> {
    unsafe {
        let hash_ptr = hash.as_ptr() as u64;
        let sig_ptr = signature.as_ptr() as u64;
        const RECOVER_REGISTER_ID: u64 = 1;
        const KECCACK_REGISTER_ID: u64 = 2;
        let result = exports::ecrecover(
            ECRECOVER_MESSAGE_SIZE,
            hash_ptr,
            ECRECOVER_SIGNATURE_LENGTH,
            sig_ptr,
            signature[64] as u64,
            ECRECOVER_MALLEABILITY_FLAG,
            RECOVER_REGISTER_ID,
        );
        if result == (true as u64) {
            // The result from the ecrecover call is in a register; we can use this
            // register directly for the input to keccak256. This is why the length is
            // set to `u64::MAX`.
            exports::keccak256(u64::MAX, RECOVER_REGISTER_ID, KECCACK_REGISTER_ID);
            let keccak_hash_bytes = [0u8; 32];
            exports::read_register(KECCACK_REGISTER_ID, keccak_hash_bytes.as_ptr() as u64);
            Ok(Address::try_from_slice(&keccak_hash_bytes[12..]).map_err(|_| ECRecoverErr)?)
        } else {
            Err(ECRecoverErr)
        }
    }
}

#[cfg(feature = "contract")]
pub fn log(data: &str) {
    log_utf8(data.as_bytes())
}

#[cfg(not(feature = "contract"))]
pub fn log(_data: &str) {
    // TODO: standalone logging
}

#[macro_export]
macro_rules! log {
    ($e: expr) => {
        #[cfg(feature = "log")]
        $crate::log($e)
    };
}

pub fn storage_byte_cost() -> u128 {
    STORAGE_PRICE_PER_BYTE
}

pub struct ECRecoverErr;

impl ECRecoverErr {
    pub fn as_str(&self) -> &'static str {
        "ERR_ECRECOVER"
    }
}

impl AsRef<[u8]> for ECRecoverErr {
    fn as_ref(&self) -> &[u8] {
        self.as_str().as_bytes()
    }
}
