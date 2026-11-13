#include <assert.h>
#include <stdio.h>
#include <string.h>
#include "crash.h"
#ifdef _WIN32
#include <windows.h>
#endif
#include "guest_abi.h"
extern void rg_atoi(void);
#ifdef _WIN32
static LONG CALLBACK fault(EXCEPTION_POINTERS *info) {
    fprintf(stderr,"FAIL translated numeric conversion exception=%08lx\n",
            info->ExceptionRecord->ExceptionCode);
    ExitProcess(3);
    return EXCEPTION_EXECUTE_HANDLER;
}
#endif
static void conversion(GuestRegs *r, const char *text, uint32_t expected) {
    uint32_t sp=r->esp;
    r->eax=(uint32_t)(uintptr_t)text;
    r->ebx=0x12345678; r->ecx=0xabcdef01; r->edx=0x76543210;
    rg_call((uint32_t)(uintptr_t)rg_atoi,r);
    assert(r->eax==expected && r->esp==sp);
    assert(r->ebx==0x12345678 && r->ecx==0xabcdef01 && r->edx==0x76543210);
}
int main(void) {
#ifdef _WIN32
    AddVectoredExceptionHandler(1,fault);
#else
    rg_crash_install();
#endif
    GuestRegs r={0};
    char *memory=rg_low_alloc(65536); assert(memory);
    r.esp=(uint32_t)(uintptr_t)memory+65536;
    const struct { const char *text; uint32_t value; } cases[]={
        {"0",0},{"  \t-123x",(uint32_t)-123},{"+42",42},
        {"not a number",0},{"",0},{"4294967295",UINT32_MAX},
        {"4294967296",0},{"2147483648",0x80000000u}
    };
    for (unsigned i=0;i<sizeof cases/sizeof cases[0];++i) {
        strcpy(memory,cases[i].text); conversion(&r,memory,cases[i].value);
    }
    puts("PASS original numeric strings, prefix, signed/overflow behavior and preserved registers");
    puts("Checking absent optional INI value without mapping address zero");
    conversion(&r,NULL,0);
    puts("PASS native missing-value numeric policy");
    assert(rg_low_free(memory));
}
