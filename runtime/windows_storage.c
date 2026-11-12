#include "windows_storage.h"
#include "movie_files.h"
#include <stdio.h>
#include <string.h>
#include <ctype.h>
static char game_root[MAX_PATH],user_root[MAX_PATH];
static HANDLE root_lock;
const char *rg_storage_root(void) {return user_root;}
static int denied(void) {SetLastError(ERROR_ACCESS_DENIED);return 0;}
static int join(char output[MAX_PATH],const char *base,const char *suffix) {
    if(snprintf(output,MAX_PATH,"%s%s%s",base,*suffix ? "\\" : "",suffix)>=MAX_PATH) {SetLastError(ERROR_FILENAME_EXCED_RANGE);return 0;}
    return 1;
}
static int inside(const char *path,const char *root) {
    size_t n=strlen(root);return n && !_strnicmp(path,root,n) && (root[n-1]=='\\' || !path[n] || path[n]=='\\');
}
static int space(const char *path) {
    if(!_strnicmp(path,"SAVEGAME",8) && (!path[8] || path[8]=='\\')) return 1;
    const char *dot=strrchr(path,'.');
    if(!_strnicmp(path,"MAPS\\",5)) return !strchr(path+5,'\\') && dot && !_stricmp(dot,".TSG");
    return !strchr(path,'\\') && (!_stricmp(path,"LOGBOOK.TXT") || (dot && !_stricmp(dot,".INI")));
}
static int relative(const char *requested,char output[MAX_PATH]) {
    char path[MAX_PATH],cwd[MAX_PATH];size_t n=strlen(requested);
    if(n>=sizeof path) return denied();
    strcpy(path,requested);for(char *p=path;*p;++p) if(*p=='/') *p='\\';
    const char *suffix=path;output[0]=0;
    if(inside(path,game_root)) suffix=path+strlen(game_root);
    else if(inside(path,user_root)) suffix=path+strlen(user_root);
    else if(path[0] && path[1]==':') {
        if(toupper((unsigned char)path[0])!='C') return denied();
        suffix=path+2;
    } else if(path[0]!='\\') {
        if(!GetCurrentDirectoryA(sizeof cwd,cwd)) return 0;
        const char *base=inside(cwd,game_root) ? game_root : inside(cwd,user_root) ? user_root : NULL;
        if(!base) return denied();
        const char *tail=cwd+strlen(base);while(*tail=='\\') ++tail;strcpy(output,tail);
    }
    while(*suffix=='\\') ++suffix;
    char *state=NULL,*part=strtok_s((char *)suffix,"\\",&state);
    while(part) {
        size_t size=strlen(part);
        if(!strcmp(part,".")) {}
        else if(!strcmp(part,"..")) {
            if(!*output) return denied();
            char *last=strrchr(output,'\\');if(last) *last=0;else *output=0;
        } else {
            if(strpbrk(part,":<>|\"") || !size || part[size-1]==' ' || part[size-1]=='.') return denied();
            n=strlen(output);if(n+size+2>=MAX_PATH) return denied();
            if(n) output[n++]='\\';
            strcpy(output+n,part);
        }
        part=strtok_s(NULL,"\\",&state);
    }
    return 1;
}
void rg_storage_release(RgStorageGuard *guard) {
    DWORD saved=GetLastError();while(guard->count) CloseHandle(guard->directories[--guard->count]);SetLastError(saved);
}
static int lock_directory(const char *path,RgStorageGuard *guard) {
    if(guard->count==64) return denied();
    HANDLE h=CreateFileA(path,FILE_READ_ATTRIBUTES,FILE_SHARE_READ|FILE_SHARE_WRITE,NULL,OPEN_EXISTING,
                        FILE_FLAG_BACKUP_SEMANTICS|FILE_FLAG_OPEN_REPARSE_POINT,NULL);
    if(h==INVALID_HANDLE_VALUE) return 0;
    BY_HANDLE_FILE_INFORMATION info;
    if(!GetFileInformationByHandle(h,&info) || (info.dwFileAttributes&FILE_ATTRIBUTE_REPARSE_POINT) || !(info.dwFileAttributes&FILE_ATTRIBUTE_DIRECTORY)) {
        CloseHandle(h);return denied();
    }
    guard->directories[guard->count++]=h;return 1;
}
static int installed_movie(const char *relative,void *context) {
    char *output=context;
    if (!join(output,game_root,relative)) return 0;
    for (char *p=output;*p;++p) if (*p=='/') *p='\\';
    DWORD attributes=GetFileAttributesA(output);
    return attributes!=INVALID_FILE_ATTRIBUTES && !(attributes&FILE_ATTRIBUTE_DIRECTORY);
}
int rg_storage_path(const char *requested,int writing,char output[MAX_PATH],RgStorageGuard *guard) {
    guard->count=0;
    if(!game_root[0]) {
        if(writing) return denied();
        DWORD n=GetFullPathNameA(requested,MAX_PATH,output,NULL);return n && n<MAX_PATH;
    }
    /* DOS animation_drive is often D:\, independently of the host install
       drive. Resolve only root-level read-only SMK requests into installed
       movie folders. An explicitly mounted image is handled before this. */
    if(!writing && !inside(requested,game_root)) {
        int movie=rg_movie_find(requested,installed_movie,output);
        if(movie) {
            if(movie>0) {
                fprintf(stderr,"MOVIE loose %s -> %s\n",requested,output);return 1;
            }
            fprintf(stderr,"MOVIE missing %s; searched GAME_DIR, MOVIES, ANIMS, VIDEO; no installed file found\n",requested);
            SetLastError(ERROR_FILE_NOT_FOUND);return 0;
        }
    }
    char rel[MAX_PATH];if(!relative(requested,rel)) return 0;
    int writable=user_root[0] && space(rel);if(writing && !writable) return denied();
    if(!join(output,writable ? user_root : game_root,rel)) return 0;
    if(!writable) return 1;
    char parent[MAX_PATH];strcpy(parent,user_root);
    if(!lock_directory(parent,guard)) return 0;
    char *part=rel;
    for(char *slash=strchr(part,'\\');slash;slash=strchr(part,'\\')) {
        *slash=0;char next[MAX_PATH];
        if(!join(next,parent,part) || !lock_directory(next,guard)) {rg_storage_release(guard);return 0;}
        strcpy(parent,next);part=slash+1;
    }
    DWORD attr=GetFileAttributesA(output);
    if(attr!=INVALID_FILE_ATTRIBUTES && (attr&FILE_ATTRIBUTE_REPARSE_POINT)) {rg_storage_release(guard);return denied();}
    return 1;
}
int rg_storage_maps(const char *pattern,char output[MAX_PATH]) {
    if(!user_root[0]) return 0;
    char rel[MAX_PATH];if(!relative(pattern,rel) || _strnicmp(rel,"MAPS\\",5) || strchr(rel+5,'\\')) return 0;
    return join(output,user_root,rel);
}
static int directory(const char *path) {
    if(!CreateDirectoryA(path,NULL) && GetLastError()!=ERROR_ALREADY_EXISTS) return 0;
    RgStorageGuard g={0};int ok=lock_directory(path,&g);rg_storage_release(&g);return ok;
}
static int copy_file(const char *source,const char *target) {
    DWORD attrs=GetFileAttributesA(target);
    if(attrs!=INVALID_FILE_ATTRIBUTES) return !(attrs&(FILE_ATTRIBUTE_DIRECTORY|FILE_ATTRIBUTE_REPARSE_POINT));
    if(GetLastError()!=ERROR_FILE_NOT_FOUND) return 0;
    static unsigned sequence;
    char temporary[MAX_PATH];HANDLE output=INVALID_HANDLE_VALUE;
    for(unsigned attempt=0;attempt<100 && output==INVALID_HANDLE_VALUE;++attempt) {
        if(snprintf(temporary,sizeof temporary,"%s.rg-import-%lu-%u",target,GetCurrentProcessId(),++sequence)>=MAX_PATH) return denied();
        output=CreateFileA(temporary,GENERIC_WRITE,0,NULL,CREATE_NEW,FILE_FLAG_OPEN_REPARSE_POINT,NULL);
        if(output==INVALID_HANDLE_VALUE && GetLastError()!=ERROR_FILE_EXISTS && GetLastError()!=ERROR_ALREADY_EXISTS) break;
    }
    if(output==INVALID_HANDLE_VALUE) return 0;
    HANDLE input=CreateFileA(source,GENERIC_READ,FILE_SHARE_READ,NULL,OPEN_EXISTING,0,NULL);
    int ok=input!=INVALID_HANDLE_VALUE;DWORD count=0;char buffer[16384];
    while(ok) {
        ok=ReadFile(input,buffer,sizeof buffer,&count,NULL);if(!ok || !count) break;
        DWORD offset=0;
        while(ok && offset<count) {DWORD written=0;ok=WriteFile(output,buffer+offset,count-offset,&written,NULL) && written>0;offset+=written;}
    }
    if(input!=INVALID_HANDLE_VALUE) CloseHandle(input);
    if(ok) ok=FlushFileBuffers(output);
    CloseHandle(output);
    if(ok && !MoveFileExA(temporary,target,MOVEFILE_WRITE_THROUGH)) {
        DWORD error=GetLastError();attrs=GetFileAttributesA(target);
        ok=(error==ERROR_ALREADY_EXISTS || error==ERROR_FILE_EXISTS) && attrs!=INVALID_FILE_ATTRIBUTES && !(attrs&(FILE_ATTRIBUTE_DIRECTORY|FILE_ATTRIBUTE_REPARSE_POINT));
    }
    DWORD saved=GetLastError();DeleteFileA(temporary);SetLastError(saved);return ok;
}
static int copy_tree(const char *source,const char *target,unsigned depth) {
    if(depth>8 || !directory(target)) return denied();
    char pattern[MAX_PATH];if(!join(pattern,source,"*")) return 0;
    WIN32_FIND_DATAA data;HANDLE h=FindFirstFileA(pattern,&data);
    if(h==INVALID_HANDLE_VALUE) return GetLastError()==ERROR_FILE_NOT_FOUND || GetLastError()==ERROR_PATH_NOT_FOUND;
    int ok=1;
    do {
        if(!strcmp(data.cFileName,".") || !strcmp(data.cFileName,"..")) continue;
        char in[MAX_PATH],out[MAX_PATH];
        if(!join(in,source,data.cFileName) || !join(out,target,data.cFileName)) {ok=0;break;}
        if(data.dwFileAttributes&FILE_ATTRIBUTE_DIRECTORY) ok=copy_tree(in,out,depth+1);
        else ok=copy_file(in,out);
        if(!ok) break;
    } while(FindNextFileA(h,&data));
    if(ok && GetLastError()!=ERROR_NO_MORE_FILES) ok=0;
    FindClose(h);return ok;
}
int rg_storage_canonical_directory(const char *path,char output[MAX_PATH]) {
    HANDLE h=CreateFileA(path,FILE_READ_ATTRIBUTES,FILE_SHARE_READ|FILE_SHARE_WRITE,NULL,OPEN_EXISTING,FILE_FLAG_BACKUP_SEMANTICS,NULL);
    if(h==INVALID_HANDLE_VALUE) return 0;
    char resolved[MAX_PATH];DWORD n=GetFinalPathNameByHandleA(h,resolved,sizeof resolved,FILE_NAME_NORMALIZED|VOLUME_NAME_DOS);
    CloseHandle(h);if(!n || n>=MAX_PATH) return 0;
    if(!strncmp(resolved,"\\\\?\\UNC\\",8)) {
        output[0]='\\';output[1]='\\';strcpy(output+2,resolved+8);return 1;
    }
    const char *value=!strncmp(resolved,"\\\\?\\",4) ? resolved+4 : resolved;
    if(strlen(value)<3 || value[1]!=':') return denied();
    strcpy(output,value);return 1;
}
int rg_user_storage_init(const char *game,const char *user) {
    char source[MAX_PATH],target[MAX_PATH];
    DWORD n=GetFullPathNameA(game,MAX_PATH,source,NULL);if(!n || n>=MAX_PATH) return 0;
    n=GetFullPathNameA(user,MAX_PATH,target,NULL);if(!n || n>=MAX_PATH) return 0;
    while(strlen(source)>3 && source[strlen(source)-1]=='\\') source[strlen(source)-1]=0;
    while(strlen(target)>3 && target[strlen(target)-1]=='\\') target[strlen(target)-1]=0;
    if(!rg_storage_canonical_directory(source,source)) return 0;
    char parent[MAX_PATH],leaf[MAX_PATH];strcpy(parent,target);
    char *slash=strrchr(parent,'\\');if(!slash) return denied();
    strcpy(leaf,slash+1);if(slash==parent+2) slash[1]=0;else *slash=0;
    char actual_parent[MAX_PATH];if(!rg_storage_canonical_directory(parent,actual_parent) || !join(target,actual_parent,leaf)) return 0;
    if(inside(target,source) || inside(source,target)) return denied();
    if(!directory(target)) return 0;
    RgStorageGuard guard={0};if(!lock_directory(target,&guard)) return 0;
    char marker[MAX_PATH];if(!join(marker,target,".redguard-storage-v1")) {rg_storage_release(&guard);return 0;}
    DWORD attrs=GetFileAttributesA(marker);int ok=1;
    if(attrs!=INVALID_FILE_ATTRIBUTES) ok=!(attrs&(FILE_ATTRIBUTE_DIRECTORY|FILE_ATTRIBUTE_REPARSE_POINT));
    else {
        char in[MAX_PATH],out[MAX_PATH];
        ok=join(in,source,"SAVEGAME") && join(out,target,"SAVEGAME") && copy_tree(in,out,0);
        if(ok) ok=join(out,target,"MAPS") && directory(out);
        const char *patterns[]={"*.INI","LOGBOOK.TXT","MAPS\\*.TSG"};
        for(unsigned i=0;ok && i<3;++i) {
            char pattern[MAX_PATH];if(!join(pattern,source,patterns[i])) {ok=0;break;}
            WIN32_FIND_DATAA data;HANDLE h=FindFirstFileA(pattern,&data);
            if(h==INVALID_HANDLE_VALUE) {ok=GetLastError()==ERROR_FILE_NOT_FOUND || GetLastError()==ERROR_PATH_NOT_FOUND;continue;}
            do {
                if(data.dwFileAttributes&FILE_ATTRIBUTE_DIRECTORY) continue;
                char suffix[MAX_PATH];if(i==2) snprintf(suffix,sizeof suffix,"MAPS\\%s",data.cFileName);else strcpy(suffix,data.cFileName);
                ok=join(in,source,suffix) && join(out,target,suffix) && copy_file(in,out);if(!ok) break;
            } while(FindNextFileA(h,&data));
            if(ok && GetLastError()!=ERROR_NO_MORE_FILES) ok=0;
            FindClose(h);
        }
        if(ok) {
            HANDLE h=CreateFileA(marker,GENERIC_WRITE,0,NULL,CREATE_NEW,FILE_FLAG_OPEN_REPARSE_POINT,NULL);
            ok=h!=INVALID_HANDLE_VALUE;if(ok) {ok=FlushFileBuffers(h);CloseHandle(h);}
        }
    }
    if(ok) {
        if(root_lock) CloseHandle(root_lock);
        root_lock=guard.directories[0];guard.count=0;strcpy(game_root,source);strcpy(user_root,target);
        fprintf(stderr,"Isolated native user storage: %s\n",user_root);
    }
    rg_storage_release(&guard);return ok;
}

