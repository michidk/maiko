#include <assert.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include "stream_pcm.h"
int main(void){
    RgPcmStream s={0};rg_stream_reset(&s);s.format=3;s.flags=1;s.rate=44100;
    int16_t first[]={1000,-2000,1000,-2000,1000,-2000,1000,-2000};
    int16_t second[]={3000,-4000,3000,-4000,3000,-4000,3000,-4000};
    assert(rg_stream_ready(&s)==0);assert(rg_stream_load(&s,0,first,sizeof first));
    memset(first,0,sizeof first);
    assert(!rg_stream_load(&s,0,first,sizeof first));
    assert(rg_stream_ready(&s)==1);assert(!rg_stream_load(&s,1,second,3));
    assert(rg_stream_load(&s,1,second,sizeof second));assert(rg_stream_ready(&s)==UINT32_MAX);
    int16_t output[16];for(unsigned i=0;i<16;++i)output[i]=10;
    rg_stream_mix(&s,(void *)output,sizeof output);
    for(unsigned i=0;i<8;++i)assert(output[i]==(i&1 ? -1990:1010));
    for(unsigned i=8;i<16;++i)assert(output[i]==(i&1 ? -3990:3010));
    assert(s.mixed_frames==8 && s.nonzero_frames==8 && rg_stream_status(&s)==2);
    assert(rg_stream_ready(&s)==0);assert(rg_stream_load(&s,0,NULL,0));
    assert(s.ended);rg_stream_reset(&s);s.format=3;s.flags=1;s.rate=44100;s.pan=0;
    assert(rg_stream_ready(&s)==0);assert(rg_stream_load(&s,0,second,sizeof second));
    for(unsigned i=0;i<16;++i)output[i]=32000;
    rg_stream_mix(&s,(void *)output,sizeof output);
    assert(output[0]==32767 && output[1]==32000);
    s.volume=0;assert(rg_stream_ready(&s)==1);assert(rg_stream_load(&s,1,second,sizeof second));
    memset(output,0,sizeof output);rg_stream_mix(&s,(void *)output,sizeof output);
    for(unsigned i=0;i<16;++i)assert(output[i]==0);
    rg_stream_reset(&s);s.format=3;s.flags=1;s.rate=44100;s.output_rate=48000;
    int16_t tone[4410*2];for(unsigned i=0;i<4410;++i){tone[i*2]=12000;tone[i*2+1]=-6000;}
    assert(rg_stream_ready(&s)==0);assert(rg_stream_load(&s,0,tone,sizeof tone));
    assert(s.buffer[0].length==4800*4);
    const int16_t *converted=(void *)s.buffer[0].data;
    assert(converted[2000]>11000 && converted[2001]<-5000);
    rg_stream_clear(&s);
    for(unsigned bits=0;bits<2;++bits)for(unsigned sign=0;sign<2;++sign){
        rg_stream_reset(&s);s.format=bits;s.flags=sign;s.rate=44100;
        unsigned char silence[256];memset(silence,sign ? 0:128,sizeof silence);
        if(bits)for(unsigned i=0;i<128;++i){silence[i*2]=0;silence[i*2+1]=sign ? 0:128;}
        assert(rg_stream_ready(&s)==0);assert(rg_stream_load(&s,0,silence,sizeof silence));
        int16_t zeros[512]={0};rg_stream_mix(&s,(void *)zeros,sizeof zeros);
        for(unsigned i=0;i<512;++i)assert(zeros[i]==0);
        assert(rg_stream_status(&s)==2);rg_stream_clear(&s);
    }
    rg_stream_reset(&s);s.format=2;s.rate=s.output_rate=22050;
    unsigned char u8[]={0,255,128,64};int16_t u8_out[4]={0};
    assert(rg_stream_ready(&s)==0);assert(rg_stream_load(&s,0,u8,sizeof u8));
    rg_stream_mix(&s,(void *)u8_out,sizeof u8_out);
    int expected_u8[]={-32768,32512,0,-16384};
    /* SDL's float conversion can round either way at the final16-bit step. */
    for(unsigned i=0;i<4;++i)assert(abs(u8_out[i]-expected_u8[i])<=1);
    rg_stream_reset(&s);s.format=3;s.flags=1;s.rate=44100;
    int16_t block[512],part[128];
    for(unsigned i=0;i<512;++i)block[i]=7000;
    for(unsigned n=0;n<2;++n){assert(rg_stream_ready(&s)==n);assert(rg_stream_load(&s,n,block,sizeof block));}
    for(unsigned cycle=0;cycle<200;++cycle){
        for(unsigned quarter=0;quarter<4;++quarter){
            memset(part,0,sizeof part);rg_stream_mix(&s,(void *)part,sizeof part);
            for(unsigned i=0;i<128;++i)assert(part[i]==7000);
            if(quarter<3)assert(rg_stream_ready(&s)==UINT32_MAX);
        }
        unsigned next=rg_stream_ready(&s);assert(next==(cycle&1));
        assert(rg_stream_load(&s,next,block,sizeof block));
    }
    assert(s.mixed_frames>=51200 && s.underrun_frames==0);
    rg_stream_clear(&s);
    assert(rg_stream_ready(&s)==0);assert(rg_stream_load(&s,0,block,sizeof block));
    int16_t large[1024]={0};rg_stream_mix(&s,(void *)large,sizeof large);
    assert(s.underrun_frames==256);rg_stream_clear(&s);
    puts("PASS PCM ownership/order, stereo signs, gain/saturation, EOF, signed/unsigned8/16bit silence,44100-to48000 duration,200 partial refills and starvation counts");
}
