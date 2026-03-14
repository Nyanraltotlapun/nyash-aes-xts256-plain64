use std::str::FromStr;
use std::{path::PathBuf};


use redb;
use clap::Parser;

use std::sync::{Arc};
use tokio::sync::RwLock as AsyncRwLock;

use tonic::{Request, Response, Status, transport::Server};


pub mod nyash_proto {
    tonic::include_proto!("nyash_proto"); // The string specified here must match the proto package name
}

use nyash_proto::nyash_luks_server::{NyashLuks, NyashLuksServer};
use nyash_proto::{
    CommitReply, ProgressReply, ProgressRequest, WorkCommit, WorkData, WorkReply, WorkRequest, work_commit,
    work_reply,
};

mod database;
mod num_utils;
mod config;

async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("Failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("Failed to install SIGTERM handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => println!("Received Ctrl+C, initiating shutdown"),
        _ = terminate => println!("Received SIGTERM, initiating shutdown"),
    }
}


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


// Background task to update progress from DB
async fn update_service_progress(progress: Arc<AsyncRwLock<f64>>, db: Arc<redb::Database>, term: Arc<AsyncRwLock<bool>>) {
    while *term.read().await == false {
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
        let rem_addr = match request.remote_addr() {
            Some(v) => v.to_string(),
            None => "None".to_string()
        };
        println!("Remoute: {}, request_work.", rem_addr);


        if *self.key_found.read().await == true {
            return Ok(Response::new(WorkReply { result: Some(work_reply::Result::NoWork(true)) }));
        }

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
        let rem_addr = match request.remote_addr() {
            Some(v) => v.to_string(),
            None => "None".to_string()
        };
        println!("Remoute: {}, commit_work.", rem_addr);

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

    async fn request_progress(&self, request: Request<ProgressRequest>) -> Result<Response<ProgressReply>, Status> {
        let rem_addr = match request.remote_addr() {
            Some(v) => v.to_string(),
            None => "None".to_string()
        };
        println!("Remoute: {}, request_progress.", rem_addr);
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



#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error>> {

    let args = ProgArgs::parse();
    log::error!("Arguments parsed: {:?}", args.config);
    println!("Path: {:?}", args.config);
    let server_config: config::ServerConfig = config::read_config(&args.config).unwrap();

    let db_dir = server_config.db_dir.unwrap();
    let db_file = format!("{}data.rdb",db_dir);
    let db_path = PathBuf::from_str(db_file.as_str()).unwrap();
    
    let mut db = database::db_open(&db_path).expect("Error Opening database");
    println!("Compact db");
    db.compact()?;

    // Get the address to bind to
    let addr = format!("{}:{}",server_config.bind_addr, server_config.listen_port);
    let addr: std::net::SocketAddr = addr.parse().expect("Invalid address");

    println!("Listening on: {}", addr);


    let found_keys = database::db_get_found_keys(&db).expect("Error getting found keys from DB");
    if found_keys.len() >0 {
        println!("Key already found!");
        for key in found_keys {
            println!("Key found! Timestump: {}, Tweak key: {}, Enc key: {}",key.timestump, key.tweak_key, key.enc_key);
        }
        println!("Exit!");
        return Ok(());
    }


    let progress = database::db_get_progress(&db).expect("Error getting progress from DB");
    
    let shared_progress = Arc::new(AsyncRwLock::new(progress));
    let shared_db = Arc::new(db);
    let shared_key_found = Arc::new(AsyncRwLock::new(false));
    let shared_termination: Arc<AsyncRwLock<bool>> = Arc::new(AsyncRwLock::new(false));
    // let service_state = Arc::new(AsyncRwLock::new(ServiceState {
    //     db: Arc::new(db),
    //     key_found: false,
    //     progress: progress,
    // }));

    let term_clone = shared_termination.clone();
    let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel();
    tokio::spawn(async move {
        shutdown_signal().await;
        //signaling that we should stop
        let mut guard = term_clone.write().await;
        *guard = true;
        let _ = shutdown_tx.send(());
    });

    // Spawn the periodic updater
    tokio::spawn(update_service_progress(shared_progress.clone(), shared_db.clone(), shared_termination.clone()));
    
    let nyash_service = NyashService{db:shared_db, progress:shared_progress, key_found:shared_key_found};


    Server::builder()
        .add_service(NyashLuksServer::new(nyash_service))
        .serve_with_shutdown(addr, async { shutdown_rx.await.ok(); })
        .await?;
    
    Ok(())
}
