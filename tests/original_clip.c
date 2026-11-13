#include <assert.h>
#include <stdio.h>
#include <math.h>
#include <string.h>
#ifdef _WIN32
#include <windows.h>
#endif
#include "guest_abi.h"
#include "crash.h"
extern void rg_clip_code(void);
extern float rg_clip_near,rg_clip_far;
extern void rg_clip_near_edge(void),rg_clip_far_edge(void),rg_clip_left_edge(void);
extern void rg_clip_right_edge(void),rg_clip_bottom_edge(void),rg_clip_top_edge(void);
static void edges(char *memory) {
    const struct {void (*target)(void); float a[3],b[3],expected[3],t;} cases[]={
        {rg_clip_near_edge,{0,0,0},{0,0,4},{0,0,1},0.25f},
        {rg_clip_far_edge,{0,0,9999},{0,0,10003},{0,0,10000},0.25f},
        {rg_clip_left_edge,{-15,0,10},{5,0,10},{-10,0,10},0.25f},
        {rg_clip_right_edge,{15,0,10},{-5,0,10},{10,0,10},0.25f},
        {rg_clip_bottom_edge,{0,-15,10},{0,5,10},{0,-10,10},0.25f},
        {rg_clip_top_edge,{0,15,10},{0,-5,10},{0,10,10},0.25f}
    };
    float *a=(float *)memory,*b=a+9,*out=b+9;
    for (unsigned i=0;i<sizeof cases/sizeof cases[0];++i) {
        memcpy(a,cases[i].a,12); memcpy(b,cases[i].b,12);
        for (unsigned j=3;j<8;++j) {a[j]=(float)j;b[j]=(float)j+10;}
        memset(out,0xcd,36);
        GuestRegs r={0};
        r.eax=(uint32_t)(uintptr_t)a; r.edx=(uint32_t)(uintptr_t)b;
        r.ebx=(uint32_t)(uintptr_t)out; r.ecx=0x23456789;
        r.esi=0x456789ab; r.edi=0x56789abc; r.ebp=0x6789abcd;
        r.esp=(uint32_t)(uintptr_t)memory+65536;
        rg_call((uint32_t)(uintptr_t)cases[i].target,&r);
        for (unsigned j=0;j<8;++j) {
            float expected=j<3 ? cases[i].expected[j] : (float)j+10*cases[i].t;
            assert(fabsf(out[j]-expected)<0.001f);
        }
        assert(!r.eax && !((uint32_t *)out)[8]);
        assert(r.esp==(uint32_t)(uintptr_t)memory+65536);
        assert(r.ecx==0x23456789 && r.edx==(uint32_t)(uintptr_t)b);
        assert(r.ebx==(uint32_t)(uintptr_t)out && r.esi==0x456789ab && r.edi==0x56789abc && r.ebp==0x6789abcd);
        printf("PASS original clip edge[%u] intersection, five attributes, mask and preserved registers\n",i);
    }
}
int main(void) {
    rg_crash_install();
    char *memory=rg_low_alloc(65536); assert(memory);
    float *vector=(float *)memory;
    rg_clip_near=1.0f; rg_clip_far=10000.0f;
    const struct { float x,y,z; unsigned mask; } cases[]={
        {0,0,10,0},{0,0,0.5f,1},{0,0,10001,2},
        {-11,0,10,4},{11,0,10,8},{0,11,10,16},{0,-11,10,32},
        {11,-11,10,40},{-11,11,10,20},{0,0,1,0},{0,0,10000,0},
        {-10,10,10,0},{10,-10,10,0}
    };
    const unsigned case_count=sizeof cases/sizeof cases[0];
    for (unsigned trial=0;trial<3*case_count;++trial) {
        unsigned i=trial%case_count;
        vector[0]=cases[i].x; vector[1]=cases[i].y; vector[2]=cases[i].z;
        GuestRegs r={0}; r.eax=(uint32_t)(uintptr_t)vector;
        r.esp=(uint32_t)(uintptr_t)memory+65536;
        r.ebx=0x12345678; r.ecx=0x23456789; r.edx=0x3456789a;
        r.esi=0x456789ab; r.edi=0x56789abc; r.ebp=0x6789abcd;
        rg_call((uint32_t)(uintptr_t)rg_clip_code,&r);
        printf("clip[%u] mask=%u expected=%u\n",i,r.eax,cases[i].mask);
        assert(r.eax==cases[i].mask);
        assert(r.esp==(uint32_t)(uintptr_t)memory+65536);
        assert(r.ebx==0x12345678 && r.ecx==0x23456789 && r.edx==0x3456789a);
        assert(r.esi==0x456789ab && r.edi==0x56789abc && r.ebp==0x6789abcd);
    }
    edges(memory);
    assert(rg_low_free(memory));
    puts("PASS original clip classifier six planes, combinations, boundaries and preserved registers");
}