int rg_storage_game_selected(void) {return game_root[0]!=0;}
int rg_storage_game_root(const char *path) {
    char canonical[MAX_PATH];if(!rg_storage_canonical_directory(path,canonical)) return 0;
    strcpy(game_root,canonical);return 1;
}
unsigned rg_host_cwd(unsigned capacity,char *buffer) {
    if(!game_root[0]) return GetCurrentDirectoryA(capacity,buffer);
    char cwd[MAX_PATH];if(!GetCurrentDirectoryA(sizeof cwd,cwd)) return 0;
    const char *base=inside(cwd,game_root) ? game_root : inside(cwd,user_root) ? user_root : NULL;
    if(!base) return 0;
    const char *tail=cwd+strlen(base);while(*tail=='\\') ++tail;
    size_t size=3+strlen(tail)+1;
    if(!capacity) return (unsigned)size;
    if(capacity<size) return 0;
    strcpy(buffer,"C:\\");strcpy(buffer+3,tail);return (unsigned)size-1;
}
int rg_host_chdir(const char *path) {
    if(!game_root[0]) return SetCurrentDirectoryA(path)!=0;
    char resolved[MAX_PATH];RgStorageGuard guard;
    if(!rg_storage_path(path,0,resolved,&guard)) return 0;
    int ok=SetCurrentDirectoryA(resolved)!=0;rg_storage_release(&guard);return ok;
}
