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


inline void aes128_InvertKey (uint *ks);
inline void aes128_ExpandKey (uint *ks, const uint *ukey);
inline void aes128_set_encrypt_key (uint *ks, const uint *ukey);
inline void aes128_set_decrypt_key (uint *ks, const uint *ukey);
inline void aes128_encrypt (const uint *ks, const uint *in, uint *out);
inline void aes128_decrypt (const uint *ks, const uint *in, uint *out);

inline void xts_mul2 (uint *in, uint *out);
inline void aes_xts256_gen_tweak (const uint *ks, const uint *sec_n, const uint block_n, uint *out);
inline void aes_xts256_enc_block (const uint *ks, const uint *T, const uint *in, uint *out);
inline void aes_xts256_dec_block (const uint *ks, const uint *T, const uint *in, uint *out);
