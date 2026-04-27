use aes;

use aes::Aes128;
use aes::cipher::array::sizes;
use aes::cipher::{Array, BlockCipherEncrypt, KeyInit};
use std::sync::mpsc;
use threadpool::ThreadPool;

/// Galois field multiplication
/// Multiplies a 128-bit block by x in GF(2^128) for AES-XTS.
/// The irreducible polynomial used is x^128 + x^7 + x^2 + x + 1 (0x87).
// #[inline]
// fn gf128_mul_by_x(block: &mut Array<u8, sizes::U16>) {
//     let mut carry = 0u8;
//     // Process bytes from most significant (15) to least significant (0)
//     for byte in block.iter_mut().rev() {
//         let temp = *byte;
//         *byte = (*byte << 1) | carry;
//         carry = temp >> 7; // The bit that was shifted out
//     }
//     // If the most significant bit of the original block was 1, apply reduction
//     if carry != 0 {
//         block[15] ^= 0x87; // XOR with the reduction polynomial
//     }
// }

// #[inline]
// fn aes_xts256_gen_tweak(
//     tweak_key: &Array<u8, sizes::U16>,
//     sector_number: &Array<u8, sizes::U16>,
//     block_number: u32,
//     tweak_buff: &mut Array<u8, sizes::U16>,
// ) {
//     let cipher = Aes128::new(tweak_key);
//     cipher.encrypt_block_b2b(sector_number, tweak_buff);

//     //Double the tweak value block_n-1 times (since first block is 0)
//     for _ in 0..block_number {
//         gf128_mul_by_x(tweak_buff);
//     }
// }

#[inline]
fn gen_tweak_zero_block(tweak_key: &Array<u8, sizes::U16>) -> Array<u8, sizes::U16> {
    let cipher = Aes128::new(tweak_key);
    let mut res: Array<u8, sizes::U16> = Array([0u8;16]);
    cipher.encrypt_block(&mut res);
    res
}

#[inline]
fn add_one_to_block_(block: &mut Array<u8, sizes::U16>) -> bool {
    let mut carry = false;
    (block[0], carry) = block[0].carrying_add(1u8, carry);

    for i in 1..16 {
        if carry == false {
            break;
        }
        (block[i], carry) = block[i].carrying_add(0u8, carry);
    }
    carry
}

#[inline]
fn xor_arrays(
    a_arr: &Array<u8, sizes::U16>,
    b_arr: &Array<u8, sizes::U16>,
    out_arr: &mut Array<u8, sizes::U16>,
) {
    out_arr
        .iter_mut()
        .zip(a_arr.iter().zip(b_arr.iter()))
        .for_each(|(o, (a, b))| *o = a ^ b);
}

#[inline]
fn xor_array_(a_arr: &mut Array<u8, sizes::U16>, b_arr: &Array<u8, sizes::U16>) {
    a_arr
        .iter_mut()
        .zip(b_arr.iter())
        .for_each(|(o, a)| *o ^= a);
}

// fn add_u64_to_arr_u8(arr: &Array<u8, sizes::U16>, b: u64) -> Array<u8, sizes::U16> {
//     let mut res: Array<u8, sizes::U16> = Array([0u8; 16]);
//     let b_bytes = b.to_le_bytes();
//     let mut carry: bool = false;
//     for i in 0..8 {
//         (res[i], carry) = arr[i].carrying_add(b_bytes[i], carry);
//     }

//     for i in 8..16 {
//         (res[i], carry) = arr[i].carrying_add(0u8, carry);
//     }

//     res
// }

fn add_u64_to_arr_u8_(arr: &mut Array<u8, sizes::U16>, b: u64) -> bool {
    let b_bytes = b.to_le_bytes();
    let mut carry: bool = false;
    for i in 0..8 {
        (arr[i], carry) = arr[i].carrying_add(b_bytes[i], carry);
    }

    for i in 8..16 {
        (arr[i], carry) = arr[i].carrying_add(0u8, carry);
    }

    carry
}

pub fn encrypt_and_check(
    start_key: &Array<u8, sizes::U16>,
    tweak: &Array<u8, sizes::U16>,
    batch_size: u64,
    in_block: &Array<u8, sizes::U16>,
    search_block: &Array<u8, sizes::U16>,
) -> Option<Array<u8, sizes::U16>> {
    let mut current_key = start_key.clone();
    let mut out_block: Array<u8, sizes::U16> = Array::from([0u8; 16]);

    for _b_num in 0..batch_size {
        // Initialize cipher
        let cipher = Aes128::new(&current_key);

        xor_arrays(in_block, tweak, &mut out_block);
        // Encrypt block in-place
        cipher.encrypt_block(&mut out_block);
        xor_array_(&mut out_block, tweak);

        if out_block.eq(search_block) {
            return Some(current_key);
        }

        let _ = add_one_to_block_(&mut current_key);
    }

    return None;
}

pub fn do_work(
    pool: &ThreadPool,
    batch_size: u64,
    start_key: &[u8; 16],
    tweak_key: &[u8; 16],
    data_block: &[u8; 16],
    search_block: &[u8; 16],
) -> Option<Array<u8, sizes::U16>> {
    
    let (tx, rx) = mpsc::channel();

    let th_num = pool.max_count();

    let mut a_start_key: Array<u8, sizes::U16> = Array::from(start_key.clone());
    let a_tweak_key: Array<u8, sizes::U16> = Array::from(tweak_key.clone());
    let a_data_block: Array<u8, sizes::U16> = Array::from(data_block.clone());
    let a_search_block: Array<u8, sizes::U16> = Array::from(search_block.clone());

    let tweak = gen_tweak_zero_block(&a_tweak_key);
    

    for _ in 0..th_num {
        let th_start_key = a_start_key.clone();
        let tx = tx.clone();
        pool.execute(move || {
            let result = encrypt_and_check(
                &th_start_key,
                &tweak,
                batch_size,
                &a_data_block,
                &a_search_block,
            );
            tx.send(result).unwrap();
        });

        add_u64_to_arr_u8_(&mut a_start_key, batch_size);
    }

    drop(tx); // Close the sending end

    // Collect all results
    let results = rx.iter()
    .find(|x| x.is_some())
    .unwrap_or(None);

    return results;
}
