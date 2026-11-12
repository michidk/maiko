#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <windows.h>
int main(int argc,char **argv) {
    char actual[MAX_PATH],expected[MAX_PATH];
    const char *directory=getenv("RG_TEST_EXPECTED_DIR");
    if ((argc!=3 && argc!=5) || strcmp(argv[1],"--game-dir") || !directory) return 99;
    if(argc==5 && (strcmp(argv[3],"--user-dir") || !strstr(argv[4],"user-data"))) return 99;
    DWORD a=GetFullPathNameA(argv[2],sizeof actual,actual,NULL);
    DWORD b=GetFullPathNameA(directory,sizeof expected,expected,NULL);
    printf("Launcher received: [%s]\n",argv[2]);
    while (a>3 && a<sizeof actual && actual[a-1]=='\\') actual[--a]=0;
    while (b>3 && b<sizeof expected && expected[b-1]=='\\') expected[--b]=0;
    if (!a || a>=sizeof actual || !b || b>=sizeof expected || _stricmp(actual,expected)) {
        fprintf(stderr,"FAIL launcher directory: [%s], expected [%s]\n",actual,expected);
        return 99;
    }
    puts("PASS CMD passes the expected directory through the Windows argument parser");
    return 7;
}
