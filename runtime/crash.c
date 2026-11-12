#include "crash.h"
#include <stdint.h>
#include <stdio.h>
#include <windows.h>
volatile uint32_t rg_call_head, rg_call_ring[64];
static volatile LONG reporting;
void rg_dump_calls(void) {
    const uint32_t head=rg_call_head;
    const uint32_t count=head<32 ? head : 32;
    uint32_t saved[32];
    for (uint32_t i=0;i<count;++i) saved[i]=rg_call_ring[(head-count+i)&63];
    for (uint32_t i=0;i<count;++i) fprintf(stderr,"CALLSITE %08x\n",saved[i]);
}
static LONG CALLBACK fault(EXCEPTION_POINTERS *info) {
    if (InterlockedCompareExchange(&reporting,1,0)) {
        TerminateProcess(GetCurrentProcess(),3);
        return EXCEPTION_EXECUTE_HANDLER;
    }
    fprintf(stderr, "BLOCKED exception=%08lx host_pc=%p context_rip=%08llx guest_sp=%08llx\n",
            info->ExceptionRecord->ExceptionCode, info->ExceptionRecord->ExceptionAddress,
            (unsigned long long)info->ContextRecord->Rip,
            (unsigned long long)info->ContextRecord->R11);
    if ((info->ExceptionRecord->ExceptionCode==EXCEPTION_ACCESS_VIOLATION ||
         info->ExceptionRecord->ExceptionCode==EXCEPTION_IN_PAGE_ERROR) &&
        info->ExceptionRecord->NumberParameters >= 2)
        fprintf(stderr, "Memory operation=%llu address=%llx\n",
                (unsigned long long)info->ExceptionRecord->ExceptionInformation[0],
                (unsigned long long)info->ExceptionRecord->ExceptionInformation[1]);
    fprintf(stderr,"REGS eax=%08llx ebx=%08llx ecx=%08llx edx=%08llx esi=%08llx edi=%08llx ebp=%08llx r8=%08llx r9=%08llx\n",
            info->ContextRecord->Rax, info->ContextRecord->Rbx, info->ContextRecord->Rcx, info->ContextRecord->Rdx,
            info->ContextRecord->Rsi, info->ContextRecord->Rdi, info->ContextRecord->Rbp, info->ContextRecord->R8, info->ContextRecord->R9);
    fprintf(stderr,"FPU thread=%lu control=%04x status=%04x tag=%02x error=%08lx data=%08lx mxcsr=%08lx\n",
            GetCurrentThreadId(),info->ContextRecord->FltSave.ControlWord,
            info->ContextRecord->FltSave.StatusWord,info->ContextRecord->FltSave.TagWord,
            info->ContextRecord->FltSave.ErrorOffset,info->ContextRecord->FltSave.DataOffset,
            info->ContextRecord->MxCsr);
    rg_dump_calls();
    fflush(stderr);
    TerminateProcess(GetCurrentProcess(),3);
    return EXCEPTION_EXECUTE_HANDLER;
}
void rg_crash_install(void) {
    SetErrorMode(SEM_FAILCRITICALERRORS | SEM_NOOPENFILEERRORBOX | SEM_NOGPFAULTERRORBOX);
    SetUnhandledExceptionFilter(fault);
}
