#include <assert.h>
#include <stdint.h>
#include <stdio.h>
#include <string.h>
#include <windows.h>
#include "guest_abi.h"
#include "game_files.h"
#include "windows_storage.h"
#include "host_os.h"
static char guest[8192];
static uint32_t ptr(void *p) {assert((uintptr_t)p<=UINT32_MAX);return (uint32_t)(uintptr_t)p;}
static GuestFrame request(unsigned ax,const char *path) {
    GuestFrame f={0};strcpy(guest,path);f.eax=ax;f.edx=ptr(guest);return f;
}
static void put(const char *path,const char *value) {
    FILE *f=fopen(path,"wb");assert(f);assert(fwrite(value,1,strlen(value),f)==strlen(value));assert(!fclose(f));
}
static void expect(const char *path,const char *value) {
    char data[128]={0};FILE *f=fopen(path,"rb");assert(f);size_t n=fread(data,1,sizeof data,f);assert(n==strlen(value) && !memcmp(data,value,n));assert(!fclose(f));
}
int main(void) {
    setbuf(stdout,NULL);
    assert(CreateDirectoryA("storage-test",NULL) && SetCurrentDirectoryA("storage-test"));
    assert(CreateDirectoryA("game",NULL) && CreateDirectoryA("game\\SAVEGAME",NULL));
    assert(CreateDirectoryA("game\\SAVEGAME\\SAVEGAME.001",NULL) && CreateDirectoryA("game\\MAPS",NULL));
    put("game\\SAVEGAME\\SAVEGAME.001\\SAVEGAME.SAV","original save");
    put("game\\MAPS\\ISLAND.TSG","original map");put("game\\MAPS\\ISLAND.RGM","asset");put("game\\SYSTEM.INI","original settings");
    assert(!rg_user_storage_init("game","game\\must-not-create"));
    assert(GetFileAttributesA("game\\must-not-create")==INVALID_FILE_ATTRIBUTES);
    assert(rg_user_storage_init("game","user"));
    expect("user\\SAVEGAME\\SAVEGAME.001\\SAVEGAME.SAV","original save");expect("user\\SYSTEM.INI","original settings");
    assert(SetCurrentDirectoryA("game"));
    char cwd[260];assert(rg_host_cwd(sizeof cwd,cwd) && !strcmp(cwd,"C:\\"));
    assert(rg_host_chdir("C:\\MAPS") && rg_host_cwd(sizeof cwd,cwd) && !strcmp(cwd,"C:\\MAPS"));
    assert(rg_host_chdir("C:\\"));
    put("INTRO.SMK","loose intro fixture");
    GuestFrame movie=request(0x3d00,"D:\\INTRO.SMK");
    assert(rg_file_service(&movie) && !(movie.flags&1));unsigned mh=movie.eax;
    movie.eax=0x3f00;movie.ebx=mh;movie.edx=ptr(guest+4096);movie.ecx=32;
    assert(rg_file_service(&movie) && !(movie.flags&1) && movie.eax==19 && !memcmp(guest+4096,"loose intro fixture",19));
    movie.eax=0x3e00;assert(rg_file_service(&movie) && !(movie.flags&1));
    movie=request(0x4300,"D:\\INTRO.SMK");assert(rg_file_service(&movie) && !(movie.flags&1));
    movie=request(0x3d02,"D:\\INTRO.SMK");assert(rg_file_service(&movie) && (movie.flags&1));
    movie=request(0x3d00,"D:\\..\\INTRO.SMK");assert(rg_file_service(&movie) && (movie.flags&1));
    movie=request(0x3d00,"D:\\ABSENT.SMK");assert(rg_file_service(&movie) && (movie.flags&1) && movie.eax==2);
    assert(CreateDirectoryA("MOVIES",NULL));assert(MoveFileA("INTRO.SMK","MOVIES\\INTRO.SMK"));
    movie=request(0x3d00,"D:\\INTRO.SMK");assert(rg_file_service(&movie) && !(movie.flags&1));
    movie.ebx=movie.eax;movie.eax=0x3e00;assert(rg_file_service(&movie) && !(movie.flags&1));
    char tiny[2];assert(rg_host_cwd(0,NULL)==4 && !rg_host_cwd(sizeof tiny,tiny));
    GuestFrame drive={0};drive.eax=0x1900;assert(rg_file_service(&drive) && (drive.eax&255)==2);
    GuestFrame f=request(0x3900,"SAVEGAME\\SAVEGAME.099");assert(rg_file_service(&f) && !(f.flags&1));
    f=request(0x3c00,"SAVEGAME\\SAVEGAME.099\\SAVEGAME.SAV");assert(rg_file_service(&f) && !(f.flags&1));unsigned h=f.eax;
    memcpy(guest+4096,"native save",11);f.eax=0x4000;f.ebx=h;f.edx=ptr(guest+4096);f.ecx=11;
    assert(rg_file_service(&f) && !(f.flags&1) && f.eax==11);
    f.eax=0x4200;f.ecx=0;f.edx=7;assert(rg_file_service(&f) && !(f.flags&1));
    f.eax=0x4200;f.ebx=h;f.ecx=0;f.edx=0;assert(rg_file_service(&f) && !(f.flags&1));
    f.eax=0x3f00;f.edx=ptr(guest+4096);f.ecx=11;
    assert(rg_file_service(&f) && !(f.flags&1) && f.eax==11 && !memcmp(guest+4096,"native save",11));
    f.eax=0x4200;f.ecx=0;f.edx=7;assert(rg_file_service(&f) && !(f.flags&1));
    f.eax=0x4000;f.ecx=0;assert(rg_file_service(&f) && !(f.flags&1));
    f.eax=0x3e00;assert(rg_file_service(&f) && !(f.flags&1));
    f=request(0x5600,"SAVEGAME\\SAVEGAME.099\\SAVEGAME.SAV");strcpy(guest+2048,"SAVEGAME\\SAVEGAME.099\\RENAME.SAV");f.edi=ptr(guest+2048);
    assert(rg_file_service(&f) && !(f.flags&1));
    f=request(0x3d00,"SAVEGAME\\SAVEGAME.099\\RENAME.SAV");assert(rg_file_service(&f) && !(f.flags&1));h=f.eax;
    f.eax=0x3f00;f.ebx=h;f.edx=ptr(guest+4096);f.ecx=32;
    assert(rg_file_service(&f) && !(f.flags&1) && f.eax==7 && !memcmp(guest+4096,"native ",7));
    f.eax=0x3e00;assert(rg_file_service(&f) && !(f.flags&1));
    f=request(0x4100,"MAPS\\ISLAND.TSG");assert(rg_file_service(&f) && !(f.flags&1));
    f=request(0x3d00,"MAPS\\ISLAND.TSG");assert(rg_file_service(&f) && (f.flags&1));
    f=request(0x3c00,"MAPS\\NATIVE.TSG");assert(rg_file_service(&f) && !(f.flags&1));h=f.eax;
    f.eax=0x3e00;f.ebx=h;assert(rg_file_service(&f) && !(f.flags&1));
    f.eax=0x1a00;f.edx=ptr(guest+6000);assert(rg_file_service(&f));
    for(unsigned i=0;i<70;++i) {
        f=request(0x4e00,"MAPS\\*.TSG");assert(rg_file_service(&f) && !(f.flags&1) && !strcmp(guest+6030,"NATIVE.TSG"));
        f.eax=0x4f00;assert(rg_file_service(&f) && (f.flags&1) && f.eax==18);
    }
    f=request(0x3c00,"MAPS\\ISLAND.RGM");assert(rg_file_service(&f) && (f.flags&1) && f.eax==5);
    f=request(0x3c00,"SAVEGAME\\..\\..\\escape.ini");assert(rg_file_service(&f) && (f.flags&1));
    assert(SetCurrentDirectoryA("..") && rg_user_storage_init("game","user"));
    assert(GetFileAttributesA("user\\MAPS\\ISLAND.TSG")==INVALID_FILE_ATTRIBUTES);
    expect("game\\SAVEGAME\\SAVEGAME.001\\SAVEGAME.SAV","original save");expect("game\\MAPS\\ISLAND.TSG","original map");expect("game\\SYSTEM.INI","original settings");
    puts("PASS Windows isolated saves/config, native create/write/seek/truncate/rename/read/delete, persistent deletion, merged map searches, immutable install and traversal rejection");
}
