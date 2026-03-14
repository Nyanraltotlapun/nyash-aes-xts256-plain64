use std::time::Duration;

use tokio;
use tonic;
use tonic::transport::Channel;
//use tonic::{Request, Response, Status};
pub mod nyash_proto {
    tonic::include_proto!("nyash_proto"); // The string specified here must match the proto package name
}

use nyash_proto::nyash_luks_client::NyashLuksClient;
use nyash_proto::{
    CommitReply, KeyData, ProgressReply, ProgressRequest, WorkCommit, WorkReply,
    WorkRequest, work_commit, work_reply,
};

use std::sync::{Arc};
use tokio::sync::RwLock as AsyncRwLock;

use crate::ocl_utils::ExecData;

pub mod client_config;
pub mod num_utils;
pub mod ocl_utils;
mod search_params;
//mod test_cl;

const S_ADDR: &str = "http://127.0.0.1:37939";
// const SRC_PATH: &str = "src/open_cl/nyash_aes_xts256_plain.cl";
// const OCL_COMP_OPT: &str = "-I src/open_cl";
const CONF_RILE_NAME: &str = "nyash_conf.json";


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

fn key_dat_from_exec_dat(ex_dat: &ExecData) -> KeyData {
    let t_k = num_utils::u128_to_u64arr(num_utils::u32arr_to_u128(num_utils::vec_to_u32_4arr(
        &ex_dat.tweak_key,
        0,
    )));
    let e_k = num_utils::u128_to_u64arr(ex_dat.get_found_key());

    KeyData {
        start_key0: e_k[0],
        start_key1: e_k[1],
        tweak_key0: t_k[0],
        tweak_key1: t_k[1],
    }
}

async fn get_progress(chanel: Channel) -> Result<ProgressReply, tonic::Status> {
    let mut client = NyashLuksClient::new(chanel);

    let request = tonic::Request::new(ProgressRequest {});
    let response = client.request_progress(request).await?;
    return Ok(response.into_inner());
}

async fn get_work(chanel: Channel, work_size: u64) -> Result<WorkReply, tonic::Status> {
    let mut client = NyashLuksClient::new(chanel);

    println!("Requesting work {} keys...", work_size);
    let request = tonic::Request::new(WorkRequest {
        pref_work_size: work_size,
    });
    let response = client.request_work(request).await?;
    return Ok(response.into_inner());
}

async fn commit_work(
    chanel: Channel,
    commit_data: WorkCommit,
) -> Result<CommitReply, tonic::Status> {
    let mut client = NyashLuksClient::new(chanel);

    let request = tonic::Request::new(commit_data);
    let response = client.commit_work(request).await?;
    return Ok(response.into_inner());
}

