#define _GNU_SOURCE
#include "paths.h"
#include "user_storage.h"
#include <stdio.h>
#include <string.h>
#include <strings.h>
#include <stdlib.h>
#include <unistd.h>
#include <dirent.h>
#include <sys/stat.h>
#include <errno.h>
static char root[4096];
const char *rg_game_root(void) {return root;}
int rg_path_root(const char *path) {
    struct stat st;
    return realpath(path,root) && !stat(root,&st) && S_ISDIR(st.st_mode);
}
static int component(char *parent,const char *name,size_t capacity) {
    size_t length=strlen(parent);
    if(length+strlen(name)+2>capacity) {errno=ENAMETOOLONG;return 0;}
    parent[length]='/';strcpy(parent+length+1,name);
    struct stat st; if(!stat(parent,&st)) return 1;
    parent[length]=0;
    DIR *dir=opendir(parent); if(!dir) return 0;
    char match[256]={0};struct dirent *entry;unsigned matches=0;
    while((entry=readdir(dir))) if(!strcasecmp(entry->d_name,name)) {
        if(strlen(entry->d_name)>=sizeof match) continue;
        strcpy(match,entry->d_name);++matches;
    }
    closedir(dir);
    if(matches!=1) {errno=matches ? EEXIST : ENOENT;return 0;}
    parent[length]='/';strcpy(parent+length+1,match);return 1;
}
int rg_relative_path(const char *path,char *out,size_t capacity) {
    char normalized[4096]; size_t size=strlen(path);
    if(!root[0] || size>=sizeof normalized || !capacity) {errno=ENAMETOOLONG;return 0;}
    strcpy(normalized,path);
    for(char *p=normalized;*p;++p) if(*p=='\\') *p='/';
    char *relative=normalized;
    if(relative[0] && relative[1]==':') {
        if(relative[0]!='C' && relative[0]!='c') {errno=ENOENT;return 0;}
        relative+=2;out[0]=0;
    } else if(*relative=='/') out[0]=0;
    else {
        char cwd[4096];if(!getcwd(cwd,sizeof cwd)) return 0;
        const char *base=root;size_t n=strlen(base);
        if(strncmp(cwd,base,n) || (cwd[n] && cwd[n]!='/')) {
            base=rg_user_root();n=strlen(base);
            if(!n || strncmp(cwd,base,n) || (cwd[n] && cwd[n]!='/')) {errno=EACCES;return 0;}
        }
        const char *suffix=cwd+n;while(*suffix=='/') ++suffix;
        if(strlen(suffix)>=capacity) {errno=ENAMETOOLONG;return 0;}
        strcpy(out,suffix);
    }
    char *save=NULL,*part=strtok_r(relative,"/",&save);
    while(part) {
        if(!strcmp(part,".")) {}
        else if(!strcmp(part,"..")) {
            if(!*out) {errno=EACCES;return 0;}
            char *slash=strrchr(out,'/');if(slash) *slash=0;else *out=0;
        } else {
            size_t used=strlen(out),n=strlen(part);
            if(used+n+2>capacity || strchr(part,':')) {errno=EACCES;return 0;}
            if(used) out[used++]='/';
            strcpy(out+used,part);
        }
        part=strtok_r(NULL,"/",&save);
    }
    return 1;
}
int rg_resolve_path(const char *path,char *out,size_t capacity) {
    char relative[4096];if(!rg_relative_path(path,relative,sizeof relative)) return 0;
    const char *base=rg_user_namespace(relative) ? rg_user_root() : root;
    if(strlen(base)>=capacity) {errno=ENAMETOOLONG;return 0;}
    strcpy(out,base);
    char *save=NULL,*part=strtok_r(relative,"/",&save);
    while(part) {
        char *next=strtok_r(NULL,"/",&save);
        if(!component(out,part,capacity)) {
            if(next || errno!=ENOENT) return 0;
            const char *dot=strchr(part,'.');size_t name=dot ? (size_t)(dot-part) : strlen(part);
            size_t ext=dot ? strlen(dot+1) : 0;
            if(name<=8 && ext<=3) return 0;
            char short_name[13];size_t used=name>8 ? 8 : name;
            memcpy(short_name,part,used);
            if(dot) {short_name[used++]='.';size_t n=ext>3 ? 3 : ext;memcpy(short_name+used,dot+1,n);used+=n;}
            short_name[used]=0;if(!component(out,short_name,capacity)) return 0;
        }
        part=next;
    }
    return 1;
}
