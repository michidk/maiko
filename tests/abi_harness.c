#include <assert.h>
#include <stdint.h>
#include <stdio.h>
#include <string.h>
#include <windows.h>
#include "guest_abi.h"
extern void abi_guest(void), abi_guest_unwind(void), rg_guest_return(void);
extern void abi_host_roundtrip(void), abi_guest_stdcall(void);
void rg_service(uint32_t service, GuestFrame *f) {
    if (service == 44) {
        uint32_t *stack=(uint32_t*)(uintptr_t)(uint32_t)f->esp;
        assert(stack[1]==0x12345678 && stack[2]==0xabcdef01);
        f->eax=0x13579bdf; return;
    }
    if (service == 43) {
        CONTEXT c = {0}; DWORD64 base, frame; PVOID handler;
        c.Rip = (uintptr_t)abi_guest_unwind;
        c.Rsp = f->edx; c.R13 = f->eax;
        PRUNTIME_FUNCTION entry = RtlLookupFunctionEntry(c.Rip, &base, NULL);
        assert(entry);
        RtlVirtualUnwind(UNW_FLAG_NHANDLER, base, c.Rip, entry, &c, &handler, &frame, NULL);
        assert(c.Rip == (uintptr_t)rg_guest_return);
        DWORD64 section_rsp=c.Rsp;
        entry=RtlLookupFunctionEntry(c.Rip,&base,NULL); assert(entry);
        RtlVirtualUnwind(UNW_FLAG_NHANDLER,base,c.Rip,entry,&c,&handler,&frame,NULL);
        assert(c.Rsp==section_rsp+552+8*8+8);
        assert(c.Rip && c.Rip!=(uintptr_t)rg_guest_return);
        return;
    }
    assert(service == 42);
    assert((uint32_t)f->eax == 0x11223344);
    assert((uint32_t)f->edx == 0x55667788);
    f->eax = 0xabcdef01;
}
int main(void) {
    GuestRegs r = {0};
    void *stack = rg_low_alloc(65536);
    assert(stack);
    uint32_t address;
    assert(rg_guest_pointer(stack, 65536, &address));
    assert(!rg_guest_pointer((void *)(uintptr_t)0x100000000ULL, 1, &address));
    assert(!rg_guest_pointer((void *)(uintptr_t)0xfffffff0ULL, 32, &address));
    r.esp = (uint32_t)(uintptr_t)stack + 65536;
    r.eax = 0x11223344; r.edx = 0x55667788;
    r.ebx = 0x10203040; r.ecx = 0x50607080;
    uint32_t initial_stack = r.esp;
    rg_call((uint32_t)(uintptr_t)abi_guest, &r);
    assert(r.eax == 0xabcdef01);
    assert(r.ebx == 0x10203040 && r.ecx == 0x50607080);
    assert(r.edx == 0x55667788 && r.esp == initial_stack);
    puts("PASS guest register arguments, native callback, return, stack and pointer bounds");
    rg_call((uint32_t)(uintptr_t)abi_guest_unwind, &r);
    puts("PASS translated-section Windows unwind frame");
    rg_call((uint32_t)(uintptr_t)abi_guest_stdcall,&r);
    assert(r.eax==0x13579bdf && r.esp==initial_stack);
    puts("PASS guest stdcall arguments and callee stack cleanup");
    assert(rg_low_free(stack));
    return 0;
}
