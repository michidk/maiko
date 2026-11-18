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
"$cc" -Wall -Wextra -Werror -O2 -no-pie -Iruntime -Ilinux tests/user_storage.c linux/paths.c linux/user_storage.c linux/game_files.c runtime/cd_image.c -o "$build/user-storage"
"$build/user-storage"
"$cc" -Wall -Wextra -Werror -O2 -Iruntime -Ilinux tests/import_interruption.c linux/paths.c linux/user_storage.c -o "$build/import-interruption"
"$build/import-interruption"

# The Mac branch also checks that source manifests reject changed inputs.
if [ -f tests/macos_source_manifest.py ]; then
    python3 -m unittest discover -s tests -p macos_source_manifest.py
fi
