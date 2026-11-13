#include <assert.h>
#include <stdint.h>
#include <stdio.h>
#include <string.h>
#include <SDL.h>
#include "guest_abi.h"
#include "audio.h"
extern void rg_original_audio_startup(void),rg_original_cd_open(void);
extern void rg_original_audio_init(void),rg_original_audio_close(void),rg_original_sample_start(void);
extern void rg_original_sample_status(void),rg_original_sample_end(void),rg_original_sample_volume(void);
extern void rg_original_sample_set_user(void),rg_original_sample_get_user(void);
extern uint32_t rg_sound_open,rg_sound_handles[16];
static uint32_t stack;
static uint32_t call(void (*fn)(void),uint32_t ax,uint32_t dx,uint32_t bx,uint32_t cx) {
    GuestRegs r={0};r.esp=stack;r.eax=ax;r.edx=dx;r.ebx=bx;r.ecx=cx;
    r.ebp=0x12345678;r.esi=0x23456789;r.edi=0x34567890;
    rg_call((uint32_t)(uintptr_t)fn,&r);
    assert(r.esp==stack && r.ebp==0x12345678 && r.esi==0x23456789 && r.edi==0x34567890);
    return r.eax;
}
static void word(unsigned char *p,unsigned offset,uint32_t value) {memcpy(p+offset,&value,4);}
int main(void) {
    setbuf(stdout,NULL);unsigned char *memory=rg_low_alloc(65536);assert(memory);stack=(uint32_t)(uintptr_t)memory+65536;
    assert(call(rg_original_audio_startup,0,0,0,0)==1);
    assert(call(rg_original_audio_startup,0,0,0,0)==0);
    call(rg_original_audio_init,0,0,0,0);assert(rg_sound_open && rg_sound_handles[15]);
    assert(call(rg_original_cd_open,0,0,0,0)==0);
    call(rg_original_audio_init,0,0,0,0);assert(rg_sound_open && rg_sound_handles[15]);
    word(memory,0,1);word(memory,4,1);word(memory,8,22050);memory[12]=64;
    word(memory,22,8192);word(memory,26,(uint32_t)(uintptr_t)(memory+1024));
    int16_t *pcm=(void *)(memory+1024);for(unsigned i=0;i<4096;++i) pcm[i]=10000;
    uint32_t handle=call(rg_original_sample_start,(uint32_t)(uintptr_t)memory,0,63,0);assert(handle<16);
    assert(call(rg_original_sample_status,handle,0,0,0)==4);SDL_Delay(350);
    assert(call(rg_original_sample_status,handle,0,0,0)==2);
    RgAudioStats stats;rg_audio_stats(&stats);assert(stats.started==1 && stats.nonzero_frames>0);
    memory[13]=255;word(memory,14,0);word(memory,18,UINT32_MAX);
    handle=call(rg_original_sample_start,(uint32_t)(uintptr_t)memory,0,63,127);assert(handle<16);
    call(rg_original_sample_set_user,handle,3,0xabcdef,0);assert(call(rg_original_sample_get_user,handle,3,0,0)==0xabcdef);
    SDL_Delay(350);assert(call(rg_original_sample_status,handle,0,0,0)==4);
    call(rg_original_sample_volume,handle,0,0,0);SDL_Delay(60);
    rg_audio_stats(&stats);uint64_t silent=stats.nonzero_frames;SDL_Delay(80);rg_audio_stats(&stats);assert(stats.nonzero_frames==silent);
    call(rg_original_sample_end,handle,0,0,0);assert(call(rg_original_sample_status,handle,0,0,0)==2);
    call(rg_original_audio_close,0,0,0,0);assert(!rg_sound_open);
    assert(call(rg_original_audio_startup,0,0,0,0)==1);
    assert(rg_low_free(memory));
    puts("PASS original audio initialization/restart, sample-start/status/end, cdecl API, real mixed output, finite/infinite loops, live volume, user data and preserved registers");
}
