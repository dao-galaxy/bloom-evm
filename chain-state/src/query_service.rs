use zmq::{Context, DEALER, ROUTER};
use common_types::ipc::*;
use rlp::{Encodable,DecoderError};
use kvdb_rocksdb::{Database,DatabaseConfig};
use std::sync::Arc;
use blockchain_db::BlockChain;
use crate::handler::{latest_blocks, account_info};

pub fn run_query_service(end_point : &str,db: Arc<dyn (::kvdb::KeyValueDB)> ) {
    let context = Context::new();
    let socket = context.socket(ROUTER).unwrap();
    socket.bind(end_point).unwrap();
    loop {
        let mut received_parts = socket.recv_multipart(0).unwrap();
        let msg_bytes = received_parts.pop().unwrap();
        let zmq_identity = received_parts.pop().unwrap();
        println!(
            "Query service received message, #zmq_identity: {:x?}; #msg_bytes: {:x?}",
            zmq_identity,
            msg_bytes
        );

        let result = query_handler(msg_bytes.clone(),db.clone());
        let result_data = result.rlp_bytes();

        socket.send_multipart(vec![zmq_identity, result_data.clone()], 0).unwrap();
    }
}

fn query_handler(data: Vec<u8>,db: Arc<dyn (::kvdb::KeyValueDB)>) -> IpcReply {

    let blockchain = BlockChain::new(db.clone());

    let request: IpcRequest = rlp::decode(data.as_slice()).unwrap();
    match request.method.as_str() {
        "AccountInfo" => {
            let req: Result<AccountInfoReq, DecoderError> = rlp::decode(request.params.as_slice());
            if req.is_err() {
                return IpcReply::default();
            }
            let req = req.unwrap();
            let resp = account_info(req, db, &blockchain);
            return IpcReply {
                id: request.id,
                result: rlp::encode(&resp)
            }
        },
        "LatestBlocks" => {
            let req: Result<LatestBlocksReq, DecoderError> = rlp::decode(request.params.as_slice());
            if req.is_err() {
                return IpcReply::default();
            }
            let req = req.unwrap();
            println!("LatestBlocks,{:?}", req.clone());
            let resp = latest_blocks(req, &blockchain);
            return IpcReply {
                id: request.id,
                result: rlp::encode(&resp),
            }
        },
        _ => {
            return IpcReply::default()
        }
    }
}

