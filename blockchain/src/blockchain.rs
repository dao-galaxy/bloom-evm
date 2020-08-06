
use ethereum_types::{U256, H256};
use common_types::header::Header;
use common_types::{BlockNumber,block::{BlockHashList,Block}};
use common_types::transaction::{TransactionHashList, UnverifiedTransaction, TransactionBody,TransactionLocation};
use parking_lot::{RwLock};
use bloom_db;
use kvdb::{DBTransaction, KeyValueDB};
use bloom_db::{Writable,Readable};

use std::sync::Arc;

pub struct BlockChain {
    db: Arc<dyn KeyValueDB>,
    best_block_header: RwLock<Header>,
}

impl BlockChain {
    pub fn new(db: Arc<dyn KeyValueDB>) -> Self {
        let bc = BlockChain {
            db,
            best_block_header: RwLock::new(Header::default()),
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

                let genesis_block_transactions = TransactionHashList::default();

                batch.write(bloom_db::COL_BODIES, &hash,&genesis_block_transactions);

                let block_number: BlockNumber = genesis.number();
                let mut block_hash_list = BlockHashList::default();
                block_hash_list.push(hash);

                batch.write(bloom_db::COL_EXTRA,&block_number,&block_hash_list);
                batch.put(bloom_db::COL_EXTRA,b"best",hash.as_bytes());

                bc.db.write(batch).expect("Low level database error when fetching 'best' block. Some issue with disk?");

                hash
            }
        };

        {
            let header = bc.get_header_by_blockhash(best_block_hash).expect("not found header");
            let mut best_block_header = bc.best_block_header.write();
            *best_block_header = header;
        }
        bc
    }

    pub fn get_header_by_blockhash(&self, hash: H256) -> Option<Header> {
        self.db.read(bloom_db::COL_HEADERS,&hash)
    }

    /// Get best block header
    pub fn best_block_header(&self) -> Header {
        self.best_block_header.read().clone()
    }


    /// Get the hash of given block's number.
    pub fn block_hash(&self, index: BlockNumber) -> Option<H256> {
        let v: Option<BlockHashList> = self.db.read(bloom_db::COL_EXTRA, &index);
        v.map(|v| {
            v.block_hashes()[0]
        })
    }

    /// Get the block of given block's number.
    pub fn block_by_number(&self, index: BlockNumber) -> Option<Block> {
        self.block_hash(index).map(|hash| self.block_by_hash(hash).unwrap())
    }

    pub fn block_by_hash(&self, hash: H256) -> Option<Block> {
        let header = self.get_header_by_blockhash(hash.clone());
        if header.is_none() {
            return None
        }

        let header = header.unwrap();
        let transaction_hash_list = self.transaction_hash_list_by_block_hash(hash.clone());
        let txs = transaction_hash_list.map(|t| {
            let mut txs:Vec<UnverifiedTransaction> = vec![];
            for tx_hash in t.transactions() {
                let body = self.transaction_body_by_hash(tx_hash.clone()).unwrap();
                txs.push(body.transaction.clone());
            }
            txs
        });
        let txs = txs.unwrap_or(vec![]);
        Some(Block::new(header,txs))
    }

    pub fn transaction_hash_list_by_block_hash(&self, block_hash: H256) -> Option<TransactionHashList> {
        self.db.read(bloom_db::COL_BODIES,&block_hash)
    }

    pub fn transaction_body_by_hash(&self,tx_hash: H256) -> Option<TransactionBody>{
        self.db.read(bloom_db::COL_TRANSACTION,&tx_hash)
    }

    pub fn insert_block(&mut self,block: Block) -> Result<(),&str> {
        let block_hash = block.header.hash();
        let transactions = TransactionHashList::from(block.transactions.clone());
        let mut batch = DBTransaction::new();
        // write block_hash -> header
        batch.put(bloom_db::COL_HEADERS, block_hash.as_bytes(), block.header.encoded().as_slice());
        // write block_hash -> transaction list
        batch.write(bloom_db::COL_BODIES, &block_hash,&transactions);

        // write block_number -> block hash list
        let block_number: BlockNumber = block.header.number();
        let mut block_hashes = self.db.read(bloom_db::COL_EXTRA,&block_number).
            map_or(BlockHashList::default(),|b| b);
        block_hashes.push(block_hash).unwrap();
        batch.write(bloom_db::COL_EXTRA,&block_number,&block_hashes);

        // write tx hash -> tx body
        for (i,tx) in block.transactions.iter().enumerate() {
            let loc = TransactionLocation::new(block_hash.clone(),block_number,i as u64);
            self.write_transaction(&mut batch, tx.clone(), loc);
        }

        self.db.write(batch).expect("Low level database error when fetching 'best' block. Some issue with disk?");

        Ok(())
    }

    fn write_transaction(&self, batch: &mut DBTransaction, tx: UnverifiedTransaction,loc : TransactionLocation){
        let tx_hash = tx.hash();
        let mut tx_body:TransactionBody = self.db.read(bloom_db::COL_TRANSACTION,&tx_hash).
                map_or(TransactionBody::new(tx.clone()),|b| b);
        tx_body.append_location(loc);
        batch.write(bloom_db::COL_TRANSACTION,&tx_hash,&tx_body);
    }

    /// Get best block hash.
    pub fn best_block_hash(&self) -> H256 {
        self.best_block_header.read().hash()
    }

    /// Get best block number.
    pub fn best_block_number(&self) -> BlockNumber {
        self.best_block_header.read().number()
    }

    /// Get best block timestamp.
    pub fn best_block_timestamp(&self) -> u64 {
        self.best_block_header.read().timestamp()
    }

    /// Get best block total difficulty.
    pub fn best_block_difficulty(&self) -> U256 {
        self.best_block_header.read().difficulty()
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

        let b0 = bc.block_by_number(header.number()).unwrap();
        println!("hash1:{}",header.hash());
        println!("hash2:{}",b0.header.hash());
        assert_eq!(header.hash(),b0.header.hash());
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

        let parent_hash = header.hash();
        header.set_parent_hash(parent_hash.clone());
        let mut block2 = Block::default();
        header.set_number(2 as u64);
        block2.header = header.clone();
        block2.transactions = vec![];
        let ret = bc.insert_block(block2.clone()).unwrap();
        assert_eq!(ret,());

        let b1 = bc.block_by_number(1).unwrap();
        let b2 = bc.block_by_number(2).unwrap();
        assert_eq!(b1.header.hash(),b2.header.parent_hash());
    }
}