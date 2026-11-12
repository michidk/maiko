#include "stream_pcm.h"
#include <limits.h>
#include <stdlib.h>
#include <string.h>
void rg_stream_clear(RgPcmStream *s) {
    for(unsigned i=0;i<2;++i){free(s->buffer[i].data);memset(&s->buffer[i],0,sizeof s->buffer[i]);}
    s->current=0;s->ended=0;s->started=0;
}
void rg_stream_reset(RgPcmStream *s) {
    rg_stream_clear(s);s->format=0;s->flags=0;s->rate=22050;s->output_rate=44100;s->volume=127;s->pan=64;
    s->mixed_frames=s->nonzero_frames=s->underrun_frames=0;
}
uint32_t rg_stream_ready(RgPcmStream *s) {
    for(unsigned i=0;i<2;++i){unsigned n=(s->current+i)&1;if(!s->buffer[n].state){s->buffer[n].state=1;return n;}}
    return UINT32_MAX;
}
uint32_t rg_stream_status(const RgPcmStream *s) {
    return s->buffer[0].state==2 || s->buffer[1].state==2 ? 4:2;
}
int rg_stream_load(RgPcmStream *s,unsigned index,const void *data,uint32_t length) {
    if(index>1 || s->buffer[index].state!=1 || length>16*1024*1024) return 0;
    RgStreamBuffer *b=&s->buffer[index];
    unsigned channels=s->format&2 ? 2:1,width=s->format&1 ? 2:1;
    if(length%(channels*width))return 0;
    if(!length){free(b->data);b->data=NULL;b->length=b->position=b->source_length=0;s->ended=1;return 1;}
    SDL_AudioFormat format=width==2 ? (s->flags&1 ? AUDIO_S16LSB:AUDIO_U16LSB):(s->flags&1 ? AUDIO_S8:AUDIO_U8);
    SDL_AudioCVT cvt;
    if(!s->output_rate || SDL_BuildAudioCVT(&cvt,format,channels,s->rate,AUDIO_S16LSB,2,s->output_rate)<0 || cvt.len_mult<1 ||
       length>INT_MAX/(unsigned)cvt.len_mult)return 0;
    Uint8 *converted=malloc((size_t)length*cvt.len_mult);if(!converted)return 0;
    memcpy(converted,data,length);cvt.buf=converted;cvt.len=(int)length;
    if(SDL_ConvertAudio(&cvt)<0 || cvt.len_cvt<=0 || cvt.len_cvt%4){free(converted);return 0;}
    free(b->data);b->data=converted;b->length=cvt.len_cvt;b->position=0;b->source_length=length;b->state=2;s->started=1;
    return 1;
}
void rg_stream_mix(RgPcmStream *s,Uint8 *output,unsigned length) {
    unsigned out=0;
    while(out+4<=length){
        RgStreamBuffer *b=&s->buffer[s->current];
        if(b->state!=2)break;
        unsigned available=b->length-b->position,n=length-out;
        if(n>available)n=available;
        n&=~3u;
        int16_t *dst=(void *)(output+out),*src=(void *)(b->data+b->position);
        unsigned gain[2]={s->pan>=64 ? (127-s->pan)*255/63:255,s->pan<=63 ? s->pan*255/63:255};
        s->mixed_frames+=n/4;
        for(unsigned i=0;i<n/2;++i){
            if(!(i&1) && s->volume && (src[i] || src[i+1]))++s->nonzero_frames;
            int sample=dst[i]+(int)((int64_t)src[i]*s->volume*gain[i&1]/(127*255));
            dst[i]=sample>32767 ? 32767:sample< -32768 ? -32768:sample;
        }
        out+=n;b->position+=n;
        if(b->position==b->length){b->state=0;s->current^=1;}
        else break;
    }
    if(s->started && !s->ended)s->underrun_frames+=(length-out)/4;
}
