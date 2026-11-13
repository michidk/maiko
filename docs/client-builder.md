# Build a package in your browser or terminal

The static GitHub Pages site accepts your owned `RGFX.EXE`, detects Windows, Linux or macOS with a manual override. Each selection builds and downloads a runtime ZIP. Mac packages contain both Intel and Apple Silicon executables in one universal binary. A worker does the work entirely on your device. The page fetches static runtime kits; it never uploads your executable, generated assembly or package.

Both interfaces use the same Rust library for input validation and package creation. The browser uses an isolated worker; the CLI uses a private temporary directory.

The pinned Windows and Linux kits predate controller support and automatic installation discovery on main. For those downloaded snapshots, follow their included README and use `RUN.cmd` on Windows or `./RUN.sh /path/to/Redguard` on Linux. The source builds described below include the newer launch behavior.

For macOS, select `RGFX.EXE`, build the ZIP and extract it. Open
`redguard-native` and select your owned installation when asked. The Mac runtime
has no command launcher. See the [Mac launch guide](../platforms/macos/LAUNCH.md)
for quarantine removal, saves and logs. The port remains experimental.

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

Download the matching runtime kit from the same release (or the pinned kit release linked in `web/kits.json`), and run:

```sh
maiko-patcher build --input /path/to/RGFX.EXE --target linux \
  --kit linux.zip --output maiko-linux-x64.zip
```

The local Wasmtime CLI supports Linux and Windows. Use `--target windows` with `windows.zip` for Windows x64. For macOS, use the browser or the [local source build](../platforms/macos/README.md). `--wasmtime PATH` selects a local Wasmtime installation. The CLI uses a private temporary directory, reads the original game without modifying it, and refuses to overwrite the output ZIP.

Extract the generated ZIP into a separate writable folder. Windows users double-click `redguard-native.exe`; Linux users run `./redguard-native` in a terminal. The program looks for the Redguard folder itself (remembered folder, its own folder and the folders around it, standard GOG folders) and asks only if it finds nothing. The Linux package includes SDL2, SDL2_mixer, GLU and OpenGL dispatch libraries that the program finds beside itself. It needs a glibc 2.36 or newer desktop with X11/XWayland and a working OpenGL driver. Saves and settings go to the package's `user-data` directory. Startup options such as `--skip-intro` are command-line options (a shortcut's Target on Windows); see the README.

## Runtime kits

The project release provides support libraries, build tools and licence/source files. It contains no original game executable, game assets or runnable game package. A recognized executable from an owned installation is required to create a package locally.

The builder checks the kit checksum, validates its input and verifies the resulting executable against the accepted runtime snapshot before offering a download. Changed or unrecognized inputs fail validation.

The Mac kit contains SR/LLasm, the browser D compiler, LLVM/LLD, runtime objects and SDL frameworks. It contains no translated engine. System-library stubs contain the accepted native build's library identities and imported names, plus the libSystem unwind ABI required by LLVM 19's Rust personality code; the kit contains no Apple SDK. The browser links fresh engine objects from the selected executable and preserves framework links and executable permissions in the ZIP. The ZIP also carries the corresponding renderer source and build files.

The Linux kit includes SDL2 2.30.11, SDL2_mixer 2.8.1, GLU and OpenGL dispatch libraries. The mixer supports WAVE playback. Corresponding notices and source/build files accompany the libraries. Keep those notices with any redistributed support libraries. Game files and generated playable packages must stay out of release uploads.

## Build and deploy the site

Use Bun, Rust's `wasm32-unknown-unknown` target and `wasm-bindgen-cli` 0.2.129:

```sh
bun install --frozen-lockfile
bun test web
gh release download client-runtime-20261113 --repo michidk/maiko \
  --pattern linux.zip --pattern windows.zip --pattern macos.zip --dir .cache/site-kits
MAIKO_KIT_DIRECTORY=.cache/site-kits bun run build
```

