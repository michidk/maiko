#include "guest_abi.h"
#include "crash.h"
#include <assert.h>
#include <pthread.h>
#include <string.h>
extern void rg_unresolved_far(void);
static void *trap(void *unused) {
    (void)unused;
    void *memory=rg_low_alloc(65536);assert(memory);
    GuestRegs regs={0};regs.esp=(uint32_t)(uintptr_t)memory+65536;
    rg_call((uint32_t)(uintptr_t)rg_unresolved_far,&regs);
    assert(!"Original UD2 must not return");return NULL;
}
int main(int argc,char **argv) {
    rg_crash_install();
    if(argc>1 && !strcmp(argv[1],"--worker")) {
        pthread_t thread;assert(!pthread_create(&thread,NULL,trap,NULL));pthread_join(thread,NULL);
    } else trap(NULL);
    return 1;
}
