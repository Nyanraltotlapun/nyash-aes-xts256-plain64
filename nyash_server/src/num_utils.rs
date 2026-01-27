// union U64orU32 {
//     l: u64,
//     i: [u32; 2],
// }

fn u128_to_u32arr(a: u128) -> [u32;4] {
    let mut res = [0u32;4];
    let a_bytes = a.to_le_bytes();
    let chunks = a_bytes.as_chunks::<4>().0;
    for i in 0..4 {
        res[i] = u32::from_le_bytes(chunks[i]);
    }
    return res;
}

pub fn add_u128_to_u256(a: &[u32; 8], b: u128) -> ([u32; 8], bool) {
    let mut res: [u32; 8] = [0; 8];
    let mut carry = false;

    // convert b u128 value to bytes then to u32 and then add it to a
    b.to_le_bytes()
        .as_chunks::<4>()
        .0
        .iter()
        .enumerate()
        .for_each(|(i, sl)| {
            (res[i], carry) = a[i].carrying_add(u32::from_le_bytes(*sl), carry);
        });

    // propagate carry till the end of a
    for idx in 4..8 {
        (res[idx], carry) = a[idx].carrying_add(0, carry);
    }

    return (res, carry);
}

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
}
