extern crate ocl;
use ocl::{
    Buffer, Context, Device, DeviceType, Kernel, Platform, Program, Queue, SpatialDims, flags,
};
use serde::de::value::Error;
use std::{
    io,
    str::{self, FromStr},
};

use crate::client_config::{AppConfig, DevConf};

mod client_config;
mod num_utils;
/// Exploded version. Boom!
///
/// The functions above use `ProQue` and other abstractions to greatly reduce
/// the amount of boilerplate and configuration necessary to do basic work.
/// Many tasks, however, will require more configuration and will necessitate
/// doing away with `ProQue` altogether. Enqueuing kernels and reading/writing
/// from buffers and images usually requires a more explicit interface.
///
/// The following function performs the exact same steps that the above
/// functions did, with many of the convenience abstractions peeled away.
///
/// See the function below this to take things a step deeper...
///
// trait FromStr {
//     fn from_str(&self);
// }
// // Define a trait with a constructor method
// trait NewFile {
//     fn new<P: AsRef<Path>>(path: P) -> std::io::Result<Self> where Self: Sized;
// }

// impl Foo for ocl::flags::DeviceType {
//     fn foo(&self) {
//         println!("foo");
//     }
// }

fn dev_type_from_str(s: &str) -> Result<flags::DeviceType, ()> {
    match s {
        "CPU" => Ok(flags::DeviceType::CPU),
        "GPU" => Ok(flags::DeviceType::GPU),
        "ALL" => Ok(flags::DeviceType::ALL),
        "CUSTOM" => Ok(flags::DeviceType::CUSTOM),
        "ACCELERATOR" => Ok(flags::DeviceType::ACCELERATOR),
        "DEFAULT" => Ok(flags::DeviceType::DEFAULT),
        _ => Err(()),
    }
}

fn str_or_empty(r: ocl::error::Result<String>) -> String {
    match r {
        Ok(s) => s,
        Err(_) => "".to_string(),
    }
}

fn print_devices(dev_list: &Vec<(Device, Platform)>) {
    let mut i = 0;
    for (dev, plt) in dev_list.iter() {
        let dev_name = str_or_empty(dev.name());
        let plt_name = str_or_empty(plt.name());
        println!("({i}) device: \"{dev_name}\" ----- platorm: \"{plt_name}\"");
        i += 1;
    }
}

fn choose_devices(devices_num: usize) -> Result<Vec<usize>, String> {
    println!("Please input desired devices to use as a white space separated list of numbers.");
    let mut result: Vec<usize> = Vec::new();

    let mut s_devs_nums = String::new();

    io::stdin()
        .read_line(&mut s_devs_nums)
        .expect("Failed to read line");

    for s_dev_num in s_devs_nums.split(' ') {
        let dev_num: usize = match s_dev_num.trim().parse() {
            Ok(num) => num,
            Err(_) => return Err("You must input a number from device list.".to_string()),
        };
        if dev_num >= devices_num {
            return Err("You must input a number from device list.".to_string());
        };
        result.push(dev_num);
    }
    return Ok(result);
}

fn list_devices(dev_type: DeviceType) -> Vec<(Device, Platform)> {
    let platforms = Platform::list();
    let mut devices: Vec<(Device, Platform)> = Vec::new();
    for plt in platforms.iter() {
        //let plat_name = str_or_empty(plt.name());
        let list_res = Device::list(plt, Some(dev_type));
        match list_res {
            Ok(dev_l) => devices.extend(dev_l.iter().map(|dev| (*dev, plt.clone()))),
            Err(_) => {}
        }
    }
    return devices;
}

// fn init_devices(
//     devices: Vec<(Device, Platform, DevConfig)>,
//     kern_name: String,
//     prog_src: String,
//     inc_dirs: Vec<String>,
// ) -> Vec<ExecContext> {
//     let mut contexts: Vec<ExecContext> = Vec::with_capacity(devices.len());
//     for (dev, plt, dev_cfg) in devices {
//         let ctx = match Context::builder()
//             .platform(plt)
//             .devices(dev.clone())
//             .build()
//         {
//             Ok(c) => c,
//             Err(_) => continue,
//         };

//         let prg = match Program::builder().devices(dev).src(&prog_src).build(&ctx) {
//             Ok(p) => p,
//             Err(_) => continue,
//         };

