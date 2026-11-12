#include <assert.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include "guest_abi.h"
#include "input.h"
#include "timing.h"
void rg_service(uint32_t service,GuestFrame *frame);
static void clean(void) {puts("PASS native close ran exit cleanup");}
int main(void) {
    setbuf(stdout,NULL);assert(atexit(clean)==0);assert(rg_clock_start());atexit(rg_clock_stop);
    uint32_t *regs=rg_low_alloc(65536);assert(regs);regs[0]=0xb;
    HWND window=rg_input_window(640,480);assert(window);
    assert(PostMessageA(window,WM_CLOSE,0,0));
    GuestFrame f={0};f.eax=0x33;f.edx=f.ebx=(uint32_t)(uintptr_t)regs;
    f.esp=(uint32_t)(uintptr_t)regs+32768;
    rg_service(8,&f);
    fputs("FAIL close returned to original mouse code\n",stderr);return 1;
}
