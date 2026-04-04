
use std::io::Read;

use ocl::{Buffer, Context, Device, Kernel, Platform, Program, Queue, flags};

use crate::num_utils;

pub struct CtxBuffers {
    tweak_params: Buffer<u32>,
    batch_size: Buffer<u32>,
    start_key: Buffer<u32>,
    tweak_key: Buffer<u32>,
    uenc_data: Buffer<u32>,
    target_data: Buffer<u32>,
    key_found: Buffer<u32>,
}

pub struct ExecData {
    pub start_key: Vec<u32>,
    pub tweak_key: Vec<u32>,
    pub uenc_data: Vec<u32>,
    pub target_data: Vec<u32>,
    pub tweak_i: u64,
    pub tweak_j: u32,
    pub key_found: Vec<u32>,
    pub batch_size: u32,
    pub work_size: usize,
}
 impl ExecData {
    // g_params[uint4]
    // g_params[0-1] - ulog g_Ti
    // g_params[2] -   g_Tj
    pub fn tweak_params(&self) -> Vec<u32> {
        let mut res: Vec<u32> = Vec::with_capacity(4);

        // the sector number (S) is first converted into a little-endian byte array 
        // before being encrypted using the second AES key (K₂)
        let tweak_i_b: [u8;8] = self.tweak_i.to_le_bytes();
        let (tweak_i_cnk,_) = tweak_i_b.as_chunks::<4>();
        res.push(u32::from_le_bytes(tweak_i_cnk[0]));
        res.push(u32::from_le_bytes(tweak_i_cnk[1]));

        //last enc block number (tweak_j)
        res.push(self.tweak_j);

        return res;
    }

    pub fn get_found_key(&self) -> u128 {
        let mut u32_arr_k = [0u32;4];
        u32_arr_k[0] = self.key_found[1];
        u32_arr_k[1] = self.key_found[2];
        u32_arr_k[2] = self.key_found[3];
        u32_arr_k[3] = self.key_found[4];
        return num_utils::u32arr_to_u128(u32_arr_k);
    }
     
 }

pub struct ExecContext {
    _ctx: Context,
    kernel: Kernel,
    _prog: Program,
    queue: Queue,
    buffers: CtxBuffers,
}

pub fn init_program(
    cl_device: Device,
    cl_platform: Platform,
    cl_src_gz_bytes: &[u8],
) -> Result<(Context, Program, Queue), ocl::Error> {
    use flate2::read::GzDecoder;
    let mut gz_decoder = GzDecoder::new(cl_src_gz_bytes);
    let mut decompressed_src = String::new();
    gz_decoder.read_to_string(&mut decompressed_src).expect("Error decompressing OCL sources!");

    let cl_context = Context::builder()
        .platform(cl_platform)
        .devices(cl_device.clone())
        .build()?;

    let cl_program = Program::builder()
        .devices(cl_device)
        .src(decompressed_src)
        .build(&cl_context)
        .unwrap();

    let cl_queue: Queue = Queue::new(&cl_context, cl_device, None)?;

    return Ok((cl_context, cl_program, cl_queue));
}

pub fn init_buffers(cl_queue: &Queue) -> Result<CtxBuffers, ocl::Error> {
    let cl_buffer_tweak_params = Buffer::<u32>::builder()
        .queue(cl_queue.clone())
        .flags(flags::MEM_READ_ONLY)
        .len(3)
        .fill_val(0u32)
        .build()?;

    let cl_buffer_batch_size = Buffer::<u32>::builder()
        .queue(cl_queue.clone())
        .flags(flags::MEM_READ_ONLY)
        .len(1)
        .fill_val(0u32)
        .build()?;

    let cl_buffer_start_key = Buffer::<u32>::builder()
        .queue(cl_queue.clone())
        .flags(flags::MEM_READ_ONLY)
        .len(4)
        .fill_val(0u32)
        .build()?;

    let cl_buffer_tweak_key = Buffer::<u32>::builder()
        .queue(cl_queue.clone())
        .flags(flags::MEM_READ_ONLY)
        .len(4)
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
        .len(5)
        .fill_val(0u32)
        .build()?;


    Ok(CtxBuffers {
        tweak_params: cl_buffer_tweak_params,
        batch_size: cl_buffer_batch_size,
        start_key: cl_buffer_start_key,
        tweak_key: cl_buffer_tweak_key,
        uenc_data: cl_buffer_uenc_data,
        target_data: cl_buffer_target_data,
        key_found: cl_buffer_key_found,
    })
}

