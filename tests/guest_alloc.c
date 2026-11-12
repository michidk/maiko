#include <assert.h>
#include <stdio.h>
#include <windows.h>
#include "guest_abi.h"
int main(void) {
    void *blocks[2048];
    LARGE_INTEGER frequency,start,end;
    assert(!rg_low_alloc(0) && !rg_low_alloc(0x10000001));
    assert(QueryPerformanceFrequency(&frequency) && QueryPerformanceCounter(&start));
    for (unsigned i=0;i<2048;++i) {
        blocks[i]=rg_low_alloc(1024);
        uint32_t pointer;
        assert(blocks[i] && rg_guest_pointer(blocks[i],1024,&pointer));
        *(unsigned *)blocks[i]=i;
        *((unsigned *)blocks[i]+255)=~i;
    }
    assert(QueryPerformanceCounter(&end));
    for (unsigned i=0;i<2048;++i) {
        assert(*(unsigned *)blocks[i]==i && *((unsigned *)blocks[i]+255)==~i);
        assert(rg_low_free(blocks[i]));
    }
    printf("PASS 2048 addressable allocations, nonoverlap, contents, release and size limits; allocation_seconds=%.6f\n",
           (double)(end.QuadPart-start.QuadPart)/frequency.QuadPart);
}
