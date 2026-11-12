#ifndef RG_STREAM_PCM_H
#define RG_STREAM_PCM_H
#include <SDL.h>
#include <stdint.h>
typedef struct {
    Uint8 *data;
    uint32_t length,position,source_length;
    int state;
} RgStreamBuffer;
typedef struct {
    RgStreamBuffer buffer[2];
    unsigned current,format,flags,rate,output_rate,volume,pan;
    int ended,started;
    uint64_t mixed_frames,nonzero_frames,underrun_frames;
} RgPcmStream;
void rg_stream_reset(RgPcmStream *s);
void rg_stream_clear(RgPcmStream *s);
uint32_t rg_stream_ready(RgPcmStream *s);
int rg_stream_load(RgPcmStream *s,unsigned index,const void *data,uint32_t length);
void rg_stream_mix(RgPcmStream *s,Uint8 *output,unsigned length);
uint32_t rg_stream_status(const RgPcmStream *s);
#endif
