#define _GNU_SOURCE
#include "paths.h"
#include "user_storage.h"
#include <assert.h>
#include <dirent.h>
#include <fcntl.h>
#include <signal.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/stat.h>
#include <sys/wait.h>
#include <unistd.h>
int main(void) {
    char root[]="/tmp/rg-import-interruption-XXXXXX";assert(mkdtemp(root) && !chdir(root));
    assert(!mkdir("game",0700) && !mkdir("game/SAVEGAME",0700));
    const off_t size=128*1024*1024;
    int source=open("game/SAVEGAME/LARGE.SAV",O_CREAT|O_EXCL|O_RDWR,0600);assert(source>=0);
    assert(!ftruncate(source,size) && pwrite(source,"owned fixture",13,size-13)==13);close(source);
    assert(rg_path_root("game"));
    pid_t child=fork();assert(child>=0);
    if(!child) _exit(rg_user_storage_init("user") ? 0 : 1);
    int interrupted=0;
    for(unsigned i=0;i<10000 && !interrupted;++i) {
        DIR *dir=opendir("user/SAVEGAME");struct dirent *entry;
        if(dir) {
            while((entry=readdir(dir))) if(!strncmp(entry->d_name,".rg-import-",11)) {
                assert(!kill(child,SIGKILL));interrupted=1;break;
            }
            closedir(dir);
        }
        if(!interrupted) usleep(100);
    }
    int status;assert(waitpid(child,&status,0)==child);
    assert(interrupted && WIFSIGNALED(status) && WTERMSIG(status)==SIGKILL);
    assert(access("user/SAVEGAME/LARGE.SAV",F_OK));
    assert(rg_user_storage_init("user"));
    int output=open("user/SAVEGAME/LARGE.SAV",O_RDONLY);assert(output>=0);
    struct stat st;char tail[13];assert(!fstat(output,&st) && st.st_size==size);
    assert(pread(output,tail,13,size-13)==13 && !memcmp(tail,"owned fixture",13));close(output);
    puts("PASS killed first import never publishes a partial save; restart imports complete original bytes");
}
