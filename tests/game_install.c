#include <assert.h>
#include <stdio.h>
#include <string.h>
#include <windows.h>
#include "game_install.h"
static int picker_calls, picker_cancel;
int rg_choose_game_directory(char *path, size_t capacity) {
    ++picker_calls;
    if (picker_cancel) return 0;
    assert(capacity > strlen("Z:\\game"));
    strcpy(path,"Z:\\game");
    return 1;
}
int main(void) {
    char *explicit_dir[]={"test","--game-dir","Z:\\game"};
    assert(rg_install_select(3,explicit_dir) && rg_exe_variant==0);
    char current[MAX_PATH];
    assert(GetCurrentDirectoryA(sizeof current,current) && !strcmp(current,"Z:\\game"));
    char *missing[]={"test","--game-dir","Z:\\work\\missing"};
    assert(!rg_install_select(3,missing));
    char *unknown[]={"test","--invalid"};
    assert(!rg_install_select(2,unknown));
    char executable[MAX_PATH];
    assert(GetModuleFileNameA(NULL,executable,sizeof executable));
    *strrchr(executable,'\\')=0;
    assert(SetCurrentDirectoryA(executable));
    HANDLE bad=CreateFileA("RGFX.EXE",GENERIC_WRITE,0,NULL,CREATE_NEW,0,NULL);
    assert(bad!=INVALID_HANDLE_VALUE); CloseHandle(bad);
    char *defaults[]={"test"};
    assert(!rg_install_select(1,defaults));
    assert(DeleteFileA("RGFX.EXE"));
    assert(CopyFileA("Z:\\game\\RGFX.EXE","RGFX.EXE",TRUE));
    assert(SetCurrentDirectoryA("C:\\"));
    assert(rg_install_select(1,defaults));
    assert(GetCurrentDirectoryA(sizeof current,current) && !strcmp(current,executable));
    HANDLE patched=CreateFileA("RGFX.EXE",GENERIC_READ|GENERIC_WRITE,0,NULL,OPEN_EXISTING,0,NULL);
    assert(patched!=INVALID_HANDLE_VALUE);
    const DWORD offsets[]={0x9a041,0xf256b,0xf256c};
    const unsigned char before[]={0x74,0xb9,0x02}, after[]={0xeb,0xeb,0x73};
    DWORD count;
    for (unsigned i=0;i<3;++i) {
        unsigned char byte;
        assert(SetFilePointer(patched,offsets[i],NULL,FILE_BEGIN)==offsets[i]);
        assert(ReadFile(patched,&byte,1,&count,NULL) && count==1 && byte==before[i]);
        assert(SetFilePointer(patched,offsets[i],NULL,FILE_BEGIN)==offsets[i]);
        assert(WriteFile(patched,after+i,1,&count,NULL) && count==1);
    }
    assert(CloseHandle(patched));
    assert(rg_install_select(1,defaults) && rg_exe_variant==1);
    patched=CreateFileA("RGFX.EXE",GENERIC_WRITE,0,NULL,OPEN_EXISTING,0,NULL);
    assert(patched!=INVALID_HANDLE_VALUE);
    unsigned char changed=0;
    assert(WriteFile(patched,&changed,1,&count,NULL) && count==1);
    assert(CloseHandle(patched));
    assert(!rg_install_select(1,defaults));
    puts("PASS exact known three-byte variant accepted; additional changes rejected");
    assert(DeleteFileA("RGFX.EXE"));
    char *choose[]={"test","--choose-game-dir"};
    assert(rg_install_select(2,choose));
    assert(picker_calls==1);
    assert(GetCurrentDirectoryA(sizeof current,current) && !strcmp(current,"Z:\\game"));
    assert(rg_install_select(1,defaults));
    assert(picker_calls==2);
    picker_cancel=1;
    assert(!rg_install_select(2,choose));
    assert(picker_calls==3);
    puts("PASS folder selection, missing default fallback and cancellation");
    puts("PASS explicit/default game root, spaces, unrelated cwd, missing/wrong image and argument errors");
    return 0;
}
