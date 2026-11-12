#include "host_os.h"
#include "host_memory.h"
#include <mach/mach.h>
#include <mach/mach_vm.h>
#include <sys/sysctl.h>

static kern_return_t region(mach_vm_address_t *address,mach_vm_size_t *size,
                            vm_region_basic_info_data_64_t *info) {
    mach_msg_type_number_t count=VM_REGION_BASIC_INFO_COUNT_64;
    mach_port_t object=MACH_PORT_NULL;
    kern_return_t result=mach_vm_region(mach_task_self(),address,size,
        VM_REGION_BASIC_INFO_64,(vm_region_info_t)info,&count,&object);
    if(object!=MACH_PORT_NULL) mach_port_deallocate(mach_task_self(),object);
    return result;
}
int rg_host_readable(const void *pointer,size_t size) {
    uintptr_t begin=(uintptr_t)pointer;
    if(!begin || size>UINTPTR_MAX-begin) return 0;
    mach_vm_address_t cursor=begin,end=begin+size;
    do {
        mach_vm_address_t address=cursor;mach_vm_size_t length;
        vm_region_basic_info_data_64_t info;
        if(region(&address,&length,&info)!=KERN_SUCCESS || address>cursor ||
           !(info.protection&VM_PROT_READ) || length>UINT64_MAX-address) return 0;
        cursor=address+length;
        if(cursor<=address) return 0;
    } while(cursor<end);
    return 1;
}
int rg_mac_memory(uint64_t *available,uint64_t *total) {
    size_t length=sizeof *total;
    if(sysctlbyname("hw.memsize",total,&length,NULL,0) || length!=sizeof *total) return 0;
    vm_statistics64_data_t statistics;
    mach_msg_type_number_t count=HOST_VM_INFO64_COUNT;
    vm_size_t page;
    mach_port_t host=mach_host_self();
    kern_return_t size_result=host_page_size(host,&page);
    kern_return_t stats_result=host_statistics64(host,HOST_VM_INFO64,
        (host_info64_t)&statistics,&count);
    mach_port_deallocate(mach_task_self(),host);
    if(size_result!=KERN_SUCCESS || stats_result!=KERN_SUCCESS) return 0;
    *available=((uint64_t)statistics.free_count+statistics.inactive_count)*page;
    if(*available>*total) *available=*total;
    return 1;
}
int rg_mac_guest_space(uint64_t *available,uint64_t *largest) {
    /* Same explicit low-address arena used by the current non-PIE x64 host.
       Native ARM64 must use SR pointer offsets and a different ABI adapter. */
    const uint64_t end=0xe0000000;
    mach_vm_address_t cursor=0x20000000;
    *available=*largest=0;
    while(cursor<end) {
        mach_vm_address_t address=cursor;mach_vm_size_t length;
        vm_region_basic_info_data_64_t info;
        kern_return_t result=region(&address,&length,&info);
        if(result!=KERN_SUCCESS && result!=KERN_INVALID_ADDRESS) return 0;
        if(result==KERN_INVALID_ADDRESS || address>end) address=end;
        if(address>cursor) {
            uint64_t gap=address-cursor;
            *available+=gap;if(gap>*largest) *largest=gap;
        }
        if(address==end) break;
        if(!length || length>UINT64_MAX-address) return 0;
        cursor=address+length;
    }
    return 1;
}
