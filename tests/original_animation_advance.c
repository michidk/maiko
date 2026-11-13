#include <assert.h>
#include <stdint.h>
#include <stdio.h>
#include <string.h>
#include "guest_abi.h"
#include "guest_address.h"
#include "crash.h"
extern void rg_animation_advance(void);
extern uint32_t rg_tick_delta,rg_animation_enabled;
static void word(unsigned char *p,uint16_t v){memcpy(p,&v,2);}
static uint16_t getword(unsigned char *p){uint16_t v;memcpy(&v,p,2);return v;}
int main(void) {
    setbuf(stdout,NULL);rg_crash_install();
    unsigned char *memory=rg_low_alloc(65536);assert(memory);
    for(unsigned pending=0;pending<2;++pending) {
        memset(memory,0,16384);unsigned char *anim=memory+4096,*group=memory+8192;
        uint32_t p=RG_ADDRESS(anim);memcpy(memory+0x247,&p,4);
        p=RG_ADDRESS(group);memcpy(anim+0x25+20*4,&p,4);
        word(anim,pending ? 0xffff : 20);word(anim+2,pending ? 20 : 0xffff);
        anim[7]=1;anim[8]=1;anim[10]=pending;anim[16]=255;
        uint32_t cmd=(1u<<4)|(2u<<14);memcpy(group+5,&cmd,3);memcpy(group+8,&cmd,3);
        rg_tick_delta=1;rg_animation_enabled=1;
        GuestRegs r={0};r.eax=RG_ADDRESS(memory);r.esp=r.eax+65536;
        r.ebx=0x12345678;r.ecx=0x23456789;r.edx=0x3456789a;
        r.esi=0x456789ab;r.edi=0x56789abc;r.ebp=0x6789abcd;
        printf("Original animation advance pending=%u\n",pending);
        rg_call(RG_ADDRESS(rg_animation_advance),&r);
        assert(getword(anim)==20 && getword(anim+2)==0xffff);
        assert(getword(memory+0x251)==1 && getword(memory+0x24f)==2);
        assert(r.esp==RG_ADDRESS(memory)+65536);
        assert(r.ebx==0x12345678 && r.ecx==0x23456789 && r.edx==0x3456789a);
        assert(r.esi==0x456789ab && r.edi==0x56789abc && r.ebp==0x6789abcd);
    }
    assert(rg_low_free(memory));
    puts("PASS original animation group advance and pending group transition with frame output and preserved registers");
}
