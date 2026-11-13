#ifdef _WIN32
#include <windows.h>
#else
#include <SDL.h>
#endif
#include <cstdio>
#include <cstdint>
#include <cassert>
#include <initializer_list>
#include "sdk2_glide.h"

static unsigned fogged_white(float depth) {
    grBufferClear(0,0,0xffff);
    GrVertex a={},b={},c={};
    a.x=40; a.y=40; b.x=400; b.y=40; c.x=40; c.y=400;
    a.oow=b.oow=c.oow=1.0f/depth;
    grDrawTriangle(&a,&b,&c);
    GrLfbInfo_t info={}; info.size=sizeof info;
    assert(grLfbLock(GR_LFB_READ_ONLY,GR_BUFFER_BACKBUFFER,GR_LFBWRITEMODE_565,
                     GR_ORIGIN_UPPER_LEFT,FXFALSE,&info));
    auto row=(const uint16_t*)((const char*)info.lfbPtr+100*info.strideInBytes);
    unsigned red=(row[100]>>11)*255/31;
    assert(grLfbUnlock(GR_LFB_READ_ONLY,GR_BUFFER_BACKBUFFER));
    return red;
}

static void check_fog_interpolation() {
    guColorCombineFunction(GR_COLORCOMBINE_CCRGB);
    grConstantColorValue(0xffffffff);
    grFogColorValue(0);
    grFogMode(GR_FOG_WITH_TABLE);
    GrFog_t fog[GR_FOG_TABLE_SIZE]={};
    for (int i=44;i<GR_FOG_TABLE_SIZE;++i)
        fog[i]=i==44?38:i==45?76:i==46?136:i==47?196:255;
    grFogTable(fog);
    // At these distances the old lookup jumps an entire table entry as a
    // vertex moves. The expected RGB values use reciprocal-depth interpolation.
    struct Sample { float depth; int red; };
    for (auto sample:{Sample{0.5f,255}, Sample{2730.65f,119}, Sample{2730.68f,119},
                      Sample{3000.0f,87}, Sample{3276.79f,59}, Sample{3276.81f,59},
                      Sample{65535.0f,0}}) {
        int red=(int)fogged_white(sample.depth);
        fprintf(stderr,"Fog depth=%.3f red=%d expected=%d\n",sample.depth,red,sample.red);
        assert(red>=sample.red-8 && red<=sample.red+8);
    }
    // Fog tables can fall as well as rise. Halfway in reciprocal depth
    // between entries 3 and 4, densities 255 and 0 must average to 127.5.
    fog[3]=255; fog[4]=0;
    grFogTable(fog);
    float midpoint=2.0f/(1.0f/guFogTableIndexToW(3)+1.0f/guFogTableIndexToW(4));
    int red=(int)fogged_white(midpoint);
    fprintf(stderr,"Fog reciprocal midpoint red=%d expected=128\n",red);
    assert(red>=120 && red<=136);
    grFogMode(GR_FOG_DISABLE);
    puts("PASS fog table interpolation at distance boundaries and reciprocal midpoint");
}

