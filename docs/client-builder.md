# Build a package in your browser or terminal

The static GitHub Pages site accepts your owned `RGFX.EXE`, detects Windows, Linux or macOS with a manual override. Each selection builds and downloads a runtime ZIP. Mac packages contain both Intel and Apple Silicon executables in one universal binary. A worker does the work entirely on your device. The page fetches static runtime kits; it never uploads your executable, generated assembly or package.

Both interfaces use the same Rust library for input validation and package creation. The browser uses an isolated worker; the CLI uses a private temporary directory.

The kits include controller support and automatic game-folder discovery. Start `redguard-native.exe` on Windows or `./redguard-native` on Linux directly; it asks for the game folder only if it cannot find it. The included README repeats this.

For macOS, select `RGFX.EXE`, build the ZIP and extract it. Open
`redguard-native` and select your owned installation when asked. The Mac runtime
has no command launcher. See the [Mac launch guide](../platforms/macos/LAUNCH.md)
for quarantine removal, saves and logs.

Mac builds run SR's LLasm translator, the D frontend, LLVM optimization and both
code generators, then link and combine the two Mach-O executables. These tools
run locally in the worker. The Mac build needs a current desktop browser and
more time and memory than Windows/Linux builds. Cancel terminates the worker
and releases its compiler state.

## Use the local CLI

