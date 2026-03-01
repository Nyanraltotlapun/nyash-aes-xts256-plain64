use clap::Parser;
use tokio_tungstenite::tungstenite::Utf8Bytes;
use std::str::FromStr;
use std::{path::PathBuf};
use log;

use redb;
use tokio::net::{TcpListener, TcpStream};
use tokio_tungstenite::{accept_async, tungstenite::protocol::Message};
use futures::{StreamExt, SinkExt};
use std::env;
use std::net::SocketAddr;


mod config;


mod num_utils;
mod database;


/// Server for AES-XTS distributed bruteforce
#[derive(clap::Parser, Debug)]
#[command(version, about, long_about = None)]
struct ProgArgs {
    /// Config file name
    #[arg(short, long, default_value = "/etc/nyash_aes_d.conf")]
    config: PathBuf,
}



#[derive(Debug)]
enum ProtocolParseError {
    ParseIntError(std::num::ParseIntError),
    WrongFormat(usize),
}

impl From<std::num::ParseIntError> for ProtocolParseError {
    fn from(err: std::num::ParseIntError) -> Self {
        ProtocolParseError::ParseIntError(err)
    }
}

impl From<usize> for ProtocolParseError {
    fn from(err: usize) -> Self {
        ProtocolParseError::WrongFormat(err)
    }
}

#[derive(Copy, Clone, Debug)]
struct NyashWorkRequest {
    pref_job_size: u64,
}

impl NyashWorkRequest {
    pub const HEADER: &str  = "WORK_REQUEST";

    pub fn to_msg(&self) -> String {
        return format!("{} {}", Self::HEADER, self.pref_job_size);
    }

    pub fn from_msg(mut it: std::str::Split<'_, &str>) -> Result<Self, ProtocolParseError> {
        Ok(Self {
            pref_job_size: it.next().ok_or(0)?.parse()?,
        })
    }
}

#[derive(Copy, Clone, Debug)]
struct NyashWorkDone {
    job_id: u64,
}

impl NyashWorkDone {
    pub const HEADER: &str  = "WORK_DONE";

    pub fn to_msg(&self) -> String {
        return format!("{} {}", Self::HEADER, self.job_id);
    }

    pub fn from_msg(mut it: std::str::Split<'_, &str>) -> Result<Self, ProtocolParseError> {
        Ok(Self {
            job_id: it.next().ok_or(0)?.parse()?,
        })
    }
}

enum NyashReq {
    NyashWorkRequest(NyashWorkRequest),
    NyashWorkDone(NyashWorkDone),
}


#[derive(Copy, Clone, Debug)]
struct NyashWorkResp {
    job_id: u64,
    tweak_key: u128,
    start_key: u128,
    len: u64,
}

impl NyashWorkResp {

    pub const HEADER: &str  = "WORK_RESP";

    pub fn from_job_record(jr: &database::JobRecord) -> Self {
        Self {
            job_id: jr.id,
            tweak_key: jr.tweak_key,
            start_key: jr.start_key,
            len: jr.len,
        }
    }

    pub fn to_msg(&self) -> String {
        return format!("{} {} {} {} {}", Self::HEADER, self.job_id, self.tweak_key, self.start_key, self.len);
    }

    pub fn from_msg(mut it: std::str::Split<'_, &str>) -> Result<Self, ProtocolParseError> {
        Ok(Self {
            job_id: it.next().ok_or(0)?.parse()?,
            tweak_key: it.next().ok_or(0)?.parse()?,
            start_key: it.next().ok_or(0)?.parse()?,
            len: it.next().ok_or(0)?.parse()?,
        })
    }
}

#[derive(Copy, Clone, Debug)]
struct NyashErrorResp {
    code: u32,
}

impl NyashErrorResp {
    pub const HEADER: &str  = "ERROR";

    pub fn to_msg(&self) -> String {
        return format!("{} {}", Self::HEADER, self.code);
    }

    pub fn from_msg(mut it: std::str::Split<'_, &str>) -> Result<Self, ProtocolParseError> {
        Ok(Self {
            code: it.next().ok_or(0)?.parse()?,
        })
    }
}

#[derive(Copy, Clone, Debug)]
struct NyashOkResp {}
impl NyashOkResp {
    pub const HEADER: &str  = "OK";

    pub fn to_msg(&self) -> String {
        return format!("{}", Self::HEADER);
    }
}

#[derive(Copy, Clone, Debug)]
struct NyashNoWorkResp {
    code: u32,
}

impl NyashNoWorkResp {
    pub const HEADER: &str  = "NO_WORK";