fn benchmark(exec_context: &mut ocl_utils::ExecContext) -> (u64, usize) {
    let mut nyan_exec_dat = ocl_utils::ExecData {
        start_key: vec![1u32; 4],
        tweak_key: vec![2u32; 4],
        uenc_data: vec![0u32; 4],
        target_data: vec![0u32; 4],
        tweak_i: 0,
        tweak_j: 0,
        key_found: vec![0u32; 5],
        batch_size: 10000000,
        work_size: 128,
    };

    let total_work: u64 = 1280000000;
    let work_sizes: [usize; 8] = [128, 256, 512, 1024, 2048, 4096, 8192, 16384];
    let mut work_time = [0f64; 8];

    ocl_utils::set_target_data(exec_context, &mut nyan_exec_dat).expect("Error set target data!");

    let mut preffered_work_size: usize = work_sizes[0];
    let mut preffered_batch_size: u64 = 0;
    for i in 0..8 {
        let test_work_s = work_sizes[i];
        let batch_size: u64 = total_work / test_work_s as u64;
        nyan_exec_dat.work_size = test_work_s;
        nyan_exec_dat.batch_size = batch_size;
        println!("Benchmarking work size {}", test_work_s);
        for _j in 0..3 {
            let (_, exec_time) =
                ocl_utils::do_work(exec_context, &mut nyan_exec_dat).expect("Error running tests!");
            work_time[i] += exec_time;
        }
        work_time[i] = work_time[i] / 3.0;
        println!("Average time {}", work_time[i]);
        if i > 0 {
            //giving 5% error for speed mesure
            if (work_time[i]*1.05) > work_time[i - 1] {
                break;
            }
        }
        preffered_work_size = work_sizes[i];
        // calculate batch size so it correspond to 30 sec job
        preffered_batch_size = (batch_size as f64 * (20.0 / work_time[i])) as u64;
        println!("batch_size {}, work_time {}, preffered_batch_size {}, preffered_work_size {}",
        batch_size,
        work_time[i],
        preffered_batch_size,
        preffered_work_size);
    }

    return (preffered_batch_size, preffered_work_size);
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    const OCL_NYAS_GZ_SRC: &[u8] =
        include_bytes!(concat!(env!("OUT_DIR"), "/nyash_aes_full.cl.gz"));

    println!("OCL bin src lenght {}", OCL_NYAS_GZ_SRC.len());
    println!("Hello, world nya!");
    //use ocl::{Buffer, Context, Device, Kernel, Platform, Program, Queue, flags};
    let (devices, mut app_conf) =
        client_config::get_devices_conf(CONF_RILE_NAME).expect("Error loading config!");

    let nyash_dev = devices[0].0;
    let nyash_plt = devices[0].1;
//
    println!(
        "Platform: {:?}, Device: {:?}",
        nyash_plt.name().unwrap(),
        nyash_dev.name().unwrap()
    );

    // reading ocl program sources
    //let prog_src = std::fs::read_to_string(SRC_PATH).expect("Error reading program sources!");

    let mut exec_context: ocl_utils::ExecContext = ocl_utils::ExecContext::new(
        nyash_dev,
        nyash_plt,
        OCL_NYAS_GZ_SRC,
        1024
    )
    .expect("Error creating exec context!");

    // need to train in order to learn optimal params
   
    // need to train in order to learn optimal params
    if (app_conf.devices[0].batch_size == 0) || (app_conf.devices[0].work_size == 0) {
        println!("Performing banchmark to determine optimal GPU parameters...");
        let (batch_size, work_size) = benchmark(&mut exec_context);
        println!("batch_size {}, work_size {}", batch_size, work_size);
        app_conf.devices[0].batch_size = batch_size;
        app_conf.devices[0].work_size = work_size;
        client_config::save_config(CONF_RILE_NAME, &app_conf).expect("Error saving config!");
    }

    let nyash_dev_cfg = &app_conf.devices[0];
    println!(
        "Preffered Work size: {}, Batch size {}",
        nyash_dev_cfg.work_size, nyash_dev_cfg.batch_size
    );

    let (_, _, encrypted_data) = search_params::get_params();
    //setting data
    let mut nyan_exec_dat = ocl_utils::ExecData {
        start_key: Vec::new(),
        tweak_key: Vec::new(),
        uenc_data: vec![0u32; 4],
        target_data: encrypted_data.to_vec(),
        tweak_i: 0,
        tweak_j: 0,
        key_found: vec![0u32; 5],
        batch_size: nyash_dev_cfg.batch_size,
        work_size: nyash_dev_cfg.work_size,
    };
     println!(
        "nyan_exec_dat Work size: {}, Batch size {}",
        nyan_exec_dat.work_size, nyan_exec_dat.batch_size
    );


    ocl_utils::set_target_data(&mut exec_context, &mut nyan_exec_dat)
        .expect("Error setting target data!");

    // Don't keep connection alive when idle
    let nya_channel: tonic::transport::Channel = tonic::transport::Endpoint::from_static(S_ADDR)
        .keep_alive_while_idle(false)
        .keep_alive_timeout(Duration::from_secs(10))
        .connect_timeout(Duration::from_secs(10))
        .connect()
        .await
        .expect("Error connecting to server!");

    let key_found = match get_progress(nya_channel.clone()).await {
        Err(_) => {
            println!("Error getting progress!");
            false
        }
        Ok(p_r) => {
            println!("Current progress {:.8}%", p_r.progress * 100.0);
            p_r.key_found
        }
    };

    let mut giga_keys_per_second: f64 = 0f64;
    let req_work_size: u64 = nyash_dev_cfg.batch_size as u64 * nyash_dev_cfg.work_size as u64;


    let shared_key_found = Arc::new(AsyncRwLock::new(key_found));
    // handling program termination
    let sh_k_f_clone = shared_key_found.clone();
    tokio::spawn(async move {
        shutdown_signal().await;
        //signaling that we should stop
        let mut guard = sh_k_f_clone.write().await;
        *guard = true;
    });

    while *shared_key_found.read().await == false {
        let mut work = get_work(nya_channel.clone(), req_work_size).await;
        while work.is_err() {
            println!("Error getting work, waiting 10 seconds and trying again...");
            tokio::time::sleep(Duration::from_secs(10)).await;
            work = get_work(nya_channel.clone(), req_work_size).await;
        }
        let work = work.expect("Error getting work!");

        let work_data = match work.result.expect("Error! Expected WorkResult!") {
            work_reply::Result::NoWork(_) => {
                println!("No work right now, try again later...");
                continue;
            }
            work_reply::Result::Error(ex) => {
                println!("Erro getting work: {}", ex);
                continue;
            }
            work_reply::Result::WorkData(wd) => wd,
        };
        println!("Got work, {} keys...", work_data.work_size);
        nyan_exec_dat.start_key = num_utils::u128_to_u32arr(num_utils::u64arr_to_u128([
            work_data.start_key0,
            work_data.start_key1,
        ]))
        .to_vec();
        nyan_exec_dat.tweak_key = num_utils::u128_to_u32arr(num_utils::u64arr_to_u128([
            work_data.tweak_key0,
            work_data.tweak_key1,
        ]))
        .to_vec();

        let mut batch_size = work_data.work_size / nyan_exec_dat.work_size as u64;
        if (work_data.work_size % nyan_exec_dat.work_size as u64) != 0 {
            batch_size += 1;
        }
        println!("Setting batch size to {}", batch_size);
        nyan_exec_dat.batch_size = batch_size;

        println!("Crunching numbers...");
        match ocl_utils::do_work(&mut exec_context, &mut nyan_exec_dat) {
            Err(_) => println!("Error doing work!"),
            Ok((k_f, work_time)) => {
                let mut w_k = WorkCommit {
                    work_id: work_data.work_id,
                    result: Some(work_commit::Result::NoKey(true)),
                };

                let g_k_p_s = (work_data.work_size as f64 / work_time) / 1000000000.0;
                if giga_keys_per_second != 0f64 {
                    giga_keys_per_second -= giga_keys_per_second / 10.0;
                    giga_keys_per_second += g_k_p_s / 10.0;
                } else {
                    giga_keys_per_second = g_k_p_s;
                }
                println!("Average speed: {:.3}GigaKeys/Sec", giga_keys_per_second);
                if k_f == true {
                    println!(
                        "We found the key! {:?} {:?}",
                        nyan_exec_dat.key_found, nyan_exec_dat.tweak_key
                    );

                    //signaling that key found
                    let mut guard = shared_key_found.write().await;
                    *guard = true;

                    w_k.result = Some(work_commit::Result::FoundKey(key_dat_from_exec_dat(
                        &nyan_exec_dat,
                    )));
                }

                let resp = commit_work(nya_channel.clone(), w_k).await;
                match resp {
                    Ok(c_r) => println!("Work commited. Progress: {:.8}%", c_r.progress * 100.0),
                    Err(_) => println!("Error commiting work..."),
                };
            }
        };
    }

    println!("Exiting!");

    Ok(())
}
