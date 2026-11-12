#include <windows.h>
#include <assert.h>
#include <stdio.h>
#include "platform_services.h"
#include "input.h"
int main(void) {
    uint32_t *registers=rg_low_alloc(4096);
    assert(registers);
    GuestFrame frame={0};
    frame.eax=0x33; frame.edx=frame.ebx=(uintptr_t)registers;
    assert(rg_int386(&frame));
    assert(registers[0]==0xffff && registers[1]==2);
    HWND window=FindWindowA("RedguardNativeWindow",NULL);
    assert(window);
    SendMessage(window,WM_SETFOCUS,0,0);
    /* Warp coordinates are DOS mouse units: horizontal coordinate doubled. */
    registers[0]=4; registers[2]=200; registers[3]=100; frame.eax=0x33;
    assert(rg_int386(&frame));
    registers[0]=0xb; frame.eax=0x33; assert(rg_int386(&frame));
    SendMessage(window,WM_MOUSEMOVE,0,MAKELPARAM(120,90));
    registers[0]=0xb; frame.eax=0x33; assert(rg_int386(&frame));
    assert((int16_t)registers[2]==20 && (int16_t)registers[3]==-10);
    registers[0]=0xb; frame.eax=0x33; assert(rg_int386(&frame));
    assert(registers[2]==0 && registers[3]==0);
    SendMessage(window,WM_LBUTTONDOWN,MK_LBUTTON,MAKELPARAM(120,90));
    registers[0]=3; frame.eax=0x33; assert(rg_int386(&frame));
    assert(registers[1]==1 && registers[2]==240 && registers[3]==90);
    SendMessage(window,WM_KEYDOWN,'A',0x001e0000);
    assert(rg_input_take_scan()==0x1e);
    SendMessage(window,WM_KILLFOCUS,0,0);
    assert(rg_input_take_scan()==0x9e);
    assert(rg_input_take_scan()==-1);
    registers[0]=3; frame.eax=0x33; assert(rg_int386(&frame));
    assert(registers[1]==0);
    registers[0]=0xc; frame.eax=0x33; assert(!rg_int386(&frame));
    puts("PASS native mouse reset, motion consumption, DOS warp units, buttons and focus loss");
}
