#include <windows.h>
#include <stdio.h>
#include <string.h>
#include <stdlib.h>
int main(int argc,char **argv) {
    if(argc!=2 && argc!=3) return 1;
    unsigned hold=argc==3 ? (unsigned)strtoul(argv[2],NULL,10) : 150;
    if(hold>5000) return 1;
    HWND w=FindWindowA("SDL_app",NULL); if(!w) return 2;
    if(!strcmp(argv[1],"close")) return PostMessageA(w,WM_CLOSE,0,0) ? 0 : 3;
    unsigned scan=0,key=0;
    if(!strcmp(argv[1],"up")){scan=0x48;key=VK_UP;}
    if(!strcmp(argv[1],"down")){scan=0x50;key=VK_DOWN;}
    if(!strcmp(argv[1],"enter")){scan=0x1c;key=VK_RETURN;}
    if(!strcmp(argv[1],"escape")){scan=1;key=VK_ESCAPE;}
    if(!strcmp(argv[1],"shift")){scan=0x2a;key=VK_SHIFT;}
    if(!strcmp(argv[1],"right")){scan=0x4d;key=VK_RIGHT;}
    if(!strcmp(argv[1],"i")){scan=0x17;key='I';}
    if(!strcmp(argv[1],"s")){scan=0x1f;key='S';}
    if(!strcmp(argv[1],"control_l")){scan=0x1d;key=VK_LCONTROL;}
    if(!strcmp(argv[1],"space")){scan=0x39;key=VK_SPACE;}
    if(!strcmp(argv[1],"l")){scan=0x26;key='L';}
    if(!strcmp(argv[1],"m")){scan=0x32;key='M';}
    if(!scan) return 1;
    DWORD_PTR result;
    if(!SendMessageTimeoutA(w,WM_SETFOCUS,0,0,SMTO_ABORTIFHUNG,2000,&result))return 3;
    if(!PostMessageA(w,WM_KEYDOWN,key,1|(scan<<16)))return 3;
    Sleep(hold);
    if(!PostMessageA(w,WM_KEYUP,key,1|(scan<<16)|(3u<<30)))return 3;
    printf("Posted %s to original game window\n",argv[1]);return 0;
}