//         let queue = match Queue::new(&ctx, dev, None) {
//             Ok(q) => q,
//             Err(_) => continue,
//         };

//         // Create Buffers:
//         let start_key_b = match Buffer::<u32>::builder()
//             .queue(queue.clone())
//             .flags(flags::MEM_READ_ONLY)
//             .len(8)
//             .fill_val(0u32)
//             .build()
//         {
//             Ok(buf) => buf,
//             Err(_) => continue,
//         };

//         let u_data_b = match Buffer::<u32>::builder()
//             .queue(queue.clone())
//             .flags(flags::MEM_READ_ONLY)
//             .len(4)
//             .fill_val(0u32)
//             .build()
//         {
//             Ok(buf) => buf,
//             Err(_) => continue,
//         };

//         let enc_data_b = match Buffer::<u32>::builder()
//             .queue(queue.clone())
//             .flags(flags::MEM_READ_ONLY)
//             .len(4)
//             .fill_val(0u32)
//             .build()
//         {
//             Ok(buf) => buf,
//             Err(_) => continue,
//         };

//         let key_found_b = match Buffer::<u32>::builder()
//             .queue(queue.clone())
//             .flags(flags::MEM_WRITE_ONLY)
//             .len(1)
//             .fill_val(0u32)
//             .build()
//         {
//             Ok(buf) => buf,
//             Err(_) => continue,
//         };

//         // (3) Create a kernel with arguments matching those in the source above:
//         let kernel = match Kernel::builder()
//             .program(&prg)
//             .name(&kern_name)
//             .queue(queue.clone())
//             .global_work_size(dev_cfg.global_work_size)
//             .arg(&start_key_b)
//             .arg(&u_data_b)
//             .arg(&enc_data_b)
//             .arg(&key_found_b)
//             .build()
//         {
//             Ok(kern) => kern,
//             Err(_) => continue,
//         };

//         contexts.push(ExecContext {
//             cfg: dev_cfg,
//             ctx: ctx,
//             kernel: kernel,
//             prog: prg,
//             queue: queue,
//             buffers: CtxBuffers {
//                 start_key: start_key_b,
//                 u_data: u_data_b,
//                 enc_data: enc_data_b,
//                 key_found: key_found_b,
//             },
//         });
//     }
//     return contexts;
// }

fn dev_sel_dialog(all_devices: &Vec<(Device, Platform)>) -> Vec<usize> {
    let devs_nums = loop {
        print_devices(&all_devices);
        match choose_devices(all_devices.len()) {
            Ok(value) => break value,
            Err(exc) => {
                println!("Error! {exc}\n")
            }
        }
    };
    return devs_nums;
}

fn get_devices_conf(file_name: &str) -> Result<(Vec<(Device, Platform)>, AppConfig), String> {
    let dev_type = dev_type_from_str("GPU").expect("Unexpected device type!");

    // Get devices to be used for key search
    let all_devices: Vec<(Device, Platform)> = list_devices(dev_type);
    if all_devices.len() == 0 {
        return Err("Cannot find any usable devices.".to_string());
    };

    let app_conf = match client_config::load_config(file_name) {
        Ok(readed_config) => {
            let dev_found = all_devices
                .iter()
                .filter(|dp| readed_config.device_exist(&dp.0))
                .count();
            if dev_found < readed_config.devices.len() {
                println!("Devices from config not found in the system!");
                let devs_nums = dev_sel_dialog(&all_devices);
                let res = AppConfig::from_dev_list(&all_devices, devs_nums);
                client_config::save_config(file_name, &res);
                res
            } else {
                readed_config
            }
        }
        Err(_) => {
            println!("Cannot find config file {}", file_name);
            let devs_nums = dev_sel_dialog(&all_devices);
            AppConfig::from_dev_list(&all_devices, devs_nums)
        }
    };

    let selected_devs = all_devices
        .iter()
        .filter(|dp| app_conf.device_exist(&dp.0))
        .cloned()
        .collect();

    return Ok((selected_devs, app_conf));
}

struct CtxBuffers {
    batch_size: u32,
    tweak_i: u64,
    tweak_j: u32,
    start_key: Buffer<u32>,
    uenc_data: Buffer<u32>,
    target_data: Buffer<u32>,
    key_found: Buffer<u32>,
}

