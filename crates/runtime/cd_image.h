#ifndef RG_CD_IMAGE_H
#define RG_CD_IMAGE_H
#include "guest_abi.h"
int rg_cd_mount(const char *path);
void rg_cd_close(void);
int rg_cd_service(GuestFrame *frame);
#endif
