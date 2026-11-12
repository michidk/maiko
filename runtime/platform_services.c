#include "platform_services.h"
#include "input.h"
#include <stdio.h>
#include <string.h>
#include <windows.h>
int rg_int386(GuestFrame *frame) {
    uint32_t input[7];
    memcpy(input,(void *)(uintptr_t)(uint32_t)frame->edx,sizeof input);
    if ((frame->eax & 255)==0x33) {
        if (!rg_mouse_service(input)) return 0;
        input[6]=0;
        memcpy((void *)(uintptr_t)(uint32_t)frame->ebx,input,sizeof input);
        frame->eax=input[0];
        return 1;
    }
    if ((frame->eax & 255)!=0x31 || input[0]!=0x500) return 0;
    uint32_t *info=(void *)(uintptr_t)input[5];
    uint64_t available=0, largest=0;
    for (uintptr_t address=0x20000000; address<0xe0000000;) {
        MEMORY_BASIC_INFORMATION m;
        if (!VirtualQuery((void *)address,&m,sizeof m)) return 0;
        uintptr_t end=(uintptr_t)m.BaseAddress+m.RegionSize;
        if (end>0xe0000000) end=0xe0000000;
        if (end<=address) return 0;
        if (m.State==MEM_FREE) {
            uint64_t bytes=end-address;
            available+=bytes;
            if (bytes>largest) largest=bytes;
        }
        address=end;
    }
    if (largest>0x10000000) largest=0x10000000; /* rg_low_alloc limit */
    MEMORYSTATUSEX status={0}; status.dwLength=sizeof status;
    if (!GlobalMemoryStatusEx(&status)) return 0;
    uint32_t pages=(uint32_t)(available/4096);
    memset(info,0,48);
    info[0]=(uint32_t)largest;
    info[1]=pages;
    info[2]=(0xc0000000u/4096);
    info[3]=pages;
    info[4]=(uint32_t)((status.ullAvailPhys/4096)<pages ? status.ullAvailPhys/4096 : pages);
    info[5]=info[4];
    info[6]=(uint32_t)((status.ullTotalPhys/4096)<info[2] ? status.ullTotalPhys/4096 : info[2]);
    info[7]=pages;
    /* No guest paging file is managed by this runtime. */
    input[6]=0;
    memcpy((void *)(uintptr_t)(uint32_t)frame->ebx,input,sizeof input);
    frame->eax=input[0];
    fprintf(stderr,"Native guest memory largest=%u available_pages=%u\n",info[0],pages);
    return 1;
}

void rg_platform_info(GuestFrame *frame) {
    static unsigned char *cached;
    unsigned char *info=(void *)(uintptr_t)(uint32_t)frame->eax;
    if (!info) {
        if (!cached) cached=rg_low_alloc(0x1bc);
        info=cached;
    }
    if (!info) { frame->eax=0; return; }
    memset(info,0,0x1bc);
    /* Only verified fields are populated. DOS/BIOS driver capabilities are absent. */
    info[0xd]=1; /* x87, required on x64 and preserved by the adapter */
    MEMORYSTATUSEX memory={0}; memory.dwLength=sizeof memory;
    if (GlobalMemoryStatusEx(&memory)) {
        uint32_t available=memory.ullAvailPhys>0xc0000000 ? 0xc0000000 : (uint32_t)memory.ullAvailPhys;
        memcpy(info+0xe,&available,4);
    }
    if (GetCurrentDirectoryA(128,(char *)info+0x40)) info[0x3f]=info[0x40];
    DWORD drives=GetLogicalDrives();
    for (unsigned drive=3;drive<26;++drive) if (drives&(1u<<(drive-1))) {
        info[0x25+drive]=1; ++info[0xd0];
    }
    strcpy((char *)info+0xfc,"Native x64 host; legacy CPU calibration disabled");
    strcpy((char *)info+0x13c,"Native Glide backend not initialized");
    strcpy((char *)info+0x17c,"Windows native x64");
    frame->eax=(uintptr_t)info;
    fprintf(stderr,"Native platform inventory: %s; DOS/BIOS features absent\n",info+0x17c);
}

int rg_video_service(GuestFrame *frame) {
    /* Glide shutdown precedes the original BIOS text-mode restore on Quit.
       The native process already has a console; there is no VGA mode to set. */
    if ((frame->eax & 0xffff)!=3) return 0;
    fputs("Original text-mode3 restore uses existing native console\n",stderr);
    return 1;
}
