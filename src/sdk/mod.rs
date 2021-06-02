use crate::prelude::Vec;

#[cfg(feature = "engine")]
mod external;

#[cfg(feature = "engine")]
pub use external::*;

trait Sdk {
    fn read_input(&self, dest: &mut [u8]);
    fn return_output(&mut self, value: &[u8]);
    fn read_storage(&self, key: &[u8]) -> Option<Vec<u8>>;
    fn read_storage_len(&self, key: &[u8]) -> Option<usize>;
}
