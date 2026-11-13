#include <assert.h>
#include <stdio.h>
#include <string.h>
#include "guest_abi.h"
#include "game_files.h"
#include <windows.h>
int main(void) {
    GuestFrame f = {0};
    char *guest = rg_low_alloc(65536);
    assert(guest);
    f.eax = 0x1900;
    assert(rg_file_service(&f) && (f.eax & 255) == 25);
    f.eax = 0x0e00; f.edx = 25;
    assert(rg_file_service(&f) && (f.eax & 255) == 26);
    f.eax = 0x1900;
    assert(rg_file_service(&f) && (f.eax & 255) == 25);
    strcpy(guest, "system.ini");
    f.eax = 0x4300; f.edx = (uintptr_t)guest;
    assert(rg_file_service(&f) && !(f.flags&1) && !(f.ecx&0x10));
    f.eax = 0x3d00; f.edx = (uintptr_t)guest;
    assert(rg_file_service(&f));
    assert(!(f.flags & 1));
    uint32_t handle = f.eax;
    f.eax = 0x3f00; f.ebx = handle; f.ecx = 32; f.edx = (uintptr_t)(guest+1024);
    assert(rg_file_service(&f) && !(f.flags&1) && f.eax == 32);
    char first[32]; memcpy(first, guest+1024, 32);
    f.eax = 0x4200; f.ecx = 0; f.edx = 0;
    assert(rg_file_service(&f) && !(f.flags&1) && f.eax == 0);
    f.eax = 0x3f00; f.ecx = 32; f.edx = (uintptr_t)(guest+1024);
    assert(rg_file_service(&f) && !(f.flags&1) && !memcmp(first,guest+1024,32));
    f.eax = 0x4400; f.ebx = handle;
    assert(rg_file_service(&f) && !(f.flags&1) && !(f.edx&0x80));
    f.eax = 0x4400; f.ebx = 255;
    assert(rg_file_service(&f) && (f.flags&1) && f.eax == 6);
    f.ebx = handle;
    f.eax = 0x3e00;
    assert(rg_file_service(&f) && !(f.flags&1));
    f.eax = 0x3f00;
    assert(rg_file_service(&f) && (f.flags&1) && f.eax == 6);
    /* DOS opens a nine-character leaf as its eight-character DOS name. */
    strcpy(guest, "system\\mm_slide.gxa");
    f.eax=0x3d00; f.edx=(uintptr_t)guest;
    assert(rg_file_service(&f) && !(f.flags&1));
    handle=f.eax;
    f.eax=0x3f00; f.ebx=handle; f.ecx=32; f.edx=(uintptr_t)(guest+1024);
    assert(rg_file_service(&f) && !(f.flags&1) && f.eax==32);
    memcpy(first,guest+1024,32);
    f.eax=0x3e00;
    assert(rg_file_service(&f) && !(f.flags&1));
    strcpy(guest, "system\\mm_slider.gxa");
    f.eax=0x4300; f.edx=(uintptr_t)guest;
    assert(rg_file_service(&f) && !(f.flags&1) && !(f.ecx&0x10));
    char absolute[MAX_PATH];
    assert(GetFullPathNameA(guest,sizeof absolute,absolute,NULL));
    strcpy(guest,absolute);
    f.eax=0x3d00; f.edx=(uintptr_t)guest;
    assert(rg_file_service(&f) && !(f.flags&1));
    handle=f.eax;
    f.eax=0x3f00; f.ebx=handle; f.ecx=32; f.edx=(uintptr_t)(guest+1024);
    assert(rg_file_service(&f) && !(f.flags&1) && f.eax==32 && !memcmp(first,guest+1024,32));
    f.eax=0x3e00;
    assert(rg_file_service(&f) && !(f.flags&1));
    strcpy(guest, "missing-test-file.bin");
    f.eax = 0x3d00; f.edx = (uintptr_t)guest;
    assert(rg_file_service(&f) && (f.flags&1) && f.eax == 2);
    strcpy(guest, "system.ini"); f.eax = 0x3d02;
    assert(rg_file_service(&f) && (f.flags&1) && f.eax == 5);
    strcpy(guest,"native stdout\n");
    f.eax=0x4000; f.ebx=1; f.ecx=14; f.edx=(uintptr_t)guest;
    assert(rg_file_service(&f) && !(f.flags&1) && f.eax==14);
    assert(CreateDirectoryA("Z:\\work\\dos files",NULL));
    const char *names[]={"ALPHA.TSG","BETA.TSG","IGNORE.TXT"};
    const DWORD sizes[]={3,5,2};
    for (unsigned i=0;i<3;++i) {
        char fixture[MAX_PATH];
        snprintf(fixture,sizeof fixture,"Z:\\work\\dos files\\%s",names[i]);
        HANDLE file=CreateFileA(fixture,GENERIC_WRITE,0,NULL,CREATE_NEW,0,NULL);
        DWORD written;
        assert(file!=INVALID_HANDLE_VALUE && WriteFile(file,"abcde",sizes[i],&written,NULL) && written==sizes[i]);
        assert(CloseHandle(file));
    }
    assert(CreateDirectoryA("Z:\\work\\dos files\\NEST.TSG",NULL));
    unsigned char *dta=(unsigned char *)guest+2048;
    memset(dta,0,43);
    f.eax=0x1a00; f.edx=(uintptr_t)dta;
    assert(rg_file_service(&f));
    strcpy(guest,"Z:\\work\\dos files\\*.TSG");
    f.eax=0x4e00; f.edx=(uintptr_t)guest; f.ecx=0;
    unsigned found=0;
    for (unsigned i=0;i<2;++i) {
        assert(rg_file_service(&f) && !(f.flags&1));
        DWORD size; memcpy(&size,dta+26,4);
        if (!strcmp((char *)dta+30,"ALPHA.TSG")) {assert(size==3);found|=1;}
        else {assert(!strcmp((char *)dta+30,"BETA.TSG") && size==5);found|=2;}
        assert(!(dta[21]&FILE_ATTRIBUTE_DIRECTORY));
        /* Watcom retains a copy of the DTA between find calls. */
        memmove(guest+4096,dta,43); dta=(unsigned char *)guest+4096;
        f.eax=0x1a00; f.edx=(uintptr_t)dta; assert(rg_file_service(&f));
        f.eax=0x4f00;
    }
    assert(found==3 && rg_file_service(&f) && (f.flags&1) && f.eax==18);
    strcpy(guest,"Z:\\work\\dos files\\*.ABSENT");
    f.eax=0x4e00; f.edx=(uintptr_t)guest; f.ecx=0;
    assert(rg_file_service(&f) && (f.flags&1) && f.eax==18);
    puts("PASS DOS DTA, wildcard file sizes, copied search state, directory filtering and exhaustion");
    puts("PASS original asset read, DOS case lookup, seek, closed handle, missing file, write denial");
    return 0;
}
