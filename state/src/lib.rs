extern crate ethereum_types;
extern crate hash_db;
extern crate keccak_hasher;
extern crate rlp;

use ethereum_types::H256;
use keccak_hasher::KeccakHasher;
use rlp::DecoderError;



#[cfg(test)]
mod tests {

    use memory_db::{MemoryDB, PrefixedKey};
    use keccak_hasher::KeccakHasher;
    use trie_db::DBValue;
    use patricia_trie_ethereum as ethtrie;
    use ethtrie::trie::TrieMut;

    use ethereum_types::{H160, H256, U256};

    #[test]
    fn debug_output_supports_pretty_print() {
        let d = vec![
            b"A".to_vec(),
            b"AA".to_vec(),
            b"AB".to_vec(),
            b"B".to_vec(),
        ];

        let mut memdb = MemoryDB::<KeccakHasher, PrefixedKey<_>, DBValue>::default();
        let mut root = Default::default();
        let root = {
            let mut t = ethtrie::TrieDBMut::new(&mut memdb, &mut root);
            for x in &d {
                t.insert(x, x).unwrap();
            }
            t.root()
        };
        let t = ethtrie::TrieDB::new(&memdb, &root).unwrap();
        format!("{:#?}", t);
    }


}
