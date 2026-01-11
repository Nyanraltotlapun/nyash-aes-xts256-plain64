// union U64orU32 {
//     l: u64,
//     i: [u32; 2],
// }

fn add_u32_to_u256(a: &[u32; 8], b: u32) -> ([u32; 8], bool) {
    let mut res: [u32; 8] = [0; 8];
    let mut carry = false;
    (res[0], carry) = a[0].carrying_add(b, carry);

    for idx in 1..8 {
        (res[idx], carry) = a[idx].carrying_add(0, carry);
    }

    return (res, carry);
}

fn add_u32_to_u256_(a: &mut [u32; 8], b: u32) -> bool {
    let mut carry = false;
    (a[0], carry) = a[0].carrying_add(b, carry);

    for idx in 1..8 {
        (a[idx], carry) = a[idx].carrying_add(0, carry);
    }

    return carry;
}

fn bytes_from_chars(chars_chunk: &[char]) -> [u8; 4] {
    let mut res: [u8; 4] = [0; 4];
    let mut idx: usize = 0;
    chars_chunk.chunks_exact(2).for_each(|b_c| {
        if idx < 4 {
            match u8::from_str_radix(&b_c.iter().collect::<String>(), 16) {
                Ok(n) => res[idx] = n,
                Err(_) => (),
            }
            idx += 1;
        }
    });
    return res;
}

fn bignum_from_hex(hex: &str) -> [u32; 8] {
    let mut res: [u32; 8] = [0; 8];
    let mut idx: usize = 0;
    let chars_hex = hex.chars().collect::<Vec<char>>();
    chars_hex
        .chunks_exact(8)
        .rev()
        .map(|chunk| bytes_from_chars(chunk))
        .for_each(|b_arr| {
            if idx < 8 {
                res[idx] = u32::from_be_bytes(b_arr);
                idx += 1;
            }
        });
    return res;
}

fn hex_fmt_byte(n: u32) -> String {
    let res: String = n
        .to_be_bytes()
        .iter()
        .map(|b| format!("{:02x}", b))
        .collect();
    return res;
}

fn bignum_to_hex(a: &[u32; 8]) -> String {
    let res: String = a
        .iter()
        .rev()
        .map(|n| hex_fmt_byte(*n))
        .collect::<Vec<String>>()
        .join("");
    return res;
}

#[cfg(test)]
mod num_utils_tests {
    use std::io::Read;

    use super::*;

    #[test]
    fn test_add() {
        use std::io::{BufRead, BufReader};
        use std::process::{Command, Stdio};

        let test_gen_cmd = "/home/kira/Development/Rust/nyash-aes-xts256-plain64/nyash_client/src/tests/gen_test_data.py";
        let mut child = Command::new(test_gen_cmd)
            .stdout(Stdio::piped())
            .spawn()
            .unwrap();

        let gen_stdout = child
            .stdout
            .take()
            .ok_or("Failed to capture stdout")
            .unwrap();
        let gen_reader = BufReader::new(gen_stdout);

        for r_line in gen_reader.lines() {
            let test_line: String = r_line.unwrap(); // Handle any I/O errors
            let test_data_line = test_line.split(' ').collect::<Vec<&str>>();
            let num_to_add = u32::from_str_radix(test_data_line[0], 10).unwrap();
            let t0 = bignum_from_hex(test_data_line[1]);

            let t1_test = add_u32_to_u256(&t0, 1).0;
            let t2_test = add_u32_to_u256(&t0, num_to_add).0;

            let res_actual = format!(
                "{} {} {} {}",
                num_to_add,
                bignum_to_hex(&t0),
                bignum_to_hex(&t1_test),
                bignum_to_hex(&t2_test)
            );
            assert_eq!(test_line, res_actual);
        }

        let _ = child.wait().unwrap();

    }

