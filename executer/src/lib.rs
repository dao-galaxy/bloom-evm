
use ethereum_types::{H160, U256, H256,Address};
use evm::executor::StackExecutor;
use evm::ExitReason;
use evm::backend::ApplyBackend;
use evm::Config;
use evm::Transfer;
use bloom_state::{State,BackendVicinity,Factories,AccountFactory};
use common_types::{
    BlockNumber,
    header::Header,
    block::Block,
    transaction::SignedTransaction,
    transaction::UnverifiedTransaction
};
use journaldb::JournalDB;
use trie_db::TrieSpec;
use ethtrie;
use std::sync::Arc;
use blockchain::BlockChain;
use bloom_db;
use keccak_hash::KECCAK_NULL_RLP;
use parity_bytes::Bytes;
use rlp::Encodable;




#[derive(Debug)]
pub enum Error
{
    /// Not enough balance to perform action
    BalanceLow,
    /// Calculating total fee overflowed
    FeeOverflow,
    /// Calculating total payment overflowed
    PaymentOverflow,
    /// Withdraw fee failed
    WithdrawFailed,
    /// Gas price is too low.
    GasPriceTooLow,
    /// Call failed
    ExitReasonFailed,
    /// Call reverted
    ExitReasonRevert,
    /// Call returned VM fatal error
    ExitReasonFatal,
    /// Nonce is invalid
    InvalidNonce,
}

// /// Check whether an account is empty.
// pub fn is_account_empty(address: &H160) -> bool {
// 	let account = Account::get(address).expect("account not exists");
// 	let account_code = AccountCode::get(address).expect("account code not exists");
// 	let code_len = account_code.code().len();
//
// 	account.nonce == U256::zero() &&
// 		account.balance == U256::zero() &&
// 		code_len == 0
// }
//
// /// Remove an account if its empty.
// pub fn remove_account_if_empty(address: &H160) {
// 	if is_account_empty(address) {
// 		remove_account(address);
// 	}
// }
//
// /// Remove an account from state.
// fn remove_account(address: &H160) {
// 	Account::remove(address);
// 	AccountCode::remove(address);
// 	// AccountStorages::remove_prefix(address);
// }

/// Execute an EVM operation.
pub fn execute_evm<F, R>(
    source: H160,
    value: U256,
    gas_limit: u32,
    gas_price: U256,
    nonce: Option<U256>,
    f: F,
    backend: & mut State
) -> Result<R, Error> where
    F: FnOnce(&mut StackExecutor<State>) -> (R, ExitReason),
{
    assert!(gas_price >= U256::zero(), Error::GasPriceTooLow);

    let config = Config::istanbul();
    let mut executor = StackExecutor::new(
        backend,
        gas_limit as usize,
        &config,
    );

    let total_fee = gas_price.checked_mul(U256::from(gas_limit)).ok_or(Error::FeeOverflow)?;
    let total_payment = value.checked_add(total_fee).ok_or(Error::PaymentOverflow)?;
    let state_account = executor.account_mut(source.clone());
    let source_account = state_account.basic.clone();
    println!("balance:{}",source_account.balance);
    println!("payment:{}",total_payment);
    assert!(source_account.balance >= total_payment, Error::BalanceLow);
    executor.withdraw(source.clone(), total_fee).map_err(|_| Error::WithdrawFailed)?;

    if let Some(nonce) = nonce {
        assert!(source_account.nonce == nonce, Error::InvalidNonce);
    }

    let (retv, reason) = f(&mut executor);

    let ret = match reason {
        ExitReason::Succeed(_) => Ok(retv),
        ExitReason::Error(_) => Err(Error::ExitReasonFailed),
        ExitReason::Revert(_) => Err(Error::ExitReasonRevert),
        ExitReason::Fatal(_) => Err(Error::ExitReasonFatal),
    };

    let actual_fee = executor.fee(gas_price);
    executor.deposit(source, total_fee.saturating_sub(actual_fee));

    let (values, logs) = executor.deconstruct();

    backend.apply(values, logs, true);

    ret
}