    pub fn to_msg(&self) -> String {
        return format!("{} {}", Self::HEADER, self.code);
    }
    pub fn from_msg(mut it: std::str::Split<'_, &str>) -> Result<Self, ProtocolParseError> {
        Ok(Self {
            code: it.next().ok_or(0)?.parse()?,
        })
    }
}




enum NyashResp {
    NyashWorkResp(NyashWorkResp),
    NyashErrorResp(NyashErrorResp),
    NyashNoWorkResp(NyashNoWorkResp),
}

fn parse_message(mut it: std::str::Split<'_, &str>) -> Result<NyashReq, ProtocolParseError> {
    let code: &str = it.next().ok_or(0)?;
    match code {
        NyashWorkRequest::HEADER => Ok(NyashReq::NyashWorkRequest(NyashWorkRequest::from_msg(it)?)),
        NyashWorkDone::HEADER => Ok(NyashReq::NyashWorkDone(NyashWorkDone::from_msg(it)?)),
        _ => Err(ProtocolParseError::WrongFormat(0))
    }
}

async fn handle_connection(stream: TcpStream, db: &redb::Database) {
    // Accept the WebSocket connection
    let ws_stream = match accept_async(stream).await {
        Ok(ws) => ws,
        Err(e) => {
            println!("Error during the websocket handshake: {}", e);
            return;
        }
    };

    // Split the WebSocket stream into a sender and receiver
    let (mut sender, mut receiver) = ws_stream.split();

    // Handle incoming messages
    while let Some(msg) = receiver.next().await {
        match msg {
            Ok(Message::Text(text)) => {
                // Reverse the received string and send it back
                let str_text = text.to_string();
                println!("Got text: {}", str_text);
                //let mesg = parse_message(str_text.split(" "));
                match parse_message(str_text.split(" ")) {
                    Ok(NyashReq::NyashWorkDone(wrk_done)) => {
                        let res = database::db_commit_job(db, wrk_done.job_id);
                        match res {
                            Ok(_) => {
                                if let Err(e) = sender.send(Message::Text(NyashOkResp::HEADER.into())).await {
                                    println!("Error sending message: {}", e);
                                }
                            },
                            Err(ex) => {
                                println!("Error comiting work: {}", ex);
                                let rest_str = NyashErrorResp{code: 0}.to_msg();
                                if let Err(e) = sender.send(Message::Text(rest_str.into())).await {
                                    println!("Error sending message: {}", e);
                                }
                            }
                        }
                    },
                    Ok(NyashReq::NyashWorkRequest(wrk_req)) => {
                        let res = database::db_create_job(db, wrk_req.pref_job_size);
                        match res {
                            Ok(Some(job_record)) => {
                                let rest_str = NyashWorkResp::from_job_record(&job_record).to_msg();
                                if let Err(e) = sender.send(Message::Text(rest_str.into())).await {
                                    println!("Error sending message: {}", e);
                                }   
                            },
                            Ok(None) => {
                                let rest_str = NyashNoWorkResp{code: 0}.to_msg();
                                if let Err(e) = sender.send(Message::Text(rest_str.into())).await {
                                    println!("Error sending message: {}", e);
                                }
                            },
                            Err(ex) => {
                                println!("Error creating job: {}", ex);
                                let rest_str = NyashErrorResp{code: 0}.to_msg();
                                if let Err(e) = sender.send(Message::Text(rest_str.into())).await {
                                    println!("Error sending message: {}", e);
                                }
                            }
                        }
                    },
                    Err(ex) => println!("Error sending message: {:?}", ex)
                }
                
            },
            Ok(Message::Close(_)) => break,
            Ok(_) => (),
            Err(e) => {
                println!("Error processing message: {}", e);
                break;
            }
        }
    }
}


async fn accept_loop(listener: TcpListener, db: redb::Database) {
    while let Ok((stream, _)) = listener.accept().await {
        // Spawn a new task for each connection
        handle_connection(stream, &db).await;
    }

}
// Так. Нам нужно база данныъ с дипазонами всей фигни. И сервить её через tls или что-то типо того.

#[tokio::main]
async fn main() {
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
    let addr: SocketAddr = addr.parse().expect("Invalid address");

    // Create the TCP listener
    let listener = TcpListener::bind(&addr).await.expect("Failed to bind");

    println!("Listening on: {}", addr);
    let res = tokio::spawn(accept_loop(listener, db)).await;
    if res.is_err() {
        print!("Something gone wrong! {:?}", res.unwrap_err());
    }

}

