mod account_cmd;
mod call_message_cmd;
mod example;
mod deposit_cmd;
mod contract_cmd;

use std::collections::BTreeMap;

use structopt::StructOpt;
use account_cmd::AccountCmd;
use call_message_cmd::CallMessageCmd;
use example::CreateContractCmd;
use deposit_cmd::DepositCmd;
use contract_cmd::ContractCmd;

use ethereum_types::{U256, H160};
use evm::backend::MemoryBackend as Backend;
use evm::backend::MemoryVicinity as Vicinity;
use evm::backend::MemoryAccount as Account;
use evm::executor::StackExecutor;
use evm::Config;

#[derive(Debug, Clone, StructOpt)]
pub enum Subcommand {
	Account(AccountCmd),
	Call(CallMessageCmd),
	Create(CreateContractCmd),
	Deposit(DepositCmd),
	Contract(ContractCmd),
}

impl Subcommand {
	pub fn run(&self) {
		let vicinity = Vicinity {
			gas_price: U256::zero(),
			origin: H160::zero(),
			chain_id: U256::zero(),
			block_hashes: Vec::new(),
			block_number: U256::zero(),
			block_coinbase: H160::zero(),
			block_timestamp: U256::zero(),
			block_difficulty: U256::zero(),
			block_gas_limit: U256::zero(),
		};
		let state = BTreeMap::<H160, Account>::new();
		let backend = Backend::new(&vicinity, state);
		let config = Config::istanbul();
		let gas_limit = 100000;
		let mut executor = StackExecutor::new(
			&backend,
			gas_limit as usize,
			&config,
		);

		match self {
			Subcommand::Account(cmd) => {
				cmd.run(backend);
			}
			Subcommand::Call(cmd) => {
				cmd.run(&mut executor);
			}
			Subcommand::Create(cmd) => {
				cmd.run(&mut executor);
			}
			Subcommand::Deposit(cmd) => {
				cmd.run(&mut executor);
			}
			Subcommand::Contract(cmd) => {
				cmd.run(backend);
			}
		}
	}
}