fn init_kernel(
    work_size: usize,
    cl_program: &Program,
    cl_queue: &Queue,
    buffs: &CtxBuffers,
) -> Result<Kernel, ocl::Error> {
    Kernel::builder()
        .program(cl_program)
        .name("search_key")
        .queue(cl_queue.clone())
        .global_work_size(work_size)
        .arg(&buffs.tweak_params)
        .arg(&buffs.batch_size)
        .arg(&buffs.start_key)
        .arg(&buffs.tweak_key)
        .arg(&buffs.uenc_data)
        .arg(&buffs.target_data)
        .arg(&buffs.key_found)
        .build()
}

impl ExecContext {
    // Constructor with parameters
    pub fn new(
        cl_device: Device,
        cl_platform: Platform,
        cl_src_gz_bytes: &[u8],
        global_work_size: usize,
    ) -> Result<Self, ocl::Error> {
        let (nya_cl_context, nya_cl_program, nya_cl_queue) =
            init_program(cl_device, cl_platform, cl_src_gz_bytes)?;

        let nya_cl_buffers = init_buffers(&nya_cl_queue)?;

        let nya_cl_kernel = init_kernel(
            global_work_size,
            &nya_cl_program,
            &nya_cl_queue,
            &nya_cl_buffers,
        )?;
        Ok(Self {
            _ctx: nya_cl_context,
            kernel: nya_cl_kernel,
            _prog: nya_cl_program,
            queue: nya_cl_queue,
            buffers: nya_cl_buffers,
        })
    }

    pub fn reinit_kernel(&mut self, global_work_size: usize) -> Result<(), ocl::Error> {
        self.kernel = init_kernel(
            global_work_size,
            &self._prog,
            &self.queue,
            &self.buffers,
        )?;
        Ok(())
    }
}

pub fn set_target_data(ex_ctx: &mut ExecContext, ex_data: &mut ExecData) -> Result<(), ocl::Error> {
    // transfer tweaks
    let t_p = ex_data.tweak_params();
    ex_ctx
        .buffers
        .tweak_params
        .cmd()
        .queue(&ex_ctx.queue)
        .offset(0)
        .write(&t_p)
        .enq()?;

    // transfen unencrypted data to device
    ex_ctx
        .buffers
        .uenc_data
        .cmd()
        .queue(&ex_ctx.queue)
        .offset(0)
        .write(&ex_data.uenc_data)
        .enq()?;

    // transfet target data
    ex_ctx
        .buffers
        .target_data
        .cmd()
        .queue(&ex_ctx.queue)
        .offset(0)
        .write(&ex_data.target_data)
        .enq()?;

    //ex_ctx.queue.finish()?;

    return Ok(());
}

pub fn do_work(ex_ctx: &mut ExecContext, ex_data: &mut ExecData) -> Result<(bool, f64), ocl::Error> {

    let b_s = vec![ex_data.batch_size];

    let start_time = std::time::Instant::now();

    //println!("Copy batch_size buffer...");
    // tranfer batch_size
    ex_ctx
        .buffers
        .batch_size
        .cmd()
        .queue(&ex_ctx.queue)
        .offset(0)
        .write(&b_s)
        .enq()?;


    //println!("Copy start_key buffer...");
    // transfer start key to device
    ex_ctx
        .buffers
        .start_key
        .cmd()
        .queue(&ex_ctx.queue)
        .offset(0)
        .write(&ex_data.start_key)
        .enq()?;

    //println!("Copy tweak_key buffer...");
    // transfet tweak key
    ex_ctx
        .buffers
        .tweak_key
        .cmd()
        .queue(&ex_ctx.queue)
        .offset(0)
        .write(&ex_data.tweak_key)
        .enq()?;

    // zero out key_found buffer
    // ex_ctx
    //     .buffers
    //     .key_found
    //     .cmd()
    //     .queue(&ex_ctx.queue)
    //     .offset(0)
    //     .fill(0u32, None)
    //     .enq()?;

    //println!("Copy data to GPU...");
    //ex_ctx.queue.finish()?;
    //println!("Run kernel...");
    // (4) Run the kernel
    unsafe {
        ex_ctx
            .kernel
            .cmd()
            .queue(&ex_ctx.queue)
            .global_work_size(ex_data.work_size)
            .enq()?;
    }

    //println!("Waiting for kernel to finish work...");
    //ex_ctx.queue.finish()?;

    //println!("Copy data back from GPU...");
    // read key_foun buffer
    ex_ctx
        .buffers
        .key_found
        .cmd()
        .queue(&ex_ctx.queue)
        .offset(0)
        .read(&mut ex_data.key_found)
        .enq()?;

    //println!("Copy results back...");
    //ex_ctx.queue.finish()?;

    let exec_duration = start_time.elapsed().as_secs_f64();

    //ex_ctx.queue.finish()?;

    if ex_data.key_found[0] == 0 {
        Ok((false, exec_duration))
    } else {
        Ok((true, exec_duration))
    }
}