Download the patcher for your system from a versioned release and install [Wasmtime](https://wasmtime.dev/). Alternatively, install Rust and build the patcher yourself:

```sh
cargo build --locked --release -p maiko-patcher --bin maiko-patcher
```

Download the matching runtime kit from the same release, and run:

```sh
maiko-patcher build --input /path/to/RGFX.EXE --target linux \
  --kit maiko-runtime-kit-linux.zip --output maiko-linux.zip
```

The local Wasmtime CLI supports Linux and Windows. Use `--target windows` with `maiko-runtime-kit-windows.zip` for Windows x64. For macOS, use the browser or the [local source build](../platforms/macos/README.md). `--wasmtime PATH` selects a local Wasmtime installation. `--v5-fixes` adds the optional [version 5 fixes](../README.md#version-5-fixes); the browser builder offers the same option as a checkbox. The CLI uses a private temporary directory, reads the original game without modifying it, and refuses to overwrite the output ZIP.

Extract the generated ZIP into a separate writable folder. Windows users double-click `redguard-native.exe`; Linux users run `./redguard-native` in a terminal. The program looks for the Redguard folder itself (remembered folder, its own folder and the folders around it, standard GOG folders) and asks only if it finds nothing. The Linux package includes SDL2, SDL2_mixer, GLU and OpenGL dispatch libraries that the program finds beside itself. It needs a glibc 2.36 or newer desktop with X11/XWayland and a working OpenGL driver. Saves and settings go to the package's `user-data` directory. Startup options such as `--skip-intro` are command-line options (a shortcut's Target on Windows); see the README.

## Runtime kits

A kit holds everything needed to turn an owned `RGFX.EXE` into the native game, and no game code:

- **Compilers**, built for WebAssembly: SR (the static recompiler), NASM and LLVM's LLD linker. The Mac kit adds SR's LLasm output, the browser D compiler, LLVM `opt` and `llc`.
- **Link inputs**: C runtime start files, system import libraries, the Rust host and SR's heap and audio objects. The Mac kit has the same objects for both architectures and system-library stubs that list only library names, versions and imported symbol names; it contains no Apple SDK.
- **The packaged runtime**: the renderer, SDL and (on Linux) GL libraries, licence notices and corresponding source.

The browser or CLI checks the executable, translates it with SR, assembles the four engine objects with NASM (Mac: compiles them with LLVM for both architectures) and links them with LLD into the program. Browser and CLI produce byte-identical executables. Changed or unrecognized executables fail validation before anything is built.

The Linux kit includes SDL2 2.30.11, SDL2_mixer 2.8.1, GLU and OpenGL dispatch libraries, and links against glibc 2.36. The mixer supports WAVE playback. Keep the notices with any redistributed support libraries. Game files and generated playable packages must stay out of release uploads.

## Build and deploy the site

Use Bun, Rust's `wasm32-unknown-unknown` target and `wasm-bindgen-cli` 0.2.129:

```sh
bun install --frozen-lockfile
bun test web
bun run build
```

A release build serves the kits that release built. Otherwise the site build downloads `maiko-runtime-kit-{linux,windows,macos}.zip` from `releases/latest/download`, checks each against that release's `SHA256SUMS`, and serves them from the same origin. Nothing is pinned in the repository. For an offline build, set `MAIKO_KIT_DIRECTORY` to a folder with those three ZIPs and a `SHA256SUMS`; `MAIKO_KIT_URL` selects another release URL. The content security policy restricts browser requests to the page's origin.

Releases deploy the site. `.github/workflows/pages.yml` redeploys main against the latest release's kits on manual dispatch. Set **Settings → Pages → Source → GitHub Actions** before deploying. The managed GitHub App needs Pages write permission to enable that setting through the API.


## Build the runtime kits

The release workflow builds the kits; no game file is involved. To build them locally:

```sh
python3 -m tools.build_kits --output kits
```

This writes `maiko-runtime-kit-linux.zip`, `maiko-runtime-kit-windows.zip`, `maiko-runtime-kit-macos.zip` and their `SHA256SUMS`. It needs Docker, Bun, Rust with `rustup`, Git, curl, patch, tar, CMake, Ninja and a native C++ compiler. The first run builds the WebAssembly compilers (`maiko-kit tools` and `maiko-kit tools-macos`: checksum-pinned LLVM 19.1.0, WASI SDK 29 and browser-d sources), which takes hours; later runs reuse `.cache/client-toolchain` and `.cache/client-macos-toolchain`. `MAIKO_JOBS` sets the compiler build's parallel jobs (default 2).

With `--exe /path/to/RGFX.EXE` (and Wasmtime), the script also builds a package from every kit through the website's real worker under Bun (`tools/browser_build.mjs`) and through the CLI, as a local end-to-end check.

The SDL, GL and notice files that have no source in the repository, and the Mac notices (`LICENSES.txt`), are carried forward from the latest release's kits, checked against that release's `SHA256SUMS`.

## Versioned releases

Run **Bump version and tag** on `main` to increment all four Rust packages together. It uses the Releaser App to publish a GitHub-signed commit and a `vX.Y.Z` tag. The workflow requires `RELEASER_APP_ID` and `RELEASER_APP_PRIVATE_KEY`, distributed through the shared workflows repository. It checks the published signature and contributor trailers before creating the tag. If tagging fails after the commit succeeds, create the matching tag on that commit to resume; rerunning the bump would increment the version again.

A stable `vX.Y.Z` tag starts the release workflow. Tags must point to a commit on main and match every package version. Source checks, the three runtime kits, Windows and Linux patcher tests/builds and the browser build must all succeed before publication; the kits job caches the WebAssembly compilers between releases. Manual dispatch defaults to a dry run: it builds and tests release artifacts on main without publishing. To retry publication, dispatch on the existing tag with `dry_run=false`.

Each release contains:

- `maiko-patcher-linux-x86_64.tar.gz` and `maiko-patcher-windows-x86_64.tar.gz`: standalone CLI, licence and usage instructions. Wasmtime is installed separately. The Linux CLI is statically linked with musl; the runtime kit has its own desktop and glibc requirements above.
- `maiko-browser.tar.gz`: the complete static builder, including its checked support kits.
- `maiko-runtime-kit-linux.zip`, `maiko-runtime-kit-windows.zip` and `maiko-runtime-kit-macos.zip`: the kits this release built and the browser serves, with their upstream notices and corresponding source files. The Linux and Windows kits also work with the CLI's `--kit` option.
- `SHA256SUMS`, `LICENSE` and `README.md`.

The workflow stages all assets in a draft and publishes only after uploads succeed. Retrying preserves an already published release. After publication, Pages deploys the browser from that release, so one run of **Bump version and tag** builds, releases and deploys everything.

Workflows carry Orcastrate headers referencing `michidk/workflows` project templates. Update those templates alongside changes here so future synchronization preserves the change. Renovate extends the same shared preset used by Cyrus and redguard-preservation.
