#define _GNU_SOURCE
#include "user_storage.h"
#include "paths.h"
#include <sys/stat.h>
#include <fcntl.h>
#include <dirent.h>
#include <unistd.h>
#include <errno.h>
#include <string.h>
#include <strings.h>
#include <stdio.h>
#include <stdlib.h>
static char user_root[4096];
const char *rg_user_root(void) {return user_root;}
static int case_name(int directory,const char *name,char result[256]) {
    int copy=openat(directory,".",O_RDONLY|O_DIRECTORY|O_CLOEXEC);
    DIR *dir=copy<0 ? NULL : fdopendir(copy);
    if(!dir) {if(copy>=0) close(copy);return 0;}
    struct dirent *entry;unsigned matches=0;
    while((entry=readdir(dir))) if(!strcasecmp(entry->d_name,name)) {
        if(strlen(entry->d_name)>=256) {closedir(dir);errno=ENAMETOOLONG;return 0;}
        strcpy(result,entry->d_name);++matches;
    }
    closedir(dir);
    if(matches>1) {errno=EEXIST;return 0;}
    if(!matches) strcpy(result,name);
    return 1;
}
int rg_user_parent(const char *path,char leaf[256]) {
    char relative[4096];
    if(!rg_relative_path(path,relative,sizeof relative)) return -1;
    if(!rg_user_namespace(relative)) {errno=EACCES;return -1;}
    int dir=open(user_root,O_RDONLY|O_DIRECTORY|O_NOFOLLOW|O_CLOEXEC);
    if(dir<0) return -1;
    char *save=NULL,*part=strtok_r(relative,"/",&save);
    while(part) {
        char *next=strtok_r(NULL,"/",&save);
        if(strlen(part)>=256 || !case_name(dir,part,leaf)) {close(dir);return -1;}
        if(!next) return dir;
        int child=openat(dir,leaf,O_RDONLY|O_DIRECTORY|O_NOFOLLOW|O_CLOEXEC);
        close(dir);if(child<0) return -1;dir=child;part=next;
    }
    close(dir);errno=EACCES;return -1;
}
int rg_user_namespace(const char *path) {
    if(!user_root[0]) return 0;
    if(!strncasecmp(path,"SAVEGAME",8) && (!path[8] || path[8]=='/')) return 1;
    if(!strncasecmp(path,"MAPS/",5)) {
        const char *leaf=path+5,*dot=strrchr(leaf,'.');
        return !strchr(leaf,'/') && dot && !strcasecmp(dot,".TSG");
    }
    if(strchr(path,'/')) return 0;
    const char *dot=strrchr(path,'.');
    return !strcasecmp(path,"LOGBOOK.TXT") || (dot && !strcasecmp(dot,".INI"));
}
/* Destination operations stay relative to opened directories and never follow
   symlinks. Seeding is resumable and preserves already-existing user files. */
