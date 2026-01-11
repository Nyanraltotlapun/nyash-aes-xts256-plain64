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

__kernel void encrypt_data(__global const ulong* g_Ti, __global const uint* g_Tj, 
                           __global const uint8* g_key,
                           __global const uint4* g_u_data,
                           __global uint* g_enc_data)
{
  const size_t g_id = get_global_id(0);
  uint d_ks[44];
  uint t_ks[44];
  uint tweak[4];
  uint enc_key[8];
  uint u_data[4];
  uint enc_data[4] = { 0 };

  uint sec_n[4];
  ulong Ti = g_Ti[g_id];
  sec_n[0] = ((uint*)&Ti)[0];
  sec_n[1] = ((uint*)&Ti)[1];
  sec_n[2] = 0;
  sec_n[3] = 0;

  uint Tj = g_Tj[g_id];

  vstore8(*g_key, 0, enc_key);
  vstore4(g_u_data[g_id], 0, u_data);

  // printf("Ti: %lu\\n", Ti);
  // printf("Tj: %u\\n", Tj);
  // printf("enc_key: %v8u\\n", *(uint8*)enc_key);
  // printf("uenc_data: %v4u\\n", *(uint4*)uenc_data);

  //calculate tweak value
  aes128_set_encrypt_key (t_ks, &enc_key[4]);
  aes_xts256_gen_tweak (t_ks, sec_n, Tj, tweak);

  // encrypt data
  aes128_set_encrypt_key (d_ks, enc_key);
  aes_xts256_enc_block (d_ks, tweak, u_data, enc_data);
  // printf("enc_data: %v4u\\n", *(uint4*)enc_data);
  vstore4(*(uint4*)enc_data, g_id, g_enc_data);
}


__kernel void decrypt_data(__global const ulong* g_Ti, __global const uint* g_Tj, 
                           __global const uint8* g_key,
                           __global const uint4* g_enc_data,
                           __global uint* g_u_data)
{
  const size_t g_id = get_global_id(0);
  uint d_ks[44];
  uint t_ks[44];
  uint tweak[4];
  uint enc_key[8];
  uint enc_data[4];
  uint u_data[4] = { 0 };

  uint sec_n[4];
  ulong Ti = g_Ti[g_id];
  sec_n[0] = ((uint*)&Ti)[0];
  sec_n[1] = ((uint*)&Ti)[1];
  sec_n[2] = 0;
  sec_n[3] = 0;

  uint Tj = g_Tj[g_id];

  vstore8(*g_key, 0, enc_key);
  vstore4(g_enc_data[g_id], 0, enc_data);


  //calculate tweak value
  aes128_set_encrypt_key (t_ks, &enc_key[4]);
  aes_xts256_gen_tweak (t_ks, sec_n, Tj, tweak);

  // decrypt data
  aes128_set_decrypt_key (d_ks, enc_key);
  aes_xts256_dec_block (d_ks, tweak, enc_data, u_data);
  // printf("enc_data: %v4u\\n", *(uint4*)enc_data);
  vstore4(*(uint4*)u_data, g_id, g_u_data);
}