/// Execute an transfer operation.
pub fn execute_transfer(
    from: H160,
    to: H160,
    value: U256,
    gas_limit: u32,
    gas_price: U256,
    backend: & mut State
) -> Result<(), Error>
{
    assert!(gas_price >= U256::zero(), Error::GasPriceTooLow);

    let config = Config::istanbul();
    let mut executor = StackExecutor::new(
        backend,
        gas_limit as usize,
        &config,
    );

    let total_fee = gas_price.checked_mul(U256::from(gas_limit)).ok_or(Error::FeeOverflow)?;
    let total_payment = value.checked_add(total_fee).ok_or(Error::PaymentOverflow)?;
    let state_account = executor.account_mut(from.clone());
    assert!(state_account.basic.balance >= total_payment, Error::BalanceLow);
    state_account.basic.nonce += U256::one();

    executor.withdraw(from.clone(), total_fee).map_err(|_| Error::WithdrawFailed)?;
    executor.transfer(Transfer{
        source: from,
        target: to,
        value,
    }).unwrap();

    let (values, logs) = executor.deconstruct();
    backend.apply(values, logs, true);

    Ok(())
}

fn apply_block(header: Header,
               transactions: Vec<SignedTransaction>,
               db: Arc<dyn (::kvdb::KeyValueDB)>) {

    let trie_layout = ethtrie::Layout::default();
    let trie_spec = TrieSpec::default();
    let trie_factory =  ethtrie::TrieFactory::new(trie_spec,trie_layout);

    let account_factory = AccountFactory::default();
    let factories = Factories{
        trie: trie_factory,
        accountdb: account_factory,
    };

    let mut bc = BlockChain::new(db.clone());
    let mut journal_db = journaldb::new(db,journaldb::Algorithm::Archive,bloom_db::COL_STATE);


    let best_header = bc.best_block_header();
    let mut root = best_header.state_root();

    execute_transaction(true, &header, root, &transactions, &factories, journal_db);

    let mut block = Block::default();
    block.header = header.clone();
    let mut txs: Vec<UnverifiedTransaction> = vec![];
    for tx in transactions {
        let utx = UnverifiedTransaction::from(tx);
        txs.push(utx);
    }
    block.transactions = txs;
    bc.insert_block(block).unwrap();
}

/// create header and not commit to state to disk
pub fn create_header(
    number: BlockNumber,
    timestamp: u64,
    author: Address,
    extra_data: Bytes,
    gas_used: U256,
    gas_limit: U256,
    difficulty: U256,
    transactions: Vec<SignedTransaction>,
    db: Arc<dyn (kvdb::KeyValueDB)>
) -> Header {

    assert!(number > 0 , "Block number must be greater 0");
    let trie_layout = ethtrie::Layout::default();
    let trie_spec = TrieSpec::default();
    let trie_factory =  ethtrie::TrieFactory::new(trie_spec,trie_layout);

    let account_factory = AccountFactory::default();
    let factories = Factories{
        trie: trie_factory,
        accountdb: account_factory,
    };

    let mut bc = BlockChain::new(db.clone());
    let mut journal_db = journaldb::new(db.clone(),journaldb::Algorithm::Archive,bloom_db::COL_STATE);

    let parent_block_number = number - 1;
    let parent_block = bc.block_by_number(parent_block_number).expect("Block body does not exist");
    let parent_header = parent_block.header;
    let mut root = parent_header.state_root();

    let mut header = Header::default();
    header.set_number(number);
    header.set_timestamp(timestamp);
    header.set_author(author);
    header.set_extra_data(extra_data);
    header.set_gas_limit(gas_limit);
    header.set_difficulty(difficulty);
    header.set_parent_hash(header.hash());
    let (total_gas_used, new_state_trie_root) = execute_transaction(
        false,
        &mut header,
        root,
        &transactions,
        &factories,
        journal_db
    );
    header.set_gas_used(total_gas_used);
    header.set_state_root(new_state_trie_root);
    header.set_transactions_root(build_transaction_trie(transactions, &db, &factories));
    header
}

