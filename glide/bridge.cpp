#include "bridge.h"
#include "glide_imports.h"
#include "sdk2_glide.h"
#include "input.h"
#include <cstdio>
#include <cstdlib>
#include <cstring>
struct Tex32 {int32_t small,large,aspect,format; uint32_t data;};
struct Lfb32 {uint32_t size,pointer,stride; int32_t mode,origin;};
static_assert(sizeof(Tex32)==20 && sizeof(Lfb32)==20,"DOS Glide structures");
static_assert(sizeof(GrVertex)==60 && sizeof(GrHwConfiguration)==148,"Pointer-free Glide layouts");
static_assert(sizeof(FxU)>=sizeof(uintptr_t),"Native Glide window argument must preserve host pointers");
static unsigned width=640,height=480;
struct Lock {void *guest; GrLfbInfo_t native; uint32_t bytes,buffer;};
static Lock locks[2]={};
static GrTexInfo texture(uint32_t p) {
    Tex32 t; memcpy(&t,(void*)(uintptr_t)p,sizeof t);
    return {t.small,t.large,t.aspect,t.format,(void*)(uintptr_t)t.data};
}
static unsigned dimensions(unsigned resolution,bool horizontal) {
    static const unsigned sizes[][2]={{320,200},{320,240},{400,256},{512,384},{640,200},
      {640,350},{640,400},{640,480},{800,600},{960,720},{856,480},{512,256},{1024,768},{1280,1024},{1600,1200},{400,300}};
    return resolution<16 ? sizes[resolution][horizontal?0:1] : 0;
}
static void capture_frame() {
    const char *directory=getenv("RG_CAPTURE_DIR");
    if (!directory) return;
    static unsigned frame;
    ++frame;
    if (frame>3 && frame%30) return;
    GrLfbInfo_t info={}; info.size=sizeof info;
    if (!grLfbLock(GR_LFB_READ_ONLY,GR_BUFFER_BACKBUFFER,GR_LFBWRITEMODE_565,GR_ORIGIN_UPPER_LEFT,FXFALSE,&info)) return;
    char path[4096];
#ifdef _WIN32
    const char *separator="\\";
#else
    const char *separator="/";
#endif
    int length=snprintf(path,sizeof path,"%s%sgame-frame-%s.ppm",directory,separator,frame==1?"first":"latest");
    FILE *out=length>0 && (unsigned)length<sizeof path ? fopen(path,"wb") : nullptr;
    if (out) {
        fprintf(out,"P6\n%u %u\n255\n",width,height);
        for (unsigned y=0;y<height;++y) for(unsigned x=0;x<width;++x) {
            uint16_t pixel; memcpy(&pixel,(char*)info.lfbPtr+y*info.strideInBytes+x*2,2);
            unsigned char rgb[]={(unsigned char)((pixel>>11)*255/31),
                (unsigned char)(((pixel>>5)&63)*255/63),(unsigned char)((pixel&31)*255/31)};
            fwrite(rgb,1,3,out);
        }
        fclose(out);
        fprintf(stderr,"Captured original frame %u %ux%u\n",frame,width,height);
    }
    grLfbUnlock(GR_LFB_READ_ONLY,GR_BUFFER_BACKBUFFER);
}
extern "C" int rg_glide_service(unsigned index,GuestFrame *f) {
    auto *a=(uint32_t*)(uintptr_t)((uint32_t)f->esp+4);
    static bool seen[124]={};
    if (index>=124) return 0;
    if (!seen[index]) {fprintf(stderr,"Native Glide entry %u\n",index);seen[index]=true;}
    switch(index) {
    case RG_GRGLIDEINIT: grGlideInit(); break;
    case RG_GRGLIDESHUTDOWN: grGlideShutdown(); break;
    case RG_GRSSTQUERYHARDWARE: f->eax=grSstQueryHardware((GrHwConfiguration*)(uintptr_t)a[0]); break;
    case RG_GRSSTSELECT: grSstSelect(a[0]); break;
    case RG_GRSSTWINOPEN: {
        width=dimensions(a[1],true); height=dimensions(a[1],false);
        RgWindow window=rg_input_window(width,height);
        if (!window) return 0;
        f->eax=grSstWinOpen((FxU)(uintptr_t)window,a[1],a[2],a[3],a[4],a[5],a[6]);
        fprintf(stderr,"Native Glide window %ux%u opened=%llu\n",width,height,(unsigned long long)f->eax); break;
    }
    case RG_GRGAMMACORRECTIONVALUE: {float value; memcpy(&value,a,4); grGammaCorrectionValue(value); break;}
    case RG_GRRENDERBUFFER: grRenderBuffer(a[0]); break;
    case RG_GRBUFFERCLEAR: grBufferClear(a[0],a[1],a[2]); break;
    case RG_GRBUFFERSWAP: if (!rg_input_pump()) return 0; capture_frame(); grBufferSwap(a[0]); break;
    case RG_GRCOLORMASK: grColorMask(a[0],a[1]); break;
    case RG_GRDEPTHBUFFERMODE: grDepthBufferMode(a[0]); break;
    case RG_GRDEPTHBUFFERFUNCTION: grDepthBufferFunction(a[0]); break;
    case RG_GRDEPTHMASK: grDepthMask(a[0]); break;
    case RG_GRDEPTHBIASLEVEL: grDepthBiasLevel(a[0]); break;
    case RG_GRTEXSOURCE: {auto info=texture(a[3]); grTexSource(a[0],a[1],a[2],&info); break;}
    case RG_GRTEXDOWNLOADMIPMAP: {auto info=texture(a[3]); grTexDownloadMipMap(a[0],a[1],a[2],&info); break;}
    case RG_GRTEXDOWNLOADTABLE: grTexDownloadTable(a[0],a[1],(void*)(uintptr_t)a[2]); break;
    case RG_GRTEXMINADDRESS: f->eax=grTexMinAddress(a[0]); break;
    case RG_GRTEXMAXADDRESS: f->eax=grTexMaxAddress(a[0]); break;
    case RG_GRTEXFILTERMODE: grTexFilterMode(a[0],a[1],a[2]); break;
    case RG_GRTEXCLAMPMODE: grTexClampMode(a[0],a[1],a[2]); break;
    case RG_GRTEXMIPMAPMODE: grTexMipMapMode(a[0],a[1],a[2]); break;
    case RG_GUTEXCOMBINEFUNCTION: guTexCombineFunction(a[0],a[1]); break;
    case RG_GUCOLORCOMBINEFUNCTION: guColorCombineFunction(a[0]); break;
    case RG_GUALPHASOURCE: guAlphaSource(a[0]); break;
    case RG_GRCONSTANTCOLORVALUE: grConstantColorValue(a[0]); break;
    case RG_GRALPHABLENDFUNCTION: grAlphaBlendFunction(a[0],a[1],a[2],a[3]); break;
    case RG_GRCHROMAKEYMODE: grChromakeyMode(a[0]); break;
    case RG_GRCHROMAKEYVALUE: grChromakeyValue(a[0]); break;
    case RG_GRDITHERMODE: grDitherMode(a[0]); break;
    case RG_GRFOGMODE: grFogMode(a[0]); break;
    case RG_GRFOGTABLE: grFogTable((GrFog_t*)(uintptr_t)a[0]); break;
    case RG_GRFOGCOLORVALUE: grFogColorValue(a[0]); break;
    case RG_GRDRAWTRIANGLE: grDrawTriangle((GrVertex*)(uintptr_t)a[0],(GrVertex*)(uintptr_t)a[1],(GrVertex*)(uintptr_t)a[2]); break;
    case RG_GRDRAWLINE: grDrawLine((GrVertex*)(uintptr_t)a[0],(GrVertex*)(uintptr_t)a[1]); break;
    case RG_GRDRAWPOINT: grDrawPoint((GrVertex*)(uintptr_t)a[0]); break;
    case RG_GRLFBLOCK: {
        if (a[0]>1 || (a[2]>2 && a[2]!=0xff)) return 0;
        Lock &lock=locks[a[0]];
        if (lock.guest) return 0;
        Lfb32 *info=(Lfb32*)(uintptr_t)a[5];
        if (info->size!=sizeof(*info)) return 0;
        lock.native={}; lock.native.size=sizeof(lock.native);
        f->eax=grLfbLock(a[0],a[1],a[2],a[3],a[4],&lock.native);
        if (!f->eax) break;
        lock.bytes=height*lock.native.strideInBytes;
        lock.guest=rg_low_alloc(lock.bytes); lock.buffer=a[1];
        if (!lock.guest) {grLfbUnlock(a[0],a[1]);f->eax=0;break;}
        memcpy(lock.guest,lock.native.lfbPtr,lock.bytes);
        *info={20,(uint32_t)(uintptr_t)lock.guest,lock.native.strideInBytes,lock.native.writeMode,lock.native.origin};
        break;
    }
    case RG_GRLFBUNLOCK: {
        if (a[0]>1) return 0;
        Lock &lock=locks[a[0]];
        if (!lock.guest || lock.buffer!=a[1]) return 0;
        if (a[0]==1) memcpy(lock.native.lfbPtr,lock.guest,lock.bytes);
        f->eax=grLfbUnlock(a[0],a[1]);
        if (!rg_low_free(lock.guest)) return 0;
        lock={}; break;
    }
    default: fprintf(stderr,"Unsupported Glide entry %u\n",index); return 0;
    }
    return 1;
}
