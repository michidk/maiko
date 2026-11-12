#include <assert.h>
#include <stdio.h>
#include <string.h>
#include <windows.h>
#include <shlobj.h>
int rg_choose_game_directory(char *path, size_t capacity);
static DWORD owner;
static HWND picker;
static BOOL CALLBACK find_picker(HWND window, LPARAM unused) {
    (void)unused;
    char klass[64];
    GetClassNameA(window,klass,sizeof klass);
    if (IsWindowVisible(window) && !strcmp(klass,"#32770")) picker=window;
    return TRUE;
}
static DWORD WINAPI select_directory(void *cancel) {
    for (unsigned i=0;i<200 && !picker;++i) {
        EnumThreadWindows(owner,find_picker,0);
        if (!picker) Sleep(50);
    }
    assert(picker);
    if (cancel) PostMessageA(picker,WM_COMMAND,IDCANCEL,0);
    else {
        SendMessageA(picker,BFFM_SETSELECTIONA,TRUE,(LPARAM)"Z:\\game");
        PostMessageA(picker,WM_COMMAND,IDOK,0);
    }
    return 0;
}
int main(void) {
    owner=GetCurrentThreadId();
    char path[MAX_PATH];
    HANDLE worker=CreateThread(NULL,0,select_directory,NULL,0,NULL);
    assert(worker && rg_choose_game_directory(path,sizeof path));
    assert(!strcmp(path,"Z:\\game"));
    assert(WaitForSingleObject(worker,1000)==WAIT_OBJECT_0); CloseHandle(worker);
    picker=NULL;
    worker=CreateThread(NULL,0,select_directory,(void *)1,0,NULL);
    assert(worker && !rg_choose_game_directory(path,sizeof path));
    assert(WaitForSingleObject(worker,1000)==WAIT_OBJECT_0); CloseHandle(worker);
    puts("PASS real Windows folder dialog selection and cancel");
}
