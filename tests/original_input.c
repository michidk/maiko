#include <assert.h>
#include <stdio.h>
#include <string.h>
#ifdef _WIN32
#include <windows.h>
#else
#include <SDL.h>
#endif
#include "guest_abi.h"
#include "input.h"
extern void rg_key_event(void);
extern unsigned short keyboard_state[128];
extern unsigned char keyboard_caps,keyboard_scroll,keyboard_num;
static void event(GuestRegs *r,unsigned scan) {
    uint32_t initial=r->esp;
    r->eax=scan;
    rg_call((uint32_t)(uintptr_t)rg_key_event,r);
    assert(r->esp==initial);
}
int main(void) {
    GuestRegs r={0};
    void *stack=rg_low_alloc(65536); assert(stack);
    r.esp=(uint32_t)(uintptr_t)stack+65536;
    memset(keyboard_state,0,256);
    event(&r,0x1e); assert(keyboard_state[0x1e]==1);
    event(&r,0x9e); assert(keyboard_state[0x1e]==0);
    event(&r,0x2a); assert(keyboard_state[0x2a]==1);
    event(&r,0xaa); assert(keyboard_state[0x2a]==0);
    keyboard_caps=keyboard_scroll=keyboard_num=0;
    event(&r,0x3a); assert(keyboard_caps==1);
    event(&r,0xba); event(&r,0x3a); assert(keyboard_caps==0);
    event(&r,0x45); assert(keyboard_num==1);
    event(&r,0x46); assert(keyboard_scroll==1);
    GuestFrame frame={0};
    rg_service(16,&frame);
    RgWindow window=rg_input_window(640,480); assert(window);
#ifdef _WIN32
    SendMessage(window,WM_SETFOCUS,0,0);
    assert(PostMessage(window,WM_KEYDOWN,VK_ESCAPE,1|(1<<16)));
#else
    SDL_Event press={0};press.type=SDL_KEYDOWN;press.key.keysym.scancode=SDL_SCANCODE_ESCAPE;
    assert(SDL_PushEvent(&press)==1);
#endif
    GuestFrame saved=frame;
    rg_service(23,&frame);
    assert(keyboard_state[1]==1 && !memcmp(&frame,&saved,sizeof frame));
#ifdef _WIN32
    assert(PostMessage(window,WM_KEYUP,VK_ESCAPE,1|(1<<16)|(3u<<30)));
#else
    press.type=SDL_KEYUP;assert(SDL_PushEvent(&press)==1);
#endif
    rg_service(23,&frame);
    assert(!keyboard_state[1] && !memcmp(&frame,&saved,sizeof frame));
    puts("PASS original key translation receives queued Escape press/release during native wait poll");
    assert(rg_low_free(stack));
    puts("PASS original translated keyboard handler press/release, shift, lock toggles and stack balance");
}
