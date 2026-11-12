#include <assert.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <windows.h>
#include "guest_abi.h"
#include "crash.h"
extern void rg_unresolved_far(void);
static DWORD WINAPI translated_fault(void *unused) {
    (void)unused;
    GuestRegs r={0};
    void *stack=rg_low_alloc(65536); assert(stack);
    r.esp=(uint32_t)(uintptr_t)stack+65536;
    DWORD64 base;
    PRUNTIME_FUNCTION entry=RtlLookupFunctionEntry((DWORD64)(uintptr_t)rg_unresolved_far,&base,NULL);
    assert(entry);
    assert(base+entry->BeginAddress <= (DWORD64)(uintptr_t)rg_unresolved_far);
    assert(base+entry->EndAddress > (DWORD64)(uintptr_t)rg_unresolved_far);
    puts("PASS original translated trap has covering Windows unwind entry");
    fflush(stdout);
    rg_call((uint32_t)(uintptr_t)rg_unresolved_far,&r);
    return 99;
}
static void cleanup(void) {puts("PASS native DOS exit runs host cleanup");}
int main(int argc,char **argv) {
    assert(argc==2);
    rg_crash_install();
    if (!strcmp(argv[1],"--exit")) {
        GuestFrame f={0};f.eax=0x4c07;
        int result=atexit(cleanup);assert(!result);
        rg_service(0x1021,&f);return 99;
    }
    if (!strcmp(argv[1],"--main")) return (int)translated_fault(NULL);
    assert(!strcmp(argv[1],"--worker"));
    HANDLE thread=CreateThread(NULL,0,translated_fault,NULL,0,NULL); assert(thread);
    WaitForSingleObject(thread,INFINITE);
    return 99;
}
