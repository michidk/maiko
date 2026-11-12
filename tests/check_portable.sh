#!/bin/sh
# Source checks that need no original game files, Docker or downloaded SDK.
set -eu
cd "$(dirname "$0")/.."
python3 -m compileall -q -x '/(\.git|\.cache|artifacts)/' .
build=$(mktemp -d)
trap 'rm -rf "$build"' EXIT HUP INT TERM
cc=${CC:-cc}
"$cc" -Wall -Wextra -Werror -O2 -no-pie -Iruntime tests/cd_image.c runtime/cd_image.c -o "$build/cd-image"
python3 tests/check_cd_image.py "$build/cd-image"
"$cc" -Wall -Wextra -Werror -O2 -no-pie -Iruntime -Ilinux tests/user_storage.c linux/paths.c runtime/movie_files.c linux/user_storage.c linux/game_files.c runtime/cd_image.c -o "$build/user-storage"
"$build/user-storage"
"$cc" -Wall -Wextra -Werror -O2 -Iruntime -Ilinux tests/import_interruption.c linux/paths.c runtime/movie_files.c linux/user_storage.c -o "$build/import-interruption"
"$build/import-interruption"

"$cc" -Wall -Wextra -Werror -O2 -no-pie -Iruntime -Ilinux tests/installed_movies.c linux/paths.c linux/user_storage.c linux/game_files.c runtime/cd_image.c runtime/movie_files.c -o "$build/installed-movies"
python3 - "$build/installed-movies" <<'PYTEST'
import subprocess,sys,tempfile
with tempfile.TemporaryDirectory() as directory:
    subprocess.run([sys.argv[1]],cwd=directory,check=True)
PYTEST
