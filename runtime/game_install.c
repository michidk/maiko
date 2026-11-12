#include "game_install.h"
#include "installed_media.h"
#include "cd_image.h"
#include "game_build.h"
#include "windows_storage.h"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <windows.h>
#include <bcrypt.h>
int rg_choose_game_directory(char *path, size_t capacity);
uint32_t rg_exe_variant;
static int validate_image(const char *path) {
    rg_exe_variant=0;
    HANDLE file=CreateFileA(path,GENERIC_READ,FILE_SHARE_READ,NULL,OPEN_EXISTING,0,NULL);
    if (file==INVALID_HANDLE_VALUE) {
        fprintf(stderr,"Original RGFX.EXE not found: %s\n",path); return 0;
    }
    BCRYPT_ALG_HANDLE algorithm=NULL;
    BCRYPT_HASH_HANDLE hash=NULL;
    PUCHAR object=NULL;
    DWORD size=0,received=0,count;
    unsigned char digest[32], buffer[16384];
    int valid=0;
    if (BCryptOpenAlgorithmProvider(&algorithm,BCRYPT_SHA256_ALGORITHM,NULL,0)<0 ||
        BCryptGetProperty(algorithm,BCRYPT_OBJECT_LENGTH,(PUCHAR)&size,sizeof size,&received,0)<0)
        goto done;
    object=malloc(size);
    if (!object || BCryptCreateHash(algorithm,&hash,object,size,NULL,0,0)<0) goto done;
    for (;;) {
        if (!ReadFile(file,buffer,sizeof buffer,&count,NULL)) goto done;
        if (!count) break;
        if (BCryptHashData(hash,buffer,count,0)<0) goto done;
    }
    if (BCryptFinishHash(hash,digest,sizeof digest,0)<0) goto done;
    char hex[65];
    for (unsigned i=0;i<32;++i) sprintf(hex+i*2,"%02x",digest[i]);
    valid=strcmp(hex,RGFX_SHA256)==0;
    if (!valid && !strcmp(hex,RGFX_ALTERNATE_SHA256)) {
        valid=1; rg_exe_variant=1;
    }
    if (valid) fprintf(stderr,"Recognized RGFX.EXE variant=%u sha256=%s\n",rg_exe_variant,hex);
    else fprintf(stderr,"Unsupported RGFX.EXE build: %s; supported %s or %s\n",hex,RGFX_SHA256,RGFX_ALTERNATE_SHA256);
done:
    if (hash) BCryptDestroyHash(hash);
    if (algorithm) BCryptCloseAlgorithmProvider(algorithm,0);
    free(object); CloseHandle(file);
    if (!valid) fputs("Game image validation failed before original code execution\n",stderr);
    return valid;
}
int rg_install_select(int argc, char **argv) {
    char root[MAX_PATH], image[MAX_PATH];
    const char *game=NULL,*user=NULL,*cd=NULL;int choose=0;
    for(int i=1;i<argc;++i) {
        if(!strcmp(argv[i],"--game-dir") && i+1<argc && !game) game=argv[++i];
        else if(!strcmp(argv[i],"--user-dir") && i+1<argc && !user) user=argv[++i];
        else if(!strcmp(argv[i],"--cd-image") && i+1<argc && !cd) cd=argv[++i];
        else if(!strcmp(argv[i],"--choose-game-dir") && !choose) choose=1;
        else {fputs("Usage: redguard-native.exe [--game-dir GAME_DIRECTORY | --choose-game-dir] [--user-dir SAVE_DIRECTORY] [--cd-image GAME_GOG_OR_ISO]\n",stderr);return 0;}
    }
    if(game && choose) return 0;
    if(game) {
        DWORD size=GetFullPathNameA(game,sizeof root,root,NULL);
        if(!size || size>=sizeof root) return 0;
    } else if(choose) {
        if(!rg_choose_game_directory(root,sizeof root)) return 0;
    } else {
        DWORD size=GetModuleFileNameA(NULL,root,sizeof root);
        if(!size || size>=sizeof root || !strrchr(root,'\\')) return 0;
        *strrchr(root,'\\')=0;
        if(snprintf(image,sizeof image,"%s\\RGFX.EXE",root)>=(int)sizeof image) return 0;
        if(GetFileAttributesA(image)==INVALID_FILE_ATTRIBUTES && !rg_choose_game_directory(root,sizeof root)) return 0;
    }
    if(!rg_storage_canonical_directory(root,root)) {fprintf(stderr,"Game directory resolution failed error=%lu\n",GetLastError());return 0;}
    if (snprintf(image,sizeof image,"%s\\RGFX.EXE",root)>=(int)sizeof image) return 0;
    if(!validate_image(image)) return 0;
    if(!rg_storage_game_root(root)) return 0;
    if(user && !rg_user_storage_init(root,user)) {fprintf(stderr,"User storage failed error=%lu\n",GetLastError());return 0;}
    if(cd && !rg_cd_mount(cd)) return 0;
    if(!SetCurrentDirectoryA(root)) return 0;
    if(!cd && rg_gog_movies_mount(rg_cd_mount)<0) return 0;
    fprintf(stderr,"Verified original game directory: %s\n",root);
    return 1;
}
