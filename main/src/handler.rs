
use common_types::ipc::*;
use common_types::transaction::SignedTransaction;
use rlp;
use evm_executer;
use blockchain_db::BlockChain;
use std::sync::Arc;
use std::hash::Hasher;
use common_types::header::Header;

pub fn handler(data: Vec<u8>,db: Arc<dyn (::kvdb::KeyValueDB)>, blockchain: &BlockChain) -> IpcReply {

    let request: IpcRequest = rlp::decode(data.as_slice()).unwrap();
    match request.method.as_str() {
        "CreateHeader" => {
            let req: CreateHeaderReq = rlp::decode(request.params.as_slice()).unwrap();
            let resp = create_header(req,db);
            return IpcReply {
                id: request.id,
                result: rlp::encode(&resp),
            }
        },
        "LatestBlocks" => {
            let req: LatestBlocksReq = rlp::decode(request.params.as_slice()).unwrap();
            let resp = latest_blocks(req,blockchain);
            return IpcReply {
                id: request.id,
                result: rlp::encode(&resp),
            }
        },
        "ApplyBlock" => {
            let req: ApplyBlockReq = rlp::decode(request.params.as_slice()).unwrap();
            let resp = apply_block(req,db);
            return IpcReply {
                id: request.id,
                result: rlp::encode(&resp),
            }
        },
        "AccountInfo" => {
            let req: AccountInfoReq = rlp::decode(request.params.as_slice()).unwrap();
            let resp = account_info(req,db);
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

fn create_header(req: CreateHeaderReq, db: Arc<dyn (::kvdb::KeyValueDB)>) -> CreateHeaderResp {
    let mut signed_txs:Vec<SignedTransaction> = vec![];
    for tx in req.transactions {
        let t = SignedTransaction::new(tx).unwrap();
        signed_txs.push(t);
    }

    let header = evm_executer::create_header(req.parent_block_hash,
                                             req.author,
                                             req.extra_data,
                                             req.gas_limit,
                                             req.difficulty,
                                             signed_txs,
                                             db.clone()).unwrap();
    CreateHeaderResp(header)
}

fn latest_blocks(req: LatestBlocksReq, blockchain: &BlockChain) -> LatestBlocksResp {
    let n = if req.0 <= 0 { 1 } else { req.0 };
    let block_block_number = blockchain.best_block_number();
    let mut headers: Vec<Header> = vec![];
    for i in 0..n {
        let number = block_block_number - i;
        let block = blockchain.block_by_number(number);
        if(block.is_none()) {
            break;
        }
        headers.push(block.unwrap().header);
    }

    LatestBlocksResp(headers)
}

fn apply_block(req: ApplyBlockReq, db: Arc<dyn (::kvdb::KeyValueDB)>) -> ApplyBlockResp {
    let mut signed_trx: Vec<SignedTransaction> = vec![];
    for tx in req.1 {
        signed_trx.push(SignedTransaction::new(tx).unwrap());
    }
    evm_executer::apply_block(req.0,signed_trx,db);
    ApplyBlockResp(true)
}

fn account_info(req: AccountInfoReq, db: Arc<dyn (::kvdb::KeyValueDB)>) -> AccountInfoResp {
    let (nonce,balance) = evm_executer::account_info(req.0,db);
    AccountInfoResp(nonce,balance)
}