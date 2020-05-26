use crate::executer;
use crate::commands::account_cmd;
use ethereum_types::{H160, U256, H256};
use evm::backend::{MemoryBackend, Apply, Basic, ApplyBackend, Backend};
use evm::executor::StackExecutor;
use hex;
use structopt::StructOpt;
use evm::Config;
use std::fs::File;
use std::io::Read;
use std::str::FromStr; // !!! Necessary for H160::from_str(address).expect("...");
use std::collections::BTreeMap;
use crate::commands::account_cmd::Account;

// target/debug/bloom-evm contract deploy --from 0000000000000000000000000000000000000001  --value 0 --gas 100000 --gas-price 0 --code-file ./code-file
// target/debug/bloom-evm contract deploy --from 0000000000000000000000000000000000000001  --value 0 --gas 100000 --gas-price 0 --code 000000
// target/debug/bloom-evm contract call --from 0000000000000000000000000000000000000001  --to 0000000000000000000000000000000000000002 --value 0 --gas 100000 --gas-price 0 --data 000000

#[derive(Debug, StructOpt, Clone)]
pub struct ContractCmd {
    #[structopt(subcommand)]
    cmd: Command
}

#[derive(StructOpt, Debug, Clone)]
enum Command {
    /// Deploy contract
    Deploy {
        /// The address which deploy contact
        #[structopt(long = "from")]
        from: String,

        /// The value to deposit in contract
        #[structopt(long = "value")]
        value: String,

        /// The gas limit for deploying contract
        #[structopt(long = "gas")]
        gas: u32,

        /// The gas price for deploying contract(Wei)
        #[structopt(long = "gas-price")]
        gas_price: String,

        /// The contract binary code
        #[structopt(long = "code")]
        code: Option<String>,

        /// The code file
        #[structopt(long = "code-file")]
        code_file: Option<String>,

    },

    /// Message call
    Call {
        /// The address which send messageCall
        #[structopt(long = "from")]
        from: String,

        /// The value (Wei) for messageCall
        #[structopt(long = "value")]
        value: String,

        /// The receiver address for messageCall
        #[structopt(long = "to")]
        to: String,

        /// The gas limit for messageCall
        #[structopt(long = "gas")]
        gas: u32,

        /// The gas price (Wei) for messageCall
        #[structopt(long = "gas-price")]
        gas_price: String,

        /// The input data for messageCall
        #[structopt(long = "data")]
        data: Option<String>,

        /// The input data file for messageCall
        #[structopt(long = "data-file")]
        data_file: Option<String>,

    }
}



impl ContractCmd {
    pub fn run(&self, mut backend: MemoryBackend) {
        match &self.cmd {
            Command::Deploy {from,value,gas,gas_price,code,code_file} => {

                let from = H160::from_str(from).expect("From should be a valid address");
                let value = U256::from_dec_str(value.as_str()).expect("Value is invalid");
                let gas_price = U256::from_dec_str(gas_price.as_str()).expect("Gas price is invalid");
                let gas_limit = *gas;

                let mut contents = String::new();

                let code = match code {
                    Some(c) => {
                        Ok(c)
                    }
                    None => {
                        let ret = match code_file {
                            Some(file) => {
                                let mut f = File::open(file).expect(" code file not found");

                                f.read_to_string(&mut contents)
                                    .expect("something went wrong reading the file");
                                Ok(&contents)
                            }

                            None => {
                                Err(())
                            }
                        };
                        ret
                    }
                }.expect("--code or --code-file must be provided one of them ");




                let code = hex::decode(code.as_str()).expect("Code is invalid");
                let config = Config::istanbul();
                let executor = StackExecutor::new(
                    &backend,
                    gas_limit as usize,
                    &config,
                );
                let nonce = Some(executor.nonce(from.clone()));


                {
                    let mut applies = Vec::<Apply<BTreeMap<H256, H256>>>::new();

                    applies.push(Apply::Delete {
                        address: from.clone(),
                    });

                    applies.push(Apply::Modify {
                        address: from.clone(),
                        basic: Basic{
                            balance: U256::from_dec_str("90000000000000000").expect("error"),
                            nonce : U256::zero(),
                        },
                        code: None,
                        storage: BTreeMap::new(),
                        reset_storage: false,
                    });

                    backend.apply(applies,Vec::new(),false);
                    let account = Account::new(&backend,from);
                    println!("{}", account);
                    println!("{:#?}", backend);
                }

                let contract_address = executer::execute_evm(
                    &mut backend,
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
                            code,
                            gas_limit as usize,
                        ))
                    },
                ).expect("Create contract failed");

