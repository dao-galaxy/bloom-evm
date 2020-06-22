use structopt::StructOpt;
use evm::backend::{ApplyBackend};
use evm::executor::StackExecutor;
use evm::Config;
use bloom_state::State;


// ./target/debug/evmbin deposit --from 0000000000000000000000000000000000000001 --value 1
#[derive(Debug, StructOpt, Clone)]
pub struct DepositCmd {
	#[structopt(long = "from")]
	pub from: String,
	#[structopt(long = "value")]
	pub value: String,
}


impl DepositCmd {
	pub fn run(&self, backend: &mut State) {
		let from = self.from.parse().expect("From should be a valid address");
		let value: u128 = self.value.parse().expect("Value is invalid");

		let config = Config::istanbul();
		let gas_limit = 100000;
		let mut executor = StackExecutor::new(
			backend,
			gas_limit as usize,
			&config,
		);
		executor.deposit(from, value.into());
		let (values, logs) = executor.deconstruct();
		backend.apply(values, logs, true);
	}
}
