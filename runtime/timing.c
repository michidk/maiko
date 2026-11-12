#include "timing.h"
#include <windows.h>
volatile uint32_t rg_bios_ticks;
static HANDLE thread,stop;
static LARGE_INTEGER frequency,began;
uint32_t rg_clock_convert(uint64_t elapsed,uint64_t rate) {
    if (!rate || rate>1000000000000ULL) return 0;
    uint64_t seconds=elapsed/rate,remainder=elapsed%rate;
    return (uint32_t)((seconds*1193182+remainder*1193182/rate)/65536);
}
static DWORD WINAPI update_clock(void *unused) {
    (void)unused;
    do {
        LARGE_INTEGER now;
        if (!QueryPerformanceCounter(&now)) return 1;
        InterlockedExchange((volatile LONG*)&rg_bios_ticks,(LONG)rg_clock_convert(now.QuadPart-began.QuadPart,frequency.QuadPart));
    } while (WaitForSingleObject(stop,1)==WAIT_TIMEOUT);
    return 0;
}
int rg_clock_start(void) {
    if (thread) return 1;
    if (!QueryPerformanceFrequency(&frequency) || frequency.QuadPart<=0 || frequency.QuadPart>1000000000000LL || !QueryPerformanceCounter(&began)) return 0;
    stop=CreateEvent(NULL,TRUE,FALSE,NULL);
    if (!stop) return 0;
    rg_bios_ticks=0;
    thread=CreateThread(NULL,0,update_clock,NULL,0,NULL);
    if (!thread) {CloseHandle(stop);stop=NULL;return 0;}
    return 1;
}
void rg_clock_stop(void) {
    if (!thread) return;
    SetEvent(stop);
    WaitForSingleObject(thread,INFINITE);
    CloseHandle(thread); CloseHandle(stop);
    thread=stop=NULL;
}

uint32_t rg_clock_milliseconds(void) {
    LARGE_INTEGER now;
    if(frequency.QuadPart<=0 || !QueryPerformanceCounter(&now)) return 0;
    uint64_t elapsed=now.QuadPart-began.QuadPart,rate=frequency.QuadPart;
    return (uint32_t)((elapsed/rate)*1000+(elapsed%rate)*1000/rate);
}
