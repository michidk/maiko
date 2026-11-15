#ifndef RG_AUDIO_H
#define RG_AUDIO_H
#include "guest_abi.h"
typedef struct {uint64_t started,mixed_frames,nonzero_frames;} RgAudioStats;
int rg_audio_open(uint32_t handles[16]);
void rg_audio_close(void);
int rg_audio_service(unsigned service,GuestFrame *frame);
void rg_audio_stats(RgAudioStats *stats);
#endif
