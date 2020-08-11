
mod handler;

use zmq::{Context, DEALER, ROUTER};
use common_types::ipc::IpcReply;
use rlp::Encodable;
use kvdb_rocksdb::{Database,DatabaseConfig};
use std::sync::Arc;
use blockchain_db::BlockChain;

const END_POINT : &'static str = "tcp://127.0.0.1:7050";
const DATA_PATH: &'static str = "evm-data";

fn main() {
    let config = DatabaseConfig::with_columns(bloom_db::NUM_COLUMNS);
    let database = Arc::new(Database::open(&config, DATA_PATH).unwrap());
    let blockchain = BlockChain::new(database.clone());
    run_server(END_POINT,database,blockchain);
}

pub fn run_server(end_point : &str,db: Arc<dyn (::kvdb::KeyValueDB)>, blockchain: BlockChain) {
    let context = Context::new();
    let socket = context.socket(ROUTER).unwrap();
    socket.bind(end_point).unwrap();
    loop {
        let mut received_parts = socket.recv_multipart(0).unwrap();
        let msg_bytes = received_parts.pop().unwrap();
        let zmq_identity = received_parts.pop().unwrap();
        println!(
            "main thread, received from client, #zmq_identity: {:x?}; #msg_bytes: {:x?}",
            zmq_identity,
            msg_bytes
        );

        let result = handler::handler(msg_bytes.clone(),db.clone(), &blockchain);
        let result_data = result.rlp_bytes();

        socket.send_multipart(vec![zmq_identity, result_data.clone()], 0).unwrap();
    }
}

