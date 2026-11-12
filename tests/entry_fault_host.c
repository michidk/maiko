#include "guest_abi.h"
#include <stdlib.h>
void rg_service(uint32_t service,GuestFrame *frame) {
    (void)service; (void)frame; abort();
}
