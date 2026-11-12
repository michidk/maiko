#include "movie_files.h"
#include <stdio.h>
#include <string.h>

const char *rg_movie_leaf(const char *requested) {
    if ((requested[0]!='D' && requested[0]!='d') || requested[1]!=':') return NULL;
    const char *leaf=requested+2;
    while (*leaf=='/' || *leaf=='\\') ++leaf;
    const char *dot=strrchr(leaf,'.');
    if (!dot || dot==leaf || strlen(dot)!=4 ||
        (dot[1]!='S' && dot[1]!='s') || (dot[2]!='M' && dot[2]!='m') ||
        (dot[3]!='K' && dot[3]!='k') || strpbrk(leaf,"\\/:*?<>|\"")) return NULL;
    return leaf;
}

int rg_movie_find(const char *requested, RgMovieLookup lookup, void *context) {
    const char *leaf=rg_movie_leaf(requested);
    if (!leaf) return 0;
    static const char *folders[]={"","MOVIES/","ANIMS/","VIDEO/"};
    char relative[4096];
    for (unsigned i=0;i<sizeof folders/sizeof *folders;++i) {
        int size=snprintf(relative,sizeof relative,"%s%s",folders[i],leaf);
        if (size<0 || (size_t)size>=sizeof relative) continue;
        if (lookup(relative,context)) return 1;
    }
    return -1;
}
