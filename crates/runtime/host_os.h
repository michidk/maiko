#ifndef RG_HOST_OS_H
#define RG_HOST_OS_H
#include <stddef.h>
#include <stdint.h>
#ifdef __cplusplus
extern "C" {
#endif
void rg_host_setup(void);
void rg_host_exit(unsigned code);
int rg_host_readable(const void *p,size_t size);
int rg_host_lock(void *p,size_t size,int unlock);
unsigned rg_host_cwd(unsigned capacity,char *buffer);
int rg_host_chdir(const char *path);
int rg_host_disk(unsigned drive,uint64_t *available,uint64_t *total);
void rg_host_time(unsigned part[4]);
#ifdef __cplusplus
}
#endif
#endif
