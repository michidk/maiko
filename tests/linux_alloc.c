#include "guest_abi.h"
#include <assert.h>
#include <stdint.h>
#include <stdio.h>
int main(void) {
    void *blocks[9000];
    for(unsigned i=0;i<9000;++i) {
        blocks[i]=rg_low_alloc(20);
        if(!blocks[i]) {fprintf(stderr,"FAIL allocation %u of9000\n",i);return 1;}
        *(uint32_t *)blocks[i]=i;
        assert(rg_low_size(blocks[i])>=20 && rg_low_size(blocks[i])<4096);
    }
    for(unsigned i=0;i<9000;++i) {
        assert(*(uint32_t *)blocks[i]==i);assert(rg_low_free(blocks[i]));
    }
    puts("PASS 9000 simultaneous low-address allocations retain data, size and release");
}
