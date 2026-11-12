#include "timing.h"
#include <SDL.h>
volatile uint32_t rg_bios_ticks;
static SDL_Thread *thread;
static SDL_atomic_t stopping;
static uint64_t began,frequency;
uint32_t rg_clock_convert(uint64_t elapsed,uint64_t rate) {
    if(!rate || rate>1000000000000ULL) return 0;
    return (uint32_t)(((elapsed/rate)*1193182+(elapsed%rate)*1193182/rate)/65536);
}
static int update(void *unused) {
    (void)unused;
    while(!SDL_AtomicGet(&stopping)) {
        uint32_t tick=rg_clock_convert(SDL_GetPerformanceCounter()-began,frequency);
        /* Aligned x86 stores match the original single timer-writer contract. */
        __atomic_store_n(&rg_bios_ticks,tick,__ATOMIC_RELAXED);
        SDL_Delay(1);
    }
    return 0;
}
int rg_clock_start(void) {
    if(thread) return 1;
    frequency=SDL_GetPerformanceFrequency();
    if(!frequency || frequency>1000000000000ULL) return 0;
    began=SDL_GetPerformanceCounter();rg_bios_ticks=0;SDL_AtomicSet(&stopping,0);
    thread=SDL_CreateThread(update,"Redguard BIOS clock",NULL);return thread!=NULL;
}
void rg_clock_stop(void) {
    if(!thread) return;
    SDL_AtomicSet(&stopping,1);SDL_WaitThread(thread,NULL);thread=NULL;
}

uint32_t rg_clock_milliseconds(void) {
    if(!frequency) return 0;
    uint64_t elapsed=SDL_GetPerformanceCounter()-began;
    return (uint32_t)((elapsed/frequency)*1000+(elapsed%frequency)*1000/frequency);
}
