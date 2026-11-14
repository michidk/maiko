/* The same DOS movie-file contract runs against both native file adapters. */
#include "guest_abi.h"
#include "game_files.h"
#include <assert.h>
#include <stdio.h>
#include <string.h>
#ifdef _WIN32
#include <windows.h>
#include "windows_storage.h"
static void directory(const char *path) {assert(CreateDirectoryA(path,NULL));}
#else
#include "paths.h"
#include <sys/stat.h>
static void directory(const char *path) {assert(!mkdir(path,0700));}
#endif
static char guest[8192];
static uint32_t address(void *p) {assert((uintptr_t)p<=UINT32_MAX);return (uint32_t)(uintptr_t)p;}
static GuestFrame request(unsigned ax,const char *path) {
    GuestFrame f={0};strcpy(guest,path);f.eax=ax;f.edx=address(guest);return f;
}
static void put(const char *path,const char *text) {
    FILE *f=fopen(path,"wb");assert(f);assert(fwrite(text,1,strlen(text),f)==strlen(text));assert(!fclose(f));
}
static void read_movie(const char *path,const char *expected) {
    GuestFrame f=request(0x3d00,path);
    assert(rg_file_service(&f) && !(f.flags&1));unsigned handle=f.eax;
    f.eax=0x3f00;f.ebx=handle;f.edx=address(guest+4096);f.ecx=4096;
    assert(rg_file_service(&f) && !(f.flags&1) && f.eax==strlen(expected));
    assert(!memcmp(guest+4096,expected,strlen(expected)));
    f.eax=0x3e00;assert(rg_file_service(&f) && !(f.flags&1));
}
static void denied(unsigned ax,const char *path) {
    GuestFrame f=request(ax,path);
    assert(rg_file_service(&f) && (f.flags&1));
}
int main(void) {
    setbuf(stdout,NULL);
    directory("game");directory("game/mOvIeS");directory("game/ANIMS");directory("game/VIDEO");
    put("game/INTRO.SMK","root");put("game/mOvIeS/intro.smk","movies");
    put("game/ANIMS/INTRO.SMK","anims");put("game/VIDEO/INTRO.SMK","video");
    put("OUTSIDE.SMK","outside");put("game/README.TXT","asset");
#ifdef _WIN32
    assert(rg_user_storage_init("game","user"));
#else
    assert(rg_path_root("game"));
#endif
    read_movie("D:\\INTRO.SMK","root");
    read_movie("d:/intro.smk","root");
    GuestFrame f=request(0x4300,"D:\\INTRO.SMK");assert(rg_file_service(&f) && !(f.flags&1) && !(f.ecx&16));
    denied(0x3d01,"D:\\INTRO.SMK");denied(0x3d02,"D:\\INTRO.SMK");
    denied(0x3c00,"D:\\INTRO.SMK");denied(0x4100,"D:\\INTRO.SMK");
    f=request(0x5600,"D:\\INTRO.SMK");strcpy(guest+2048,"C:\\NEW.SMK");f.edi=address(guest+2048);
    assert(rg_file_service(&f) && (f.flags&1));
    read_movie("D:\\INTRO.SMK","root");
    denied(0x3d00,"D:\\..\\OUTSIDE.SMK");denied(0x3d00,"D:\\MOVIES\\INTRO.SMK");
    denied(0x3d00,"D:\\*.SMK");denied(0x3d00,"D:\\README.TXT");
    /* A CD release installed from another drive asks for that drive (animation_drive). */
    read_movie("E:\\INTRO.SMK","root");
    f=request(0x3d00,"D:\\ABSENT.SMK");assert(rg_file_service(&f) && (f.flags&1) && f.eax==2);
    assert(!remove("game/INTRO.SMK"));read_movie("D:\\INTRO.SMK","movies");
    assert(!remove("game/mOvIeS/intro.smk"));read_movie("D:\\INTRO.SMK","anims");
    assert(!remove("game/ANIMS/INTRO.SMK"));read_movie("D:\\INTRO.SMK","video");
    directory("game/INTRO.SMK");read_movie("D:\\INTRO.SMK","video");
    read_movie("C:\\README.TXT","asset");
    puts("PASS installed movies without an image: shared folder order, case, read/attributes, write rejection and path boundaries");
}
