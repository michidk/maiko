#include "guest_abi.h"
#include <assert.h>
#include <stdint.h>
#include <stdio.h>
#include <string.h>
extern void abi_guest(void),abi_guest_stdcall(void);
void rg_service(uint32_t service,GuestFrame *frame) {
    assert(service==42 || service==44);
    if(service==44) {
        uint32_t *stack=(void *)(uintptr_t)(uint32_t)frame->esp;
        assert(stack[1]==0x12345678 && stack[2]==0xabcdef01);
    }
    assert(frame->ebx==0x10203040 && frame->ecx==0x50607080);
    assert(frame->edx==0x55667788);
    /* A real host callback may change both x87 and SSE state. */
    __asm__ volatile("fninit");
    frame->eax=0xabcdef01;
}
int main(void) {
    void *memory=rg_low_alloc(65536); assert(memory);
    uint32_t address=0;
    assert(rg_guest_pointer(memory,65536,&address));
    assert(!rg_guest_pointer((void *)(uintptr_t)0x100000000ULL,1,&address));
    assert(!rg_guest_pointer((void *)(uintptr_t)0xfffffff0ULL,32,&address));
    GuestRegs r={0}; r.esp=(uint32_t)(uintptr_t)memory+65536;
    r.ebx=0x10203040; r.ecx=0x50607080; r.edx=0x55667788;
    unsigned short before,after;
    __asm__ volatile("fnstcw %0":"=m"(before));
    rg_call((uint32_t)(uintptr_t)abi_guest,&r);
    __asm__ volatile("fnstcw %0":"=m"(after));
    assert(before==after && r.eax==0xabcdef01);
    assert(r.esp==(uint32_t)(uintptr_t)memory+65536);
    assert(r.ebx==0x10203040 && r.ecx==0x50607080 && r.edx==0x55667788);
    rg_call((uint32_t)(uintptr_t)abi_guest_stdcall,&r);
    assert(r.eax==0xabcdef01 && r.esp==(uint32_t)(uintptr_t)memory+65536);
    puts("PASS Linux System V entry, native callback, guest registers, x87 and stdcall stack cleanup");
    return 0;
}
