#define _GNU_SOURCE
#include "game_install.h"
#include "installed_media.h"
#include "cd_image.h"
#include "game_build.h"
#include "paths.h"
#include "user_storage.h"
#include <openssl/evp.h>
#include <stdio.h>
#include <string.h>
#include <unistd.h>
uint32_t rg_exe_variant;
int rg_install_select(int argc,char **argv) {
    const char *game=NULL,*user=NULL,*cd=NULL;
    for(int i=1;i<argc;++i) {
        if(!strcmp(argv[i],"--game-dir") && i+1<argc && !game) game=argv[++i];
        else if(!strcmp(argv[i],"--user-dir") && i+1<argc && !user) user=argv[++i];
        else if(!strcmp(argv[i],"--cd-image") && i+1<argc && !cd) cd=argv[++i];
        else {game=NULL;break;}
    }
    if(!game) {
        fputs("Usage: redguard-native --game-dir GAME_DIRECTORY [--user-dir SAVE_DIRECTORY] [--cd-image GAME_GOG_OR_ISO]\n",stderr);return 0;
    }
    if(!rg_path_root(game)) {perror("Game directory");return 0;}
    if(user && !rg_user_storage_init(user)) {perror("User storage");return 0;}
    if(cd && !rg_cd_mount(cd)) return 0;
    if(chdir(rg_game_root())) {perror("Game directory");return 0;}
    char path[4096];if(!rg_resolve_path("C:\\RGFX.EXE",path,sizeof path)) {perror("RGFX.EXE");return 0;}
    FILE *file=fopen(path,"rb");if(!file) return 0;
    EVP_MD_CTX *ctx=EVP_MD_CTX_new();unsigned char buffer[16384],digest[32];unsigned size;size_t count;
    int ok=ctx && EVP_DigestInit_ex(ctx,EVP_sha256(),NULL);
    while(ok && (count=fread(buffer,1,sizeof buffer,file))) ok=EVP_DigestUpdate(ctx,buffer,count);
    ok=ok && !ferror(file) && EVP_DigestFinal_ex(ctx,digest,&size) && size==32;
    EVP_MD_CTX_free(ctx);fclose(file);if(!ok) return 0;
    char hex[65];for(unsigned i=0;i<32;++i) sprintf(hex+2*i,"%02x",digest[i]);
    if(!strcmp(hex,RGFX_SHA256)) rg_exe_variant=0;
    else if(!strcmp(hex,RGFX_ALTERNATE_SHA256)) rg_exe_variant=1;
    else {fprintf(stderr,"Unsupported RGFX.EXE sha256=%s\n",hex);return 0;}
    fprintf(stderr,"Recognized RGFX.EXE variant=%u sha256=%s\n",rg_exe_variant,hex);
    if(!cd && rg_gog_movies_mount(rg_cd_mount)<0) return 0;
    return 1;
}
