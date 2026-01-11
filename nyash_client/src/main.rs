extern crate ocl;
use ocl::{
    Buffer, Context, Device, DeviceType, Kernel, Platform, Program, Queue, SpatialDims, flags,
};
use std::io;
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

struct CtxBuffers {
    start_key: Buffer<u32>,
    u_data: Buffer<u32>,
    enc_data: Buffer<u32>,
    key_found: Buffer<u32>,
}

struct DevConfig {
    train_work_size: bool,
    global_work_size: SpatialDims,
}

struct ExecContext {
    cfg: DevConfig,
    ctx: Context,
    kernel: Kernel,
    prog: Program,
    queue: Queue,
    buffers: CtxBuffers,
}

fn init_devices(
    devices: Vec<(Device, Platform, DevConfig)>,
    kern_name: String,
    prog_src: String,
    inc_dirs: Vec<String>,
) -> Vec<ExecContext> {

    let mut contexts: Vec<ExecContext> = Vec::with_capacity(devices.len());
    for (dev, plt, dev_cfg) in devices {
        let ctx = match Context::builder()
            .platform(plt)
            .devices(dev.clone())
            .build()
        {
            Ok(c) => c,
            Err(_) => continue,
        };

        let prg = match Program::builder().devices(dev).src(&prog_src).build(&ctx) {
            Ok(p) => p,
            Err(_) => continue,
        };

        let queue = match Queue::new(&ctx, dev, None) {
            Ok(q) => q,
            Err(_) => continue,
        };

        // Create Buffers:
        let start_key_b = match Buffer::<u32>::builder()
            .queue(queue.clone())
            .flags(flags::MEM_READ_ONLY)
            .len(8)
            .fill_val(0u32)
            .build()
        {
            Ok(buf) => buf,
            Err(_) => continue,
        };

        let u_data_b = match Buffer::<u32>::builder()
            .queue(queue.clone())
            .flags(flags::MEM_READ_ONLY)
            .len(4)
            .fill_val(0u32)
            .build()
        {
            Ok(buf) => buf,
            Err(_) => continue,
        };

        let enc_data_b = match Buffer::<u32>::builder()
            .queue(queue.clone())
            .flags(flags::MEM_READ_ONLY)
            .len(4)
            .fill_val(0u32)
            .build()
        {
            Ok(buf) => buf,
            Err(_) => continue,
        };

        let key_found_b = match Buffer::<u32>::builder()
            .queue(queue.clone())
            .flags(flags::MEM_WRITE_ONLY)
            .len(1)
            .fill_val(0u32)
            .build()
        {
            Ok(buf) => buf,
            Err(_) => continue,
        };

        // (3) Create a kernel with arguments matching those in the source above:
        let kernel = match Kernel::builder()
            .program(&prg)
            .name(&kern_name)
            .queue(queue.clone())
            .global_work_size(dev_cfg.global_work_size)
            .arg(&start_key_b)
            .arg(&u_data_b)
            .arg(&enc_data_b)
            .arg(&key_found_b)
            .build() {
                Ok(kern) => kern,
                Err(_) => continue,
            };

        contexts.push(ExecContext {
            cfg: dev_cfg,
            ctx: ctx,
            kernel: kernel,
            prog: prg,
            queue: queue,
            buffers: CtxBuffers {
                start_key: start_key_b,
                u_data: u_data_b,
                enc_data: enc_data_b,
                key_found: key_found_b,
            },
        });
    }
    return contexts;
}

fn main() {
    println!("Hello, world nya!");
    //use ocl::{Buffer, Context, Device, Kernel, Platform, Program, Queue, flags};
    let dev_type = dev_type_from_str("GPU").expect("pur");

    // Get devices to be used for key search
    let mut all_devices: Vec<(Device, Platform)> = list_devices(dev_type);

    let devs_nums = loop {
        print_devices(&all_devices);
        match choose_devices(all_devices.len()) {
            Ok(value) => break value,
            Err(exc) => {
                println!("Error! {exc}\n")
            }
        }
    };
    let devices: Vec<(Device, Platform)> = devs_nums.iter().map(|&i| all_devices[i]).collect();
    all_devices.clear();
    println!("{:?}", devices);
    

    // let devices: Vec<_> = platforms.iter().flat_map(|p| Device::list(p, Some(dev_type)).iter()).collect();
    // let device = Device::first(platform)?;
    // let context = Context::builder()
    //     .platform(platform)
    //     .devices(device.clone())
    //     .build()?;
}
