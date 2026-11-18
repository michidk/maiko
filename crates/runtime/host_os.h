#ifndef RG_HOST_OS_H
#define RG_HOST_OS_H
#include <stddef.h>
#include <stdint.h>
#ifdef __cplusplus
extern "C" {
#endif
void rg_host_setup(void);
int rg_host_readable(const void *p,size_t size);
unsigned rg_host_cwd(unsigned capacity,char *buffer);
int rg_host_chdir(const char *path);
#ifdef __cplusplus
}
#endif
#endif
