#ifndef RG_MAC_HOST_MEMORY_H
#define RG_MAC_HOST_MEMORY_H
#include <stdint.h>
int rg_mac_memory(uint64_t *available,uint64_t *total);
int rg_mac_guest_space(uint64_t *available,uint64_t *largest);
#endif
