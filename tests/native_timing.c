#include <assert.h>
#include <stdio.h>
#ifdef _WIN32
#include <windows.h>
#else
#include <unistd.h>
#define Sleep(ms) usleep((ms)*1000)
#endif
#include "timing.h"
int main(void) {
    assert(rg_clock_convert(0,1193182)==0);
    assert(rg_clock_convert(65535,1193182)==0);
    assert(rg_clock_convert(65536,1193182)==1);
    assert(rg_clock_convert(1193182,1193182)==18);
    assert(rg_clock_convert(3600ULL*10000000,10000000)==65543);
    /* Avoid overflow of elapsed*PIT_frequency after long host uptimes. */
    assert(rg_clock_convert(30ULL*86400*10000000,10000000)==47191280);
    assert(rg_clock_start());
    uint32_t first=rg_bios_ticks;
    Sleep(160);
    uint32_t next=rg_bios_ticks;
    assert(next>first && next-first<1000);
    rg_clock_stop();
    next=rg_bios_ticks;
    Sleep(70); assert(rg_bios_ticks==next);
    assert(rg_clock_start());
    rg_clock_stop();
    puts("PASS native BIOS-rate tick conversion, live updates, stop and restart");
}
