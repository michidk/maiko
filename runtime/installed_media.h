#ifndef RG_INSTALLED_MEDIA_H
#define RG_INSTALLED_MEDIA_H
/* Call with game cwd selected. 0: absent, 1: mounted, -1: invalid bundle. */
int rg_gog_movies_mount(int (*mount_readonly)(const char *path));
#endif
