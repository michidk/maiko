#include <assert.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <windows.h>
#include "crash.h"
extern volatile uint32_t rg_call_head,rg_call_ring[64];
static volatile LONG caught;
static DWORD owner;
static volatile LONG access_caught, custom_caught;
static LONG CALLBACK component_handler(EXCEPTION_POINTERS *info) {
    if (GetCurrentThreadId()==owner && (info->ExceptionRecord->ExceptionCode==0x406d1388 ||
        info->ExceptionRecord->ExceptionCode==EXCEPTION_ACCESS_VIOLATION ||
        info->ExceptionRecord->ExceptionCode==0xe04d4149)) {
        InterlockedIncrement(&caught);
        if(info->ExceptionRecord->ExceptionCode==EXCEPTION_ACCESS_VIOLATION) InterlockedIncrement(&access_caught);
        if(info->ExceptionRecord->ExceptionCode==0xe04d4149) InterlockedIncrement(&custom_caught);
        return EXCEPTION_CONTINUE_EXECUTION;
    }
    return EXCEPTION_CONTINUE_SEARCH;
}
static void unsafe_cleanup(void) { fputs("UNEXPECTED crash cleanup ran\n",stderr); }
static DWORD WINAPI producer(void *event) {
    for (unsigned i=0;i<64;++i) rg_call_ring[i]=0x12340000+i;
    rg_call_head=64;
    SetEvent(event);
    for (;;) {
        uint32_t head=rg_call_head;
        rg_call_ring[head&63]=head;
        InterlockedIncrement((volatile LONG *)&rg_call_head);
    }
    return 0;
}
int main(int argc,char **argv) {
    assert(argc==2);
    setbuf(stdout,NULL);setbuf(stderr,NULL);
    owner=GetCurrentThreadId();
    rg_crash_install();
    if (!strcmp(argv[1],"--handled")) {
        PVOID handler=AddVectoredExceptionHandler(0,component_handler); assert(handler);
        ULONG_PTR name[]={0x1000,(ULONG_PTR)"driver worker",GetCurrentThreadId(),0};
        RaiseException(0x406d1388,0,4,name);
        ULONG_PTR access[]={0,0x1234};
        RaiseException(EXCEPTION_ACCESS_VIOLATION,0,2,access);
        RaiseException(0xe04d4149,0,0,NULL);
        printf("Handled main-thread callbacks=%ld access=%ld custom=%ld\n",caught,access_caught,custom_caught);
        assert(access_caught==1 && custom_caught==1 && caught>=2 && caught<=3);
        puts("PASS component handles first-chance thread notification and access exception");
        return 0;
    }
    assert(!strcmp(argv[1],"--fatal-busy"));
    HANDLE event=CreateEventA(NULL,TRUE,FALSE,NULL);assert(event);
    HANDLE thread=CreateThread(NULL,0,producer,event,0,NULL);assert(thread);
    assert(WaitForSingleObject(event,3000)==WAIT_OBJECT_0);
    CloseHandle(thread);CloseHandle(event);
    assert(!atexit(unsafe_cleanup));
    __asm__ volatile("ud2");
    return 99;
}
