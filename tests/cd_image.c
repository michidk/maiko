/* Catch sector-header leakage, wrong seek/EOF, shared cursors, write access,
   malformed extents and fall-through into host paths on the virtual CD. */
#include "cd_image.h"
#include <assert.h>
#include <stdio.h>
#include <stdint.h>
#include <string.h>
static unsigned char buffer[8192];
static uint32_t address(void *p) {assert((uintptr_t)p<=UINT32_MAX);return (uint32_t)(uintptr_t)p;}
static GuestFrame request(unsigned ax,const char *path) {
    GuestFrame f={0};strcpy((char *)buffer,path);f.eax=ax;f.edx=address(buffer);return f;
}
static unsigned open_movie(void) {
    GuestFrame f=request(0x3d00,"d:\\intro.smk");assert(rg_cd_service(&f) && !(f.flags&1));return f.eax;
}
static void seek(unsigned handle,unsigned origin,int32_t distance,int good,unsigned expected) {
    GuestFrame f={0};f.eax=0x4200|origin;f.ebx=handle;f.ecx=(uint32_t)distance>>16;f.edx=(uint16_t)distance;
    assert(rg_cd_service(&f));assert((!(f.flags&1))==good);
    if(good) assert(((f.edx<<16)|(f.eax&65535))==expected);
}
static void read_at(unsigned handle,unsigned length,unsigned expected,unsigned start) {
    GuestFrame f={0};f.eax=0x3f00;f.ebx=handle;f.edx=address(buffer);f.ecx=length;
    memset(buffer,0,sizeof buffer);assert(rg_cd_service(&f) && !(f.flags&1) && f.eax==expected);
    for(unsigned i=0;i<expected;++i) assert(buffer[i]==(unsigned char)((start+i)*13+7));
}
int main(int argc,char **argv) {
    assert(argc==3);setbuf(stdout,NULL);
    if(!strcmp(argv[2],"invalid")) {assert(!rg_cd_mount(argv[1]));puts("PASS malformed CD rejected");return 0;}
    assert(rg_cd_mount(argv[1]));
    GuestFrame f=request(0x3d00,"C:\\INTRO.SMK");assert(!rg_cd_service(&f));
    unsigned h=open_movie(),other=open_movie();assert(h!=other);
    read_at(h,3000,3000,0);read_at(other,16,16,0);
    seek(h,0,2040,1,2040);read_at(h,40,40,2040);
    seek(h,2,-10,1,4990);read_at(h,40,10,4990);read_at(h,40,0,5000);
    seek(h,0,6000,1,6000);read_at(h,1,0,6000);seek(h,1,-6001,0,0);seek(h,2,0,1,5000);
    f=request(0x3d02,"D:\\INTRO.SMK");assert(rg_cd_service(&f) && (f.flags&1) && f.eax==5);
    f=request(0x3c00,"D:\\INTRO.SMK");assert(rg_cd_service(&f) && (f.flags&1) && f.eax==5);
    f=request(0x3d00,"D:\\..\\INTRO.SMK");assert(rg_cd_service(&f) && (f.flags&1));
    f=request(0x3d00,"D:\\MISSING.SMK");assert(rg_cd_service(&f) && (f.flags&1) && f.eax==2);
    f=request(0x4300,"D:\\INTRO.SMK");assert(rg_cd_service(&f) && !(f.flags&1) && (f.ecx&1));
    f.eax=0x4000;f.ebx=h;assert(rg_cd_service(&f) && (f.flags&1) && f.eax==5);
    f.eax=0x3e00;f.ebx=h;assert(rg_cd_service(&f) && !(f.flags&1));
    f.eax=0x3f00;assert(rg_cd_service(&f) && (f.flags&1) && f.eax==6);
    f.eax=0x3e00;f.ebx=other;assert(rg_cd_service(&f) && !(f.flags&1));
    for(unsigned i=0;i<300;++i) {h=open_movie();f.eax=0x3e00;f.ebx=h;assert(rg_cd_service(&f) && !(f.flags&1));}
    rg_cd_close();puts("PASS CD payload, independent handles, sector seeks, EOF, read-only and repeated close");
}