struct ExecData {
    start_key: Vec<u32>,
    uenc_data: Vec<u32>,
    target_data: Vec<u32>,
    key_found: Vec<u32>,
    batch_size: u32,
    work_size: usize,
}
struct ExecContext {
    ctx: Context,
    kernel: Kernel,
    prog: Program,
    queue: Queue,
    buffers: CtxBuffers,
    exec_data: ExecData,
}

fn init_program(
    cl_device: Device,
    cl_platform: Platform,
    cl_src: &str,
    cl_cmplr_opt: &str,
) -> Result<(Context, Program, Queue), ocl::Error> {
    let cl_context = Context::builder()
        .platform(cl_platform)
        .devices(cl_device.clone())
        .build()?;

    let cl_program = Program::builder()
        .devices(cl_device)
        .src(cl_src)
        .cmplr_opt(cl_cmplr_opt)
        .build(&cl_context)
        .unwrap();

    let cl_queue: Queue = Queue::new(&cl_context, cl_device, None)?;

    return Ok((cl_context, cl_program, cl_queue));
}

fn init_buffers(cl_queue: Queue) -> Result<CtxBuffers, ocl::Error> {
    let cl_buffer_start_key = Buffer::<u32>::builder()
        .queue(cl_queue.clone())
        .flags(flags::MEM_READ_ONLY)
        .len(8)
        .fill_val(0u32)
        .build()?;

    let cl_buffer_uenc_data = Buffer::<u32>::builder()
        .queue(cl_queue.clone())
        .flags(flags::MEM_READ_ONLY)
        .len(4)
        .fill_val(0u32)
        .build()?;

    let cl_buffer_target_data = Buffer::<u32>::builder()
        .queue(cl_queue.clone())
        .flags(flags::MEM_READ_ONLY)
        .len(4)
        .fill_val(0u32)
        .build()?;

    let cl_buffer_key_found = Buffer::<u32>::builder()
        .queue(cl_queue.clone())
        .flags(flags::MEM_WRITE_ONLY)
        .len(9)
        .fill_val(0u32)
        .build()?;

    Ok(CtxBuffers {
        batch_size: 0,
        tweak_i: 0,
        tweak_j: 0,
        start_key: cl_buffer_start_key,
        uenc_data: cl_buffer_uenc_data,
        target_data: cl_buffer_target_data,
        key_found: cl_buffer_key_found,
    })
}

fn init_kernel(
    work_size: usize,
    cl_program: Program,
    cl_queue: Queue,
    buffs: &CtxBuffers,
) -> Result<Kernel, ocl::Error> {
    Kernel::builder()
        .program(&cl_program)
        .name("search_key")
        .queue(cl_queue.clone())
        .global_work_size(work_size)
        .arg(&buffs.batch_size)
        .arg(&buffs.tweak_i)
        .arg(&buffs.tweak_j)
        .arg(&buffs.start_key)
        .arg(&buffs.uenc_data)
        .arg(&buffs.target_data)
        .arg(&buffs.key_found)
        .build()
}

fn do_work(ex_ctx: &mut ExecContext) -> Result<bool, ocl::Error> {
    ex_ctx.buffers.batch_size = ex_ctx.exec_data.batch_size;
    ex_ctx
        .buffers
        .start_key
        .cmd()
        .queue(&ex_ctx.queue)
        .offset(0)
        .write(&ex_ctx.exec_data.start_key)
        .enq()?;

    // (4) Run the kernel
    unsafe {
        ex_ctx
            .kernel
            .cmd()
            .queue(&ex_ctx.queue)
            .global_work_size(ex_ctx.exec_data.work_size)
            .enq()?;
    }

    ex_ctx
        .buffers
        .key_found
        .cmd()
        .queue(&ex_ctx.queue)
        .offset(0)
        .read(&mut ex_ctx.exec_data.key_found)
        .enq()?;

    if ex_ctx.exec_data.key_found[0] == 0{Ok(false)}
    else {Ok(true)}
}

fn main() {
    println!("Hello, world nya!");
    //use ocl::{Buffer, Context, Device, Kernel, Platform, Program, Queue, flags};
    let devices = get_devices_conf("test.json");
    println!("{:?}", devices);

    // let devices: Vec<_> = platforms.iter().flat_map(|p| Device::list(p, Some(dev_type)).iter()).collect();
    // let device = Device::first(platform)?;
    // let context = Context::builder()
    //     .platform(platform)
    //     .devices(device.clone())
    //     .build()?;
}
