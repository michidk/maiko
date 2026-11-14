#include <cassert>
#include <initializer_list>
#include <cstring>
#include <cstdio>
#include "bridge.h"
#include "glide_imports.h"
#include "sdk2_glide.h"

// The game provides these; the bridge test never plays audio, so it only needs them to link.
extern "C" {
int Game_AIL_allocate_sample_handle(...) { return 0; }
int Game_AIL_end_sample(...) { return 0; }
int Game_AIL_init_sample(...) { return 0; }
int Game_AIL_release_sample_handle(...) { return 0; }
int Game_AIL_sample_status(...) { return 0; }
int Game_AIL_set_sample_address(...) { return 0; }
int Game_AIL_set_sample_loop_count(...) { return 0; }
int Game_AIL_set_sample_pan(...) { return 0; }
int Game_AIL_set_sample_playback_rate(...) { return 0; }
int Game_AIL_set_sample_type(...) { return 0; }
int Game_AIL_set_sample_volume(...) { return 0; }
int Game_AIL_start_sample(...) { return 0; }
}

struct Tex32 {int32_t small,large,aspect,format; uint32_t data;};
struct Lfb32 {uint32_t size,pointer,stride; int32_t mode,origin;};
static uint32_t *args;
static uint32_t call(unsigned id,std::initializer_list<uint32_t> values) {
    memset(args,0,128); unsigned i=1; for(auto value:values) args[i++]=value;
    GuestFrame f={}; f.esp=(uintptr_t)args;
    assert(rg_glide_service(id,&f)); return f.eax;
}
int main() {
    args=(uint32_t*)rg_low_alloc(4096); assert(args);
    auto *data=(unsigned char*)rg_low_alloc(1024*1024); assert(data);
    call(RG_GRGLIDEINIT,{});
    assert(call(RG_GRSSTQUERYHARDWARE,{(uint32_t)(uintptr_t)data}));
    assert(*(uint32_t*)data>=1);
    call(RG_GRSSTSELECT,{0});
    assert(call(RG_GRSSTWINOPEN,{0,GR_RESOLUTION_640x480,GR_REFRESH_60Hz,GR_COLORFORMAT_ARGB,GR_ORIGIN_UPPER_LEFT,2,1}));
    call(RG_GRRENDERBUFFER,{GR_BUFFER_BACKBUFFER});
    call(RG_GRBUFFERCLEAR,{0,0,0xffff});
    auto *tex=(Tex32*)(data+256); auto *pixels=(uint16_t*)(data+512);
    for (unsigned i=0;i<256;++i) pixels[i]=0x07e0;
    *tex={GR_LOD_16,GR_LOD_16,GR_ASPECT_1x1,GR_TEXFMT_RGB_565,(uint32_t)(uintptr_t)pixels};
    uint32_t address=call(RG_GRTEXMINADDRESS,{0});
    call(RG_GRTEXDOWNLOADMIPMAP,{0,address,GR_MIPMAPLEVELMASK_BOTH,(uint32_t)(uintptr_t)tex});
    call(RG_GRTEXSOURCE,{0,address,GR_MIPMAPLEVELMASK_BOTH,(uint32_t)(uintptr_t)tex});
    call(RG_GUTEXCOMBINEFUNCTION,{0,GR_TEXTURECOMBINE_DECAL});
    call(RG_GUCOLORCOMBINEFUNCTION,{GR_COLORCOMBINE_DECAL_TEXTURE});
    auto *v=(GrVertex*)(data+2048); memset(v,0,3*sizeof(*v));
    for(unsigned i=0;i<3;++i) {v[i].a=v[i].r=v[i].g=v[i].b=255;v[i].oow=v[i].tmuvtx[0].oow=1;}
    v[0].x=40; v[0].y=40; v[1].x=400; v[1].y=40; v[2].x=40; v[2].y=400;
    call(RG_GRDRAWTRIANGLE,{(uint32_t)(uintptr_t)v,(uint32_t)(uintptr_t)(v+1),(uint32_t)(uintptr_t)(v+2)});
    auto *lfb=(Lfb32*)(data+4096); lfb->size=sizeof(*lfb);
    memset(lfb+1,0xab,16);
    assert(call(RG_GRLFBLOCK,{0,GR_BUFFER_BACKBUFFER,GR_LFBWRITEMODE_565,0,0,(uint32_t)(uintptr_t)lfb}));
    assert(lfb->size==20 && lfb->pointer && lfb->stride==1280);
    auto pixel=*(uint16_t*)(uintptr_t)(lfb->pointer+100*lfb->stride+200);
    assert((pixel&0x07e0)>0x0600 && (pixel&0xf81f)==0);
    for(unsigned i=0;i<16;++i) assert(((unsigned char*)(lfb+1))[i]==0xab);
    assert(call(RG_GRLFBUNLOCK,{0,GR_BUFFER_BACKBUFFER}));
    call(RG_GRBUFFERSWAP,{0});
    call(RG_GRGLIDESHUTDOWN,{});
    GuestFrame unsupported={}; unsupported.esp=(uintptr_t)args;
    assert(!rg_glide_service(RG_GRAADRAWLINE,&unsupported));
    puts("PASS guest Glide stack arguments, 32-bit texture layout, vertex transfer, low framebuffer and bounds");
}
