#include <assert.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include "guest_abi.h"
#include "game_install.h"
#include "host_os.h"
#include "crash.h"
extern void rg_init(void),rg_object_io_init(void),rg_object_file_read(void);
extern uint32_t g_1A9EC8,g_1A9ECC,g_1A9EB8,g_1A9EBC,g_1A9EC0,g_1A9EE9;
int main(void) {
    setbuf(stdout,NULL);rg_host_setup();rg_crash_install();
    char *args[]={"file-read-test","--game-dir","/game"};
    assert(rg_install_select(3,args));
    char *memory=rg_low_alloc(65536);void *stack=rg_low_alloc(1024*1024);
    assert(memory && stack && rg_host_cwd(240,memory));
    strcat(memory,"\\rgfx.exe");memory[256]=memory[257]=memory[512]=0;
    g_1A9EBC=(uint32_t)(uintptr_t)(memory+512);g_1A9EC0=(uint32_t)(uintptr_t)memory;
    g_1A9EE9=(uint32_t)(uintptr_t)(memory+256);g_1A9EC8=(uint32_t)(uintptr_t)stack;
    g_1A9ECC=g_1A9EB8=(uint32_t)(uintptr_t)stack+1024*1024;
    GuestRegs r={0};r.eax=255;r.esp=g_1A9ECC;
    rg_call((uint32_t)(uintptr_t)rg_init,&r);
    rg_call((uint32_t)(uintptr_t)rg_object_io_init,&r);
    FILE *source=fopen("SYSTEM.INI","rb");assert(source);
    unsigned char expected[32768];size_t size=fread(expected,1,sizeof expected,source);
    assert(size>0 && size<sizeof expected && !ferror(source));fclose(source);
    strcpy(memory+1024,"SYSTEM.INI");memset(memory+4096,0,32768);
    r.eax=(uint32_t)(uintptr_t)(memory+1024);r.edx=(uint32_t)(uintptr_t)(memory+4096);
    r.ebx=0x12345678;r.ecx=0x23456789;r.esi=0x456789ab;r.edi=0x56789abc;r.ebp=0x6789abcd;
    rg_call((uint32_t)(uintptr_t)rg_object_file_read,&r);
    printf("Original object-file read result=%u bytes=%zu first=%02x\n",r.eax,size,(unsigned char)memory[4096]);
    assert(r.eax==1 && !memcmp(memory+4096,expected,size));
    assert(r.esp==g_1A9ECC && r.ebx==0x12345678 && r.ecx==0x23456789);
    assert(r.esi==0x456789ab && r.edi==0x56789abc && r.ebp==0x6789abcd);
    r.eax=(uint32_t)(uintptr_t)(memory+1024);r.edx=0;
    rg_call((uint32_t)(uintptr_t)rg_object_file_read,&r);assert(!r.eax && r.esp==g_1A9ECC);
    assert(rg_low_free(memory) && rg_low_free(stack));
    puts("PASS original object-file reader accepts zero-filled storage, copies exact bytes and rejects null storage");
}
