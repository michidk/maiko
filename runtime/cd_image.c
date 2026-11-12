/* Read-only ISO root files behind DOS D:. No extraction and no CD-audio emulation.
   MODE1 layout follows src/import/cdrom.rs; both hosts share this adapter. */
#define _FILE_OFFSET_BITS 64
#define _POSIX_C_SOURCE 200809L
#include "cd_image.h"
#include <stdio.h>
#include <string.h>
#include <stdlib.h>
#ifdef _WIN32
#define seek64 _fseeki64
#define tell64 _ftelli64
#else
#define seek64 fseeko
#define tell64 ftello
#endif
#define CD_HANDLES 64
#define CD_FIRST_HANDLE 512
static FILE *image;
static unsigned sector,header,entry_count;
static uint64_t image_size;
static struct {char name[64];uint32_t extent,size;} entries[512];
static struct {unsigned entry,used;uint32_t position;} handles[CD_HANDLES];
static int exit_registered;
static uint32_t le32(const unsigned char *p) {return (uint32_t)p[0]|(uint32_t)p[1]<<8|(uint32_t)p[2]<<16|(uint32_t)p[3]<<24;}
static int both32(const unsigned char *p,uint32_t *value) {
    *value=le32(p);return *value==((uint32_t)p[7]|(uint32_t)p[6]<<8|(uint32_t)p[5]<<16|(uint32_t)p[4]<<24);
}
static int equal(const char *a,const char *b) {
    while(*a && *b) {
        unsigned x=(unsigned char)*a++,y=(unsigned char)*b++;
        if(x>='a' && x<='z') x-=32;
        if(y>='a' && y<='z') y-=32;
        if(x!=y) return 0;
    }
    return !*a && !*b;
}
static int read_physical(uint64_t offset,void *data,size_t size) {
    return offset<=image_size && size<=image_size-offset && !seek64(image,(int64_t)offset,SEEK_SET) && fread(data,1,size,image)==size;
}
static int extent_valid(uint32_t extent,uint32_t size,uint32_t volume) {
    uint64_t blocks=((uint64_t)size+2047)/2048;
    return extent<=volume && blocks<=volume-extent && ((uint64_t)extent+blocks)*sector<=image_size;
}
void rg_cd_close(void) {
    if(image) fclose(image);
    image=NULL;entry_count=0;memset(handles,0,sizeof handles);
}
int rg_cd_mount(const char *path) {
    rg_cd_close();image=fopen(path,"rb");if(!image) goto invalid;
    if(seek64(image,0,SEEK_END)) goto invalid;
    int64_t length=tell64(image);if(length<0) goto invalid;image_size=(uint64_t)length;
    unsigned char pvd[2048];sector=2048;header=0;
    if(!read_physical(16*sector,pvd,sizeof pvd) || memcmp(pvd,"\1CD001\1",7)) {
        sector=2352;header=16;
        if(!read_physical(16*sector+header,pvd,sizeof pvd) || memcmp(pvd,"\1CD001\1",7)) goto invalid;
    }
    uint32_t volume,root,size;
    if(pvd[128]!=0 || pvd[129]!=8 || pvd[130]!=8 || pvd[131]!=0 ||
       pvd[156]<34 || !(pvd[181]&2) || pvd[157] ||
       !both32(pvd+80,&volume) || !both32(pvd+158,&root) || !both32(pvd+166,&size) ||
       (uint64_t)volume*sector>image_size || size>16*1024*1024 || !size || !extent_valid(root,size,volume)) goto invalid;
    for(uint32_t block=0;block<((uint64_t)size+2047)/2048;++block) {
        unsigned char data[2048];
        if(!read_physical(((uint64_t)root+block)*sector+header,data,sizeof data)) goto invalid;
        unsigned limit=size-block*2048;if(limit>2048) limit=2048;
        for(unsigned offset=0;offset<limit && data[offset];) {
            const unsigned char *r=data+offset;unsigned length=r[0];offset+=length;
            if(length<34 || offset>limit || 33u+r[32]>length) goto invalid;
            if(r[25]&2) continue;
            if(r[1] || r[26] || r[27] || (r[25]&0x80) || !r[32] || r[32]>=64 || entry_count==512) goto invalid;
            unsigned n=0;
            while(n<r[32] && r[33+n]!=';') {
                unsigned char c=r[33+n];if(c<32 || c>126 || c=='/' || c=='\\' || c==':') goto invalid;
                entries[entry_count].name[n]=(char)c;++n;
            }
            entries[entry_count].name[n]=0;if(!n) goto invalid;
            for(unsigned i=0;i<entry_count;++i) if(equal(entries[i].name,entries[entry_count].name)) goto invalid;
            if(!both32(r+2,&entries[entry_count].extent) || !both32(r+10,&entries[entry_count].size) ||
               !extent_valid(entries[entry_count].extent,entries[entry_count].size,volume)) goto invalid;
            ++entry_count;
        }
    }
    if(!exit_registered) {atexit(rg_cd_close);exit_registered=1;}
    fprintf(stderr,"Native CD image mounted read-only as D: layout=%u/%u root_files=%u\n",sector,header,entry_count);return 1;
invalid:
    fprintf(stderr,"CD image unavailable or invalid: %s\n",path);rg_cd_close();return 0;
}
static void error(GuestFrame *f,unsigned code) {f->eax=code;f->flags|=1;}
static void success(GuestFrame *f,uint32_t value) {f->eax=value;f->flags&=~(uint64_t)1;}
static const char *cd_path(uint32_t pointer) {
    const char *p=(const char *)(uintptr_t)pointer;
    return p && (p[0]=='D' || p[0]=='d') && p[1]==':' ? p+2 : NULL;
}
int rg_cd_service(GuestFrame *f) {
    if(!image) return 0;
    unsigned op=(f->eax>>8)&255,h=(uint32_t)f->ebx;
    if(op==0x3d || op==0x43 || op==0x39 || op==0x3a || op==0x3c || op==0x41 || op==0x56 || op==0x4e) {
        const char *name=cd_path((uint32_t)f->edx);
        if(op==0x56 && !name && cd_path((uint32_t)f->edi)) {error(f,5);return 1;}
        if(!name) return 0;
        if(op!=0x3d && op!=0x43) {error(f,op==0x4e ? 18 : 5);return 1;}
        if((op==0x3d && (f->eax&3)) || (op==0x43 && (f->eax&255))) {error(f,5);return 1;}
        while(*name=='/' || *name=='\\') ++name;
        if(!*name && op==0x43) {f->ecx=17;success(f,(uint32_t)f->eax);return 1;}
        if(!*name || strchr(name,'/') || strchr(name,'\\') || strchr(name,':') || !strcmp(name,".") || !strcmp(name,"..")) {error(f,3);return 1;}
        unsigned entry=0;while(entry<entry_count && !equal(name,entries[entry].name)) ++entry;
        if(entry==entry_count) {error(f,2);return 1;}
        if(op==0x43) {f->ecx=1;success(f,(uint32_t)f->eax);return 1;}
        for(unsigned i=0;i<CD_HANDLES;++i) if(!handles[i].used) {
            handles[i].used=1;handles[i].entry=entry;handles[i].position=0;success(f,CD_FIRST_HANDLE+i);
            fprintf(stderr,"OPEN D:\\%s -> %u (CD image)\n",entries[entry].name,CD_FIRST_HANDLE+i);return 1;
        }
        error(f,4);return 1;
    }
    if(h<CD_FIRST_HANDLE || h>=CD_FIRST_HANDLE+CD_HANDLES ||
       (op!=0x3e && op!=0x3f && op!=0x40 && op!=0x42 && op!=0x44)) return 0;
    unsigned slot=h-CD_FIRST_HANDLE;
    if(!handles[slot].used) {error(f,6);return 1;}
    unsigned entry=handles[slot].entry;uint32_t position=handles[slot].position;
    if(op==0x3e) {handles[slot].used=0;success(f,0);return 1;}
    if(op==0x40) {error(f,5);return 1;}
    if(op==0x44) {
        if(f->eax&255) {error(f,1);return 1;}
        f->edx=0;success(f,0);return 1;
    }
    if(op==0x42) {
        unsigned origin=f->eax&255;int32_t distance=(int32_t)(((f->ecx&65535)<<16)|(f->edx&65535));
        int64_t result=distance;
        if(origin==1) result+=position;else if(origin==2) result+=entries[entry].size;
        if(origin>2 || result<0 || result>UINT32_MAX) {error(f,1);return 1;}
        handles[slot].position=(uint32_t)result;success(f,(uint32_t)result&65535);f->edx=(uint32_t)result>>16;return 1;
    }
    uint32_t count=(uint32_t)f->ecx,available=position<entries[entry].size ? entries[entry].size-position : 0;
    if(count>available) count=available;
    unsigned char *out=(void *)(uintptr_t)(uint32_t)f->edx;
    for(uint32_t done=0;done<count;) {
        uint32_t part=2048-position%2048;if(part>count-done) part=count-done;
        uint64_t offset=((uint64_t)entries[entry].extent+position/2048)*sector+header+position%2048;
        if(!read_physical(offset,out+done,part)) {error(f,5);return 1;}
        done+=part;position+=part;
    }
    handles[slot].position=position;success(f,count);return 1;
}
