#include "guest_abi.h"
#include "crash.h"
#include "game_files.h"
#include "game_install.h"
#include "platform_services.h"
#include "input.h"
#include "timing.h"
#include "bridge.h"
#include "glide_imports.h"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include "host_os.h"
#ifdef RG_USE_SDL_AUDIO
#include "audio.h"
extern uint32_t rg_sound_driver,rg_sound_open,rg_sound_handles[16];
#endif
uint32_t X86_InterruptFlag;
extern void rg_init(void), rg_main(void);
extern uint32_t g_1A9EC8, g_1A9ECC, g_1A9EB8, g_1A9EBC, g_1A9EC0, g_1A9EE9;
extern uint32_t g_34131C, g_341320;
extern uint32_t debug_type, object_asm_size, object_c_size, final_version, memory_manager;
extern void rg_key_event(void);
extern unsigned char keyboard_extended_handler,keyboard_caps,keyboard_scroll,keyboard_num;
extern unsigned short keyboard_initialized,keyboard_state[128];
static int keyboard_installed;
static uint32_t key_stack;
static unsigned allocations;
static int trace_alloc=-1;
static uint32_t signal_handlers[13];
static unsigned keyboard_locks;
static uint32_t hard_error_callback;
/* Logical vector24 offset only. Native file errors return status and never
   invoke a DOS handler. Segment selectors are flattened by the SR adapter. */
static uint32_t critical_vector;
extern uint32_t rg_movie_clock_callback,rg_movie_clock_shutdown;
extern void host_movie_clock_read(void),host_movie_clock_stop(void);
extern unsigned char loc_1A3865[],loc_1A824C[],loc_1A4620[],loc_1A4624[],loc_1EEC48[],loc_1EEC54[],loc_1F29C4[],loc_1F29E0[];
extern uint32_t rg_tick_delta;
static uint32_t read32(const void *p) {uint32_t n;memcpy(&n,p,4);return n;}
static void runtime_state(void) {
    static unsigned frames,rows;static uint32_t last,previous_actor,previous_startup=UINT32_MAX,previous_ui=UINT32_MAX;
    uint32_t now=rg_clock_milliseconds(),startup=read32(&loc_1A4624);
    uint32_t actor=read32(&loc_1A3865),ui=read32(&loc_1A824C);
    ++frames;
    if(rows>=2048 || (frames>16 && actor==previous_actor && ui==previous_ui &&
                     startup==previous_startup && now-last<250)) return;
    int readable=rg_host_readable((void *)(uintptr_t)actor,0x276);
    unsigned char *p=(unsigned char *)(uintptr_t)actor;
    fprintf(stderr,"STATE ms=%u frame=%u ui=%u actor=%08x startup=%u whole=%u fine=%u scale=%u fps=%u fly=%u vertical_bypass=%u",
            now,frames,ui,actor,startup,rg_tick_delta,read32(&loc_1A4620),
            read32(&loc_1EEC48),read32(&loc_1EEC54),read32(&loc_1F29C4),read32(&loc_1F29E0));
    if(readable) {
        uint32_t anim=read32(p+0x247);unsigned char *a=(unsigned char *)(uintptr_t)anim;
        fprintf(stderr," xyz=%d,%d,%d motion=%u,%u,%u origin=%d timer=%u jump_key=%u",
                (int32_t)read32(p+0xcb),(int32_t)read32(p+0xcf),(int32_t)read32(p+0xd3),
                p[0x1b1],p[0x1b2],p[0x1b3],(int32_t)read32(p+0x1c2),p[0x1c6],keyboard_state[0x39]);
        if(rg_host_readable(a,12)) {uint16_t group;memcpy(&group,a,2);fprintf(stderr," anim=%u model_vertical=%u",group,a[11]);}
    }
    fputc('\n',stderr);last=now;previous_actor=actor;previous_startup=startup;previous_ui=ui;++rows;
}
static void check_close(void) {
    if(!rg_input_pump()) {
        fputs("Native window close requested; running native exit cleanup\n",stderr);
        exit(0);
    }
}

