#include "input.h"
#include <SDL.h>
#include <assert.h>
#include <stdio.h>
static void push(SDL_Event *event) {assert(SDL_PushEvent(event)==1);assert(rg_input_pump());}
int main(void) {
    RgWindow window=rg_input_window(640,480);assert(window);assert(rg_input_pump());
    while(rg_input_take_scan()>=0) {}
    const struct {SDL_Scancode key;unsigned code;} cases[]={
        {SDL_SCANCODE_KP_0,0x52},{SDL_SCANCODE_KP_1,0x4f},{SDL_SCANCODE_KP_2,0x50},
        {SDL_SCANCODE_KP_3,0x51},{SDL_SCANCODE_KP_4,0x4b},{SDL_SCANCODE_KP_5,0x4c},
        {SDL_SCANCODE_KP_6,0x4d},{SDL_SCANCODE_KP_7,0x47},{SDL_SCANCODE_KP_8,0x48},
        {SDL_SCANCODE_KP_9,0x49},{SDL_SCANCODE_KP_PERIOD,0x53},
        {SDL_SCANCODE_KP_PLUS,0x4e},{SDL_SCANCODE_KP_MINUS,0x4a},{SDL_SCANCODE_KP_MULTIPLY,0x37},
        {SDL_SCANCODE_KP_DIVIDE,0x135},{SDL_SCANCODE_KP_ENTER,0x11c},
        {SDL_SCANCODE_F11,0x57},{SDL_SCANCODE_F12,0x58},{SDL_SCANCODE_NONUSBACKSLASH,0x56}
    };
    for(unsigned i=0;i<sizeof cases/sizeof cases[0];++i) {
        SDL_Event key={0};key.type=SDL_KEYDOWN;key.key.keysym.scancode=cases[i].key;push(&key);
        if(cases[i].code&256) assert(rg_input_take_scan()==0xe0);
        assert(rg_input_take_scan()==(int)(cases[i].code&127));
        key.type=SDL_KEYUP;push(&key);
        if(cases[i].code&256) assert(rg_input_take_scan()==0xe0);
        assert(rg_input_take_scan()==(int)((cases[i].code&127)|128));
        assert(rg_input_take_scan()==-1);
    }
    puts("PASS SDL keypad, F11/F12 and non-US key press/release scancodes");
    SDL_Event event={0};event.type=SDL_KEYDOWN;event.key.keysym.scancode=SDL_SCANCODE_UP;push(&event);
    assert(rg_input_take_scan()==0xe0 && rg_input_take_scan()==0x48);
    event.type=SDL_WINDOWEVENT;event.window.event=SDL_WINDOWEVENT_FOCUS_LOST;push(&event);
    assert(rg_input_take_scan()==0xe0 && rg_input_take_scan()==0xc8 && rg_input_take_scan()==-1);
    event.type=SDL_KEYDOWN;event.key.keysym.scancode=SDL_SCANCODE_A;push(&event);assert(rg_input_take_scan()==-1);
    event.type=SDL_WINDOWEVENT;event.window.event=SDL_WINDOWEVENT_FOCUS_GAINED;push(&event);
    event.type=SDL_MOUSEMOTION;event.motion.x=100;event.motion.y=60;push(&event);
    event.motion.x=120;event.motion.y=70;push(&event);
    event.type=SDL_MOUSEBUTTONDOWN;event.button.button=SDL_BUTTON_LEFT;push(&event);
    uint32_t regs[7]={3};assert(rg_mouse_service(regs));assert(regs[1]==1 && regs[2]==240 && regs[3]==70);
    regs[0]=0xb;assert(rg_mouse_service(regs));assert(regs[2]==20 && regs[3]==10);
    regs[0]=0xb;assert(rg_mouse_service(regs));assert(!regs[2] && !regs[3]);
    event.type=SDL_WINDOWEVENT;event.window.event=SDL_WINDOWEVENT_FOCUS_LOST;push(&event);
    regs[0]=3;assert(rg_mouse_service(regs));assert(!regs[1]);
    event.type=SDL_QUIT;assert(SDL_PushEvent(&event)==1);assert(!rg_input_pump());
    puts("PASS SDL extended keys, focus releases, mouse position/deltas/buttons and close");
}
