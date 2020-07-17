
use ethereum_types::{U256, H256};
use journaldb::JournalDB;
use common_types::header::Header;
use parking_lot::{Mutex, RwLock};
use bloom_db;


pub struct BlockChain {
    db: Box<dyn JournalDB>,
    best_block: RwLock<Header>,
}


impl BlockChain {
    pub fn new(db: Box<dyn JournalDB>) -> Self {
        let mut bc = BlockChain {
            db,
            best_block: RwLock::new(Header::default()),
        };

        let best_block_hash = match bc.db.backing().get(bloom_db::COL_EXTRA,b"best")
            .expect("Low-level database error when fetching 'best' block. Some issue with disk?") {
            Some(best) => {
                H256::from_slice(&best)
            },

            None => {
                let genesis = Header::genesis();
                genesis.hash()
            }
        };

        bc

    }

    /// Get best block header
    pub fn best_block_header(&self) -> Header {
        self.best_block.read().clone()
    }

    pub fn set_best_block(&mut self, b: Header) {
        let mut bb = self.best_block.write();
        *bb = b;
    }
}