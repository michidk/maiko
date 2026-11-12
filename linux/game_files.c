#define _GNU_SOURCE
#include "game_files.h"
#include "cd_image.h"
#include "movie_files.h"
#include "paths.h"
#include "user_storage.h"
#include <stdio.h>
#include <string.h>
#include <strings.h>
#include <stdint.h>
#include <errno.h>
#include <unistd.h>
#include <fcntl.h>
#include <dirent.h>
#include <sys/stat.h>
#include <fnmatch.h>
#include <ctype.h>
#include <time.h>
static int handles[256];
static uint32_t dta_address,search_id;
static struct {DIR *dir,*overlay;uint32_t id;unsigned attributes;int maps,overlay_phase;char path[4096],pattern[256];} searches[64];
#pragma pack(push,1)
typedef struct {unsigned char reserved[21],attributes;uint16_t time,date;uint32_t size;char name[13];} DosDta;
#pragma pack(pop)
_Static_assert(sizeof(DosDta)==43,"DOS DTA");
static unsigned dos_error(void) {
    switch(errno) {case ENOENT:return 2;case ENOTDIR:return 3;case EMFILE:case ENFILE:return 4;case EBADF:return 6;default:return 5;}
}
static void error(GuestFrame *f,unsigned code) {f->eax=code;f->flags|=1;}
static void success(GuestFrame *f,uint32_t result) {f->eax=result;f->flags&=~(uint64_t)1;}
static int save_handle(GuestFrame *f,int fd,const char *requested) {
    if(fd<0) {error(f,dos_error());return 1;}
    struct stat st;
    if(fstat(fd,&st) || !S_ISREG(st.st_mode)) {close(fd);error(f,5);return 1;}
    for(unsigned i=5;i<256;++i) if(!handles[i]) {
        handles[i]=fd+1;success(f,i);fprintf(stderr,"OPEN %s -> %u\n",requested,i);return 1;
    }
    close(fd);error(f,4);return 1;
}
static int mutate(GuestFrame *f,unsigned op) {
    const char *requested=(void *)(uintptr_t)(uint32_t)f->edx;
    char leaf[256];int parent=rg_user_parent(requested,leaf);
    if(parent<0) {error(f,dos_error());return 1;}
    int result=-1;
    switch(op) {
    case 0x39:result=mkdirat(parent,leaf,0700);break;
    case 0x3a:result=unlinkat(parent,leaf,AT_REMOVEDIR);break;
    case 0x41: {
        struct stat st;
        if(!fstatat(parent,leaf,&st,AT_SYMLINK_NOFOLLOW) && S_ISREG(st.st_mode)) result=unlinkat(parent,leaf,0);
        else errno=EACCES;
        break;
    }
    case 0x56: {
        char target[256];int destination=rg_user_parent((void *)(uintptr_t)(uint32_t)f->edi,target);
        if(destination>=0) {
            struct stat st;
            if(fstatat(parent,leaf,&st,AT_SYMLINK_NOFOLLOW) || !S_ISREG(st.st_mode)) errno=EACCES;
            else if(!fstatat(destination,target,&st,AT_SYMLINK_NOFOLLOW)) errno=EEXIST;
            else if(errno==ENOENT) result=renameat(parent,leaf,destination,target);
            close(destination);
        }
        break;
    }
    case 0x3c: {
        int fd=openat(parent,leaf,O_RDWR|O_CREAT|O_TRUNC|O_NOFOLLOW|O_CLOEXEC,0600);
        close(parent);return save_handle(f,fd,requested);
    }
    }
    int saved=errno;close(parent);errno=saved;
    if(result<0) error(f,dos_error());else success(f,0);
    fprintf(stderr,"FILE mutation=%02x path=%s result=%u carry=%u\n",op,requested,(unsigned)f->eax,(unsigned)f->flags&1);
    return 1;
}
static int find_file(GuestFrame *f,unsigned operation) {
    if(!dta_address) return 0;
    DosDta *dta=(void *)(uintptr_t)dta_address;unsigned slot=64;
    if(operation==0x4e) {
        for(unsigned i=0;i<64;++i) if(!searches[i].dir) {slot=i;break;}
        if(slot==64) {error(f,4);return 1;}
        char request[4096];const char *pattern=(void *)(uintptr_t)(uint32_t)f->edx;
        if(strlen(pattern)>=sizeof request) {error(f,3);return 1;}
        strcpy(request,pattern);for(char *p=request;*p;++p) if(*p=='\\') *p='/';
        char *leaf=strrchr(request,'/');const char *parent=".";
        if(leaf) {*leaf++=0;parent=*request ? request : "C:/";} else leaf=request;
        if(strlen(leaf)>=sizeof searches[slot].pattern) {error(f,3);return 1;}
        strcpy(searches[slot].pattern,!strcmp(leaf,"*.*") ? "*" : leaf);
        if(!rg_resolve_path(parent,searches[slot].path,sizeof searches[slot].path) ||
           !(searches[slot].dir=opendir(searches[slot].path))) {error(f,dos_error());return 1;}
        char relative[4096];
        searches[slot].maps=rg_user_root()[0] && rg_relative_path(parent,relative,sizeof relative) && !strcasecmp(relative,"MAPS");
        searches[slot].overlay_phase=0;searches[slot].overlay=NULL;
        if(searches[slot].maps) {
            char name[256];int fd=rg_user_parent("C:/MAPS/_.TSG",name);
            searches[slot].overlay=fd<0 ? NULL : fdopendir(fd);
            if(!searches[slot].overlay) {
                int saved=errno;if(fd>=0) close(fd);closedir(searches[slot].dir);searches[slot].dir=NULL;
                errno=saved;error(f,dos_error());return 1;
            }
        }
        searches[slot].id=++search_id;searches[slot].attributes=f->ecx;
        fprintf(stderr,"FIND %s attributes=%02x\n",pattern,(unsigned)f->ecx&255);
    } else {
        uint32_t token[2];memcpy(token,dta->reserved,sizeof token);
        if(token[0]==0x52474454) for(unsigned i=0;i<64;++i)
            if(searches[i].dir && searches[i].id==token[1]) {slot=i;break;}
        if(slot==64) {error(f,18);return 1;}
    }
    struct dirent *entry;
    for(;;) {
        entry=readdir(searches[slot].dir);
        if(!entry) {
            closedir(searches[slot].dir);searches[slot].dir=searches[slot].overlay;
            searches[slot].overlay=NULL;searches[slot].overlay_phase=1;
            if(searches[slot].dir) continue;
            error(f,18);return 1;
        }
        if(searches[slot].maps) {
            const char *dot=strrchr(entry->d_name,'.');int snapshot=dot && !strcasecmp(dot,".TSG");
            if(snapshot!=searches[slot].overlay_phase) continue;
        }
        if(fnmatch(searches[slot].pattern,entry->d_name,FNM_CASEFOLD)) continue;
        struct stat st;if(fstatat(dirfd(searches[slot].dir),entry->d_name,&st,0)) continue;
        unsigned attributes=S_ISDIR(st.st_mode) ? 16 : 0;
        if(entry->d_name[0]=='.' && strcmp(entry->d_name,".") && strcmp(entry->d_name,"..")) attributes|=2;
        if(attributes&~searches[slot].attributes) continue;
        /* Host launcher metadata has no DOS name. Keep enumerating rather than
           aborting or inventing aliases that path lookup cannot reopen. */
        if(strlen(entry->d_name)>=sizeof dta->name) continue;
        if(strcmp(entry->d_name,".") && strcmp(entry->d_name,"..")) {
            const char *dot=strchr(entry->d_name,'.');
            size_t stem=dot ? (size_t)(dot-entry->d_name) : strlen(entry->d_name);
            if(stem>8 || (dot && (strlen(dot+1)>3 || strchr(dot+1,'.')))) continue;
        }
        memset(dta,0,sizeof *dta);uint32_t token[]={0x52474454,searches[slot].id};
        memcpy(dta->reserved,token,sizeof token);dta->attributes=attributes;
        struct tm t;localtime_r(&st.st_mtime,&t);
        if(t.tm_year>=80 && t.tm_year<208) {
            dta->date=((t.tm_year-80)<<9)|((t.tm_mon+1)<<5)|t.tm_mday;
            dta->time=(t.tm_hour<<11)|(t.tm_min<<5)|(t.tm_sec/2);
        }
        dta->size=(uint32_t)st.st_size;strcpy(dta->name,entry->d_name);
        for(char *p=dta->name;*p;++p) *p=(char)toupper((unsigned char)*p);
        success(f,0);return 1;
    }
}
int rg_file_service(GuestFrame *f) {
    if(rg_cd_service(f)) return 1;
    unsigned op=(f->eax>>8)&255,h=(uint32_t)f->ebx;
    char path[4096];
    switch(op) {
    case 0x39:case 0x3a:case 0x3c:case 0x41:case 0x56:return mutate(f,op);
    case 0x1a:dta_address=(uint32_t)f->edx;return 1;
    case 0x4e:case 0x4f:return find_file(f,op);
    case 0x19:f->eax=(f->eax&~(uint64_t)255)|2;return 1;
    case 0x0e:f->eax=(f->eax&~(uint64_t)255)|3;return 1;
    case 0x40: {
        int fd=h==1 || h==2 ? (int)h : h<256 && handles[h] ? handles[h]-1 : -1;
        if(fd<0) {error(f,6);return 1;}
        if(!f->ecx && h!=1 && h!=2) {
            off_t position=lseek(fd,0,SEEK_CUR);
            if(position<0 || ftruncate(fd,position)) error(f,dos_error());else success(f,0);
            return 1;
        }
        ssize_t count=write(fd,(void *)(uintptr_t)(uint32_t)f->edx,(uint32_t)f->ecx);
        if(count<0) error(f,dos_error());else success(f,(uint32_t)count);return 1;
    }
    case 0x43: {
        if(f->eax&255) {error(f,5);return 1;}
        struct stat st;
        if(!rg_resolve_path((void *)(uintptr_t)(uint32_t)f->edx,path,sizeof path) || stat(path,&st)) error(f,dos_error());
        else {f->ecx=S_ISDIR(st.st_mode) ? 16 : 0;success(f,(uint32_t)f->eax);} return 1;
    }
    case 0x44: {
        if(f->eax&255) return 0;
        int fd=h==1 || h==2 ? (int)h : h<256 && handles[h] ? handles[h]-1 : -1;
        if(fd<0) error(f,6);else {f->edx=isatty(fd) ? 0x80 : 0;success(f,(uint32_t)f->edx);}return 1;
    }
    case 0x3d: {
        const char *requested=(void *)(uintptr_t)(uint32_t)f->edx;
        unsigned access=f->eax&3;
        if(access==3) {error(f,12);return 1;}
        if(rg_movie_leaf(requested)) {
            if(access) {error(f,5);return 1;}
            if(!rg_resolve_path(requested,path,sizeof path)) {error(f,dos_error());return 1;}
            return save_handle(f,open(path,O_RDONLY|O_CLOEXEC),requested);
        }
        char relative[4096];
        if(!rg_relative_path(requested,relative,sizeof relative)) {error(f,dos_error());return 1;}
        int fd;
        if(rg_user_namespace(relative)) {
            char leaf[256];int parent=rg_user_parent(requested,leaf);
            if(parent<0) {error(f,dos_error());return 1;}
            fd=openat(parent,leaf,(access==0 ? O_RDONLY : access==1 ? O_WRONLY : O_RDWR)|O_NOFOLLOW|O_CLOEXEC);
            int saved=errno;close(parent);errno=saved;
        } else {
            if(access) {error(f,5);return 1;}
            if(!rg_resolve_path(requested,path,sizeof path)) {error(f,dos_error());return 1;}
            fd=open(path,O_RDONLY|O_CLOEXEC);
        }
        return save_handle(f,fd,requested);
    }
    case 0x3e:case 0x3f:case 0x42: {
        if(h>=256 || !handles[h]) {error(f,6);return 1;}
        int fd=handles[h]-1;
        if(op==0x3e) {if(close(fd)) error(f,dos_error());else {handles[h]=0;success(f,0);}}
        else if(op==0x3f) {
            ssize_t count=read(fd,(void *)(uintptr_t)(uint32_t)f->edx,(uint32_t)f->ecx);
            if(count<0) error(f,dos_error());else success(f,(uint32_t)count);
        } else {
            int32_t distance=(int32_t)(((f->ecx&65535)<<16)|(f->edx&65535));
            unsigned origin=f->eax&255;
            if(origin>2) {error(f,1);return 1;}
            off_t offset=lseek(fd,distance,origin==0 ? SEEK_SET : origin==1 ? SEEK_CUR : SEEK_END);
            if(offset<0 || offset>UINT32_MAX) error(f,5);
            else {success(f,(uint32_t)offset&65535);f->edx=((uint32_t)offset>>16)&65535;}
        }
        return 1;
    }
    default:return 0;
    }
}
