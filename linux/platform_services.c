#define _GNU_SOURCE
#include "platform_services.h"
#include "input.h"
#include "host_os.h"
#include <stdio.h>
#include <string.h>
#include <sys/sysinfo.h>
static uint64_t free_guest_space(uint64_t *largest) {
    FILE *maps=fopen("/proc/self/maps","r");if(!maps) return 0;
    char line[512];unsigned long start,end;uint64_t cursor=0x20000000,total=0;*largest=0;
    while(fgets(line,sizeof line,maps)) {
        if(sscanf(line,"%lx-%lx",&start,&end)!=2 || end<=cursor) continue;
        if(start>0xe0000000) start=0xe0000000;
        if(start>cursor) {uint64_t bytes=start-cursor;total+=bytes;if(bytes>*largest) *largest=bytes;}
        if(end>cursor) cursor=end;
        if(cursor>=0xe0000000) break;
    }
    fclose(maps);return total;
}
int rg_int386(GuestFrame *f) {
    uint32_t r[7];memcpy(r,(void *)(uintptr_t)(uint32_t)f->edx,sizeof r);
    if((f->eax&255)==0x33) {
        if(!rg_mouse_service(r)) return 0;
    } else if((f->eax&255)==0x31 && r[0]==0x500) {
        struct sysinfo memory;if(sysinfo(&memory)) return 0;
        uint64_t largest,available=free_guest_space(&largest);if(!available) return 0;
        uint32_t *info=(void *)(uintptr_t)r[5],pages=available/4096;
        if(largest>0x10000000) largest=0x10000000;
        memset(info,0,48);info[0]=largest;info[1]=info[3]=info[7]=pages;info[2]=0xc0000000u/4096;
        uint64_t physical=(uint64_t)memory.freeram*memory.mem_unit/4096;
        info[4]=info[5]=physical<pages ? physical : pages;
        physical=(uint64_t)memory.totalram*memory.mem_unit/4096;
        info[6]=physical<info[2] ? physical : info[2];
        fprintf(stderr,"Native guest memory largest=%u available_pages=%u\n",info[0],pages);
    } else return 0;
    r[6]=0;memcpy((void *)(uintptr_t)(uint32_t)f->ebx,r,sizeof r);f->eax=r[0];return 1;
}
void rg_platform_info(GuestFrame *f) {
    static unsigned char *cached;
    unsigned char *info=(void *)(uintptr_t)(uint32_t)f->eax;
    if(!info) {if(!cached) cached=rg_low_alloc(0x1bc);info=cached;}
    if(!info) {f->eax=0;return;}
    memset(info,0,0x1bc);info[0xd]=1;
    struct sysinfo memory;
    if(!sysinfo(&memory)) {
        uint64_t bytes=(uint64_t)memory.freeram*memory.mem_unit;
        uint32_t available=bytes>0xc0000000 ? 0xc0000000 : (uint32_t)bytes;memcpy(info+0xe,&available,4);
    }
    if(rg_host_cwd(128,(char *)info+0x40)) info[0x3f]='C';
    info[0x28]=1;info[0xd0]=1;
    strcpy((char *)info+0xfc,"Native x64 host; legacy CPU calibration disabled");
    strcpy((char *)info+0x13c,"Native Glide backend not initialized");
    strcpy((char *)info+0x17c,"Linux native x64");
    f->eax=(uintptr_t)info;fprintf(stderr,"Native platform inventory: %s; DOS/BIOS features absent\n",info+0x17c);
}
int rg_video_service(GuestFrame *f) {
    if((f->eax&0xffff)!=3) return 0;
    fputs("Original text-mode3 restore uses existing native console\n",stderr);return 1;
}
