mod executer;
mod parser;
mod commands;

use commands::Subcommand;
use structopt::StructOpt;

#[derive(Debug, StructOpt)]
pub struct Cli {
	#[structopt(subcommand)]
	pub subcmd: Option<Subcommand>,

	#[structopt(flatten)]
	pub run: RunCmd,
}

/// The `run` command used to run a node.
#[derive(Debug, StructOpt, Clone)]
pub struct RunCmd {
	/// From account.
	#[structopt(long = "from")]
	pub from: Option<String>,
	/// Value.
	#[structopt(long = "value")]
	pub value: Option<String>,
}

fn main() {
	let cli = Cli::from_args();

	if let Some(ref subcmd) = cli.subcmd {
		subcmd.run();
	} else {
		println!("{:#?}", cli);
	}
}
