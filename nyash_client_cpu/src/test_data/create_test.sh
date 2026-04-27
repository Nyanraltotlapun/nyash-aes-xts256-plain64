#!/bin/bash

truncate -s 255M test_btrfs.img
dd if=/dev/urandom of=master.key bs=32 count=1

cryptsetup luksFormat --type=luks2 --sector-size 512 --pbkdf=pbkdf2 --pbkdf-force-iterations=1000 --hash=sha256 --key-size=256 --cipher=aes-xts-plain64 --master-key-file ./test_master.key ./test.img

cryptsetup luksFormat --type=luks2 --pbkdf=pbkdf2 --pbkdf-force-iterations=1000 --hash=sha256 --key-size=256 --cipher=aes-xts-plain64 ./luks-container.img

sudo cryptsetup luksOpen ./test.img luks-container-crypt

sudo mkfs.btrfs /dev/mapper/luks-container-crypt

sudo dd if=/dev/mapper/luks-container-crypt of=./test_btrfs_luks_unencrypt.img bs=1M count=255

sudo cryptsetup close luks-container-crypt
