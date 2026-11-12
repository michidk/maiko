#define _GNU_SOURCE
#include "host_os.h"
#include "paths.h"
#include "user_storage.h"
#include <stdlib.h>
#include <stdio.h>
#include <string.h>
#include <unistd.h>
#include <sys/mman.h>
#include <sys/statvfs.h>
#include <time.h>
void rg_host_setup(void) {}
void rg_host_exit(unsigned code) {exit((int)code);}
int rg_host_readable(const void *p,size_t size) {
    if(!p || size>UINTPTR_MAX-(uintptr_t)p) return 0;
    FILE *maps=fopen("/proc/self/maps","r"); if(!maps) return 0;
    char line[512],mode[5]; unsigned long start,end; int found=0;
    while(fgets(line,sizeof line,maps)) {
        if(sscanf(line,"%lx-%lx %4s",&start,&end,mode)==3 && mode[0]=='r' &&
           (uintptr_t)p>=start && (uintptr_t)p+size<=end) {found=1;break;}
    }
    fclose(maps); return found;
}
int rg_host_lock(void *p,size_t size,int unlock) {return (unlock ? munlock(p,size) : mlock(p,size))==0;}
unsigned rg_host_cwd(unsigned capacity,char *buffer) {
    /* Original CRT stores DOS paths in fixed buffers. C: maps to the selected install. */
    char cwd[4096]; if(!getcwd(cwd,sizeof cwd)) return 0;
    const char *root=rg_game_root(); size_t n=strlen(root);
    if(strncmp(cwd,root,n) || (cwd[n] && cwd[n]!='/')) {
        root=rg_user_root();n=strlen(root);
        if(!n || strncmp(cwd,root,n) || (cwd[n] && cwd[n]!='/')) return 0;
    }
    const char *suffix=cwd+n; size_t size=3+(*suffix ? strlen(suffix+1) : 0)+1;
    if(!capacity) return (unsigned)size;
    if(capacity<size) return 0;
    strcpy(buffer,"C:\\");
    if(*suffix) strcpy(buffer+3,suffix+1);
    for(char *p=buffer;*p;++p) if(*p=='/') *p='\\';
    return (unsigned)strlen(buffer);
}
int rg_host_chdir(const char *path) {
    char resolved[4096]; return rg_resolve_path(path,resolved,sizeof resolved) && chdir(resolved)==0;
}
void rg_host_time(unsigned part[4]) {
    struct timespec ts; clock_gettime(CLOCK_REALTIME,&ts);
    struct tm tm; localtime_r(&ts.tv_sec,&tm);
    part[0]=tm.tm_hour;part[1]=tm.tm_min;part[2]=tm.tm_sec;part[3]=(unsigned)(ts.tv_nsec/10000000);
}

int rg_host_disk(unsigned drive,uint64_t *available,uint64_t *total) {
    if(drive && drive!=3) return 0;
    struct statvfs fs;if(statvfs(".",&fs)) return 0;
    *available=(uint64_t)fs.f_bavail*fs.f_frsize;
    *total=(uint64_t)fs.f_blocks*fs.f_frsize;return 1;
}