int main(int argc,char **) {
#ifdef _WIN32
    if (argc>1) assert(SetCurrentDirectoryA("Z:\\readonly"));
#else
    (void)argc;
    SDL_SetMainReady();
    assert(!SDL_Init(SDL_INIT_VIDEO));
    SDL_Window *window=SDL_CreateWindow("Native Glide smoke",0,0,640,480,SDL_WINDOW_OPENGL);
    assert(window);
    if (argc>1) assert((uintptr_t)window>UINT32_MAX);
    fprintf(stderr,"SDL window=%p native Glide argument bytes=%zu\n",(void *)window,sizeof(FxU));
#endif
    static_assert(sizeof(FxU32)==4 && sizeof(void*)==8, "x64 backend");
    static_assert(sizeof(FxU)>=sizeof(uintptr_t),"Native Glide handle preserves host pointer");
    grGlideInit();
    GrHwConfiguration hardware={};
    assert(grSstQueryHardware(&hardware) && hardware.num_sst>0);
    grSstSelect(0);

#ifdef _WIN32
    FxU window_handle=0;
#else
    FxU window_handle=(FxU)(uintptr_t)window;
#endif
    assert(grSstWinOpen(window_handle,GR_RESOLUTION_640x480,GR_REFRESH_60Hz,GR_COLORFORMAT_ARGB,
                        GR_ORIGIN_UPPER_LEFT,2,1));
    grRenderBuffer(GR_BUFFER_BACKBUFFER);
    grDepthBufferMode(GR_DEPTHBUFFER_DISABLE);
    grBufferClear(0,0,0xffff);
    uint16_t texture[16*16];
    for (auto &pixel:texture) pixel=0xf800;
    GrTexInfo tex={GR_LOD_16,GR_LOD_16,GR_ASPECT_1x1,GR_TEXFMT_RGB_565,texture};
    auto address=grTexMinAddress(GR_TMU0);
    grTexDownloadMipMap(GR_TMU0,address,GR_MIPMAPLEVELMASK_BOTH,&tex);
    grTexSource(GR_TMU0,address,GR_MIPMAPLEVELMASK_BOTH,&tex);
    grTexFilterMode(GR_TMU0,GR_TEXTUREFILTER_POINT_SAMPLED,GR_TEXTUREFILTER_POINT_SAMPLED);
    guTexCombineFunction(GR_TMU0,GR_TEXTURECOMBINE_DECAL);
    guColorCombineFunction(GR_COLORCOMBINE_DECAL_TEXTURE);
    GrVertex a={},b={},c={};
    a.x=40; a.y=40; b.x=400; b.y=40; c.x=40; c.y=400;
    for (auto vertex:{&a,&b,&c}) {
        vertex->r=vertex->g=vertex->b=vertex->a=255;
        vertex->oow=1; vertex->tmuvtx[0].oow=1;
        vertex->tmuvtx[0].sow=8; vertex->tmuvtx[0].tow=8;
    }
    grDrawTriangle(&a,&b,&c);
    GrLfbInfo_t info={}; info.size=sizeof info;
    assert(grLfbLock(GR_LFB_READ_ONLY,GR_BUFFER_BACKBUFFER,GR_LFBWRITEMODE_565,
                     GR_ORIGIN_UPPER_LEFT,FXFALSE,&info));
    auto row=(uint16_t*)((char*)info.lfbPtr+100*info.strideInBytes);
    auto background=(uint16_t*)((char*)info.lfbPtr+450*info.strideInBytes);
    fprintf(stderr,"Rendered triangle pixel=%04x background=%04x stride=%u\n",row[100],background[600],info.strideInBytes);
    assert((row[100]&0xf800)>0xc000 && (row[100]&0x07ff)<0x100);
    assert(background[600]==0);

#ifdef _WIN32
    const char *capture="Z:\\work\\backend-smoke.ppm";
#elif defined(__APPLE__)
    const char *capture="backend-smoke.ppm";
#else
    const char *capture="/work/backend-smoke.ppm";
#endif
    FILE *image=fopen(capture,"wb"); assert(image);
    fprintf(image,"P6\n640 480\n255\n");
    for(unsigned y=0;y<480;++y) for(unsigned x=0;x<640;++x) {
        uint16_t p=*(uint16_t*)((char*)info.lfbPtr+y*info.strideInBytes+x*2);
        unsigned char rgb[]={static_cast<unsigned char>((p>>11)*255/31),
          static_cast<unsigned char>(((p>>5)&63)*255/63),static_cast<unsigned char>((p&31)*255/31)};
        fwrite(rgb,1,3,image);
    }
    fclose(image);
    assert(grLfbUnlock(GR_LFB_READ_ONLY,GR_BUFFER_BACKBUFFER));
    grBufferSwap(0);
    check_fog_interpolation();
    grGlideShutdown();
    puts("PASS native x64 Glide textured triangle, framebuffer readback and swap");
}
