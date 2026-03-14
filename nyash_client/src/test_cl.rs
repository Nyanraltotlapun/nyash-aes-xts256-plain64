
#[cfg(test)]
mod test_cl {
    #[test]
    fn test_encryption() {
        use ocl::{Device, Platform};
        use crate::ocl_utils;
        use crate::num_utils;
        

        const SRC_PATH: &str = "src/open_cl/nyash_aes_xts256_plain.cl";
        const OCL_COMP_OPT: &str = "-I src/open_cl";
        const ENCRYPTED_DATA: [u8; 16] = [
            198, 255, 55, 185, 15, 226, 223, 174, 119, 8, 36, 239, 242, 89, 126, 230
        ];
        const KEY_DATA: [u8; 32] = [
            206, 193, 83, 54, 46, 234, 185, 41, 146, 244, 130, 6, 212, 68, 106, 162, 165, 97, 188,
            218, 39, 111, 141, 236, 67, 159, 157, 157, 166, 79, 89, 134
        ];

        // let key_bytes_reversed: Vec<u8> = KEY_DATA.iter().rev().map(|e| *e).collect();
        // let data_bytes_reversed: Vec<u8> = ENCRYPTED_DATA.iter().rev().map(|e| *e).collect();

        let mut tweak_key_b: [u8;16] = [0u8;16];
        let mut data_key_b: [u8;16] = [0u8;16];

        data_key_b.copy_from_slice(&KEY_DATA[0..16]);
        tweak_key_b.copy_from_slice(&KEY_DATA[16..32]);

        // getting keys
        let data_key = u128::from_le_bytes(data_key_b);
        let tweak_key = u128::from_le_bytes(tweak_key_b);
        let data_key = num_utils::u128_to_u32arr(data_key);
        let tweak_key = num_utils::u128_to_u32arr(tweak_key);


        // converting bytes raw data to u32 arr
        let mut encrypted_data: [u32; 4] = [0u32; 4];
        let (enc_dat_bytes_chunks, _) = ENCRYPTED_DATA.as_chunks::<4>();
        for i in 0..4 {
            encrypted_data[i] = u32::from_le_bytes(enc_dat_bytes_chunks[i]);
        }

        // init devices
        let platform = Platform::first().expect("Error getting platform!");
        let device = Device::first(platform).expect("Error getting device!");

        println!("Platform: {:?}, Device: {:?}", platform.name().unwrap(), device.name().unwrap());

        // reading ocl program sources
        let prog_src = std::fs::read_to_string(SRC_PATH).expect("Error reading program sources!");

        let mut nyan_context =
            ocl_utils::ExecContext::new(device, platform, prog_src.as_str(), OCL_COMP_OPT, 256)
                .expect("Error creating execution nyan context!");


        //setting data
        let mut nyan_exec_dat = ocl_utils::ExecData {
            start_key: data_key.to_vec(),
            tweak_key: tweak_key.to_vec(),
            uenc_data: vec![0u32;4],
            target_data: encrypted_data.to_vec(),
            tweak_i: 0,
            tweak_j: 0,
            key_found: vec![0u32;5],
            batch_size: 1000000,
            work_size: 256,
        };

        println!("Set target data");
        ocl_utils::set_target_data(&mut nyan_context, &mut nyan_exec_dat).expect("Error set target data!");
        
        let found_flag = ocl_utils::do_work(&mut nyan_context, &mut nyan_exec_dat).expect("Error do work!");
        println!("Found?: {}", found_flag);
        println!("Key found: {:?}", nyan_exec_dat.key_found);
        assert_eq!(true, found_flag);
    }
}
