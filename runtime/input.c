#include "input.h"
#include <windowsx.h>
#include <stdio.h>
#include <stdlib.h>
static HWND window;
static int width=640,height=480,x,y,dx,dy,position_known,focused,closed,hidden;
static unsigned buttons;
static unsigned char scans[256];
static unsigned read_scan,write_scan;
static unsigned char held[128];
static void queue_scan(unsigned code) {
    if (write_scan-read_scan>=sizeof scans) { fputs("BLOCKED native keyboard queue overflow\n",stderr);exit(2); }
    scans[write_scan++ & 255]=(unsigned char)code;
}
int rg_input_take_scan(void) {
    return read_scan==write_scan ? -1 : scans[read_scan++ & 255];
}
static LRESULT CALLBACK window_proc(HWND hwnd,UINT message,WPARAM wp,LPARAM lp) {
    switch(message) {
    case WM_CLOSE: closed=1; return 0;
    case WM_SETFOCUS: focused=1; position_known=0; return 0;
    case WM_KEYDOWN: case WM_SYSKEYDOWN:
    case WM_KEYUP: case WM_SYSKEYUP: {
        if (!focused) return 0;
        unsigned scan=(lp>>16)&127;
        if (!scan) return 0;
        int release=message==WM_KEYUP || message==WM_SYSKEYUP;
        if (lp&(1L<<24)) queue_scan(0xe0);
        queue_scan(scan|(release?0x80:0)); held[scan]=!release;
        return 0;
    }
    case WM_KILLFOCUS:
        for (unsigned i=1;i<128;++i) if (held[i]) {queue_scan(i|0x80);held[i]=0;}
        focused=0; buttons=0; dx=dy=0; position_known=0; ReleaseCapture(); return 0;
    case WM_SETCURSOR:
        if (LOWORD(lp)==HTCLIENT && hidden) { SetCursor(NULL); return TRUE; }
        break;
    case WM_MOUSEMOVE: {
        RECT client; GetClientRect(hwnd,&client);
        if (!focused || !client.right || !client.bottom) return 0;
        int nx=GET_X_LPARAM(lp)*width/client.right;
        int ny=GET_Y_LPARAM(lp)*height/client.bottom;
        if (position_known) { dx+=nx-x; dy+=ny-y; }
        x=nx; y=ny; position_known=1; return 0;
    }
    case WM_LBUTTONDOWN: if (focused) { buttons|=1; SetCapture(hwnd); } return 0;
    case WM_RBUTTONDOWN: if (focused) { buttons|=2; SetCapture(hwnd); } return 0;
    case WM_LBUTTONUP: buttons&=~1u; if (!buttons) ReleaseCapture(); return 0;
    case WM_RBUTTONUP: buttons&=~2u; if (!buttons) ReleaseCapture(); return 0;
    }
    return DefWindowProc(hwnd,message,wp,lp);
}
HWND rg_input_window(unsigned w,unsigned h) {
    if (!w || !h || w>8192 || h>8192) return NULL;
    width=(int)w; height=(int)h;
    if (!window) {
        WNDCLASSA klass={0}; klass.lpfnWndProc=window_proc;
        klass.hInstance=GetModuleHandle(NULL); klass.lpszClassName="RedguardNativeWindow";
        klass.hCursor=LoadCursor(NULL,IDC_ARROW);
        if (!RegisterClassA(&klass)) return NULL;
        window=CreateWindowA(klass.lpszClassName,"Redguard native experiment",WS_OVERLAPPEDWINDOW|WS_CLIPCHILDREN|WS_CLIPSIBLINGS,
          CW_USEDEFAULT,CW_USEDEFAULT,640,480,NULL,NULL,klass.hInstance,NULL);
        if (!window) return NULL;
    }
    RECT r={0,0,width,height};
    AdjustWindowRect(&r,WS_OVERLAPPEDWINDOW,FALSE);
    SetWindowPos(window,NULL,0,0,r.right-r.left,r.bottom-r.top,SWP_NOMOVE|SWP_NOZORDER);
    ShowWindow(window,SW_SHOW); SetFocus(window);
    return window;
}
int rg_input_pump(void) {
    MSG message;
    while (PeekMessage(&message,NULL,0,0,PM_REMOVE)) {
        if (message.message==WM_QUIT) closed=1;
        TranslateMessage(&message); DispatchMessage(&message);
    }
    return !closed;
}
int rg_mouse_service(uint32_t r[7]) {
    unsigned function=r[0]&0xffff;
    if (!rg_input_pump()) return 0;
    switch(function) {
    case 0:
        if (!rg_input_window(640,480)) return 0;
        buttons=0; dx=dy=0; position_known=0;
        r[0]=0xffff; r[1]=2; return 1;
    case 1: hidden=0; SetCursor(LoadCursor(NULL,IDC_ARROW)); return 1;
    case 2: hidden=1; if (GetFocus()==window) SetCursor(NULL); return 1;
    case 3: r[1]=buttons; r[2]=(uint16_t)(x*2); r[3]=(uint16_t)y; return 1;
    case 4: {
        if (!window) return 0;
        x=(uint16_t)r[2]/2; y=(uint16_t)r[3];
        RECT client; GetClientRect(window,&client);
        POINT point={x*client.right/width,y*client.bottom/height};
        ClientToScreen(window,&point);
        if (focused && !SetCursorPos(point.x,point.y)) return 0;
        rg_input_pump(); x=(uint16_t)r[2]/2; y=(uint16_t)r[3];
        dx=dy=0; position_known=1; return 1;
    }
    case 0xb:
        r[2]=(uint16_t)dx; r[3]=(uint16_t)dy; dx=dy=0; return 1;
    default: return 0;
    }
}
