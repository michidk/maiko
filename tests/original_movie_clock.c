#include <assert.h>
#include <stdint.h>
#include <stdio.h>
#ifdef _WIN32
#include <windows.h>
#else
#include <unistd.h>
#define Sleep(ms) usleep((ms)*1000)
#endif
#include "guest_abi.h"
#include "timing.h"
extern void rg_movie_clock_init(void);
extern uint32_t rg_movie_clock_callback,rg_movie_clock_shutdown;
static uint32_t call(uint32_t fn,uint32_t stack) {
    GuestRegs r={0};r.esp=stack;r.ebp=0x12345678;r.edi=0x23456789;r.esi=0x34567890;
    rg_call(fn,&r);
    assert(r.esp==stack && r.ebp==0x12345678 && r.edi==0x23456789 && r.esi==0x34567890);
    return r.eax;
}
int main(void) {
    setbuf(stdout,NULL);assert(rg_clock_start());
    unsigned char *memory=rg_low_alloc(65536);assert(memory);
    uint32_t stack=(uint32_t)(uintptr_t)memory+65536;
    call((uint32_t)(uintptr_t)rg_movie_clock_init,stack);
    assert(rg_movie_clock_callback && rg_movie_clock_shutdown);
    uint32_t first=call(rg_movie_clock_callback,stack);Sleep(160);
    uint32_t next=call(rg_movie_clock_callback,stack);
    assert(next-first>=100 && next-first<5000);
    call(rg_movie_clock_shutdown,stack);
    call((uint32_t)(uintptr_t)rg_movie_clock_init,stack);
    assert(call(rg_movie_clock_callback,stack)>=next);
    assert(rg_low_free(memory));rg_clock_stop();
    puts("PASS original movie timer dispatch returns elapsed milliseconds, advances, restarts and preserves callee registers");
}
