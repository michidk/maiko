#include <assert.h>
#include <stdint.h>
#include <stdio.h>
#include <string.h>
#include "guest_abi.h"
#include "crash.h"
extern void rg_rob_vertex_decode(void);
extern uint32_t rg_rob_vertex_type,rg_rob_vertex_x,rg_rob_vertex_y,rg_rob_vertex_z,rg_rob_vertex_extra;
int main(void) {
    setbuf(stdout,NULL);
    rg_crash_install();
    unsigned char *memory=rg_low_alloc(65536);assert(memory);
    /* Preserve the original bit-fill patterns exactly. Its 20-bit path ORs
       0xff000000, while its packed 6-bit path ORs 0x0fffffc0. Callers may
       transform these values further; this test covers this boundary only. */
    const struct {uint32_t packed,x,y,z,extra;} cases[]={
        {5,0,0,0,0},
        {0xfffef1,0xff0fffef,0,0,0},
        {0xfeeff0,0xfffffeff,0xfffffffb,0,0},
        {0x613776,0x0ffffff7,13,0x0fffffe1,0},
        {0x123452,0x12345,0,0,0}
    };
    for(unsigned i=0;i<sizeof cases/sizeof cases[0];++i) {
        uint32_t packed=cases[i].packed;memcpy(memory,&packed,4);
        rg_rob_vertex_type=rg_rob_vertex_x=rg_rob_vertex_y=rg_rob_vertex_z=rg_rob_vertex_extra=0xcccccccc;
        GuestRegs r={0};r.eax=(uint32_t)(uintptr_t)memory;r.esp=r.eax+65536;
        r.ebx=0x12345678;r.ecx=0x23456789;r.edx=0x3456789a;
        r.esi=0x456789ab;r.edi=0x56789abc;r.ebp=0x6789abcd;
        rg_call((uint32_t)(uintptr_t)rg_rob_vertex_decode,&r);
        printf("ROB vertex[%u] type=%u xyz=%08x/%08x/%08x extra=%08x\n",i,r.eax,rg_rob_vertex_x,rg_rob_vertex_y,rg_rob_vertex_z,rg_rob_vertex_extra);
        assert(r.eax==(packed&15) && rg_rob_vertex_type==r.eax);
        assert(rg_rob_vertex_x==cases[i].x && rg_rob_vertex_y==cases[i].y);
        assert(rg_rob_vertex_z==cases[i].z && rg_rob_vertex_extra==cases[i].extra);
        assert(r.esp==(uint32_t)(uintptr_t)memory+65536);
        assert(r.ebx==0x12345678 && r.ecx==0x23456789 && r.edx==0x3456789a);
        assert(r.esi==0x456789ab && r.edi==0x56789abc && r.ebp==0x6789abcd);
    }
    assert(rg_low_free(memory));
    puts("PASS original ROB packed vertex decoder, switch targets, output globals and preserved registers");
}
