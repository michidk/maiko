#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <windows.h>

uint32_t X86_InterruptFlag;
extern void rg_run(void);

static LONG CALLBACK fault(EXCEPTION_POINTERS *info) {
    fprintf(stderr, "BLOCKED exception=%08lx host_pc=%p; menu not reached\n",
            info->ExceptionRecord->ExceptionCode, info->ExceptionRecord->ExceptionAddress);
    fflush(stderr);
    ExitProcess(3);
    return EXCEPTION_EXECUTE_HANDLER;
}

__declspec(noreturn) void probe_interrupt(uint32_t number, uint32_t eax, uint32_t return_pc) {
    fprintf(stderr, "BLOCKED original program requested INT %02x EAX=%08x translated_return=%08x\n",
            number, eax, return_pc);
    fputs("DOS runtime is not implemented. This is an execution-boundary probe, not a playable port.\n", stderr);
    fflush(stderr);
    ExitProcess(2);
}

__declspec(noreturn) void probe_boundary(void) {
    fputs("BLOCKED unimplemented SR runtime helper; menu not reached\n", stderr);
    fflush(stderr);
    ExitProcess(2);
}

int main(void) {
    setbuf(stdout, NULL);
    setbuf(stderr, NULL);
    AddVectoredExceptionHandler(1, fault);
    puts("Redguard SR diagnostic probe: entering statically translated RGFX entry point");
    rg_run();
    fputs("Unexpected return from DOS entry point\n", stderr);
    return 4;
}
