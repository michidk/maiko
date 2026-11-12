#define _GNU_SOURCE
#include "guest_abi.h"
#include "game_files.h"
#include "host_os.h"
#include "paths.h"
#include <assert.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>
#include <sys/stat.h>
int main(void) {
    char fixture[]="/tmp/rg-platform-XXXXXX";
    assert(mkdtemp(fixture) && !chdir(fixture));
    assert(mkdir("linux-fixture",0700)==0);
    assert(mkdir("linux-fixture/System",0700)==0);
    FILE *file=fopen("linux-fixture/System/MM_SLIDE.GXA","wb"); assert(file);
    assert(fwrite("assets",1,6,file)==6); assert(!fclose(file));
    assert(rg_path_root("linux-fixture")); assert(!chdir(rg_game_root()));
    char path[4096],cwd[128];
    assert(rg_resolve_path("c:\\SYSTEM\\MM_SLIDER.GXA",path,sizeof path));
    file=fopen(path,"rb"); assert(file && fgetc(file)=='a'); fclose(file);
    assert(rg_host_cwd(sizeof cwd,cwd) && !strcmp(cwd,"C:\\"));
    assert(rg_host_chdir("system"));
    assert(rg_host_cwd(sizeof cwd,cwd) && !strcmp(cwd,"C:\\System"));
    assert(rg_host_chdir("C:\\"));
    assert(!rg_resolve_path("C:\\..\\outside",path,sizeof path));
    char *data=rg_low_alloc(65536); assert(data);
    strcpy(data,"c:\\system\\mm_slider.gxa");
    GuestFrame f={0}; f.eax=0x3d00;f.edx=(uint32_t)(uintptr_t)data;
    assert(rg_file_service(&f) && !(f.flags&1)); unsigned handle=f.eax;
    f.eax=0x3f00;f.ebx=handle;f.ecx=6;f.edx=(uint32_t)(uintptr_t)(data+512);
    assert(rg_file_service(&f) && !(f.flags&1) && f.eax==6 && !memcmp(data+512,"assets",6));
    f.eax=0x3e00; assert(rg_file_service(&f) && !(f.flags&1));
    f.eax=0x3d01;f.edx=(uint32_t)(uintptr_t)data;
    assert(rg_file_service(&f) && (f.flags&1) && f.eax==5);
    f.eax=0x1a00;f.edx=(uint32_t)(uintptr_t)(data+1024);assert(rg_file_service(&f));
    strcpy(data,"c:\\system\\*.*");
    f.eax=0x4e00;f.ecx=0;f.edx=(uint32_t)(uintptr_t)data;
    assert(rg_file_service(&f) && !(f.flags&1));
    assert(!strcmp(data+1024+30,"MM_SLIDE.GXA"));
    f.eax=0x4f00;assert(rg_file_service(&f) && (f.flags&1) && f.eax==18);
    assert(!mkdir("LongNames",0700));
    file=fopen("LongNames/goggame-1234567890.info","wb");assert(file);fclose(file);
    file=fopen("LongNames/ASSET.DAT","wb");assert(file);fclose(file);
    /* Host metadata must not abort the DOS view or consume search slots. */
    for(unsigned iteration=0;iteration<70;++iteration) {
        strcpy(data,"C:\\LongNames\\*.*");
        f.eax=0x4e00;f.ecx=0;f.edx=(uint32_t)(uintptr_t)data;
        assert(rg_file_service(&f) && !(f.flags&1));
        assert(!strcmp(data+1024+30,"ASSET.DAT"));
        f.eax=0x4f00;assert(rg_file_service(&f) && (f.flags&1) && f.eax==18);
    }
    puts("PASS long host filenames do not abort DOS enumeration or leak search slots");
    assert(rg_low_size(data)>=65536 && rg_host_readable(data,65536));
    assert(!rg_low_size(data+1) && !rg_low_free(data+1));
    assert(rg_low_free(data) && !rg_low_size(data));
    puts("PASS Linux DOS drive/case/8.3 paths, read-only files, DTA search and low-memory bounds");
}
