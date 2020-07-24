
use ethereum_types::{H160, U256, H256};
use evm::executor::StackExecutor;
use evm::ExitReason;
use evm::backend::ApplyBackend;
use evm::Config;
use evm::Transfer;
use bloom_state::{State,BackendVicinity,Factories,AccountFactory};
use common_types::header::Header;
use common_types::transaction::SignedTransaction;
use journaldb::JournalDB;
use trie_db::TrieSpec;
use ethtrie;
use std::sync::Arc;
use blockchain::BlockChain;
use bloom_db;


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

    //println!("{:?}", &backend);

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

fn apply_block(block_header: &Header,
               transactions: &Vec<SignedTransaction>,
               db: Arc<dyn (::kvdb::KeyValueDB)>,
               is_commit: bool ) {


    let trie_layout = ethtrie::Layout::default();
    let trie_spec = TrieSpec::default();
    let trie_factory =  ethtrie::TrieFactory::new(trie_spec,trie_layout);

    let account_factory = AccountFactory::default();
    let factories = Factories{
        trie: trie_factory,
        accountdb: account_factory,
    };

    let bc = BlockChain::new(db.clone());
    let mut journal_db = journaldb::new(db,journaldb::Algorithm::Archive,bloom_db::COL_STATE);


    // todo get parent block state root
    let best_header = bc.best_block_header();
    let root = best_header.state_root();

    for tx in transactions{

        let vicinity = BackendVicinity {
            gas_price: tx.gas_price,
            origin: tx.sender(),
            chain_id: U256::zero(),
            block_hashes: Vec::new(),
            block_number: U256::from(best_header.number()),
            block_coinbase: best_header.author(),
            block_timestamp: U256::from(best_header.timestamp()),
            block_difficulty: best_header.difficulty(),
            block_gas_limit: best_header.gas_limit(),
        };

        let mut backend = match root == H256::zero() {
            true => {
                State::new(&vicinity,journal_db.boxed_clone(),factories.clone())
            },
            false => {
                State::from_existing(root,&vicinity,journal_db.boxed_clone(),factories.clone()).unwrap()
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
                let contract_address = execute_evm(
                    from.clone(),
                    value,
                    gas_limit,
                    gas_price,
                    nonce,
                    |executor| {
                        (executor.create_address(
                            evm::CreateScheme::Legacy { caller: from.clone() },
                        ), executor.transact_create(
                            from,
                            value,
                            tx.data.clone(),
                            gas_limit as usize,
                        ))
                    },
                    &mut backend
                ).expect("Create contract failed");
            },

            Some(contract_address) => {
                let retv = execute_evm(
                    from,
                    value,
                    gas_limit,
                    gas_price,
                    nonce,
                    |executor| ((), executor.transact_call(
                        from,
                        contract_address,
                        value,
                        tx.data.clone(),
                        gas_limit as usize,
                    )),
                    &mut backend
                ).expect("Call message failed");
            }
        }

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
    fn apply_block_test() {
        let memory_db = Arc::new(::kvdb_memorydb::create(bloom_db::NUM_COLUMNS));
        let bc = BlockChain::new(memory_db.clone());
        let header = Header::genesis();
        apply_block(&header,&vec![],memory_db.clone(),false);
    }

}

