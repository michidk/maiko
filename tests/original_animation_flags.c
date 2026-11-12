#include <assert.h>
#include <stdint.h>
#include <stdio.h>
#include <string.h>
#include "guest_abi.h"
#include "crash.h"
extern void rg_animation_flags(void);
extern uint32_t rg_tick_delta;
int main(void) {
    setbuf(stdout,NULL);rg_crash_install();
    unsigned char *memory=rg_low_alloc(65536);assert(memory);
    const struct {uint16_t type;int missing;unsigned expected;} cases[]={
        {0x12,0,7},{0x1e,0,7},{0x20,0,4},{0,1,4}
    };
    for(unsigned i=0;i<sizeof cases/sizeof cases[0];++i) {
        memset(memory,0,8192);
        uint32_t animation=cases[i].missing ? 0 : (uint32_t)(uintptr_t)(memory+4096);
        memcpy(memory+0x247,&animation,4);memcpy(memory+4096,&cases[i].type,2);
        memory[0xc1]=1;memory[0x1b6]=7;memory[0x1ba]=4;memory[0x1be]=3;rg_tick_delta=2;
        GuestRegs r={0};r.eax=(uint32_t)(uintptr_t)memory;r.esp=r.eax+65536;
        r.ebx=0x12345678;r.ecx=0x23456789;r.edx=0x3456789a;
        r.esi=0x456789ab;r.edi=0x56789abc;r.ebp=0x6789abcd;
        printf("Original animation flags[%u] type=%x missing=%d\n",i,cases[i].type,cases[i].missing);
        rg_call((uint32_t)(uintptr_t)rg_animation_flags,&r);
        assert(memory[0x1b6]==cases[i].expected && memory[0x1ba]==4 && memory[0x1be]==5);
        assert(r.esp==(uint32_t)(uintptr_t)memory+65536);
        assert(r.ebx==0x12345678 && r.ecx==0x23456789 && r.edx==0x3456789a);
        assert(r.esi==0x456789ab && r.edi==0x56789abc && r.ebp==0x6789abcd);
    }
    assert(rg_low_free(memory));
    puts("PASS original animation flags for special/generic types and DOS static-object null animation, timer and preserved registers");
}
