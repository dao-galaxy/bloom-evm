use zmq::{Context, DEALER, ROUTER};
use common_types::ipc::{IpcReply,IpcRequest,AccountInfoReq,AccountInfoResp};
use rlp::{Encodable,DecoderError};
use kvdb_rocksdb::{Database,DatabaseConfig};
use std::sync::Arc;
use blockchain_db::BlockChain;

pub fn run_query_service(end_point : &str,db: Arc<dyn (::kvdb::KeyValueDB)> ) {
    let context = Context::new();
    let socket = context.socket(ROUTER).unwrap();
    socket.bind(end_point).unwrap();
    loop {
        let mut received_parts = socket.recv_multipart(0).unwrap();
        let msg_bytes = received_parts.pop().unwrap();
        let zmq_identity = received_parts.pop().unwrap();
        println!(
            "chain-state thread, received from client, #zmq_identity: {:x?}; #msg_bytes: {:x?}",
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
            let req: Result<AccountInfoReq,DecoderError> = rlp::decode(request.params.as_slice());
            if req.is_err() {
                return IpcReply::default();
            }
            let req = req.unwrap();
            let resp = account_info(req,db,&blockchain);
            return IpcReply {
                id: request.id,
                result: rlp::encode(&resp)
            }
        },
        _ => {
            return IpcReply::default()
        }
    }
}

fn account_info(req: AccountInfoReq, db: Arc<dyn (::kvdb::KeyValueDB)>,bc: & BlockChain ) -> AccountInfoResp {
    let best_header = bc.best_block_header();
    let state_root = best_header.state_root();
    let (nonce,balance) = evm_executer::account_info(req.0,db,state_root);
    AccountInfoResp(nonce,balance)
}