    #[test]
    fn test_cl_add() {
        extern crate ocl;
        use ocl::{
            Buffer, Context, Device, DeviceType, Kernel, Platform, Program, Queue, SpatialDims,
            flags,
        };
        use std::fs::File;
        use std::io::{BufRead, BufReader};
        use std::process::{Command, Stdio};

        let cl_test_path = "/home/kira/Development/Rust/nyash-aes-xts256-plain64/nyash_client/src/open_cl/test_num_utils.cl";
        let cl_include_opt =
            "-I /home/kira/Development/Rust/nyash-aes-xts256-plain64/nyash_client/src/open_cl";
        let mut cl_src = String::new();
        // read ocl source
        BufReader::new(File::open(cl_test_path).unwrap()).read_to_string(&mut cl_src);

        const G_WORK_SIZE: usize = 4096;
        
        let cl_platform = Platform::default();
        let cl_device = Device::first(cl_platform).unwrap();
        let cl_context = Context::builder()
            .platform(cl_platform)
            .devices(cl_device.clone())
            .build()
            .unwrap();
        let cl_program = Program::builder()
            .devices(cl_device)
            .src(cl_src)
            .cmplr_opt(cl_include_opt)
            .build(&cl_context)
            .unwrap();
        let cl_queue = Queue::new(&cl_context, cl_device, None).unwrap();

        let cl_buffer_num = Buffer::<u32>::builder()
            .queue(cl_queue.clone())
            .flags(flags::MEM_READ_ONLY)
            .len(G_WORK_SIZE)
            .fill_val(0u32)
            .build()
            .unwrap();

        let cl_buffer_t0 = Buffer::<u32>::builder()
            .queue(cl_queue.clone())
            .flags(flags::MEM_READ_ONLY)
            .len(G_WORK_SIZE * 8)
            .fill_val(0u32)
            .build()
            .unwrap();

        let cl_buffer_t1 = Buffer::<u32>::builder()
            .queue(cl_queue.clone())
            .flags(flags::MEM_WRITE_ONLY)
            .len(G_WORK_SIZE * 8)
            .fill_val(0u32)
            .build()
            .unwrap();

        let cl_buffer_t2 = Buffer::<u32>::builder()
            .queue(cl_queue.clone())
            .flags(flags::MEM_WRITE_ONLY)
            .len(G_WORK_SIZE * 8)
            .fill_val(0u32)
            .build()
            .unwrap();

        // (3) Create a kernel with arguments matching those in the source above:
        let kernel = Kernel::builder()
            .program(&cl_program)
            .name("test_add")
            .queue(cl_queue.clone())
            .global_work_size(G_WORK_SIZE)
            .arg(&cl_buffer_num)
            .arg(&cl_buffer_t0)
            .arg(&cl_buffer_t1)
            .arg(&cl_buffer_t2)
            .build()
            .unwrap();

        let test_gen_cmd = "/home/kira/Development/Rust/nyash-aes-xts256-plain64/nyash_client/src/tests/gen_test_data.py";
        let mut child = Command::new(test_gen_cmd)
            .stdout(Stdio::piped())
            .spawn()
            .unwrap();

        let gen_stdout = child
            .stdout
            .take()
            .ok_or("Failed to capture stdout")
            .unwrap();
        let gen_reader = BufReader::new(gen_stdout);


        let mut buffer_num: Vec<u32> = vec![0u32; G_WORK_SIZE];
        let mut buffer_t0: Vec<u32> = vec![0u32; G_WORK_SIZE*8];
        let mut exp_buffer_t1: Vec<u32> = vec![0u32; G_WORK_SIZE*8];
        let mut exp_buffer_t2: Vec<u32> = vec![0u32; G_WORK_SIZE*8];

        let mut act_buffer_t1: Vec<u32> = vec![0u32; G_WORK_SIZE*8];
        let mut act_buffer_t2: Vec<u32> = vec![0u32; G_WORK_SIZE*8];

        let mut w_id: usize = 0;

        for r_line in gen_reader.lines() {
            let test_line: String = r_line.unwrap(); // Handle any I/O errors
            let test_data_line = test_line.split(' ').collect::<Vec<&str>>();
            let num_to_add = u32::from_str_radix(test_data_line[0], 10).unwrap();
            buffer_num[w_id] = num_to_add;
            let slise_id = w_id*8;
            buffer_t0[slise_id..slise_id+8].copy_from_slice(&bignum_from_hex(test_data_line[1]));
            exp_buffer_t1[slise_id..slise_id+8].copy_from_slice(&bignum_from_hex(test_data_line[2]));
            exp_buffer_t2[slise_id..slise_id+8].copy_from_slice(&bignum_from_hex(test_data_line[3]));

            w_id += 1;
            if w_id >= G_WORK_SIZE {
                w_id = 0; // reset counter
                cl_buffer_num.cmd().queue(&cl_queue).offset(0).write(&buffer_num).enq().unwrap();
                cl_buffer_t0.cmd().queue(&cl_queue).offset(0).write(&buffer_t0).enq().unwrap();
                
                // (4) Run the kernel
                unsafe {
                    kernel
                        .cmd()
                        .queue(&cl_queue)
                        .global_work_size(G_WORK_SIZE)
                        .enq().unwrap();
                }

                cl_buffer_t1.cmd().queue(&cl_queue).offset(0).read(&mut act_buffer_t1).enq().unwrap();
                cl_buffer_t2.cmd().queue(&cl_queue).offset(0).read(&mut act_buffer_t2).enq().unwrap();

                assert_eq!(exp_buffer_t1, act_buffer_t1);
                assert_eq!(exp_buffer_t2, act_buffer_t2);
            }

        }

        let _ = child.wait().unwrap();
    }
}
