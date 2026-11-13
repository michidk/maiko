#!/bin/sh
set -eu
package_dir=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
case "$(uname -m)" in
    arm64) architecture=arm64 ;;
    x86_64) architecture=x86_64 ;;
    *) echo "This build requires an Intel or Apple Silicon Mac." >&2; exit 1 ;;
esac
cd "$package_dir"
exec "$package_dir/redguard-native-$architecture" --game-dir "$package_dir/game" "$@"
