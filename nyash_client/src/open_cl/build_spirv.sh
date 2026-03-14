#!/bin/sh

out_file="$1"

clang -c -target spir64 -O0 -finclude-default-header -I ./ -emit-llvm -o nyash_aes_xts256_plain.bc nyash_aes_xts256_plain.cl

#llc -march=spir64 nyash_aes_xts256_plain.bc -filetype=obj -o $out_file
llvm-spirv -o $out_file nyash_aes_xts256_plain.bc