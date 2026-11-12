#include <windows.h>
#include <shlobj.h>
#include <string.h>
#include <stdio.h>

int rg_choose_game_directory(char *path, size_t capacity) {
    HRESULT initialized=CoInitializeEx(NULL,COINIT_APARTMENTTHREADED);
    if (FAILED(initialized)) return 0;
    BROWSEINFOA options={0};
    options.lpszTitle="Select your Redguard game-data folder containing RGFX.EXE";
    options.ulFlags=BIF_RETURNONLYFSDIRS|BIF_NEWDIALOGSTYLE;
    PIDLIST_ABSOLUTE selected=SHBrowseForFolderA(&options);
    char chosen[MAX_PATH];
    int valid=selected && SHGetPathFromIDListA(selected,chosen) && strlen(chosen)<capacity;
    if (valid) strcpy(path,chosen);
    CoTaskMemFree(selected);
    CoUninitialize();
    if (!valid) return 0;
    if (snprintf(chosen,sizeof chosen,"%s\\RGFX.EXE",path)>=(int)sizeof chosen ||
        GetFileAttributesA(chosen)==INVALID_FILE_ATTRIBUTES) {
        MessageBoxA(NULL,"Choose the inner Redguard folder containing RGFX.EXE, SYSTEM.INI and FXART.",
                    "Redguard game files not found",MB_OK|MB_ICONERROR);
        return 0;
    }
    return 1;
}