void rg_service(uint32_t service, GuestFrame *f) {
    if (service & 0x80000000) {
        if (service == 0x800d92d0) {
            static unsigned count;
            const void *pointer=(const void *)(uintptr_t)(uint32_t)f->eax;
            int readable=rg_host_readable(pointer,12);
            if (count<8 || (!readable && count<32)) {
                fprintf(stderr,"CLIP entry=%u vector=%08x ecx=%08x edx=%08x readable=%d",
                        count,(unsigned)f->eax,(unsigned)f->ecx,(unsigned)f->edx,readable);
                if (readable) {
                    float vector[3]; memcpy(vector,pointer,sizeof vector);
                    fprintf(stderr," xyz=%g,%g,%g",vector[0],vector[1],vector[2]);
                }
                fputc('\n',stderr);
            }
            if (count<32) ++count;
            return;
        }
        if (service == 0x8002bd78) {
            static int trace_ini=-1;
            if (trace_ini<0) trace_ini=getenv("RG_TRACE_INI")!=NULL;
            if (!trace_ini) return;
            fprintf(stderr,"INI query file=%s section=%s key=%s\n",
                    (char *)(uintptr_t)(uint32_t)f->eax,
                    (char *)(uintptr_t)(uint32_t)f->edx,
                    (char *)(uintptr_t)(uint32_t)f->ebx);
            return;
        }
        fprintf(stderr, "TRACE original=%x eax=%08x edx=%08x\n", service & 0x7fffffff, (unsigned)f->eax, (unsigned)f->edx);
        return;
    }
    if(service==23 || service==8 || service==0x2000+RG_GRBUFFERSWAP) check_close();
    if (keyboard_installed) {
        int scan;
        while ((scan=rg_input_take_scan())>=0) {
            GuestRegs key={0}; key.eax=(unsigned)scan; key.esp=key_stack;
            rg_call((uint32_t)(uintptr_t)rg_key_event,&key);
        }
    }
    if(service==0x301b){
        /* SmackSoundUseMSS installs its callbacks only with a zero clock slot.
           Replace our timer placeholder, then retain the original callback setup. */
        if(rg_movie_clock_callback==(uint32_t)(uintptr_t)host_movie_clock_read)rg_movie_clock_callback=0;
        return;
    }
    if ((service&0xf000)==0x2000) {
        if(rg_glide_service(service-0x2000,f)) {
            if(service==0x2000+RG_GRBUFFERSWAP) runtime_state();
            return;
        }
        check_close();
    }
#ifdef RG_USE_SDL_AUDIO
    if((service&0xff00)==0x3000) {
        if(service==0x3000) {
            if(rg_audio_open(rg_sound_handles)) {GuestFrame driver={0};rg_audio_service(19,&driver);rg_sound_driver=driver.eax;rg_sound_open=1;return;}
        } else if(service==0x3001) {
            rg_audio_close();rg_sound_driver=rg_sound_open=0;memset(rg_sound_handles,0,sizeof rg_sound_handles);return;
        } else if(rg_audio_service(service-0x3000,f)) return;
    }
#endif
    uint32_t result;
    switch (service) {
    case 24:
        rg_movie_clock_callback=(uint32_t)(uintptr_t)host_movie_clock_read;
        rg_movie_clock_shutdown=(uint32_t)(uintptr_t)host_movie_clock_stop;
        f->eax=0;return;
    case 25: f->eax=rg_clock_milliseconds();return;
    case 26: return; /* Native clock is shared; no PIT/VxD state to release. */
    case 23: return;
    case 22: {
        void *pointer=(void *)(uintptr_t)(uint32_t)f->eax;
        size_t capacity=rg_low_size(pointer);
        if(!capacity) break;
        f->eax=(uint32_t)capacity;
        return;
    }
    case 21:
        fputs("Legacy VGA DAC clear has no effect on the native Glide display; original Glide palette upload retained\n",stderr); return;
    case 20:
        if (!rg_input_window(640,480)) break;
        fputs("Native window prepared for original Glide initialization; VGA mode13 unavailable\n",stderr); return;
    case 19:
        critical_vector=(uint32_t)f->eax; f->eax=0;
        fputs("Native critical-file policy replaces DOS vector 24h; errors return status\n",stderr); return;
    case 16:
        if (!key_stack) {void *stack=rg_low_alloc(65536);if (!stack) break;key_stack=(uint32_t)(uintptr_t)stack+65536;}
        keyboard_extended_handler=1; keyboard_installed=1;
        fputs("Native keyboard event delivery installed; original key translation retained\n",stderr); return;
    case 17: keyboard_caps=keyboard_scroll=keyboard_num=0; return;
    case 18: keyboard_installed=0; keyboard_initialized=0; memset(keyboard_state,0,sizeof keyboard_state); return;
    case 15: rg_platform_info(f); return;
    case 13: hard_error_callback=(uint32_t)f->eax; fputs("Native file errors return status without a DOS critical-error dialog\n",stderr); return;
    case 14: hard_error_callback=0; return;
    case 11: f->eax = rg_host_lock((void *)(uintptr_t)(uint32_t)f->eax,(uint32_t)f->edx,0); return;
    case 12: f->eax = rg_host_lock((void *)(uintptr_t)(uint32_t)f->eax,(uint32_t)f->edx,1); return;
    case 9: fprintf(stderr,"Original SYSTEM.INI parsed final_version=%u memory_manager=%u\n",final_version,memory_manager); keyboard_locks &= ~0x10u; fputs("Native input scroll lock cleared\n",stderr); return;
    case 10: keyboard_locks &= ~0x20u; fputs("Native input num lock cleared\n",stderr); return;
    case 8: if (rg_int386(f)) return; check_close(); break;
    case 1: {
        uint32_t size = (uint32_t)f->eax;
        void *p = rg_low_alloc(size);
        if (p && !rg_guest_pointer(p, (uint32_t)f->eax, &result)) abort();
        f->eax = (uintptr_t)p;
        if(trace_alloc<0) trace_alloc=getenv("RG_TRACE_ALLOC")!=NULL;
        if(trace_alloc) fprintf(stderr,"ALLOC size=%u pointer=%08x caller=%08x\n",size,(unsigned)f->eax,*(uint32_t *)(uintptr_t)(uint32_t)f->esp);
        if (++allocations < 12) fprintf(stderr, "malloc(%u) -> %08llx\n", size, (unsigned long long)f->eax);
        return;
    }
    case 2:
        if(trace_alloc>0) fprintf(stderr,"FREE pointer=%08x size=%zu caller=%08x\n",(unsigned)f->eax,rg_low_size((void *)(uintptr_t)(uint32_t)f->eax),*(uint32_t *)(uintptr_t)(uint32_t)f->esp);
        if (f->eax && !rg_low_free((void *)(uintptr_t)(uint32_t)f->eax)) {
            fprintf(stderr, "BLOCKED invalid guest free %08llx\n", (unsigned long long)f->eax); rg_host_exit(2);
        }
        return;
    case 6: {
        fprintf(stderr,"getcwd buffer=%08x size=%u\n",(unsigned)f->eax,(unsigned)f->edx);
        unsigned size = rg_host_cwd(0, NULL);
        fprintf(stderr,"getcwd required=%u\n",size);
        char *buffer = (char *)(uintptr_t)(uint32_t)f->eax;
        uint32_t capacity = (uint32_t)f->edx;
        if (!buffer) { capacity = capacity > size ? capacity : size; buffer = rg_low_alloc(capacity); }
        if (!buffer || capacity < size || !rg_host_cwd(capacity, buffer)) { f->eax = 0; return; }
        fprintf(stderr,"getcwd result=%s\n",buffer);
        f->eax = (uintptr_t)buffer; return;
    }
    case 7:
        f->eax = rg_host_chdir((char *)(uintptr_t)(uint32_t)f->eax) ? 0 : UINT32_MAX;
        return;
    case 5: {
        unsigned sig = (uint32_t)f->eax;
        if (sig < 1 || sig > 12) { f->eax = 3; return; }
        uint32_t old = signal_handlers[sig];
        signal_handlers[sig] = (uint32_t)f->edx;
        f->eax = old;
        return;
    }
    case 3: fputs("Native debug text display initialized\n", stderr); return;
    case 4: fputc((unsigned char)f->eax, stderr); return;
    case 0x1010:
        if (rg_video_service(f)) return;
        break;
    case 0x1021:
        if (((f->eax>>8)&255)==0x36) {
            uint64_t available,total;
            if(!rg_host_disk(f->edx&255,&available,&total)) {f->eax=65535;return;}
            /* DOS exposes 16-bit cluster counts. Cap the native volume to the
               representable view, without granting write access. */
            available/=32768;total/=32768;
            f->eax=64;f->ecx=512;
            f->ebx=available>65535 ? 65535 : available;
            f->edx=total>65535 ? 65535 : total;return;
        }
        if ((f->eax & 0xffff)==0x3524) { f->ebx=critical_vector; return; }
        if ((f->eax & 0xffff)==0x2524) { critical_vector=(uint32_t)f->edx; return; }
        if (((f->eax >> 8) & 255)==0x4c) {
            unsigned status=(unsigned)f->eax&255;
            fprintf(stderr,"Original DOS termination status=%u; exiting native process\n",status);
            exit((int)status);
        }
        if (rg_file_service(f)) return;
        if (((f->eax >> 8) & 255) == 0x2c) {
            unsigned t[4]; rg_host_time(t);
            f->ecx = (f->ecx & 0xffff0000) | (t[0] << 8) | t[1];
            f->edx = (f->edx & 0xffff0000) | (t[2] << 8) | t[3];
            return;
        }
        break;
    }
    fprintf(stderr, "BLOCKED native service=%x eax=%08llx edx=%08llx return=%08x\n", service,
            (unsigned long long)f->eax, (unsigned long long)f->edx,
            *(uint32_t *)(uintptr_t)(uint32_t)f->esp);
    rg_dump_calls(); fflush(stderr); rg_host_exit(2);
}
int main(int argc, char **argv) {
    rg_host_setup();
    setbuf(stdout, NULL); setbuf(stderr, NULL); rg_crash_install();
    if (!rg_install_select(argc,argv)) return 1;
    if (!rg_clock_start()) return 1;
    atexit(rg_clock_stop);
    char *storage = rg_low_alloc(65536);
    void *stack = rg_low_alloc(1024 * 1024);
    if (!storage || !stack) return 1;
    if (!rg_host_cwd(240, storage)) return 1;
    strcat(storage, "\\rgfx.exe");
    storage[256] = 0; storage[257] = 0;
    strcpy(storage + 512, "");
    g_1A9EBC = (uint32_t)(uintptr_t)(storage + 512);
    g_1A9EC0 = (uint32_t)(uintptr_t)storage;
    g_1A9EE9 = (uint32_t)(uintptr_t)(storage + 256);
    g_1A9EC8 = (uint32_t)(uintptr_t)stack;
    g_1A9ECC = g_1A9EB8 = (uint32_t)(uintptr_t)stack + 1024 * 1024;
    GuestRegs r = {0}; r.esp = g_1A9ECC; r.eax = 255;
    puts("Entering original CRT initialization dispatcher");
    rg_call((uint32_t)(uintptr_t)rg_init, &r);
    printf("CRT complete argc=%u argv=%08x; entering original main\n", g_34131C, g_341320);
    fprintf(stderr, "main preconditions stack=%08x argv0=%s sizes=%u/%u debug=%u\n", r.esp, (char *)(uintptr_t)*(uint32_t *)(uintptr_t)g_341320, object_asm_size, object_c_size, debug_type);
    r.eax = g_34131C; r.edx = g_341320;
    rg_call((uint32_t)(uintptr_t)rg_main, &r);
    printf("Original main returned %u\n", r.eax);
    return (int)r.eax;
}
