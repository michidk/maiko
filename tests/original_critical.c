#include <assert.h>
#include <stdio.h>
#include "guest_abi.h"
/* DOS vector24 is game-local bookkeeping. Never a host interrupt handler. */
static GuestFrame frame(unsigned ax) {
    GuestFrame f={0};f.eax=ax;f.ebx=0x12345678;f.ecx=0x23456789;
    static uint32_t *stack;if(!stack) {stack=rg_low_alloc(4096);assert(stack);stack[0]=0;}
    f.esp=(uint32_t)(uintptr_t)stack;f.edx=0x34567890;f.esi=0x456789ab;f.edi=0x56789abc;f.flags=0x202;
    return f;
}
int main(void) {
    setbuf(stdout,NULL);
    GuestFrame f=frame(0x3524);rg_service(0x1021,&f);
    assert(f.ebx==0 && f.eax==0x3524 && f.edx==0x34567890 && f.flags==0x202);
    f=frame(0x01234567);rg_service(19,&f);assert(f.eax==0);
    f=frame(0x3524);rg_service(0x1021,&f);assert(f.ebx==0x01234567);
    f=frame(0x2524);f.edx=0x07654321;rg_service(0x1021,&f);
    assert(f.eax==0x2524 && f.ebx==0x12345678 && f.flags==0x202);
    f=frame(0x3524);rg_service(0x1021,&f);assert(f.ebx==0x07654321);
    f=frame(0x2524);f.edx=0;rg_service(0x1021,&f);
    f=frame(0x3524);rg_service(0x1021,&f);assert(!f.ebx);
    f=frame(0x3600);f.edx=0;rg_service(0x1021,&f);
    assert(f.eax && f.eax<=65535 && f.ecx==512 && f.ebx<=f.edx && f.edx<=65535);
    assert(f.flags==0x202);
    f=frame(0x3600);f.edx=255;rg_service(0x1021,&f);assert(f.eax==65535);
    puts("PASS native critical-file handler save/install/restore with preserved DOS registers and flags");
}
