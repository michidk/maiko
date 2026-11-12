#include "guest_abi.h"
#include <assert.h>
#include <stdio.h>
int main(void) {
    void *live[9000];
    for(unsigned i=0;i<9000;++i) {
        live[i]=rg_low_alloc(64);assert(live[i]);
        assert((uintptr_t)live[i]+rg_low_size(live[i])<=0x80000000ULL);
        *(unsigned *)live[i]=i;
    }
    for(unsigned i=0;i<30000;++i) {
        unsigned char *p=rg_low_alloc(80);assert(p);
        assert((uintptr_t)p+rg_low_size(p)<=0x80000000ULL);
        p[rg_low_size(p)-1]=0x5a;assert(rg_low_free(p));
        assert(!rg_low_size(p) && !rg_low_free(p));
    }
    for(unsigned i=0;i<9000;++i) {assert(*(unsigned *)live[i]==i);assert(rg_low_free(live[i]));}
    puts("PASS 9000 retained blocks and30000 transient allocations preserve data and signed32 pointer range; invalid/double frees rejected");
}
