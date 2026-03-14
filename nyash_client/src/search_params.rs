

pub fn get_params() ->([u32; 4],[u32; 4],[u32; 4]) {
    use crate::num_utils;

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

    return (data_key, tweak_key, encrypted_data);
}