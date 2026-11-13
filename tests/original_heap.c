#include <assert.h>
#include <stdio.h>
#include "crash.h"
#ifdef _WIN32
#include <windows.h>
#endif
#include "guest_abi.h"
extern void host_msize(void);
#ifdef _WIN32
static LONG CALLBACK fault(EXCEPTION_POINTERS *info) {
    fprintf(stderr,"FAIL original heap-capacity query exception=%08lx\n",
            info->ExceptionRecord->ExceptionCode);
    ExitProcess(3);
    return EXCEPTION_EXECUTE_HANDLER;
}
#endif
int main(void) {
#ifdef _WIN32
    AddVectoredExceptionHandler(1,fault);
#else
    rg_crash_install();
#endif
    GuestRegs r={0};
    void *stack=rg_low_alloc(65536), *block=rg_low_alloc(50000);
    assert(stack && block);
    r.esp=(uint32_t)(uintptr_t)stack+65536;
    r.eax=(uint32_t)(uintptr_t)block;
    r.edx=0x12345678; r.ebx=0x76543210;
    rg_call((uint32_t)(uintptr_t)host_msize,&r);
    assert(r.esp==(uint32_t)(uintptr_t)stack+65536);
    assert(r.eax>=50000 && r.eax<50000+4096);
    assert(r.edx==0x12345678 && r.ebx==0x76543210);
    ((volatile unsigned char *)block)[r.eax-1]=0x5a;
    assert(((volatile unsigned char *)block)[r.eax-1]==0x5a);
    assert(rg_low_free(block)); assert(rg_low_free(stack));
    puts("PASS original heap capacity boundary matches accessible native allocation and preserves registers");
}
