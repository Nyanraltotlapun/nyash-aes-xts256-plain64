// aes-xts-pur64 is OpenCL code for aes-xts256-plain64 encryption compatible with LUKS
//
//     Copyright (C) 2025  Kirill Shakirov
//
//     This program is free software: you can redistribute it and/or modify
//     it under the terms of the GNU General Public License as published by
//     the Free Software Foundation, either version 3 of the License, or
//     (at your option) any later version.
//
//     This program is distributed in the hope that it will be useful,
//     but WITHOUT ANY WARRANTY; without even the implied warranty of
//     MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
//     GNU General Public License for more details.
//
//     You should have received a copy of the GNU General Public License
//     along with this program.  If not, see <https://www.gnu.org/licenses/>.


#include "aes256_xts_plain.cl"
#include "num_utils.cl"

// batch_size uint - batch size
// Ti - sector index ulong
// Tj - encryption block number  (16 bytes blocks)

// Tk - tweak key uint[4]
// s_Dk - start of data key uint[4]

// t_e_d - target_enc_data uint[4]
// u_d - unencrypted data to be encrypted uint[4]

// g_key_found uint[9] - 0 element - flag that sets to 1 if key found.
// Other 8 elements is found key

__kernel void search_key_test(const uint batch_size, const ulong g_Ti, const uint g_Tj, 
                              __global const uint8* g_start_enc_key,
                              __global const uint4* g_uenc_data,
                              __global const uint4* g_target_data,
                              __global uint* g_key_found)
{
  const uint g_id = get_global_id(0);

  uint enc_key[8];
  uint tweak[4];
  uint uenc_data[4];
  uint4 target_data = *g_target_data;
  uint4 enc_data = (uint4)(0);
  uint d_ks[44]; // data expanded key
  uint t_ks[44]; // tweak expanded key

  uint sec_n[4] = {0};
  sec_n[0] = ((uint*)&g_Ti)[0];
  sec_n[1] = ((uint*)&g_Ti)[1];

  uint Tj = g_Tj;

  vstore4(*g_uenc_data, 0, uenc_data);
  vstore8(*g_start_enc_key, 0, enc_key);


  // Set initial start key for every work thread
  uint k_data_carry = add_uint_to_bigint4_ (enc_key, (g_id*batch_size));
  uint k_tweak_carry = add_one_to_bigint4_ (&enc_key[4]);
  if (k_tweak_carry != 0u) return; // if reached max key value exit thread

  // Generate tweak
  aes128_set_encrypt_key (t_ks, &enc_key[4]);
  aes_xts256_gen_tweak (t_ks, sec_n, Tj, tweak);

  for (uint batch_id = 0u; (batch_id < batch_size); batch_id++)
  {
    // Data encrypt key always changing because we increment from 0 index to 8
    aes128_set_encrypt_key (d_ks, enc_key);

    // encrypt data
    aes_xts256_enc_block (d_ks, tweak, uenc_data, (uint*)&enc_data);

    // check if we found the key!
    if (all(enc_data==target_data)) 
    {
      g_key_found[0] = 1;
      vstore8(vload8(0, enc_key), 0, &g_key_found[1]);
      return;
    }

    // Increment data key part by 1.
    k_data_carry = add_one_to_bigint4_ (enc_key);

    // Tweak changes only once in 2^128 times
    if (k_data_carry != 0u) {
      // Increment tweak part
      k_tweak_carry = add_one_to_bigint4_ (&enc_key[4]);
      if (k_tweak_carry != 0u) return; // if reached max key value exit thread
      // Gen new tweak
      aes128_set_encrypt_key (t_ks, &enc_key[4]);
      aes_xts256_gen_tweak (t_ks, sec_n, Tj, tweak);
    }

  }
}

