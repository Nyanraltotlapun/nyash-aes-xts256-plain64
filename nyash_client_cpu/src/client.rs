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
    CommitReply, KeyData, ProgressReply, ProgressRequest, WorkCommit, WorkReply, WorkRequest,
    work_commit, work_reply,
};

use std::sync::Arc;
use threadpool::ThreadPool;
use tokio::sync::RwLock as AsyncRwLock;
mod aes_cpu;
pub mod num_utils;
mod search_params;
//mod test_cl;

const S_ADDR: &str = "http://93.113.25.180:37939";


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

// fn key_dat_from_exec_dat(ex_dat: &ExecData) -> KeyData {
//     let t_k = num_utils::u128_to_u64arr(num_utils::u32arr_to_u128(num_utils::vec_to_u32_4arr(
//         &ex_dat.tweak_key,
//         0,
//     )));
//     let e_k = num_utils::u128_to_u64arr(ex_dat.get_found_key());

//     KeyData {
//         start_key0: e_k[0],
//         start_key1: e_k[1],
//         tweak_key0: t_k[0],
//         tweak_key1: t_k[1],
//     }
// }

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

fn benchmark() -> (u64, usize) {
    let total_work: u64 = 1280000000;
    let thread_num_list: [usize; 9] = [4, 6, 8, 10, 16, 20, 32, 64, 128];
    let mut work_time = [0f64; 9];

    let mut preffered_th_num: usize = thread_num_list[0];
    let mut preffered_batch_size: u64 = 0;

    let start_key = [1u8; 16];
    let tweak_key = [2u8; 16];
    let data_block = [3u8; 16];
    let search_block = [4u8; 16];

    for i in 0..9 {
        let test_th_num = thread_num_list[i];
        let pool = ThreadPool::new(test_th_num);
        let batch_size: u64 = total_work / test_th_num as u64;

        println!("Benchmarking thread num: {}", test_th_num);
        for _j in 0..3 {
            println!("Run number {}", _j);
            let start_time = std::time::Instant::now();
            let res = aes_cpu::do_work(
                &pool,
                batch_size,
                &start_key,
                &tweak_key,
                &data_block,
                &search_block,
            );
            if res.is_some() {
                println!("Key found!")
            }
            let exec_duration = start_time.elapsed().as_secs_f64();
            work_time[i] += exec_duration;
        }
        work_time[i] = work_time[i] / 3.0;
        println!("Average time {}", work_time[i]);
        if i > 0 {
            //giving 2% error for speed mesure
            if (work_time[i] * 1.02) > work_time[i - 1] {
                break;
            }
        }
        preffered_th_num = test_th_num;
        // calculate batch size so it correspond to 30 sec job
        preffered_batch_size = (batch_size as f64 * (10.0 / work_time[i])) as u64;
        println!(
            "batch_size {}, work_time {}, preffered_batch_size {}, preffered_th_num {}",
            batch_size, work_time[i], preffered_batch_size, preffered_th_num
        );
    }

    return (preffered_batch_size, preffered_th_num);
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Performing banchmark to determine optimal parameters...");
    let (pref_batch_size, th_num) = benchmark();
    println!("batch_size {}, th_num {}", pref_batch_size, th_num);

    let encrypted_data = search_params::get_encrypted_data();
    let unencrypt_data = [0u8; 16];
    let th_pool = ThreadPool::new(th_num);

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
    let req_work_size: u64 = pref_batch_size * th_num as u64;

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
            println!("Error getting work, waiting 15 seconds and trying again...");
            tokio::time::sleep(Duration::from_secs(15)).await;
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
        let start_key = num_utils::u64arr_to_u8arr([work_data.start_key0, work_data.start_key1]);
        let tweak_key = num_utils::u64arr_to_u8arr([work_data.tweak_key0, work_data.tweak_key1]);

        let mut batch_size = work_data.work_size / th_num as u64;
        if (work_data.work_size % th_num as u64) != 0 {
            batch_size += 1;
        }
        println!("Setting batch size to {}", batch_size);

        println!("Crunching numbers...");

        let start_time = std::time::Instant::now();
        let work_res = aes_cpu::do_work(
            &th_pool,
            batch_size,
            &start_key,
            &tweak_key,
            &unencrypt_data,
            &encrypted_data,
        );
        let exec_duration = start_time.elapsed().as_secs_f64();

        let g_k_p_s = (work_data.work_size as f64 / exec_duration) / 1000000000.0;
        if giga_keys_per_second != 0f64 {
            giga_keys_per_second = giga_keys_per_second * 0.9 + g_k_p_s * 0.1;
        } else {
            giga_keys_per_second = g_k_p_s;
        }
        println!("Average speed: {:.3}GigaKeys/Sec", giga_keys_per_second);

        let work_c = match work_res {
            Some(found_key) => {
                println!("We found the key! {:?} {:?}", found_key, tweak_key);

                //signaling that key found
                let mut guard = shared_key_found.write().await;
                *guard = true;

                let found_key_u64 = num_utils::u8arr_to_u64arr(found_key.into());
                WorkCommit {
                    work_id: work_data.work_id,
                    result: Some(work_commit::Result::FoundKey(KeyData {
                        tweak_key0: work_data.tweak_key0,
                        tweak_key1: work_data.tweak_key1,
                        start_key0: found_key_u64[0],
                        start_key1: found_key_u64[0],
                    })),
                }
            }
            None => WorkCommit {
                work_id: work_data.work_id,
                result: Some(work_commit::Result::NoKey(true)),
            },
        };
        let resp = commit_work(nya_channel.clone(), work_c).await;
        match resp {
            Ok(c_r) => println!("Work commited. Progress: {:.8}%", c_r.progress * 100.0),
            Err(_) => println!("Error commiting work..."),
        };
    }

    println!("Exiting!");

    Ok(())
}
