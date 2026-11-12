#ifndef RG_TIMING_H
#define RG_TIMING_H
#include <stdint.h>
#include <stdatomic.h>
extern _Atomic(uint32_t) rg_bios_ticks;
uint32_t rg_clock_convert(uint64_t elapsed,uint64_t frequency);
uint32_t rg_clock_milliseconds(void);
int rg_clock_start(void);
void rg_clock_stop(void);
#endif
