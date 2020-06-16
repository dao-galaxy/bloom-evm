use ethereum_types::{H160, U256};
use evm::executor::StackExecutor;
use evm::backend::Backend;
use evm::backend::MemoryVicinity as Vicinity;
use evm::backend::MemoryAccount as Account;
use evm::ExitReason;
use evm::backend::ApplyBackend;
use evm::Config;
use std::collections::BTreeMap;

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
pub fn execute_evm<F, R,B>(
	source: H160,
	value: U256,
	gas_limit: u32,
	gas_price: U256,
	nonce: Option<U256>,
	f: F,
	backend: & mut B
) -> Result<R, Error> where
	F: FnOnce(&mut StackExecutor<B>) -> (R, ExitReason),
	B: Backend + ApplyBackend + Clone,
{
	assert!(gas_price >= U256::zero(), Error::GasPriceTooLow);

//	let vicinity = Vicinity {
//		gas_price: U256::zero(),
//		origin: H160::zero(),
//		chain_id: U256::zero(),
//		block_hashes: Vec::new(),
//		block_number: U256::zero(),
//		block_coinbase: H160::zero(),
//		block_timestamp: U256::zero(),
//		block_difficulty: U256::zero(),
//		block_gas_limit: U256::zero(),
//	};
//	let state = BTreeMap::<H160, Account>::new();
//	let mut backend = Backend::new(&vicinity, state);
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