#include <assert.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <SDL.h>
#include <SDL_mixer.h>
#include "Game_vars.h"
#include "Albion-sound.h"
#include "guest_abi.h"
extern int32_t AIL_preference[19];
static SDL_atomic_t left,right,signed_total;
static void postmix(void *unused,Uint8 *bytes,int length) {
    (void)unused;int16_t *pcm=(void *)bytes;int l=0,r=0;
    for(int i=0;i<length/2;i+=2) {l+=abs(pcm[i]);r+=abs(pcm[i+1]);}
    SDL_AtomicAdd(&left,l);SDL_AtomicAdd(&right,r);
    int total=0;for(int i=0;i<length/2;++i) total+=pcm[i];SDL_AtomicAdd(&signed_total,total);
}
int main(int argc,char **argv) {
    assert(argc==2);setbuf(stdout,NULL);Game_AudioRate=22050;SDL_SetMainReady();
    assert(!SDL_Init(SDL_INIT_AUDIO|SDL_INIT_TIMER));
    assert(!Mix_OpenAudio(Game_AudioRate,Game_AudioFormat,2,256));Mix_SetPostMix(postmix,NULL);
    int16_t *pcm=rg_low_alloc(8192);assert(pcm);
    for(unsigned i=0;i<4096;++i) pcm[i]=10000;
    struct _AIL_sample *s=Game_AIL_allocate_sample_handle(NULL);assert(s && (uintptr_t)s<=UINT32_MAX);
    Game_AIL_set_sample_type(s,1,1);Game_AIL_set_sample_address(s,pcm,8192);
    Game_AIL_set_sample_playback_rate(s,22050);Game_AIL_set_sample_volume(s,127);
    Game_AIL_set_sample_pan(s,0);
    Game_AIL_set_sample_loop_count(s,!strcmp(argv[1],"loop") || !strcmp(argv[1],"volume") ? 0 : 1);
    Game_AIL_start_sample(s);SDL_Delay(70);
    int l=SDL_AtomicGet(&left),r=SDL_AtomicGet(&right);
    printf("mixed left=%d right=%d status=%u\n",l,r,Game_AIL_sample_status(s));
    if(!strcmp(argv[1],"pan")) assert(l>0 && r==0);
    else if(!strcmp(argv[1],"loop")) {SDL_Delay(450);assert(Game_AIL_sample_status(s)==4);}
    else if(!strcmp(argv[1],"volume")) {
        Game_AIL_set_sample_volume(s,0);SDL_Delay(30);
        SDL_AtomicSet(&left,0);SDL_AtomicSet(&right,0);SDL_Delay(60);
        assert(!SDL_AtomicGet(&left) && !SDL_AtomicGet(&right));
    } else if(!strcmp(argv[1],"cache")) {
        Game_AIL_end_sample(s);
        for(unsigned i=0;i<4096;++i) pcm[i]=-10000;
        for(unsigned i=1;i<=7;++i) {unsigned index=(((8192-4)*i)/8)&0xfffffffcU;pcm[index/2]=pcm[index/2+1]=10000;}
        Game_AIL_start_sample(s);SDL_Delay(20);SDL_AtomicSet(&signed_total,0);SDL_Delay(70);
        assert(SDL_AtomicGet(&signed_total)<0);
    } else abort();
    Game_AIL_end_sample(s);assert(Game_AIL_sample_status(s)==2);
    Game_AIL_release_sample_handle(s);Mix_CloseAudio();SDL_Quit();assert(rg_low_free(pcm));
    for(unsigned i=0;i<GAME_SAMPLE_CACHE_SIZE;++i) free(Game_SampleCache[i]);
    puts("PASS SR sample backend preserves the tested Miles sample behavior");
}