                let account = account_cmd::Account::new(&backend, contract_address.clone());
                println!("Create contract successful, {}", account);
                println!("{:#?}", backend);

                let code = backend.code(contract_address.clone());
                let code_str = hex::encode(code);
                println!("code={:?}",code_str);
            }

            Command::Call {from,value,to,gas,gas_price,data,data_file} => {
                let from = H160::from_str(from).expect("From should be a valid address");
                let to = H160::from_str(to).expect("To should be a valid address");
                let value = U256::from_dec_str(value.as_str()).expect("Value is invalid");
                let gas_price = U256::from_dec_str(gas_price.as_str()).expect("Gas price is invalid");
                let gas_limit = *gas;

                let mut contents = String::new();

                let data = match data {
                    Some(d) => {
                        Ok(d)
                    }
                    None => {
                        let ret = match data_file {
                            Some(file) => {
                                let mut f = File::open(file).expect(" data file not found");

                                f.read_to_string(&mut contents)
                                    .expect("something went wrong reading the file");
                                Ok(&contents)
                            }

                            None => {
                                Err(())
                            }
                        };
                        ret
                    }
                }.unwrap_or(&contents);


                {
                    let mut applies = Vec::<Apply<BTreeMap<H256, H256>>>::new();

                    applies.push(Apply::Delete {
                        address: from.clone(),
                    });

                    applies.push(Apply::Modify {
                        address: from.clone(),
                        basic: Basic{
                            balance: U256::from_dec_str("90000000000000000").expect("error"),
                            nonce : U256::zero(),
                        },
                        code: None,
                        storage: BTreeMap::new(),
                        reset_storage: false,
                    });

                    let code_str = String::from("60806040526004361061004c576000357c0100000000000000000000000000000000000000000000000000000000900463ffffffff1680636057361d14610051578063b05784b81461008c575b600080fd5b34801561005d57600080fd5b5061008a6004803603602081101561007457600080fd5b81019080803590602001909291905050506100b7565b005b34801561009857600080fd5b506100a1610162565b6040518082815260200191505060405180910390f35b80600081905550600260009054906101000a900473ffffffffffffffffffffffffffffffffffffffff1673ffffffffffffffffffffffffffffffffffffffff166108fc60019081150290604051600060405180830381858888f19350505050158015610127573d6000803e3d6000fd5b507f69404ebde4a368ae324ed310becfefc3edfe9e5ebca74464e37ffffd8309a3c1816040518082815260200191505060405180910390a150565b6000805490509056fea165627a7a7230582027d96b9c1b889b14ef1473414b98ab575bd02a8f400c56ff03153ce3cd968b440029");
                    let code = hex::decode(code_str.as_str()).expect("code is invalid");
                    let mut storage:BTreeMap<H256, H256> = BTreeMap::new();
                    let key1 = H256::from_str("0000000000000000000000000000000000000000000000000000000000000001").expect("");
                    let val1 = H256::from_str("000000000000000000000000000000000000000000000000000000000000000b").expect("");
                    storage.insert(key1,val1);

                    let key1 = H256::from_str("0000000000000000000000000000000000000000000000000000000000000002").expect("");
                    let val1 = H256::from_str("0000000000000000000000000000000000000000000000000000000000000002").expect("");
                    storage.insert(key1,val1);

                    applies.push(Apply::Modify {
                        address: to.clone(),
                        basic: Basic{
                            balance: U256::from_dec_str("10000000000000000").expect("error"),
                            nonce : U256::one(),
                        },
                        code: Some(code),
                        storage: storage,
                        reset_storage: false,
                    });

                    backend.apply(applies,Vec::new(),false);
                    let account = Account::new(&backend,from);
                    println!("{}", account);
                    println!("{:#?}", backend);
                }

                let input = hex::decode(data.as_str()).expect("Input is invalid");
                let config = Config::istanbul();
                let executor = StackExecutor::new(
                    &backend,
                    gas_limit as usize,
                    &config,
                );
                let nonce = Some(executor.nonce(from.clone()));

                executer::execute_evm(
                    &mut backend,
                    from.clone(),
                    value,
                    gas_limit,
                    gas_price,
                    nonce,
                    |executor| ((), executor.transact_call(
                        from,
                        to,
                        value,
                        input,
                        gas_limit as usize,
                    )),
                ).expect("Call message failed");

                println!("Contract Called, State OK.");
                println!("{:#?}", backend);
            }
        }
    }
}