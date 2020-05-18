use structopt::StructOpt;
use evm::backend::Backend;
use evm::backend::MemoryBackend;

// ./target/debug/evmbin account --from 0000000000000000000000000000000000000001
#[derive(Debug, StructOpt, Clone)]
pub struct AccountCmd {
	#[structopt(long = "from")]
	pub from: String,
}

impl AccountCmd {
	pub fn run(&self, backend: MemoryBackend) {
		let from = self.from.parse().expect("From should be a valid address");
		let account = backend.basic(from);
		println!("{:?}", account);
	}
}