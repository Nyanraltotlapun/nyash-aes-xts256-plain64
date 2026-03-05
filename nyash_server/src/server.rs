
use redb;

use tonic::{transport::Server, Request, Response, Status};

use nyash_server::nyash_luks_server::{NyashLuks, NyashLuksServer};
use nyash_server::{WorkRequest, WorkData, WorkReply, work_reply, WorkCommit, CommitReply, ProgressRequest, ProgressReply};

pub mod nyash_server {
    tonic::include_proto!("nyash_server"); // The string specified here must match the proto package name
}

mod database;
mod num_utils;

fn work_rec_to_work_data(work_rec: database::JobRecord) -> WorkData {
    let tw_key = num_utils::u128_to_u64arr(work_rec.tweak_key);
    let st_key = num_utils::u128_to_u64arr(work_rec.start_key);
    WorkData {
        work_id: work_rec.id,
        work_size: work_rec.len,
        // tweak key is 128 bit. We send it as two uint64 values
        tweak_key0: tw_key[0],
        tweak_key1: tw_key[1],
        // start key is 128 bit. We send it as two uint64 values
        start_key0: st_key[0],
        start_key1: st_key[1],
    }
}

#[derive(Debug, Default)]
pub struct NyashService {
    db: redb::Database,
    progress: f64,
}

#[tonic::async_trait]
impl NyashLuks for NyashService {
    async fn request_work(&self, request: Request<WorkRequest>) -> Result<Response<WorkReply>, Status> { 
        println!("Got a request: {:?}", request);
        let pref_work_size: u64 = request.into_inner().pref_work_size;
        match database::db_create_job(&self.db, pref_work_size) {
            Ok(Some(work_rec)) => {
                let reply = WorkReply {
                    result: Some(work_reply::Result::WorkData(work_rec_to_work_data(work_rec))),
                };
                Ok(Response::new(reply)) // Send back our formatted greeting
            },
            Ok(None) => {
                let reply = WorkReply {
                    result: Some(work_reply::Result::NoWork(true)),
                };
                Ok(Response::new(reply))
            },
            Err(_ex) => {
                let reply = WorkReply {
                    result: Some(work_reply::Result::Error("DB Error!".to_string())),
                };
                Ok(Response::new(reply))
            }
        }        
    }

    async fn commit_work(&self, request: Request<WorkCommit>) -> Result<Response<CommitReply>, Status> {
        println!("Got a request: {:?}", request);
        let work_id: u64 = request.into_inner().work_id;
        match database::db_commit_job(&self.db, work_id) {
            Ok(true) => Ok(Response::new(CommitReply {status_code: 0, progress: self.progress})),
            Ok(false) => Ok(Response::new(CommitReply {status_code: 1, progress: self.progress})),
            Err(_) => Err(Status::internal("DB error"))
        }
    }

    async fn request_progress(&self, _request: Request<ProgressRequest>) -> Result<Response<ProgressReply>, Status> {
        Ok(Response::new(ProgressReply {progress: self.progress}))
    }
}


#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let addr = "[::1]:50051".parse()?;
    let greeter = MyGreeter::default();

    Server::builder()
        .add_service(GreeterServer::new(greeter))
        .serve(addr)
        .await?;

    Ok(())
}