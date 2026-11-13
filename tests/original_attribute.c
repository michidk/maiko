#include <assert.h>
#include <stdint.h>
#include <stdio.h>
#include <string.h>
#include "guest_abi.h"
#include "guest_address.h"
#include "crash.h"
extern void rg_set_attribute(void);
static void call(unsigned char *memory,uint32_t actor,unsigned attribute,unsigned value) {
    GuestRegs r={0};r.eax=actor;r.edx=attribute;r.ebx=value;r.esp=RG_ADDRESS(memory)+65536;
    r.ecx=0x23456789;r.esi=0x456789ab;r.edi=0x56789abc;r.ebp=0x6789abcd;
    rg_call(RG_ADDRESS(rg_set_attribute),&r);
    assert(!r.eax && r.ebx==value && r.ecx==0x23456789);
    assert(r.esi==0x456789ab && r.edi==0x56789abc && r.ebp==0x6789abcd);
    assert(r.esp==RG_ADDRESS(memory)+65536);
}
int main(void) {
    setbuf(stdout,NULL);rg_crash_install();
    unsigned char *memory=rg_low_alloc(65536);assert(memory);memset(memory,0,8192);
    uint32_t attrs=RG_ADDRESS(memory+4096),actor=RG_ADDRESS(memory);
    memcpy(memory+0x272,&attrs,4);
    call(memory,actor,0x75,0x1ff);assert(memory[4096+0x75]==255);
    call(memory,actor,6,37);assert(memory[4096+6]==37 && memory[4096+115]==37);
    call(memory,actor,1,18);assert(memory[4096+1]==18 && memory[4096+119]==18);
    memset(memory+0x272,0,4);call(memory,actor,0x75,42);assert(memory[4096+0x75]==255);
    puts("PASS original actor attribute writes, mirrored fields and absent attribute table");
    puts("Checking startup New Game before a player exists");call(memory,0,0x75,0);
    assert(rg_low_free(memory));
    puts("PASS original attribute setter preserves registers and treats missing startup player as DOS absent table");
}
