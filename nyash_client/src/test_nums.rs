





#[cfg(test)]
mod cl_num_utils_tests {
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