pub fn execute_transaction(
    commit : bool,
    header : &Header,
    state_trie_root: H256,
    transactions: &Vec<SignedTransaction>,
    factories : &Factories,
    journal_db : Box<dyn JournalDB> ) -> (U256, H256) {

    let mut total_gas_used = U256::zero();
    let mut new_state_trie_root = state_trie_root;

    for tx in transactions {
        let vicinity = BackendVicinity {
            gas_price: tx.gas_price,
            origin: tx.sender(),
            chain_id: U256::zero(),
            block_hashes: Vec::new(),
            block_number: U256::from(header.number()),
            block_coinbase: header.author(),
            block_timestamp: U256::from(header.timestamp()),
            block_difficulty: header.difficulty(),
            block_gas_limit: header.gas_limit(),
        };

        let mut backend = match state_trie_root == KECCAK_NULL_RLP {
            true => {
                State::new(&vicinity,journal_db.boxed_clone(),factories.clone())
            },
            false => {
                State::from_existing(state_trie_root, &vicinity, journal_db.boxed_clone(), factories.clone()).unwrap()
            }
        };

        let from = tx.sender();
        let to = tx.receiver();
        let value = tx.value;
        let gas_limit = tx.gas.as_u32();
        let gas_price = tx.gas_price;
        let nonce = Some(tx.nonce);

        let config = Config::istanbul();
        let executor = StackExecutor::new(
            &mut backend,
            gas_limit as usize,
            &config,
        );

        match to {
            None => {
                let (contract_address,gas_left) = execute_evm(
                    from.clone(),
                    value,
                    gas_limit,
                    gas_price,
                    nonce,
                    |executor| {

                        let contract_addr = executor.create_address(
                            evm::CreateScheme::Legacy { caller: from.clone()});

                        let retv = executor.transact_create(
                            from,
                            value,
                            tx.data.clone(),
                            gas_limit as usize,
                        );

                        let gas_left = executor.gas();
                        ((contract_addr,gas_left),retv)
                    },
                    &mut backend
                ).expect("Create contract failed");
                let gas_used = gas_limit - gas_left as u32;
                total_gas_used = total_gas_used + U256::from(gas_used);
            },

            Some(contract_address) => {
                let gas_left = execute_evm(
                    from,
                    value,
                    gas_limit,
                    gas_price,
                    nonce,
                    |executor| {

                        let retv = executor.transact_call(
                            from,
                            contract_address,
                            value,
                            tx.data.clone(),
                            gas_limit as usize,
                        );

                        let gas_left = executor.gas();

                        (gas_left, retv)
                    },
                    &mut backend
                ).expect("Call message failed");

                let gas_used = gas_limit - gas_left as u32;
                total_gas_used = total_gas_used + U256::from(gas_used);
            }
        }
        if commit {
            backend.commit();
        }
        new_state_trie_root = backend.root();
    }

    (total_gas_used, new_state_trie_root)
}

pub fn build_transaction_trie(transactions: Vec<SignedTransaction>, db: &Arc<dyn (kvdb::KeyValueDB)>, factories : &Factories) -> H256 {
    // create transaction trie in memory-db
    let mut transaction_trie_root = H256::default();
    {
        let mut journal_db = journaldb::new(db.clone(), journaldb::Algorithm::Archive, bloom_db::COL_TRANSACTION);
        let mut transaction_trie = factories.trie.create(journal_db.as_hash_db_mut(), &mut transaction_trie_root);
        for tx in transactions {
            let utx = UnverifiedTransaction::from(tx);
            let tx_hash = utx.hash();
            let tx_raw_data = utx.rlp_bytes();
            transaction_trie.insert(tx_hash.as_bytes(), tx_raw_data.as_slice()).unwrap();
        }
    }
    transaction_trie_root
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
    fn apply_block_test() {
        let memory_db = Arc::new(::kvdb_memorydb::create(bloom_db::NUM_COLUMNS));
        let bc = BlockChain::new(memory_db.clone());
        let mut header = Header::default();
        let best_header = bc.best_block_header();
        header.set_parent_hash(best_header.hash());
        apply_block(header,vec![],memory_db.clone());
    }

    #[test]
    fn create_header_test() {
        let memory_db = Arc::new(::kvdb_memorydb::create(bloom_db::NUM_COLUMNS));
        let number: BlockNumber = 1;
        let ttimestamp: u64  = 1;
        let author: Address = Address::default();
        let extra_data: Bytes = vec![];
        let gas_used: U256 = U256::zero();
        let gas_limit: U256 = U256::zero();
        let difficulty: U256 = U256::zero();
        let transactions: Vec<SignedTransaction> = vec![];

        let header = create_header(number,ttimestamp,author,extra_data,gas_used,gas_limit,difficulty,transactions,memory_db);
        println!("{:?}",header);
    }

}