static int copy_file(int source,int destination,const char *name) {
    int input=openat(source,name,O_RDONLY|O_CLOEXEC);
    if(input<0) return 0;
    struct stat st;
    if(fstat(input,&st) || !S_ISREG(st.st_mode)) {close(input);errno=EACCES;return 0;}
    if(!fstatat(destination,name,&st,AT_SYMLINK_NOFOLLOW)) {
        close(input);if(S_ISREG(st.st_mode)) return 1;errno=EACCES;return 0;
    }
    if(errno!=ENOENT) {close(input);return 0;}
    static unsigned sequence;
    char temporary[96];int output=-1;
    for(unsigned attempt=0;attempt<100 && output<0;++attempt) {
        snprintf(temporary,sizeof temporary,".rg-import-%ld-%u",(long)getpid(),++sequence);
        output=openat(destination,temporary,O_WRONLY|O_CREAT|O_EXCL|O_NOFOLLOW|O_CLOEXEC,0600);
        if(output<0 && errno!=EEXIST) break;
    }
    if(output<0) {close(input);return 0;}
    char buffer[16384];ssize_t got;int ok=1;
    while((got=read(input,buffer,sizeof buffer))>0) {
        ssize_t sent=0;
        while(sent<got) {
            ssize_t count=write(output,buffer+sent,(size_t)(got-sent));
            if(count<0 && errno==EINTR) continue;
            if(count<=0) {ok=0;break;}
            sent+=count;
        }
        if(!ok) break;
    }
    if(got<0 || fsync(output)) ok=0;
    if(close(output)) ok=0;
    close(input);
    if(ok && linkat(destination,temporary,destination,name,0))
        ok=errno==EEXIST && !fstatat(destination,name,&st,AT_SYMLINK_NOFOLLOW) && S_ISREG(st.st_mode);
    int saved=errno;unlinkat(destination,temporary,0);errno=saved;
    if(ok) ok=!fsync(destination);
    return ok;
}
static int ensure_directory(int parent,const char *name) {
    if(mkdirat(parent,name,0700) && errno!=EEXIST) return -1;
    return openat(parent,name,O_RDONLY|O_DIRECTORY|O_NOFOLLOW|O_CLOEXEC);
}
static int copy_directory(int source,int destination,unsigned depth) {
    if(depth>8) {errno=ELOOP;return 0;}
    int iteration=openat(source,".",O_RDONLY|O_DIRECTORY|O_CLOEXEC);
    DIR *dir=iteration<0 ? NULL : fdopendir(iteration);
    if(!dir) {if(iteration>=0) close(iteration);return 0;}
    struct dirent *entry;int ok=1;
    while((entry=readdir(dir))) {
        if(!strcmp(entry->d_name,".") || !strcmp(entry->d_name,"..")) continue;
        struct stat st;
        if(fstatat(source,entry->d_name,&st,0)) {ok=0;break;}
        if(S_ISREG(st.st_mode)) ok=copy_file(source,destination,entry->d_name);
        else if(S_ISDIR(st.st_mode)) {
            int in=openat(source,entry->d_name,O_RDONLY|O_DIRECTORY|O_CLOEXEC);
            int out=ensure_directory(destination,entry->d_name);
            ok=in>=0 && out>=0 && copy_directory(in,out,depth+1);
            if(in>=0) close(in);
            if(out>=0) close(out);
        } else {errno=EACCES;ok=0;}
        if(!ok) break;
    }
    closedir(dir);return ok;
}
int rg_user_storage_init(const char *directory) {
    char resolved[4096];
    if(!realpath(directory,resolved)) {
        if(errno!=ENOENT || strlen(directory)>=sizeof resolved) return 0;
        char path[4096],parent[4096];strcpy(path,directory);
        char *leaf=strrchr(path,'/');const char *name=path;
        if(leaf) {*leaf++=0;name=leaf;}
        if(!*name || !strcmp(name,".") || !strcmp(name,"..") ||
           !realpath(leaf ? (*path ? path : "/") : ".",parent)) return 0;
        if(strlen(parent)+strlen(name)+2>sizeof resolved) {errno=ENAMETOOLONG;return 0;}
        strcpy(resolved,parent);strcat(resolved,"/");strcat(resolved,name);
    }
    const char *game=rg_game_root();size_t length=strlen(game);
    size_t user_length=strlen(resolved);
    if(!length || (!strncmp(resolved,game,length) && (!resolved[length] || resolved[length]=='/')) ||
       (!strncmp(game,resolved,user_length) && (!game[user_length] || game[user_length]=='/'))) {errno=EACCES;return 0;}
    if(mkdir(resolved,0700) && errno!=EEXIST) return 0;
    int output=open(resolved,O_RDONLY|O_DIRECTORY|O_NOFOLLOW|O_CLOEXEC);
    if(output<0) return 0;
    struct stat st;
    if(!fstatat(output,".redguard-storage-v1",&st,AT_SYMLINK_NOFOLLOW)) {
        int ok=S_ISREG(st.st_mode);close(output);
        if(ok) strcpy(user_root,resolved);else errno=EACCES;
        return ok;
    }
    if(errno!=ENOENT) {close(output);return 0;}
    int input=open(game,O_RDONLY|O_DIRECTORY|O_CLOEXEC);int ok=input>=0;
    if(ok) {
        char name[256];
        int source=case_name(input,"SAVEGAME",name) ? openat(input,name,O_RDONLY|O_DIRECTORY|O_CLOEXEC) : -1;
        int missing=source<0 && errno==ENOENT;
        int target=ensure_directory(output,"SAVEGAME");
        ok=target>=0 && (source>=0 ? copy_directory(source,target,0) : missing);
        if(source>=0) close(source);
        if(target>=0) close(target);
    }
    if(ok) {
        int target=ensure_directory(output,"MAPS");
        char name[256];
        int source=case_name(input,"MAPS",name) ? openat(input,name,O_RDONLY|O_DIRECTORY|O_CLOEXEC) : -1;
        DIR *dir=source<0 ? NULL : fdopendir(source);
        ok=target>=0 && (dir || errno==ENOENT);
        if(dir) {
            struct dirent *entry;
            while(ok && (entry=readdir(dir))) {
                const char *dot=strrchr(entry->d_name,'.');
                if(dot && !strcasecmp(dot,".TSG")) ok=copy_file(source,target,entry->d_name);
            }
            closedir(dir);
        }
        if(target>=0) close(target);
    }
    if(ok) {
        int iteration=openat(input,".",O_RDONLY|O_DIRECTORY|O_CLOEXEC);
        DIR *dir=iteration<0 ? NULL : fdopendir(iteration);
        if(!dir) {if(iteration>=0) close(iteration);ok=0;}
        else {
            struct dirent *entry;
            while(ok && (entry=readdir(dir))) {
                const char *dot=strrchr(entry->d_name,'.');
                if(!strcasecmp(entry->d_name,"LOGBOOK.TXT") || (dot && !strcasecmp(dot,".INI")))
                    ok=copy_file(input,output,entry->d_name);
            }
            closedir(dir);
        }
    }
    if(input>=0) close(input);
    if(ok) {
        int marker=openat(output,".redguard-storage-v1",O_WRONLY|O_CREAT|O_EXCL|O_NOFOLLOW|O_CLOEXEC,0600);
        ok=marker>=0;
        if(marker>=0) {ok=!fsync(marker);close(marker);}
        if(ok) ok=!fsync(output);
    }
    close(output);
    if(ok) strcpy(user_root,resolved);
    return ok;
}
