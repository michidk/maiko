#include <assert.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <SDL.h>
#include "guest_abi.h"
#include "input.h"
#include "timing.h"
static int dos_exit;
static void clean(void) {
    assert(SDL_WasInit(SDL_INIT_VIDEO));
    puts("PASS native shutdown preserves SDL video through exit callbacks");
    puts(dos_exit ? "PASS native DOS exit runs host cleanup" : "PASS native close ran exit cleanup");
}
int main(int argc,char **argv) {
    assert(argc==2);
    dos_exit=!strcmp(argv[1],"--exit");
    assert(dos_exit || !strcmp(argv[1],"--close"));
    setbuf(stdout,NULL);assert(atexit(clean)==0);assert(rg_clock_start());atexit(rg_clock_stop);
    uint32_t *regs=rg_low_alloc(65536);assert(regs);regs[0]=0xb;
    assert(rg_input_window(640,480));
    GuestFrame f={0};f.esp=(uint32_t)(uintptr_t)regs+32768;
    if(dos_exit){f.eax=0x4c07;rg_service(0x1021,&f);}
    SDL_Event event={0};event.type=SDL_QUIT;assert(SDL_PushEvent(&event)==1);
    f.eax=0x33;f.edx=f.ebx=(uint32_t)(uintptr_t)regs;
    rg_service(8,&f);
    fputs("FAIL shutdown returned to original code\n",stderr);return 1;
}
