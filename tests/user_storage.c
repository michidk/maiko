#define _GNU_SOURCE
#include "guest_abi.h"
#include "game_files.h"
#include "paths.h"
#include "user_storage.h"
#include <assert.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>
#include <sys/stat.h>
static char guest[8192];
static uint32_t address(const void *p) {assert((uintptr_t)p<=UINT32_MAX);return (uint32_t)(uintptr_t)p;}
static GuestFrame request(unsigned ax,const char *path) {
    GuestFrame f={0};strcpy(guest,path);f.eax=ax;f.edx=address(guest);return f;
}
static void put(const char *path,const char *value) {
    FILE *f=fopen(path,"wb");assert(f);assert(fwrite(value,1,strlen(value),f)==strlen(value));assert(!fclose(f));
}
static void expect(const char *path,const char *value) {
    char data[128]={0};FILE *f=fopen(path,"rb");assert(f);size_t n=fread(data,1,sizeof data,f);assert(n==strlen(value) && !memcmp(data,value,n));assert(!fclose(f));
}
int main(void) {
    setbuf(stdout,NULL);char root[]="/tmp/rg-user-files-XXXXXX";assert(mkdtemp(root) && !chdir(root));
    assert(!mkdir("game",0700) && !mkdir("game/Savegame",0700) && !mkdir("game/Savegame/SAVEGAME.001",0700));
    assert(!mkdir("game/Maps",0700));
    put("game/Savegame/SAVEGAME.001/SAVEGAME.SAV","original save");
    put("game/Maps/ISLAND.TSG","original map");put("game/Maps/ISLAND.RGM","asset");
    put("game/SYSTEM.INI","original settings");
    assert(rg_path_root("game"));
    assert(!rg_user_storage_init("game/must-not-create"));
    assert(access("game/must-not-create",F_OK));
    assert(!rg_user_storage_init("."));
    assert(rg_user_storage_init("user"));
    expect("user/SAVEGAME/SAVEGAME.001/SAVEGAME.SAV","original save");
    expect("user/MAPS/ISLAND.TSG","original map");expect("user/SYSTEM.INI","original settings");
    assert(!chdir(rg_game_root()));
    GuestFrame f=request(0x3900,"C:\\SAVEGAME\\SAVEGAME.099");
    assert(rg_file_service(&f) && !(f.flags&1));
    f=request(0x3c00,"C:\\SAVEGAME\\SAVEGAME.099\\SAVEGAME.SAV");
    assert(rg_file_service(&f) && !(f.flags&1));unsigned handle=f.eax;
    memcpy(guest+4096,"native save",11);f.eax=0x4000;f.ebx=handle;f.edx=address(guest+4096);f.ecx=11;
    assert(rg_file_service(&f) && !(f.flags&1) && f.eax==11);
    f.eax=0x4200;f.ecx=0;f.edx=7;assert(rg_file_service(&f) && !(f.flags&1));
    f.eax=0x4200;f.ebx=handle;f.ecx=0;f.edx=0;assert(rg_file_service(&f) && !(f.flags&1));
    f.eax=0x3f00;f.edx=address(guest+4096);f.ecx=11;
    assert(rg_file_service(&f) && !(f.flags&1) && f.eax==11 && !memcmp(guest+4096,"native save",11));
    f.eax=0x4200;f.ecx=0;f.edx=7;assert(rg_file_service(&f) && !(f.flags&1));
    f.eax=0x4000;f.ecx=0;assert(rg_file_service(&f) && !(f.flags&1) && !f.eax);
    f.eax=0x3e00;assert(rg_file_service(&f) && !(f.flags&1));
    f=request(0x5600,"C:\\SAVEGAME\\SAVEGAME.099\\SAVEGAME.SAV");strcpy(guest+2048,"C:\\SAVEGAME\\SAVEGAME.099\\RENAME.SAV");f.edi=address(guest+2048);
    assert(rg_file_service(&f) && !(f.flags&1));
    f=request(0x3d00,"C:\\SAVEGAME\\SAVEGAME.099\\RENAME.SAV");assert(rg_file_service(&f) && !(f.flags&1));handle=f.eax;
    f.eax=0x3f00;f.ebx=handle;f.edx=address(guest+4096);f.ecx=32;
    assert(rg_file_service(&f) && !(f.flags&1) && f.eax==7 && !memcmp(guest+4096,"native ",7));
    f.eax=0x3e00;assert(rg_file_service(&f) && !(f.flags&1));
    f=request(0x4100,"C:\\MAPS\\ISLAND.TSG");assert(rg_file_service(&f) && !(f.flags&1));
    f=request(0x3d00,"C:\\MAPS\\ISLAND.TSG");assert(rg_file_service(&f) && (f.flags&1));
    f=request(0x3c00,"C:\\MAPS\\NATIVE.TSG");assert(rg_file_service(&f) && !(f.flags&1));
    handle=f.eax;f.eax=0x3e00;f.ebx=handle;assert(rg_file_service(&f) && !(f.flags&1));
    f.eax=0x1a00;f.edx=address(guest+6000);assert(rg_file_service(&f));
    for(unsigned i=0;i<70;++i) {
        f=request(0x4e00,"C:\\MAPS\\*.TSG");
        assert(rg_file_service(&f) && !(f.flags&1) && !strcmp(guest+6030,"NATIVE.TSG"));
        f.eax=0x4f00;assert(rg_file_service(&f) && (f.flags&1) && f.eax==18);
    }
    f=request(0x3c00,"C:\\MAPS\\ISLAND.RGM");assert(rg_file_service(&f) && (f.flags&1) && f.eax==5);
    f=request(0x3c00,"C:\\SAVEGAME\\..\\..\\escape.ini");assert(rg_file_service(&f) && (f.flags&1));
    char target[4096];snprintf(target,sizeof target,"%s/outside",root);assert(!mkdir(target,0700));
    char link[4096];snprintf(link,sizeof link,"%s/user/SAVEGAME/SAVEGAME.666",root);assert(!symlink(target,link));
    f=request(0x3c00,"C:\\SAVEGAME\\SAVEGAME.666\\ESCAPE.SAV");assert(rg_file_service(&f) && (f.flags&1));
    assert(!chdir(root));assert(rg_user_storage_init("user"));
    assert(access("user/MAPS/ISLAND.TSG",F_OK) && access("outside/ESCAPE.SAV",F_OK));
    expect("game/Savegame/SAVEGAME.001/SAVEGAME.SAV","original save");expect("game/Maps/ISLAND.TSG","original map");expect("game/Maps/ISLAND.RGM","asset");expect("game/SYSTEM.INI","original settings");
    puts("PASS isolated imported saves/config, native create/write/seek/truncate/rename/read/delete, persistent deletion, immutable install, traversal and symlink rejection");
}
