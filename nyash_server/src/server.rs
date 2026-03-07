use std::ops::DerefMut;
use std::str::FromStr;
use std::sync::atomic::AtomicBool;
use std::{path::PathBuf};


use redb;
use clap::Parser;

use std::sync::{Arc, Mutex};
use tokio::sync::RwLock as AsyncRwLock;
use tokio::time::{interval, Duration};

use tonic::{Request, Response, Status, transport::Server};


pub mod nyash_proto {
    tonic::include_proto!("nyash_proto"); // The string specified here must match the proto package name
}

use nyash_proto::nyash_luks_server::{NyashLuks, NyashLuksServer};
use nyash_proto::{
    CommitReply, KeyData, ProgressReply, ProgressRequest, WorkCommit, WorkData, WorkReply, WorkRequest, work_commit,
    work_reply,
};

mod database;
mod num_utils;
mod config;

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

// Shared state
// #[derive(Clone, Debug)]
// pub struct ServiceState {
//     pub db: Arc<redb::Database>,
//     pub key_found: bool,
//     pub progress: f64,
// }

// Background task to update state
// async fn update_service_state(state: Arc<AsyncRwLock<ServiceState>>) {
//     loop {
//         tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
//         let mut guard = state.write().await;
//         match database::db_get_progress(&guard.db) {
//             Ok(p) => {
//                 guard.progress = p;
//             },
//             Err(_) => println!("Error getting progress from DB")
//         }
//     }
// }

// Background task to update progress from DB
async fn update_service_progress(progress: Arc<AsyncRwLock<f64>>, db: Arc<redb::Database>) {
    loop {
        tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
        match database::db_get_progress(&db) {
            Ok(p) => {
                let mut guard = progress.write().await;
                *guard = p;
            },
            Err(_) => println!("Error getting progress from DB")
        }
    }
}

#[derive(Debug)]
pub struct NyashService {
    //state: Arc<AsyncRwLock<ServiceState>>,
    db: Arc<redb::Database>,
    key_found: Arc<AsyncRwLock<bool>>,
    progress: Arc<AsyncRwLock<f64>>,
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
            }
            Ok(None) => {
                let reply = WorkReply {
                    result: Some(work_reply::Result::NoWork(true)),
                };
                Ok(Response::new(reply))
            }
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
        let work_commit = request.into_inner();

        let put_k_no_err: bool = match work_commit.result {
            Some(work_commit::Result::FoundKey(key_data)) => {
                {
                    let mut guard = self.key_found.write().await;
                    *guard = true;
                }
                let tw_k = num_utils::u64arr_to_u128((key_data.tweak_key0, key_data.tweak_key1));
                let en_k = num_utils::u64arr_to_u128((key_data.start_key0, key_data.start_key1));
                match database::db_put_found_key(&self.db, tw_k, en_k) {
                    Ok(_) => true,
                    Err(_) => false,
                }
            }
            _ => true,
        };

        let comm_no_err: bool = match database::db_commit_job(&self.db, work_commit.work_id) {
            Ok(true) => true,
            Ok(false) => false,
            Err(_) => false,
        };

        if (put_k_no_err && comm_no_err) == true {
            Ok(Response::new(CommitReply {
                status_code: 0,
                progress: *self.progress.read().await
            }))
        } else {
            Err(Status::internal("DB error"))
        }
    }

    async fn request_progress(&self, _request: Request<ProgressRequest>) -> Result<Response<ProgressReply>, Status> {
        Ok(Response::new(ProgressReply {
            key_found: *self.key_found.read().await,
            progress: *self.progress.read().await,
        }))
    }
}


/// Server for AES-XTS distributed bruteforce
#[derive(clap::Parser, Debug)]
#[command(version, about, long_about = None)]
struct ProgArgs {
    /// Config file name
    #[arg(short, long, default_value = "/etc/nyash_aes_d.conf")]
    config: PathBuf,
}



#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {

    let args = ProgArgs::parse();
    log::error!("Arguments parsed: {:?}", args.config);
    println!("Path: {:?}", args.config);
    let server_config: config::ServerConfig = config::read_config(&args.config).unwrap();

    let db_dir = server_config.db_dir.unwrap();
    let db_file = format!("{}data.rdb",db_dir);
    let db_path = PathBuf::from_str(db_file.as_str()).unwrap();
    
    let db = database::db_open(&db_path).expect("Error Opening database");

    // Get the address to bind to
    let addr = format!("{}:{}",server_config.bind_addr, server_config.listen_port);
    let addr: std::net::SocketAddr = addr.parse().expect("Invalid address");

    println!("Listening on: {}", addr);


    let found_keys = database::db_get_found_keys(&db).expect("Error getting found keys from DB");
    if found_keys.len() >0 {
        println!("Key already found!");
        for key in found_keys {
            println!("Key found: {:?}", key);
        }
        println!("Exit!");
        return Ok(());
    }


    let progress = database::db_get_progress(&db).expect("Error getting progress from DB");
    
    let shared_progress = Arc::new(AsyncRwLock::new(progress));
    let shared_db = Arc::new(db);
    let shared_key_found = Arc::new(AsyncRwLock::new(false));

    // let service_state = Arc::new(AsyncRwLock::new(ServiceState {
    //     db: Arc::new(db),
    //     key_found: false,
    //     progress: progress,
    // }));

    // Spawn the periodic updater
    tokio::spawn(update_service_progress(shared_progress.clone(), shared_db.clone()));
    
    let nyash_service = NyashService{db:shared_db, progress:shared_progress, key_found:shared_key_found};


    Server::builder()
        .add_service(NyashLuksServer::new(nyash_service))
        .serve(addr)
        .await?;
    Ok(())
}
