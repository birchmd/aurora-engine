use near_vm_logic::mocks::mock_external::MockedExternal;

pub const MAINNET_AVERAGE_TRIE_DEPTH: u64 = 10;

#[derive(Clone)]
pub(crate) struct MockedExternalWithTrie {
    pub underlying: MockedExternal,
    trie_node_count: std::cell::Cell<u64>,
}

impl MockedExternalWithTrie {
    pub fn new(ext: MockedExternal) -> Self {
        Self {
            underlying: ext,
            trie_node_count: std::cell::Cell::new(0),
        }
    }

    fn increment_trie_node_count(&self, amount: u64) {
        let cell_value = self.trie_node_count.get();
        self.trie_node_count.set(cell_value + amount);
    }
}

impl near_vm_logic::External for MockedExternalWithTrie {
    fn storage_set(&mut self, key: &[u8], value: &[u8]) -> Result<(), near_vm_logic::VMLogicError> {
        self.increment_trie_node_count(MAINNET_AVERAGE_TRIE_DEPTH);
        self.underlying.storage_set(key, value)
    }

    fn storage_get<'a>(
        &'a self,
        key: &[u8],
    ) -> Result<Option<Box<dyn near_vm_logic::ValuePtr + 'a>>, near_vm_logic::VMLogicError> {
        self.increment_trie_node_count(MAINNET_AVERAGE_TRIE_DEPTH);
        self.underlying.storage_get(key)
    }

    fn storage_remove(&mut self, key: &[u8]) -> Result<(), near_vm_logic::VMLogicError> {
        self.increment_trie_node_count(MAINNET_AVERAGE_TRIE_DEPTH);
        self.underlying.storage_remove(key)
    }

    fn storage_remove_subtree(&mut self, prefix: &[u8]) -> Result<(), near_vm_logic::VMLogicError> {
        self.underlying.storage_remove_subtree(prefix)
    }

    fn storage_has_key(&mut self, key: &[u8]) -> Result<bool, near_vm_logic::VMLogicError> {
        self.underlying.storage_has_key(key)
    }

    fn validator_stake(
        &self,
        account_id: &near_primitives::types::AccountId,
    ) -> Result<Option<near_primitives::types::Balance>, near_vm_logic::VMLogicError> {
        self.underlying.validator_stake(account_id)
    }

    fn validator_total_stake(
        &self,
    ) -> Result<near_primitives::types::Balance, near_vm_logic::VMLogicError> {
        self.underlying.validator_total_stake()
    }

    fn generate_data_id(&mut self) -> near_primitives::hash::CryptoHash {
        self.underlying.generate_data_id()
    }

    fn get_trie_nodes_count(&self) -> near_primitives::types::TrieNodesCount {
        let db_reads = self.trie_node_count.get();
        near_primitives::types::TrieNodesCount {
            db_reads,
            mem_reads: 0,
        }
    }
}
