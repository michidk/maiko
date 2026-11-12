#include <assert.h>
#include <stdio.h>
#include <string.h>
#include <SDL.h>
#include "audio.h"
static uint32_t *args;
static uint32_t call(unsigned op,uint32_t a,uint32_t b,uint32_t c,uint32_t d,uint32_t e) {
    args[0]=0;args[1]=a;args[2]=b;args[3]=c;args[4]=d;args[5]=e;
    GuestFrame f={0};f.esp=(uintptr_t)args;assert(rg_audio_service(op,&f));return f.eax;
}
int main(void) {
    setbuf(stdout,NULL);args=rg_low_alloc(65536);assert(args);
    uint32_t handles[16];assert(rg_audio_open(handles));
    uint32_t driver=call(19,0,0,0,0,0);assert(driver);
    assert(call(28,0,4096,0,0,0)==0);assert(call(29,0,4096,0,0,0)==0);
    GuestFrame invalid={0};invalid.esp=(uintptr_t)args;args[1]=0;assert(!rg_audio_service(22,&invalid));
    uint32_t stream=call(20,driver,0,0,0,0);assert(stream && stream!=handles[0]);
    call(2,stream,0,0,0,0);call(3,stream,1,1,0,0);call(5,stream,22050,0,0,0);
    call(6,stream,64,0,0,0);call(10,stream,127,0,0,0);
    int16_t *pcm=(void *)(args+32);for(unsigned i=0;i<4096;++i)pcm[i]=12000;
    assert(call(22,stream,0,0,0,0)==0);
    call(23,stream,0,(uintptr_t)pcm,8192,0);
    assert(call(22,stream,0,0,0,0)==1);
    call(23,stream,1,(uintptr_t)pcm,8192,0);
    assert(call(22,stream,0,0,0,0)==UINT32_MAX);
    assert(call(11,stream,0,0,0,0)==4);
    Uint32 deadline=SDL_GetTicks()+5000;uint32_t ready;
    while((ready=call(22,stream,0,0,0,0))==UINT32_MAX && SDL_GetTicks()<deadline)SDL_Delay(1);
    assert(ready==0);
    call(23,stream,0,0,0,0);
    deadline=SDL_GetTicks()+5000;
    while(call(11,stream,0,0,0,0)==4 && SDL_GetTicks()<deadline)SDL_Delay(1);
    assert(call(11,stream,0,0,0,0)==2);
    uint32_t *info=args+8;
    assert(call(24,stream,(uintptr_t)&info[0],(uintptr_t)&info[1],(uintptr_t)&info[2],(uintptr_t)&info[3])==1);
    assert(info[0]==0 && info[1]==0 && info[2]==8192 && info[3]==8192);
    RgAudioStats stats;rg_audio_stats(&stats);assert(stats.nonzero_frames>14000);
    call(21,stream,0,0,0,0);
    for(unsigned i=0;i<24;++i){stream=call(20,driver,0,0,0,0);assert(stream);call(21,stream,0,0,0,0);}
    rg_audio_close();assert(rg_low_free(args));
    puts("PASS streamed PCM double buffers consume in order, expose ready/position/EOF, mix nonzero audio and release/reallocate");
}
