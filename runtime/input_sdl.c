#include "input.h"
#include <SDL.h>
static SDL_Window *window;
static unsigned width=640,height=480,buttons;
static int x,y,dx,dy,known,closed,focused;
static unsigned char queue[256],held[512];static unsigned head,tail;
static void enqueue(unsigned code) {if(head-tail>=256) {closed=1;return;}queue[head++&255]=(unsigned char)code;}
static void scan_event(unsigned code,int release) {
    if(code&256) enqueue(0xe0);
    enqueue((code&127)|(release ? 128 : 0));held[code]=!release;
}
int rg_input_take_scan(void) {return head==tail ? -1 : queue[tail++&255];}
static unsigned dos_scan(SDL_Scancode key) {
    static const unsigned char letters[]={0x1e,0x30,0x2e,0x20,0x12,0x21,0x22,0x23,0x17,0x24,0x25,0x26,0x32,0x31,0x18,0x19,0x10,0x13,0x1f,0x14,0x16,0x2f,0x11,0x2d,0x15,0x2c};
    if(key>=SDL_SCANCODE_A && key<=SDL_SCANCODE_Z) return letters[key-SDL_SCANCODE_A];
    if(key>=SDL_SCANCODE_1 && key<=SDL_SCANCODE_9) return 2+key-SDL_SCANCODE_1;
    if(key>=SDL_SCANCODE_F1 && key<=SDL_SCANCODE_F10) return 0x3b+key-SDL_SCANCODE_F1;
    switch(key) {
    case SDL_SCANCODE_0:return 0xb;case SDL_SCANCODE_RETURN:return 0x1c;
    case SDL_SCANCODE_ESCAPE:return 1;case SDL_SCANCODE_SPACE:return 0x39;
    case SDL_SCANCODE_TAB:return 0xf;case SDL_SCANCODE_BACKSPACE:return 0xe;
    case SDL_SCANCODE_MINUS:return 0xc;case SDL_SCANCODE_EQUALS:return 0xd;
    case SDL_SCANCODE_LEFTBRACKET:return 0x1a;case SDL_SCANCODE_RIGHTBRACKET:return 0x1b;
    case SDL_SCANCODE_SEMICOLON:return 0x27;case SDL_SCANCODE_APOSTROPHE:return 0x28;
    case SDL_SCANCODE_GRAVE:return 0x29;case SDL_SCANCODE_BACKSLASH:return 0x2b;
    case SDL_SCANCODE_COMMA:return 0x33;case SDL_SCANCODE_PERIOD:return 0x34;case SDL_SCANCODE_SLASH:return 0x35;
    case SDL_SCANCODE_LSHIFT:return 0x2a;case SDL_SCANCODE_RSHIFT:return 0x36;
    case SDL_SCANCODE_LCTRL:return 0x1d;case SDL_SCANCODE_RCTRL:return 0x11d;
    case SDL_SCANCODE_LALT:return 0x38;case SDL_SCANCODE_RALT:return 0x138;
    case SDL_SCANCODE_CAPSLOCK:return 0x3a;case SDL_SCANCODE_NUMLOCKCLEAR:return 0x45;
    case SDL_SCANCODE_SCROLLLOCK:return 0x46;
    case SDL_SCANCODE_UP:return 0x148;case SDL_SCANCODE_DOWN:return 0x150;
    case SDL_SCANCODE_LEFT:return 0x14b;case SDL_SCANCODE_RIGHT:return 0x14d;
    case SDL_SCANCODE_HOME:return 0x147;case SDL_SCANCODE_END:return 0x14f;
    case SDL_SCANCODE_PAGEUP:return 0x149;case SDL_SCANCODE_PAGEDOWN:return 0x151;
    case SDL_SCANCODE_INSERT:return 0x152;case SDL_SCANCODE_DELETE:return 0x153;
    case SDL_SCANCODE_KP_0:return 0x52;case SDL_SCANCODE_KP_1:return 0x4f;
    case SDL_SCANCODE_KP_2:return 0x50;case SDL_SCANCODE_KP_3:return 0x51;
    case SDL_SCANCODE_KP_4:return 0x4b;case SDL_SCANCODE_KP_5:return 0x4c;
    case SDL_SCANCODE_KP_6:return 0x4d;case SDL_SCANCODE_KP_7:return 0x47;
    case SDL_SCANCODE_KP_8:return 0x48;case SDL_SCANCODE_KP_9:return 0x49;
    case SDL_SCANCODE_KP_PERIOD:return 0x53;case SDL_SCANCODE_KP_PLUS:return 0x4e;
    case SDL_SCANCODE_KP_MINUS:return 0x4a;case SDL_SCANCODE_KP_MULTIPLY:return 0x37;
    case SDL_SCANCODE_KP_DIVIDE:return 0x135;case SDL_SCANCODE_KP_ENTER:return 0x11c;
    case SDL_SCANCODE_F11:return 0x57;case SDL_SCANCODE_F12:return 0x58;
    case SDL_SCANCODE_NONUSBACKSLASH:return 0x56;
    default:return 0;
    }
}
RgWindow rg_input_window(unsigned w,unsigned h) {
    if(!w || !h || w>8192 || h>8192) return NULL;
    width=w;height=h;
    if(!window) {
        if(SDL_InitSubSystem(SDL_INIT_VIDEO|SDL_INIT_EVENTS)) return NULL;
        SDL_GL_SetAttribute(SDL_GL_CONTEXT_MAJOR_VERSION,2);SDL_GL_SetAttribute(SDL_GL_CONTEXT_MINOR_VERSION,1);
        SDL_GL_SetAttribute(SDL_GL_DOUBLEBUFFER,1);SDL_GL_SetAttribute(SDL_GL_DEPTH_SIZE,24);
        window=SDL_CreateWindow("Redguard native experiment",SDL_WINDOWPOS_UNDEFINED,SDL_WINDOWPOS_UNDEFINED,w,h,SDL_WINDOW_OPENGL|SDL_WINDOW_SHOWN);
    } else SDL_SetWindowSize(window,w,h);
    focused=1;return window;
}
int rg_input_pump(void) {
    SDL_Event event;
    while(SDL_PollEvent(&event)) {
        switch(event.type) {
        case SDL_QUIT:closed=1;break;
        case SDL_WINDOWEVENT:
            if(event.window.event==SDL_WINDOWEVENT_CLOSE) closed=1;
            if(event.window.event==SDL_WINDOWEVENT_FOCUS_GAINED) {focused=1;known=0;}
            if(event.window.event==SDL_WINDOWEVENT_FOCUS_LOST) {
                for(unsigned i=1;i<512;++i) if(held[i]) scan_event(i,1);
                focused=0;buttons=0;dx=dy=known=0;SDL_CaptureMouse(SDL_FALSE);
            }
            break;
        case SDL_KEYDOWN:case SDL_KEYUP: {
            unsigned code=dos_scan(event.key.keysym.scancode);
            if(focused && code) scan_event(code,event.type==SDL_KEYUP);
            break;
        }
        case SDL_MOUSEMOTION: {
            int w,h;SDL_GetWindowSize(window,&w,&h);
            if(focused && w && h) {
                int nx=event.motion.x*(int)width/w,ny=event.motion.y*(int)height/h;
                if(known) {dx+=nx-x;dy+=ny-y;}x=nx;y=ny;known=1;
            }
            break;
        }
        case SDL_MOUSEBUTTONDOWN:case SDL_MOUSEBUTTONUP: {
            unsigned mask=event.button.button==SDL_BUTTON_LEFT ? 1 : event.button.button==SDL_BUTTON_RIGHT ? 2 : 0;
            if(event.type==SDL_MOUSEBUTTONUP) buttons&=~mask;else if(focused) buttons|=mask;
            SDL_CaptureMouse(buttons ? SDL_TRUE : SDL_FALSE);break;
        }
        }
    }
    return !closed;
}
int rg_mouse_service(uint32_t r[7]) {
    if(!rg_input_pump()) return 0;
    switch(r[0]&65535) {
    case 0:if(!rg_input_window(640,480)) return 0;buttons=0;dx=dy=known=0;r[0]=65535;r[1]=2;return 1;
    case 1:SDL_ShowCursor(SDL_ENABLE);return 1;
    case 2:SDL_ShowCursor(SDL_DISABLE);return 1;
    case 3:r[1]=buttons;r[2]=(uint16_t)(x*2);r[3]=(uint16_t)y;return 1;
    case 4: {
        if(!window) return 0;
        int w,h;SDL_GetWindowSize(window,&w,&h);x=(uint16_t)r[2]/2;y=(uint16_t)r[3];
        if(focused) SDL_WarpMouseInWindow(window,x*w/(int)width,y*h/(int)height);
        dx=dy=0;known=1;return 1;
    }
    case 0xb:r[2]=(uint16_t)dx;r[3]=(uint16_t)dy;dx=dy=0;return 1;
    default:return 0;
    }
}
