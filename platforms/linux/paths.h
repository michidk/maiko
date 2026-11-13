#ifndef RG_LINUX_PATHS_H
#define RG_LINUX_PATHS_H
#include <stddef.h>
int rg_path_root(const char *path);
const char *rg_game_root(void);
int rg_resolve_path(const char *path,char *out,size_t capacity);
int rg_relative_path(const char *path,char *out,size_t capacity);
#endif
