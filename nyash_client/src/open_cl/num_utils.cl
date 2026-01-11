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



typedef struct __attribute__((aligned(32))) {
    uint carry;
    uint carry4;
}  bigintRes;

typedef union {
    ulong l;
    uint i[2];
} __attribute__((aligned(32))) ul_ui_union;



inline uint add_one_to_bigint4_(uint *_n)
{
  ul_ui_union t;
  t.l = (ulong)_n[0] + 1ul;
  _n[0] = t.i[0];

  t.l = (ulong)_n[1] + (ulong)t.i[1];
  _n[1] = t.i[0];
  t.l = (ulong)_n[2] + (ulong)t.i[1];
  _n[2] = t.i[0];
  t.l = (ulong)_n[3] + (ulong)t.i[1];
  _n[3] = t.i[0];

  return t.i[1];
}

inline uint add_uint_to_bigint4_ (uint* _n, const uint b)
{
  ul_ui_union t;
  t.l = (ulong)_n[0] + (ulong)b;
  _n[0] = t.i[0];

  t.l = (ulong)_n[1] + (ulong)t.i[1];
  _n[1] = t.i[0];
  t.l = (ulong)_n[2] + (ulong)t.i[1];
  _n[2] = t.i[0];
  t.l = (ulong)_n[3] + (ulong)t.i[1];
  _n[3] = t.i[0];

  return t.i[1];
}

inline bigintRes add_one_to_bigint8(const uint *n, uint *out)
{
  bigintRes res;
  ul_ui_union t;
  t.l = (ulong)n[0] + 1ul;
  out[0] = t.i[0];

  t.l = (ulong)n[1] + (ulong)t.i[1];
  out[1] = t.i[0];
  t.l = (ulong)n[2] + (ulong)t.i[1];
  out[2] = t.i[0];
  t.l = (ulong)n[3] + (ulong)t.i[1];
  out[3] = t.i[0];

  res.carry4 = t.i[1];
  t.l = (ulong)n[4] + (ulong)t.i[1];
  out[4] = t.i[0];
  t.l = (ulong)n[5] + (ulong)t.i[1];
  out[5] = t.i[0];
  t.l = (ulong)n[6] + (ulong)t.i[1];
  out[6] = t.i[0];
  t.l = (ulong)n[7] + (ulong)t.i[1];
  out[7] = t.i[0];
  res.carry = t.i[1];
  return res;
}

inline bigintRes add_one_to_bigint8_(uint *_n)
{
  bigintRes res;
  ul_ui_union t;
  t.l = (ulong)_n[0] + 1ul;
  _n[0] = t.i[0];

  t.l = (ulong)_n[1] + (ulong)t.i[1];
  _n[1] = t.i[0];
  t.l = (ulong)_n[2] + (ulong)t.i[1];
  _n[2] = t.i[0];
  t.l = (ulong)_n[3] + (ulong)t.i[1];
  _n[3] = t.i[0];

  res.carry4 = t.i[1];
  t.l = (ulong)_n[4] + (ulong)t.i[1];
  _n[4] = t.i[0];
  t.l = (ulong)_n[5] + (ulong)t.i[1];
  _n[5] = t.i[0];
  t.l = (ulong)_n[6] + (ulong)t.i[1];
  _n[6] = t.i[0];
  t.l = (ulong)_n[7] + (ulong)t.i[1];
  _n[7] = t.i[0];

  res.carry = t.i[1];
  return res;
}


inline bigintRes add_uint_to_bigint8 (const uint *n, const uint b, uint *out)
{
  bigintRes res;
  ul_ui_union t;
  t.l = (ulong)n[0] + (ulong)b;
  out[0] = t.i[0];

  t.l = (ulong)n[1] + (ulong)t.i[1];
  out[1] = t.i[0];
  t.l = (ulong)n[2] + (ulong)t.i[1];
  out[2] = t.i[0];
  t.l = (ulong)n[3] + (ulong)t.i[1];
  out[3] = t.i[0];

  res.carry4 = t.i[1];
  t.l = (ulong)n[4] + (ulong)t.i[1];
  out[4] = t.i[0];
  t.l = (ulong)n[5] + (ulong)t.i[1];
  out[5] = t.i[0];
  t.l = (ulong)n[6] + (ulong)t.i[1];
  out[6] = t.i[0];
  t.l = (ulong)n[7] + (ulong)t.i[1];
  out[7] = t.i[0];

  res.carry = t.i[1];
  return res;
}

inline bigintRes add_uint_to_bigint8_ (uint* _n, const uint b)
{
  bigintRes res;
  ul_ui_union t;
  t.l = (ulong)_n[0] + (ulong)b;
  _n[0] = t.i[0];

  t.l = (ulong)_n[1] + (ulong)t.i[1];
  _n[1] = t.i[0];
  t.l = (ulong)_n[2] + (ulong)t.i[1];
  _n[2] = t.i[0];
  t.l = (ulong)_n[3] + (ulong)t.i[1];
  _n[3] = t.i[0];

  res.carry4 = t.i[1];
  t.l = (ulong)_n[4] + (ulong)t.i[1];
  _n[4] = t.i[0];
  t.l = (ulong)_n[5] + (ulong)t.i[1];
  _n[5] = t.i[0];
  t.l = (ulong)_n[6] + (ulong)t.i[1];
  _n[6] = t.i[0];
  t.l = (ulong)_n[7] + (ulong)t.i[1];
  _n[7] = t.i[0];

  res.carry = t.i[1];
  return res;
}
