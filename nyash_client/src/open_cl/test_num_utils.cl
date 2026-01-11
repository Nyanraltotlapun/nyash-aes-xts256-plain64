#include "num_utils.cl"


__kernel void test_add(__global const uint* g_num_to_add,
                           __global const uint8* g_t0,
                           __global uint* g_t1,
                           __global uint* g_t2)
{
    const size_t g_id = get_global_id(0); 
    uint t0[8] = {0};
    uint t1[8] = {0};
    uint t2[8] = {0};
    uint num_to_add = g_num_to_add[g_id];

    vstore8(g_t0[g_id], 0, t0);
    add_one_to_bigint8 (t0, t1);
    add_uint_to_bigint8 (t0, num_to_add, t2);
    
    // save results
    vstore8(*(uint8*)t1, 0, &g_t1[g_id*8]);
    vstore8(*(uint8*)t2, 0, &g_t2[g_id*8]);

}




