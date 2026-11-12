#!/bin/sh
# Source checks that need no original game files, Docker or downloaded SDK.
set -eu
cd "$(dirname "$0")/.."
python3 -m unittest discover -s tests -p package_media.py
python3 -m unittest discover -s tests -p jump_trace_cases.py
python3 -m unittest discover -s tests -p book_menu_cases.py
python3 -m unittest discover -s tests -p startup_timing_cases.py
python3 -m compileall -q -x '/(\.git|\.cache|artifacts)/' .
build=$(mktemp -d)
trap 'rm -rf "$build"' EXIT HUP INT TERM
cc=${CC:-cc}
rust_runtime=$(python3 -m tools.rust_runtime --target x86_64-unknown-linux-gnu)
"$cc" -Wall -Wextra -Werror -O2 -no-pie -Iruntime tests/cd_image.c "$rust_runtime" -Wl,--gc-sections -ldl -lpthread -lm -o "$build/cd-image"
python3 tests/check_cd_image.py "$build/cd-image"
"$cc" -Wall -Wextra -Werror -O2 -no-pie -Iruntime -Ilinux tests/user_storage.c "$rust_runtime" -Wl,--gc-sections -ldl -lpthread -lm -o "$build/user-storage"
"$build/user-storage"
"$cc" -Wall -Wextra -Werror -O2 -Iruntime -Ilinux tests/import_interruption.c "$rust_runtime" -Wl,--gc-sections -ldl -lpthread -lm -o "$build/import-interruption"
"$build/import-interruption"

"$cc" -Wall -Wextra -Werror -O2 -no-pie -Iruntime -Ilinux tests/installed_movies.c "$rust_runtime" -Wl,--gc-sections -ldl -lpthread -lm -o "$build/installed-movies"
python3 - "$build/installed-movies" <<'PYTEST'
import subprocess,sys,tempfile
with tempfile.TemporaryDirectory() as directory:
    subprocess.run([sys.argv[1]],cwd=directory,check=True)
PYTEST

"$cc" -Wall -Wextra -Werror -O2 -Iruntime tests/native_timing.c "$rust_runtime" -Wl,--gc-sections -ldl -lpthread -lm -o "$build/timing"
"$build/timing"
