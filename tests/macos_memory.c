#include "host_os.h"
#include "host_memory.h"
#include <assert.h>
#include <stdio.h>
#include <unistd.h>
#include <sys/mman.h>
int main(void) {
    unsigned char stack_byte=0;
    assert(rg_host_readable(&stack_byte,1));
    assert(rg_host_readable((const void *)(uintptr_t)main,1));
    assert(rg_host_readable((const void *)(uintptr_t)printf,1));
    size_t page=(size_t)sysconf(_SC_PAGESIZE);
    void *memory=mmap(NULL,page*3,PROT_READ|PROT_WRITE,MAP_PRIVATE|MAP_ANONYMOUS,-1,0);
    assert(memory!=MAP_FAILED);
    assert(rg_host_readable(memory,page*3));
    assert(!rg_host_readable(NULL,1));
    assert(!rg_host_readable((void *)(uintptr_t)(UINTPTR_MAX-3),8));
    assert(!mprotect((char *)memory+page,page,PROT_NONE));
    assert(!rg_host_readable(memory,page*3));
    assert(!rg_host_readable((char *)memory+page,1));
    assert(rg_host_readable(memory,page));
    assert(!munmap(memory,page*3));
    uint64_t available,total,largest;
    assert(rg_mac_memory(&available,&total) && total>0 && available<=total);
    assert(rg_mac_guest_space(&available,&largest) && largest<=available && available<=0xc0000000);
    printf("PASS Darwin readability/protection and memory queries; host RAM=%llu low-arena available=%llu largest=%llu; arena mapping not tested\n",
        (unsigned long long)total,(unsigned long long)available,(unsigned long long)largest);
    return 0;
}
