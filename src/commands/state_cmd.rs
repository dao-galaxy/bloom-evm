use structopt::StructOpt;
use kvdb_rocksdb::{Database};
use bloom_state as state;
use ethereum_types::{U256,H256};

use std::sync::Arc;


#[derive(Debug, StructOpt, Clone)]
pub struct StateCmd {
    #[structopt(subcommand)]
    cmd: Command
}

#[derive(StructOpt,Debug,Clone)]
enum Command {
    History{},
}

impl StateCmd {
    pub fn run(&self, db: Arc<Database>,count: U256) -> bool{
        match self.cmd {
            Command::History {} => {
                //println!("count={:?}",count);
                let total_count = count.as_u32();
                let mut l = total_count;
                loop {
                    if l == 0 {
                        break;
                    }
                    let i = l as u32;
                    let key = U256::from(i);

                    let mut arr = [0u8;32];
                    key.to_big_endian(&mut arr);
                    let v =  db.get(state::COL_BLOCK,&arr[..]);

                    let root = v.unwrap().unwrap();
                    let root = H256::from_slice(root.as_slice());
                    println!("{:?}",root);
                    l -=  1;
                }
            }
        }
        false
    }
}