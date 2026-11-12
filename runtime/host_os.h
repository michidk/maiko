#ifndef RG_HOST_OS_H
#define RG_HOST_OS_H
#include <stddef.h>
#include <stdint.h>
#ifdef _WIN32
#include <windows.h>
static inline void rg_host_setup(void) {SetErrorMode(SEM_FAILCRITICALERRORS|SEM_NOOPENFILEERRORBOX);}
static inline void rg_host_exit(unsigned code) {ExitProcess(code);}
static inline int rg_host_readable(const void *p,size_t size) {
    MEMORY_BASIC_INFORMATION m;
    return p && VirtualQuery(p,&m,sizeof m) && m.State==MEM_COMMIT &&
        !(m.Protect&(PAGE_NOACCESS|PAGE_GUARD)) &&
        (uintptr_t)p+size<=(uintptr_t)m.BaseAddress+m.RegionSize;
}
static inline int rg_host_lock(void *p,size_t size,int unlock) {
    return unlock ? VirtualUnlock(p,size)!=0 : VirtualLock(p,size)!=0;
}
unsigned rg_host_cwd(unsigned capacity,char *buffer);
int rg_host_chdir(const char *path);
static inline int rg_host_disk(unsigned drive,uint64_t *available,uint64_t *total) {
    if(drive>26) return 0;
    char root[]="A:\\";root[0]+=(char)(drive ? drive-1 : 0);
    ULARGE_INTEGER free_bytes,total_bytes;
    if(!GetDiskFreeSpaceExA(drive ? root : NULL,&free_bytes,&total_bytes,NULL)) return 0;
    *available=free_bytes.QuadPart;*total=total_bytes.QuadPart;return 1;
}
static inline void rg_host_time(unsigned part[4]) {
    SYSTEMTIME t; GetLocalTime(&t);
    part[0]=t.wHour; part[1]=t.wMinute; part[2]=t.wSecond; part[3]=t.wMilliseconds/10;
}
#else
void rg_host_setup(void);
void rg_host_exit(unsigned code);
int rg_host_readable(const void *p,size_t size);
int rg_host_lock(void *p,size_t size,int unlock);
unsigned rg_host_cwd(unsigned capacity,char *buffer);
int rg_host_chdir(const char *path);
int rg_host_disk(unsigned drive,uint64_t *available,uint64_t *total);
void rg_host_time(unsigned part[4]);
#endif
#endif
