

use ethereum_types::{Address, H256, U256};

pub struct State<'db> {
    trie_db: TrieDb,
    root: H256,
}

impl<'db> State<'db> {
    pub fn new(trie_db: TrieDb) -> Self{
        State{
            trie_db,
            root: H256::zero(),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::State;
    use bloom_trie_db::{TrieDb,RocksDb};
    use cita_trie::codec::RLPNodeCodec;



    #[test]
    fn test_state() {
        let test_dir = "data";
        let mut rocks_db = RocksDb::new(test_dir);
        let mut trie_db = TrieDb::new(&mut rocks_db,RLPNodeCodec::default());
        let state = State::new(trie_db);
    }

}