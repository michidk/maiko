#include "audio.h"
#include "host_os.h"
#include "Game_vars.h"
#include "Albion-sound.h"
#include "stream_pcm.h"
#include <SDL.h>
#include <SDL_mixer.h>
#include <stdatomic.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
uint32_t Game_SoundMasterVolume=127;
int Game_MixerChannels,Game_AudioRate=44100,Game_ResamplingQuality=1,Game_SwapSoundChannels;
unsigned Game_AudioFormat=AUDIO_S16LSB,Game_AudioChannels=2;
Game_sample *Game_SampleCache[GAME_SAMPLE_CACHE_SIZE];
int32_t AIL_preference[19]={[5]=127};
static struct {struct _AIL_sample *handle;uint32_t user[8],length;unsigned type;RgPcmStream stream;} samples[24];
static SDL_mutex *stream_lock;
static uint32_t *digital_driver;
static int opened,exit_registered,output_observed,miles_started;
static _Atomic uint64_t starts,mixed,nonzero,stream_buffers,stream_nonzero,stream_mixed,stream_underrun;
static unsigned captured_pcm;
static int capture_failed;
static int capture_pcm(const void *data,unsigned length) {
    const char *path=getenv("RG_MOVIE_PCM_CAPTURE");
    if(!path || !*path || !length || captured_pcm==262144)return 1;
    if(capture_failed)return 0;
    unsigned count=262144-captured_pcm;if(count>length)count=length;
    FILE *file=fopen(path,captured_pcm ? "ab":"wb");
    if(!file){capture_failed=1;return 0;}
    int ok=fwrite(data,1,count,file)==count;
    if(fclose(file))ok=0;
    if(!ok){capture_failed=1;return 0;}
    captured_pcm+=count;return 1;
}
static void postmix(void *unused,Uint8 *data,int size) {
    (void)unused;SDL_LockMutex(stream_lock);
    for(unsigned i=16;i<24;++i)if(samples[i].handle){
        uint64_t before=samples[i].stream.nonzero_frames;
        uint64_t frames=samples[i].stream.mixed_frames,starved=samples[i].stream.underrun_frames;
        rg_stream_mix(&samples[i].stream,data,(unsigned)size);
        atomic_fetch_add(&stream_nonzero,samples[i].stream.nonzero_frames-before);
        atomic_fetch_add(&stream_mixed,samples[i].stream.mixed_frames-frames);
        atomic_fetch_add(&stream_underrun,samples[i].stream.underrun_frames-starved);
    }
    SDL_UnlockMutex(stream_lock);const int16_t *pcm=(void *)data;unsigned count=0;
    for(int i=0;i<size/2;i+=2) count+=pcm[i]!=0 || pcm[i+1]!=0;
    atomic_fetch_add_explicit(&mixed,(unsigned)size/4,memory_order_relaxed);
    atomic_fetch_add_explicit(&nonzero,count,memory_order_relaxed);
}
void rg_audio_stats(RgAudioStats *s) {
    s->started=atomic_load(&starts);s->mixed_frames=atomic_load(&mixed);s->nonzero_frames=atomic_load(&nonzero);
}
void rg_audio_close(void) {
    miles_started=0;
    if(!opened) return;
    Mix_SetPostMix(NULL,NULL);
    for(unsigned i=0;i<24;++i) if(samples[i].handle) {rg_stream_clear(&samples[i].stream);if(i<16)Game_AIL_release_sample_handle(samples[i].handle);else rg_low_free(samples[i].handle);}
    memset(samples,0,sizeof samples);Mix_CloseAudio();
    SDL_DestroyMutex(stream_lock);stream_lock=NULL;
    if(digital_driver){rg_low_free(digital_driver);digital_driver=NULL;}
    SDL_QuitSubSystem(SDL_INIT_AUDIO);
    fprintf(stderr,"Native movie stream buffers=%llu nonzero_frames=%llu consumed_frames=%llu starved_frames=%llu\n",(unsigned long long)atomic_load(&stream_buffers),(unsigned long long)atomic_load(&stream_nonzero),(unsigned long long)atomic_load(&stream_mixed),(unsigned long long)atomic_load(&stream_underrun));
    for(unsigned i=0;i<GAME_SAMPLE_CACHE_SIZE;++i) {free(Game_SampleCache[i]);Game_SampleCache[i]=NULL;}
    Game_MixerChannels=0;opened=0;
    RgAudioStats s;rg_audio_stats(&s);
    fprintf(stderr,"Native audio closed samples_started=%llu mixed_frames=%llu nonzero_frames=%llu\n",
            (unsigned long long)s.started,(unsigned long long)s.mixed_frames,(unsigned long long)s.nonzero_frames);
}
int rg_audio_open(uint32_t handles[16]) {
    int started=miles_started;rg_audio_close();miles_started=started;memset(handles,0,16*sizeof *handles);
    SDL_SetMainReady();
    if(SDL_InitSubSystem(SDL_INIT_AUDIO) || Mix_OpenAudio(44100,AUDIO_S16LSB,2,1024)) {
        fprintf(stderr,"Native audio device failed: %s\n",SDL_GetError());SDL_QuitSubSystem(SDL_INIT_AUDIO);return 0;
    }
    opened=1;output_observed=0;captured_pcm=0;capture_failed=0;atomic_store(&starts,0);atomic_store(&mixed,0);atomic_store(&nonzero,0);atomic_store(&stream_buffers,0);atomic_store(&stream_nonzero,0);
    atomic_store(&stream_mixed,0);atomic_store(&stream_underrun,0);
    int channels;Uint16 format;
    if(!Mix_QuerySpec(&Game_AudioRate,&format,&channels) || format!=AUDIO_S16LSB || channels!=2) {rg_audio_close();return 0;}
    Game_AudioFormat=format;Game_AudioChannels=(unsigned)channels;
    for(unsigned i=0;i<16;++i) {
        samples[i].handle=Game_AIL_allocate_sample_handle(NULL);
        if(!samples[i].handle || !rg_guest_pointer(samples[i].handle,644,&handles[i])) {rg_audio_close();return 0;}
    }
    stream_lock=SDL_CreateMutex();digital_driver=rg_low_alloc(4);
    if(!stream_lock || !digital_driver){rg_audio_close();return 0;}
    *digital_driver=0x52474155;Mix_SetPostMix(postmix,NULL);
    if(!exit_registered) {atexit(rg_audio_close);exit_registered=1;}
    fprintf(stderr,"Native audio device=%s rate=%d channels=%u backend=SR/SDL_mixer\n",SDL_GetCurrentAudioDriver(),Game_AudioRate,Game_AudioChannels);
    return 1;
}
int rg_audio_service(unsigned service,GuestFrame *f) {
    /* AIL_startup only prepares DOS callbacks/drivers and optional MSS_DEBUG.
       SDL owns playback timing and memory; preserve its first/repeated result. */
    /* No MSCDEX device is exposed yet. Miles returns NULL for absent media. */
    if(service==18) {fputs("Native CD audio unavailable: no virtual CD device configured; continuing without CD music\n",stderr);f->eax=0;return 1;}
    if(service==17) {f->eax=!miles_started;miles_started=1;return 1;}
    if(service==19){f->eax=(uintptr_t)digital_driver;return opened;}
    const uint32_t *a=(void *)(uintptr_t)(uint32_t)f->esp;
    unsigned argc;
    switch(service) {
    case 2:case 9:case 11:case 12:case 15:case 16:argc=1;break;
    case 5:case 6:case 7:case 10:case 14:case 28:case 29:argc=2;break;
    case 3:case 4:case 8:case 13:case 25:argc=3;break;
    case 20:case 21:case 22:argc=1;break;
    case 23:argc=4;break;
    case 24:argc=5;break;
    default:return 0;
    }
    if(!rg_host_readable(a,(argc+1)*4)) return 0;
    if(service==15) {
        void *p=rg_low_alloc(a[1]);f->eax=(uintptr_t)p;return 1;
    }
    if(service==16) return rg_low_free((void *)(uintptr_t)a[1]);
    if(service==28 || service==29){
        if(!a[2] || !rg_host_readable((void *)(uintptr_t)a[1],a[2])){f->eax=0;return 1;}
        f->eax=rg_host_lock((void *)(uintptr_t)a[1],a[2],service==28);return 1;
    }
    if(service==20) {
        if(!opened || a[1]!=(uintptr_t)digital_driver)return 0;
        SDL_LockMutex(stream_lock);
        for(unsigned i=16;i<24;++i)if(!samples[i].handle){
            samples[i].handle=rg_low_alloc(644);
            if(samples[i].handle){rg_stream_reset(&samples[i].stream);samples[i].stream.output_rate=Game_AudioRate;}
            f->eax=(uintptr_t)samples[i].handle;SDL_UnlockMutex(stream_lock);return 1;
        }
        SDL_UnlockMutex(stream_lock);f->eax=0;return 1;
    }
    if(service==25) {
        if(!opened || a[1]!=(uintptr_t)digital_driver || a[2]<512 || a[2]>96000 || a[3]>3)return 0;
        /* Native mixer uses1024 output frames. Give the original ring two
           callbacks of source PCM, rounded to Miles'256-byte alignment. */
        unsigned stride=(a[3]&1 ? 2:1)*(a[3]&2 ? 2:1);
        f->eax=(((uint64_t)a[2]*stride*2048+Game_AudioRate-1)/Game_AudioRate+255)&~255u;return 1;
    }
    if(!a[1])return 0;
    unsigned slot=0;
    while(slot<24 && (uintptr_t)samples[slot].handle!=a[1]) ++slot;
    if(!opened || slot==24) return 0;
    struct _AIL_sample *s=samples[slot].handle;
    if(slot>=16 && service==23 && a[4]) {
        unsigned index=a[2];RgPcmStream prepared={0};
        if(index>1 || !rg_host_readable((void *)(uintptr_t)a[3],a[4]))return 0;
        SDL_LockMutex(stream_lock);
        RgPcmStream *stream=&samples[slot].stream;
        int reserved=stream->buffer[index].state==1;
        prepared.format=stream->format;prepared.flags=stream->flags;
        prepared.rate=stream->rate;prepared.output_rate=stream->output_rate;
        prepared.buffer[index].state=1;
        SDL_UnlockMutex(stream_lock);
        /* Only the game thread submits buffers. The audio thread cannot consume
           a reserved slot. Copy/convert before publishing to keep it unblocked. */
        if(!reserved || !rg_stream_load(&prepared,index,(void *)(uintptr_t)a[3],a[4]))return 0;
        if(!capture_pcm((void *)(uintptr_t)a[3],a[4])){rg_stream_clear(&prepared);return 0;}
        SDL_LockMutex(stream_lock);
        free(stream->buffer[index].data);stream->buffer[index]=prepared.buffer[index];stream->started=1;
        SDL_UnlockMutex(stream_lock);atomic_fetch_add(&stream_buffers,1);return 1;
    }
    if(slot>=16) {
        int ok=1;RgPcmStream *stream=&samples[slot].stream;
        SDL_LockMutex(stream_lock);
        switch(service){
        case 2:rg_stream_reset(stream);stream->output_rate=Game_AudioRate;memset(samples[slot].user,0,sizeof samples[slot].user);break;
        case 3:if(a[2]>3 || a[3]>1)ok=0;else{stream->format=a[2];stream->flags=a[3];}break;
        case 5:if(a[2]<512 || a[2]>96000)ok=0;else {stream->rate=a[2];fprintf(stderr,"Native movie PCM source_rate=%u format=%u flags=%u output_rate=%u\n",stream->rate,stream->format,stream->flags,stream->output_rate);}break;
        case 6:if(a[2]>127)ok=0;else stream->pan=a[2];break;
        case 10:if(a[2]>127)ok=0;else stream->volume=a[2];break;
        case 11:f->eax=rg_stream_status(stream);break;
        case 12:rg_stream_clear(stream);break;
        case 13:if(a[2]>=8)ok=0;else{f->eax=samples[slot].user[a[2]];samples[slot].user[a[2]]=a[3];}break;
        case 14:if(a[2]>=8)ok=0;else f->eax=samples[slot].user[a[2]];break;
        case 21:rg_stream_clear(stream);rg_low_free(s);memset(&samples[slot],0,sizeof samples[slot]);break;
        case 22:f->eax=rg_stream_ready(stream);break;
        case 23:
            if(a[4] && !rg_host_readable((void *)(uintptr_t)a[3],a[4]))ok=0;
            else {ok=rg_stream_load(stream,a[2],(void *)(uintptr_t)a[3],a[4]);
                if(ok && a[4]){ok=capture_pcm((void *)(uintptr_t)a[3],a[4]);atomic_fetch_add(&stream_buffers,1);}
            }
            break;
        case 24:
            for(unsigned i=0;i<4;++i)if(a[i+2] && !rg_host_readable((void *)(uintptr_t)a[i+2],4))ok=0;
            if(ok){
                uint32_t info[4];
                for(unsigned i=0;i<2;++i){RgStreamBuffer *b=&stream->buffer[i];info[2*i]=b->length ? (uint64_t)b->position*b->source_length/b->length:0;info[2*i+1]=b->source_length;}
                for(unsigned i=0;i<4;++i)if(a[i+2])memcpy((void *)(uintptr_t)a[i+2],&info[i],4);
                f->eax=stream->ended && rg_stream_status(stream)==2;
            }break;
        default:ok=0;break;
        }
        SDL_UnlockMutex(stream_lock);return ok;
    }
    switch(service) {
    case 2:
        Game_AIL_end_sample(s);Game_AIL_init_sample(s);samples[slot].length=0;return 1;
    case 3:
        if(a[2]>3 || a[3]>1) return 0;
        samples[slot].type=a[2];Game_AIL_set_sample_type(s,(int32_t)a[2],a[3]);return 1;
    case 4:
        if(a[3]>16*1024*1024 || !rg_host_readable((void *)(uintptr_t)a[2],a[3])) return 0;
        samples[slot].length=a[3];Game_AIL_set_sample_address(s,(void *)(uintptr_t)a[2],a[3]);return 1;
    case 5:
        if(a[2]<512 || a[2]>44000 || Game_AIL_sample_status(s)==4) return 0;
        Game_AIL_set_sample_playback_rate(s,(int32_t)a[2]);return 1;
    case 6:
        if(a[2]>127) return 0;
        Game_AIL_set_sample_pan(s,(int32_t)a[2]);return 1;
    case 7:
        if(a[2]>INT32_MAX) return 0;
        Game_AIL_set_sample_loop_count(s,(int32_t)a[2]);return 1;
    case 8:
        /* Every shipped SFX block spans the entire sample. */
        return a[2]==0 && a[3]==UINT32_MAX;
    case 9: {
        unsigned stride=(samples[slot].type&1 ? 2 : 1)*(samples[slot].type&2 ? 2 : 1);
        if(!samples[slot].length || samples[slot].length%stride) return 0;
        Game_AIL_start_sample(s);atomic_fetch_add(&starts,1);return 1;
    }
    case 10:
        if(a[2]>127) return 0;
        Game_AIL_set_sample_volume(s,(int32_t)a[2]);return 1;
    case 11:
        f->eax=Game_AIL_sample_status(s);
        if(!output_observed && atomic_load(&nonzero)) {
            output_observed=1;RgAudioStats stats;rg_audio_stats(&stats);
            fprintf(stderr,"Native audio mixed nonzero_frames=%llu samples_started=%llu\n",
                    (unsigned long long)stats.nonzero_frames,(unsigned long long)stats.started);
        }
        return 1;
    case 12:Game_AIL_end_sample(s);return 1;
    case 13: {
        if(a[2]>=8) return 0;
        uint32_t old=samples[slot].user[a[2]];samples[slot].user[a[2]]=a[3];f->eax=old;return 1;
    }
    case 14:
        if(a[2]>=8) return 0;
        f->eax=samples[slot].user[a[2]];return 1;
    default:return 0;
    }
}
