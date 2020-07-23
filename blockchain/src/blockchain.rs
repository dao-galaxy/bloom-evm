
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

    pub fn best_block(&self) -> Block {
        self.best_block.read().clone()
    }

    pub fn set_best_block(&mut self, b: Block) {
        let mut bb = self.best_block.write();
        *bb = b;
    }

    pub fn block_by_hash(&self, hash: H256) -> Option<Block> {
        self.db.read(bloom_db::COL_BLOCK, &hash)
    }

    /// Get the hash of given block's number.
    pub fn block_hash(&self, index: BlockNumber) -> Option<H256> {
        self.db.read(bloom_db::COL_EXTRA, &index)
    }

    pub fn insert_block(&mut self,block: Block) -> Result<(),&str> {
        let best_hash = self.best_block_hash();
        let parent_hash = block.header.parent_hash();
        if best_hash != *parent_hash {
            return Err("not right block");
        }

        let block_hash = block.header.hash();
        let mut batch = DBTransaction::new();
        batch.put(bloom_db::COL_HEADERS, block_hash.as_bytes(), block.header.encoded().as_slice());
        batch.put(bloom_db::COL_BLOCK, block_hash.as_bytes(),block.rlp_bytes().as_slice());

        let block_number: BlockNumber = block.header.number();
        batch.put(bloom_db::COL_EXTRA,&block_number.key().as_ref(),block_hash.as_bytes());
        batch.put(bloom_db::COL_EXTRA,b"best",block_hash.as_bytes());

        self.db.write(batch).expect("Low level database error when fetching 'best' block. Some issue with disk?");

        self.set_best_block(block.clone());

        Ok(())
    }

    /// Get best block hash.
    pub fn best_block_hash(&self) -> H256 {
        self.best_block.read().header.hash()
    }

    /// Get best block number.
    pub fn best_block_number(&self) -> BlockNumber {
        self.best_block.read().header.number()
    }

    /// Get best block timestamp.
    pub fn best_block_timestamp(&self) -> u64 {
        self.best_block.read().header.timestamp()
    }

    /// Get best block total difficulty.
    pub fn best_block_difficulty(&self) -> U256 {
        *self.best_block.read().header.difficulty()
    }

}

#[cfg(test)]
mod tests {

    use bloom_db;
    use std::sync::Arc;
    use super::*;
    use common_types::header::Header;
    use ethereum_types::{Address, H256, U256};
    use std::str::FromStr;


    #[test]
    fn genesis_test() {
        let memory_db = Arc::new(::kvdb_memorydb::create(bloom_db::NUM_COLUMNS));
        let bc = BlockChain::new(memory_db);
        let header = Header::genesis();
        let best_header = bc.best_block_header();
        assert_eq!(header.hash(),best_header.hash());
    }

    #[test]
    fn block_hash_test() {
        let memory_db = Arc::new(::kvdb_memorydb::create(bloom_db::NUM_COLUMNS));
        let mut bc = BlockChain::new(memory_db);


        let mut header = Header::default();


        let parent_hash = bc.best_block_hash();
        header.set_parent_hash(parent_hash.clone());


        let author = "5a0b54d5dc17e0aadc383d2db43b0a0d3e029c4c";
        let author = Address::from_str(author).unwrap();
        header.set_author(author);

        let state_root = "1dcc4de8dec75d7aab85b567b6ccd41ad312451b948a7413f0a142fd40d49347";
        let state_root = H256::from_str(state_root).unwrap();
        header.set_state_root(state_root);

        let tx_root = "e4505deee005a3c3dc4aa696ab429562e51a08190861b81a09c652487426ac72";
        let tx_root = H256::from_str(tx_root).unwrap();
        header.set_transactions_root(tx_root);

        let difficulty = U256::zero();
        header.set_difficulty(difficulty);

        header.set_number(1 as u64);
        header.set_gas_limit(U256::zero());
        header.set_gas_used(U256::zero());
        header.set_timestamp(13000000 as u64);

        let data = b"hello".to_vec();
        header.set_extra_data(data);

        let mut block1 = Block::default();
        block1.header = header.clone();
        block1.transactions = vec![];


        let ret = bc.insert_block(block1).unwrap();
        assert_eq!(ret,());

        let parent_hash = bc.best_block_hash();
        header.set_parent_hash(parent_hash.clone());
        let mut block2 = Block::default();
        block2.header = header.clone();
        block2.transactions = vec![];
        let ret = bc.insert_block(block2.clone()).unwrap();
        assert_eq!(ret,());

        assert_eq!(bc.best_block_hash(),block2.header.hash());

    }
}