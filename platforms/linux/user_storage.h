#ifndef RG_USER_STORAGE_H
#define RG_USER_STORAGE_H
/* Opt-in storage only. The source install is never writable through this API. */
int rg_user_storage_init(const char *directory);
const char *rg_user_root(void);
int rg_user_namespace(const char *relative);
/* Returns an opened parent directory, and the case-correct leaf. Caller closes. */
int rg_user_parent(const char *path,char leaf[256]);
#endif