`web/kits.json` pins Maiko release URLs and SHA-256 hashes. The site build checks the downloads and serves them from the same origin. The workflow downloads the public, pinned kits and verifies their hashes before building. For a local build, `MAIKO_KIT_DIRECTORY` can point to a directory with the same checked `linux.zip`, `windows.zip` and `macos.zip`. The content security policy restricts browser requests to the page's origin.

`.github/workflows/pages.yml` calls the reusable source-check and browser-build workflows and deploys on main or manual dispatch. Set **Settings → Pages → Source → GitHub Actions** before deploying. The managed GitHub App needs Pages write permission to enable that setting through the API.


## Rebuild the Mac compiler kit

On a Linux build host, install Bun, Rust, Git, curl, patch, tar, gzip, CMake,
Ninja and a native C++ compiler. Keep at least 60 GB free and run one build at a
time. The first compiler build is substantial; its source and object caches stay
under `.cache/client-macos-toolchain`.

```sh
cargo run --locked --release -p maiko-patcher --bin maiko-kit -- tools-macos
```

This downloads checksum-pinned LLVM 19.1.0, WASI SDK 29 and browser-d sources,
applies the documented patches and builds SR/LLasm, `opt`, `llc` and Mach-O LLD
for WebAssembly. LLVM includes X86 and AArch64 backends. The D compiler and
runtime notices accompany the tools. No game input is read by this command.

Run the [Mac backend check](../platforms/macos/README.md) with an owned
translation to obtain the source/runtime objects and build report. Add complete
runtime notices as described there. Verify a universal executable produced by
the browser toolchain, then export its kit:

```sh
cargo run --locked --release -p maiko-patcher --bin maiko-kit -- export-macos \
  artifacts/<timestamp>-macos-backend \
  artifacts/<timestamp>-macos-backend/runtime \
  /path/to/accepted-universal-executable REVISION macos.zip
```

Export checks the native build's source and artifact hashes, selects only the
fixed runtime object inventory and derives system-library stubs from the native
executables. It never copies either translated engine into the kit. The
accepted universal executable supplies only the expected output hash. Confirm a
fresh build through the real browser worker reproduces that executable before
publishing the kit and updating `web/kits.json`. A changed executable fails the
package gate. Keep generated playable packages private.

## Versioned releases

Run **Bump version and tag** on `main` to increment all four Rust packages together. It uses the Releaser App to publish a GitHub-signed commit and a `vX.Y.Z` tag. The workflow requires `RELEASER_APP_ID` and `RELEASER_APP_PRIVATE_KEY`, distributed through the shared workflows repository. It checks the published signature and contributor trailers before creating the tag. If tagging fails after the commit succeeds, create the matching tag on that commit to resume; rerunning the bump would increment the version again.

A stable `vX.Y.Z` tag starts the release workflow. Tags must point to a commit on main and match every package version. Source checks, Windows and Linux patcher tests/builds, and the browser build must all succeed before publication. Manual dispatch defaults to a dry run: it builds and tests release artifacts on main without publishing. To retry publication, dispatch on the existing tag with `dry_run=false`.

Each release contains:

- `maiko-patcher-linux-x86_64.tar.gz` and `maiko-patcher-windows-x86_64.tar.gz`: standalone CLI, licence and usage instructions. Wasmtime is installed separately. The Linux CLI is statically linked with musl; the runtime kit has its own desktop and glibc requirements above.
- `maiko-browser.tar.gz`: the complete static builder, including its checked support kits.
- The runtime kits embedded in the browser build include their upstream notices and corresponding source files. The Linux and Windows kits also work with the CLI's `--kit` option.
- `SHA256SUMS`, `LICENSE` and `README.md`.

The workflow stages all assets in a draft and publishes only after uploads succeed. Retrying preserves an already published release. After publication, Pages deploys the browser artifact from that release run. Main also continues to deploy development builds. Published kits remain pinned snapshots until `web/kits.json` is deliberately updated; a version bump does not regenerate them.

Workflows carry Orcastrate headers referencing `michidk/workflows` project templates. Update those templates alongside changes here so future synchronization preserves the change. Renovate extends the same shared preset used by Cyrus and redguard-preservation.
