use structopt::StructOpt;
use evm::backend:: MemoryBackend as Backend;
use evm::executor::StackExecutor;

// ./target/debug/evmbin deposit --from 0000000000000000000000000000000000000001 --value 1
#[derive(Debug, StructOpt, Clone)]
pub struct DepositCmd {
	#[structopt(long = "from")]
	pub from: String,
	#[structopt(long = "value")]
	pub value: String,
}


impl DepositCmd {
	pub fn run(&self, executor: &mut StackExecutor<Backend>) {
		let from = self.from.parse().expect("From should be a valid address");
		let value: u128 = self.value.parse().expect("Value is invalid");

		executor.deposit(from, value.into());
	}
}
