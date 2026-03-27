pub fn get_params() -> [u32; 4] {

    const ENCRYPTED_DATA: [u8; 16] = [
        10, 51, 110, 227, 194, 181, 104, 65, 151, 47, 69, 37, 66, 223, 71, 137,
    ];

    // let key_bytes_reversed: Vec<u8> = KEY_DATA.iter().rev().map(|e| *e).collect();
    // let data_bytes_reversed: Vec<u8> = ENCRYPTED_DATA.iter().rev().map(|e| *e).collect();




    // converting bytes raw data to u32 arr
    let mut encrypted_data: [u32; 4] = [0u32; 4];
    let (enc_dat_bytes_chunks, _) = ENCRYPTED_DATA.as_chunks::<4>();
    for i in 0..4 {
        encrypted_data[i] = u32::from_le_bytes(enc_dat_bytes_chunks[i]);
    }

    return encrypted_data;
}
