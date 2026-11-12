#ifndef RG_WIN_USER_STORAGE_H
#define RG_WIN_USER_STORAGE_H
#include <windows.h>
/* Directory handles remain open through each mutation to prevent parent moves. */
typedef struct {HANDLE directories[64];unsigned count;} RgStorageGuard;
int rg_storage_canonical_directory(const char *path,char output[MAX_PATH]);
int rg_storage_game_root(const char *path);
int rg_storage_game_selected(void);
int rg_user_storage_init(const char *game,const char *user);
int rg_storage_path(const char *requested,int writing,char output[MAX_PATH],RgStorageGuard *guard);
void rg_storage_release(RgStorageGuard *guard);
const char *rg_storage_root(void);
int rg_storage_maps(const char *pattern,char output[MAX_PATH]);
#endif
