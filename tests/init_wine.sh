#!/bin/sh
# Initialize the amd64 Wine prefix without waiting on unavailable 32-bit applets.
set -eu
prefix=${1:?Wine prefix required}
export WINEPREFIX="$prefix" WINEARCH=win64 WINEDEBUG=-all
export WINEDLLOVERRIDES='winemenubuilder.exe,mscoree,mshtml='
result=0
env -u DISPLAY timeout -k 3 20 /usr/lib/wine/wine64 cmd.exe /c exit 0 || result=$?
if [ "$result" != 0 ] && [ "$result" != 124 ]; then exit "$result"; fi
/usr/lib/wine/wineserver64 -k

/usr/lib/wine/wineserver64 -w
