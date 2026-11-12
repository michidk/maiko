#include "game_files.h"
#include "cd_image.h"
#include "windows_storage.h"
#include <stdio.h>
#include <string.h>
#include <windows.h>
static HANDLE handles[256];
static unsigned char writable[256];
static uint32_t dta_address;
static uint32_t search_id;
static struct {HANDLE handle; uint32_t id; unsigned attributes; int maps,overlay_phase; char overlay[MAX_PATH];} searches[64];
#pragma pack(push,1)
typedef struct {
    unsigned char reserved[21], attributes;
    WORD time,date;
    DWORD size;
    char name[13];
} DosDta;
#pragma pack(pop)
_Static_assert(sizeof(DosDta)==43,"Original DOS search result size");
/* Keep native parent directories intact; DOS normalizes the requested leaf. */
static int dos_leaf(const char *path, char result[MAX_PATH]) {
    size_t length = strlen(path);
    if (length >= MAX_PATH) return 0;
    const char *leaf = path;
    for (const char *p = path; *p; ++p)
        if (*p == '/' || *p == '\\' || *p == ':') leaf = p+1;
    if (!*leaf || !strcmp(leaf,".") || !strcmp(leaf,"..")) return 0;
    const char *dot = strchr(leaf,'.');
    size_t name = dot ? (size_t)(dot-leaf) : strlen(leaf);
    size_t extension = dot ? strlen(dot+1) : 0;
    if (name <= 8 && extension <= 3) return 0;
    size_t parent = (size_t)(leaf-path);
    memcpy(result,path,parent);
    size_t used = name > 8 ? 8 : name;
    memcpy(result+parent,leaf,used);
    used += parent;
    if (dot) {
        result[used++] = '.';
        size_t count = extension > 3 ? 3 : extension;
        memcpy(result+used,dot+1,count);
        used += count;
    }
    result[used] = 0;
    return 1;
}
static unsigned dos_error(DWORD error) {
    switch (error) {
    case ERROR_FILE_NOT_FOUND: return 2;
    case ERROR_PATH_NOT_FOUND: return 3;
    case ERROR_TOO_MANY_OPEN_FILES: return 4;
    case ERROR_INVALID_HANDLE: return 6;
    default: return 5;
    }
}
static void error(GuestFrame *f, unsigned code) { f->eax = code; f->flags |= 1; }
static void success(GuestFrame *f, uint32_t result) { f->eax = result; f->flags &= ~(uint64_t)1; }
static int find_file(GuestFrame *f, unsigned operation) {
    if (!dta_address) return 0;
    DosDta *dta=(DosDta *)(uintptr_t)dta_address;
    unsigned slot=64;
    WIN32_FIND_DATAA data;
    BOOL found;
    if (operation==0x4e) {
        for (unsigned i=0;i<64;++i) if (!searches[i].handle) {slot=i;break;}
        if (slot==64) {error(f,4);return 1;}
        const char *pattern=(const char *)(uintptr_t)(uint32_t)f->edx;
        fprintf(stderr,"FIND %s attributes=%02x\n",pattern,(unsigned)f->ecx&255);
        char resolved[MAX_PATH];RgStorageGuard guard;
        if(!rg_storage_path(pattern,0,resolved,&guard)) {error(f,dos_error(GetLastError()));return 1;}
        searches[slot].maps=rg_storage_maps(pattern,searches[slot].overlay);
        searches[slot].overlay_phase=0;
        HANDLE search=FindFirstFileA(resolved,&data);
        rg_storage_release(&guard);
        if(search==INVALID_HANDLE_VALUE && searches[slot].maps) {
            search=FindFirstFileA(searches[slot].overlay,&data);searches[slot].overlay_phase=1;
        }
        if (search==INVALID_HANDLE_VALUE) {
            DWORD code=GetLastError();
            error(f,code==ERROR_FILE_NOT_FOUND ? 18 : dos_error(code)); return 1;
        }
        searches[slot].handle=search;
        searches[slot].id=++search_id;
        searches[slot].attributes=(unsigned)f->ecx;
        found=TRUE;
    } else {
        uint32_t token[2]; memcpy(token,dta->reserved,sizeof token);
        if (token[0]==0x52474454)
            for (unsigned i=0;i<64;++i)
                if (searches[i].handle && searches[i].id==token[1]) {slot=i;break;}
        if (slot==64) {error(f,18);return 1;}
        found=FindNextFileA(searches[slot].handle,&data);
    }
    for(;;) {
        if(!found && searches[slot].maps && !searches[slot].overlay_phase) {
            FindClose(searches[slot].handle);
            searches[slot].handle=FindFirstFileA(searches[slot].overlay,&data);
            searches[slot].overlay_phase=1;found=searches[slot].handle!=INVALID_HANDLE_VALUE;
        }
        if(!found) break;
        const char *dot=strrchr(data.cFileName,'.');
        int snapshot=dot && !_stricmp(dot,".TSG");
        int skip=searches[slot].maps && snapshot!=searches[slot].overlay_phase;
        if(!skip && !(data.dwFileAttributes & (FILE_ATTRIBUTE_HIDDEN|FILE_ATTRIBUTE_SYSTEM|FILE_ATTRIBUTE_DIRECTORY) & ~searches[slot].attributes)) break;
        found=FindNextFileA(searches[slot].handle,&data);
    }
    if (!found) {
        DWORD code=GetLastError();
        FindClose(searches[slot].handle); searches[slot].handle=NULL;
        error(f,(code==ERROR_NO_MORE_FILES || code==ERROR_FILE_NOT_FOUND) ? 18 : dos_error(code)); return 1;
    }
    const char *name=data.cAlternateFileName[0] ? data.cAlternateFileName : data.cFileName;
    if (strlen(name)>=sizeof dta->name) {
        FindClose(searches[slot].handle); searches[slot].handle=NULL;
        error(f,50); return 1;
    }
    memset(dta,0,sizeof *dta);
    uint32_t token[]={0x52474454,searches[slot].id};
    memcpy(dta->reserved,token,sizeof token);
    dta->attributes=(unsigned char)(data.dwFileAttributes & 0x3f);
    FILETIME local;
    if (FileTimeToLocalFileTime(&data.ftLastWriteTime,&local))
        FileTimeToDosDateTime(&local,&dta->date,&dta->time);
    dta->size=data.nFileSizeLow;
    strcpy(dta->name,name); CharUpperBuffA(dta->name,(DWORD)strlen(dta->name));
    success(f,0); return 1;
}
int rg_file_service(GuestFrame *f) {
    if(rg_cd_service(f)) return 1;
    unsigned op = ((uint32_t)f->eax >> 8) & 255;
    unsigned h = (uint32_t)f->ebx;
    switch (op) {
    case 0x39:case 0x3a:case 0x41:case 0x56: {
        char path[MAX_PATH];RgStorageGuard guard;
        if(!rg_storage_path((const char *)(uintptr_t)(uint32_t)f->edx,1,path,&guard)) {error(f,dos_error(GetLastError()));return 1;}
        int ok=0;
        if(op==0x39) ok=CreateDirectoryA(path,NULL);
        else if(op==0x3a) ok=RemoveDirectoryA(path);
        else if(op==0x41) ok=DeleteFileA(path);
        else {
            char target[MAX_PATH];RgStorageGuard destination;
            if(rg_storage_path((const char *)(uintptr_t)(uint32_t)f->edi,1,target,&destination)) {
                ok=MoveFileA(path,target);rg_storage_release(&destination);
            }
        }
        DWORD code=GetLastError();rg_storage_release(&guard);
        if(ok) success(f,0);else error(f,dos_error(code));return 1;
    }
    case 0x1a: dta_address=(uint32_t)f->edx; return 1;
    case 0x4e: case 0x4f: return find_file(f,op);
    case 0x19: {
        if(rg_storage_game_selected()) {f->eax=(f->eax&~(uint64_t)255)|2;return 1;}
        char current[MAX_PATH];
        if (!GetCurrentDirectoryA(sizeof current, current) || current[1] != ':') return 0;
        f->eax = (f->eax & ~(uint64_t)255) | ((current[0] & ~32) - 'A');
        return 1;
    }
    case 0x0e: {
        if(rg_storage_game_selected()) {f->eax=(f->eax&~(uint64_t)255)|4;return 1;}
        unsigned drive = f->edx & 255;
        if (drive < 26) {
            char path[] = "A:"; path[0] += drive;
            /* DOS leaves the selected drive unchanged when it does not exist. */
            SetCurrentDirectoryA(path);
        }
        DWORD drives = GetLogicalDrives();
        unsigned highest = 0;
        while (drives) { ++highest; drives >>= 1; }
        f->eax = (f->eax & ~(uint64_t)255) | highest;
        return 1;
    }
    case 0x40: {
        if(h!=1 && h!=2 && (h>=256 || !handles[h] || !writable[h])) {error(f,5);return 1;}
        DWORD count;
        HANDLE stream=h==1 || h==2 ? GetStdHandle(h==1 ? STD_OUTPUT_HANDLE : STD_ERROR_HANDLE) : handles[h];
        if(!f->ecx && h!=1 && h!=2) {
            if(SetEndOfFile(stream)) success(f,0);else error(f,dos_error(GetLastError()));return 1;
        }
        if (!WriteFile(stream,(void *)(uintptr_t)(uint32_t)f->edx,(uint32_t)f->ecx,&count,NULL)) error(f,dos_error(GetLastError()));
        else success(f,count);
        return 1;
    }
    case 0x43: {
        if ((f->eax & 255) != 0) { error(f,5); return 1; }
        const char *path = (const char *)(uintptr_t)(uint32_t)f->edx;
        char resolved[MAX_PATH];RgStorageGuard guard;
        if(!rg_storage_path(path,0,resolved,&guard)) {error(f,dos_error(GetLastError()));return 1;}
        path=resolved;
        DWORD attributes = GetFileAttributesA(path);
        char fallback[MAX_PATH];
        if (attributes == INVALID_FILE_ATTRIBUTES && GetLastError() == ERROR_FILE_NOT_FOUND && dos_leaf(path,fallback))
            attributes = GetFileAttributesA(fallback);
        rg_storage_release(&guard);
        if (attributes == INVALID_FILE_ATTRIBUTES) error(f,dos_error(GetLastError()));
        else { f->ecx = attributes & 0x3f; success(f,(uint32_t)f->eax); }
        return 1;
    }
    case 0x44: {
        if ((f->eax & 255) != 0) return 0;
        HANDLE stream = h == 1 ? GetStdHandle(STD_OUTPUT_HANDLE) : h == 2 ? GetStdHandle(STD_ERROR_HANDLE) : h < 256 ? handles[h] : NULL;
        if (!stream) { error(f,6); return 1; }
        DWORD type = GetFileType(stream);
        if (type == FILE_TYPE_UNKNOWN && GetLastError()) error(f,dos_error(GetLastError()));
        else { f->edx = type == FILE_TYPE_CHAR ? 0x80 : 0; success(f,(uint32_t)f->edx); }
        return 1;
    }
    case 0x3c:case 0x3d: {
        const char *requested=(const char *)(uintptr_t)(uint32_t)f->edx;
        unsigned access=op==0x3c ? 2 : f->eax&3;
        if(access==3) {error(f,12);return 1;}
        char resolved[MAX_PATH];RgStorageGuard guard;
        if(!rg_storage_path(requested,access!=0,resolved,&guard)) {error(f,dos_error(GetLastError()));return 1;}
        const char *path=resolved;
        DWORD rights=access==0 ? GENERIC_READ : access==1 ? GENERIC_WRITE : GENERIC_READ|GENERIC_WRITE;
        HANDLE file = CreateFileA(path,rights,FILE_SHARE_READ,NULL,op==0x3c ? OPEN_ALWAYS : OPEN_EXISTING,
                                 access ? FILE_FLAG_OPEN_REPARSE_POINT : 0,NULL);
        char fallback[MAX_PATH];
        if (file == INVALID_HANDLE_VALUE && GetLastError() == ERROR_FILE_NOT_FOUND && dos_leaf(path,fallback)) {
            file = CreateFileA(fallback, GENERIC_READ, FILE_SHARE_READ, NULL, OPEN_EXISTING, 0, NULL);
            if (file != INVALID_HANDLE_VALUE) fprintf(stderr,"DOS leaf %s -> %s\n",path,fallback);
        }
        DWORD code=GetLastError();
        if(file!=INVALID_HANDLE_VALUE) {
            BY_HANDLE_FILE_INFORMATION info;
            if(!GetFileInformationByHandle(file,&info) || (info.dwFileAttributes&(FILE_ATTRIBUTE_DIRECTORY|(access ? FILE_ATTRIBUTE_REPARSE_POINT : 0))) ||
               (op==0x3c && !SetEndOfFile(file))) {CloseHandle(file);file=INVALID_HANDLE_VALUE;code=ERROR_ACCESS_DENIED;}
        }
        rg_storage_release(&guard);
        if (file == INVALID_HANDLE_VALUE) { error(f, dos_error(code)); return 1; }
        for (unsigned n=5; n<256; ++n) if (!handles[n]) {
            handles[n] = file;writable[n]=access!=0; success(f,n); fprintf(stderr,"OPEN %s -> %u\n",path,n); return 1;
        }
        CloseHandle(file); error(f,4); return 1;
    }
    case 0x3e:
    case 0x3f:
    case 0x42:
        if (h >= 256 || !handles[h]) { error(f,6); return 1; }
        if (op == 0x3e) {
            if (!CloseHandle(handles[h])) error(f,dos_error(GetLastError()));
            else { handles[h] = NULL; success(f,0); }
        } else if (op == 0x3f) {
            DWORD count;
            if (!ReadFile(handles[h],(void *)(uintptr_t)(uint32_t)f->edx,(uint32_t)f->ecx,&count,NULL)) error(f,dos_error(GetLastError()));
            else success(f,count);
        } else {
            LARGE_INTEGER distance, result;
            distance.QuadPart = (int32_t)(((f->ecx & 65535) << 16) | (f->edx & 65535));
            if ((f->eax & 255)>2 || !SetFilePointerEx(handles[h],distance,&result,(DWORD)f->eax&255)) error(f,dos_error(GetLastError()));
            else { success(f,result.LowPart & 65535); f->edx = (result.LowPart >> 16) & 65535; }
        }
        return 1;
    default: return 0;
    }
}
