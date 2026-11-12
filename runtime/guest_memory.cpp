/* Pinned SR heap implementation is copied into the build directory by the
   pipeline. Its MIT notice remains in sr_Game_memory.c and Game_memory.h. */
#include "sr_Game_memory.c"
#include "guest_abi.h"
#include <climits>
static struct {void *pointer;size_t size;} rg_allocations[49152];
static bool rg_heap_initialized;
extern "C" int rg_guest_pointer(const void *pointer,size_t size,uint32_t *out) {
    uintptr_t address=(uintptr_t)pointer;
    if(!address || address>UINT32_MAX || size>UINT32_MAX-address+(uint64_t)1) return 0;
    *out=(uint32_t)address;return 1;
}
extern "C" void *rg_low_alloc(size_t size) {
    if(!size || size>0x10000000) return NULL;
    if(!rg_heap_initialized) {
        if(x86_init_malloc()) return NULL;
        rg_heap_initialized=true;
    }
    unsigned slot=0;
    while(slot<49152 && rg_allocations[slot].pointer) ++slot;
    if(slot==49152) return NULL;
    void *p=x86_malloc((unsigned)size);
    if(!p) return NULL;
    memblock_t *block=(memblock_t *)((uint8_t *)p-sizeof(memblock_t));
    size_t capacity=(size_t)block->block_size-sizeof(memblock_t);
    if((uintptr_t)p+capacity>0x80000000ULL) {x86_free(p);return NULL;}
    memset(p,0,capacity);
    rg_allocations[slot].pointer=p;rg_allocations[slot].size=capacity;return p;
}
extern "C" int rg_low_free(void *pointer) {
    if(!pointer) return 1;
    for(unsigned i=0;i<49152;++i) if(rg_allocations[i].pointer==pointer) {
        x86_free(pointer);memset(&rg_allocations[i],0,sizeof rg_allocations[i]);return 1;
    }
    return 0;
}
extern "C" size_t rg_low_size(const void *pointer) {
    for(unsigned i=0;i<49152;++i) if(rg_allocations[i].pointer==pointer) return rg_allocations[i].size;
    return 0;
}
