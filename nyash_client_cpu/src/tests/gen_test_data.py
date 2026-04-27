#!/usr/bin/python
import random


def main(iters: int):
    for i in range(iters):
        rand_u32 = random.randint(1,0xffffffff)
        t0 = int.from_bytes(random.randbytes(32))
        t1 = t0 + 1
        t2 = t0 + rand_u32
        print(f"{rand_u32} {t0.to_bytes(32).hex()} {t1.to_bytes(32).hex()} {t2.to_bytes(32).hex()}")

if __name__ == '__main__':
    main(1000000)







