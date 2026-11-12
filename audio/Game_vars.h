#pragma once
#include "Game_defs.h"
extern uint32_t Game_SoundMasterVolume;
extern int Game_MixerChannels,Game_AudioRate,Game_ResamplingQuality,Game_SwapSoundChannels;
extern unsigned Game_AudioFormat,Game_AudioChannels;
extern Game_sample *Game_SampleCache[GAME_SAMPLE_CACHE_SIZE];
