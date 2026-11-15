#ifndef RG_MOVIE_FILES_H
#define RG_MOVIE_FILES_H

/* Root-level DOS D: SMK requests are aliases for installed movie files.
   Image-backed reads, when explicitly enabled, take precedence in file services. */
typedef int (*RgMovieLookup)(const char *relative, void *context);
const char *rg_movie_leaf(const char *requested);
/* Return 0 for other requests, 1 when lookup found a file, -1 when missing.
   Lookup receives a path relative to the game installation, using '/'. */
int rg_movie_find(const char *requested, RgMovieLookup lookup, void *context);

#endif
