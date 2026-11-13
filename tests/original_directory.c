#include <assert.h>
#include <stdio.h>
#include "crash.h"
#ifdef _WIN32
#include <windows.h>
#endif
#include "guest_abi.h"
extern void rg_directory_next(void);
extern uint32_t rg_directory_stack, rg_directory_mode;
#ifdef _WIN32
static LONG CALLBACK fault(EXCEPTION_POINTERS *info) {
    fprintf(stderr,"FAIL original directory exhaustion exception=%08lx address=%llx\n",
            info->ExceptionRecord->ExceptionCode,
            (unsigned long long)info->ExceptionRecord->ExceptionInformation[1]);
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
    void *stack=rg_low_alloc(65536); assert(stack);
    rg_directory_mode=0;
    for (unsigned depth=0;depth<=2;++depth) {
        rg_directory_stack=0;
        for (unsigned i=0;i<depth;++i) {
            uint32_t *node=rg_low_alloc(0x8c); assert(node);
            node[0]=rg_directory_stack;
            /* Null directory handle represents an exhausted/failed subtree. */
            node[1]=0;
            rg_directory_stack=(uint32_t)(uintptr_t)node;
        }
        GuestRegs r={0};
        r.esp=(uint32_t)(uintptr_t)stack+65536;
        r.ebx=0x12345678; r.ecx=0x23456789; r.edx=0x3456789a;
        r.esi=0x456789ab; r.edi=0x56789abc; r.ebp=0x6789abcd;
        rg_call((uint32_t)(uintptr_t)rg_directory_next,&r);
        assert(!r.eax && !rg_directory_stack);
        assert(r.esp==(uint32_t)(uintptr_t)stack+65536);
        assert(r.ebx==0x12345678 && r.ecx==0x23456789 && r.edx==0x3456789a);
        assert(r.esi==0x456789ab && r.edi==0x56789abc && r.ebp==0x6789abcd);
        printf("PASS original directory exhaustion depth=%u and preserved registers\n",depth);
    }
    assert(rg_low_free(stack));
}
