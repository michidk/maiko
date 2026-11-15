#ifndef RG_PLATFORM_SERVICES_H
#define RG_PLATFORM_SERVICES_H
#include "guest_abi.h"
void rg_platform_info(GuestFrame *frame);
int rg_video_service(GuestFrame *frame);
int rg_int386(GuestFrame *frame);
#endif
