#include <assert.h>
#include <stdint.h>
#include <stdio.h>
#include <string.h>
#include "platform_services.h"
int main(void) {
    uint32_t *memory = rg_low_alloc(65536);
    assert(memory);
    uint32_t *input=memory, *output=memory+8, *info=memory+32;
    input[0]=0x500; input[5]=(uintptr_t)info;
    GuestFrame f={0}; f.eax=0x31; f.edx=(uintptr_t)input; f.ebx=(uintptr_t)output;
    assert(rg_int386(&f));
    assert(output[6]==0 && output[0]==0x500 && f.eax==0x500);
    assert(info[0]>0 && info[0]<=0x10000000);
    assert(info[1]>=info[0]/4096 && info[2]>=info[1]);
    assert(info[3]==info[1] && info[8]==0);
    memset(output,0,28); input[0]=0x501; f.eax=0x31;
    assert(!rg_int386(&f));
    f.eax=(uintptr_t)(memory+256);
    rg_platform_info(&f);
    assert(f.eax==(uintptr_t)(memory+256));
    unsigned char *platform=(unsigned char *)(memory+256);
    assert(strstr((char *)platform+0x17c,"Windows"));
    assert(platform[0xd]==1 && platform[0x1d]==0 && platform[0xd6]==0);
    assert(platform[0x3f]>='A' && platform[0x3f]<='Z');
    puts("PASS native memory information, guest limits and unsupported interrupt rejection");
    GuestFrame video={0}; video.eax=3; video.ebx=0x12345678;video.edx=0x23456789;video.flags=0x202;
    GuestFrame saved=video;
    assert(rg_video_service(&video) && !memcmp(&video,&saved,sizeof video));
    video.eax=0x13;
    assert(!rg_video_service(&video));
    video.eax=0x0300;
    assert(!rg_video_service(&video));
    puts("PASS exact text-mode3 native console restore and unsupported video rejection");
    return 0;
}
