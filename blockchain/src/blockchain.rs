
use ethereum_types::{U256, H256};
use common_types::header::Header;
use common_types::BlockNumber;
use common_types::block::Block;
use parking_lot::{Mutex, RwLock};
use bloom_db;
use kvdb::{DBTransaction, KeyValueDB};
use bloom_db::{Key,Writable,Readable};

use std::sync::Arc;

pub struct BlockChain {
    db: Arc<dyn KeyValueDB>,
    best_block: RwLock<Block>,
}


impl BlockChain {
    pub fn new(db: Arc<dyn KeyValueDB>) -> Self {
        let mut bc = BlockChain {
            db,
            best_block: RwLock::new(Block::default()),
        };

        let best_block_hash = match bc.db.get(bloom_db::COL_EXTRA,b"best")
            .expect("Low-level database error when fetching 'best' block. Some issue with disk?") {
            Some(best) => {
                H256::from_slice(&best)
            },

            None => {
                let genesis = Header::genesis();

                let mut batch = DBTransaction::new();
                let hash = genesis.hash();
                batch.put(bloom_db::COL_HEADERS, hash.as_bytes(), genesis.encoded().as_slice());

                let mut genesis_block = Block::default();
                genesis_block.header = genesis.clone();
                genesis_block.transactions = vec![];
                batch.put(bloom_db::COL_BLOCK, hash.as_bytes(),genesis_block.rlp_bytes().as_slice());

                let block_number: BlockNumber = genesis.number();
                batch.put(bloom_db::COL_EXTRA,&block_number.key().as_ref(),hash.as_bytes());
                batch.put(bloom_db::COL_EXTRA,b"best",hash.as_bytes());

                bc.db.write(batch).expect("Low level database error when fetching 'best' block. Some issue with disk?");

                hash
            }
        };

        {
            let body = bc.db.read(bloom_db::COL_BLOCK,&best_block_hash).expect("not found body");
            let mut best_block = bc.best_block.write();
            *best_block = body;
        }

        bc

    }

    /// Get best block header
    pub fn best_block_header(&self) -> Header {
        self.best_block.read().header.clone()
    }

    pub fn set_best_block(&mut self, b: Block) {
        let mut bb = self.best_block.write();
        *bb = b;
    }
}

#[cfg(test)]
mod tests {

    use bloom_db;
    use std::sync::Arc;
    use super::*;
    use common_types::header::Header;

    #[test]
    fn genesis_test() {
        let memory_db = Arc::new(::kvdb_memorydb::create(bloom_db::NUM_COLUMNS));
        let bc = BlockChain::new(memory_db);
        let header = Header::genesis();
        let best_header = bc.best_block_header();
        assert_eq!(header.hash(),best_header.hash());
    }
}