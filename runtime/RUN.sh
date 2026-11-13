#!/bin/sh
set -eu
runtime_dir=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
if [ "$#" -ne 1 ]; then
    printf 'Usage: %s /path/to/Redguard\nSelect the folder containing RGFX.EXE.\n' "$0" >&2
    exit 2
fi
case "$1" in
    /*) game_dir=$1 ;;
    *) game_dir=$PWD/$1 ;;
esac
cd "$runtime_dir"
export LD_LIBRARY_PATH="$runtime_dir${LD_LIBRARY_PATH:+:$LD_LIBRARY_PATH}"
exec ./redguard-native --game-dir "$game_dir" --user-dir "$runtime_dir/user-data"
