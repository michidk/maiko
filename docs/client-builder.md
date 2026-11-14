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
bun run build
```

Runtime kits are assets of the latest release. The site build downloads `maiko-runtime-kit-{linux,windows,macos}.zip` from `releases/latest/download`, checks each against that release's `SHA256SUMS`, and serves them from the same origin. Nothing is pinned in the repository. For an offline build, set `MAIKO_KIT_DIRECTORY` to a folder with those three ZIPs and a `SHA256SUMS`; `MAIKO_KIT_URL` selects another release URL. The content security policy restricts browser requests to the page's origin.

`.github/workflows/pages.yml` calls the reusable source-check and browser-build workflows and deploys on main or manual dispatch. Set **Settings → Pages → Source → GitHub Actions** before deploying. The managed GitHub App needs Pages write permission to enable that setting through the API.


## Build the runtime kits

One command builds the Linux, Windows and macOS kits from an owned `RGFX.EXE`, with the same arguments and outputs for every system:

```sh
python3 -m tools.build_kits --exe /path/to/RGFX.EXE --output kits
```

Inputs: the accepted original executable (the SHA-256 is checked), a new `--output` folder and `wasmtime` on `PATH` (or `--wasmtime`). The build host needs Docker, Bun, Rust with `rustup`, `wasm-bindgen-cli` 0.2.129, Git, curl, patch, tar, CMake, Ninja and a native C++ compiler. Keep at least 60 GB free, run one build at a time and expect a long run: the Mac backend check alone takes about 10 minutes and 9 GB of memory. The first run also builds the Windows/Linux tools and, if `.cache/client-macos-toolchain` is missing, the Mac compilers (`maiko-kit tools-macos`: checksum-pinned LLVM 19.1.0, WASI SDK 29 and browser-d sources, SR/LLasm, `opt`, `llc` and Mach-O LLD for WebAssembly), which is the slowest step.

Outputs in `--output`:

- `maiko-runtime-kit-linux.zip`, `maiko-runtime-kit-windows.zip` and `maiko-runtime-kit-macos.zip`, named like the release assets.

Each kit is proven by building a package from it. Linux and Windows run the real `maiko-patcher build` on the executable. macOS runs the website's real `web/worker.js` under Bun (`tools/browser_build.mjs`) because the Mac kit pins the hash of a universal executable that only the browser toolchain produces. The script therefore exports a bootstrap Mac kit with a placeholder hash, lets the worker build the universal executable (it ends with "Compiled executable differs from the accepted build" by design), exports the final kit with that executable's hash and runs the worker again. It requires the identical executable and a passing package gate. Generated game code and executables stay in work folders that are removed after success, and a failed build keeps them for diagnosis; delete them yourself afterwards.

The SDL, GL and notice files that have no source in the repository, and the Mac notices (`LICENSES.txt`), are carried forward from the latest release's kits, which the script checks against that release's `SHA256SUMS`.

Kits are published only as assets of a versioned `vX.Y.Z` release. Every release carries the latest kits forward, so a release without kit changes needs nothing. After a kit changes, replace the three assets on the latest release and refresh their lines in `SHA256SUMS`, then run **Deploy browser builder** so the site picks them up:

```sh
cd kits
gh release download vX.Y.Z --repo michidk/maiko --pattern SHA256SUMS
grep -v ' maiko-runtime-kit-' SHA256SUMS > sums && sha256sum maiko-runtime-kit-*.zip >> sums && mv sums SHA256SUMS
gh release upload vX.Y.Z --repo michidk/maiko --clobber maiko-runtime-kit-*.zip SHA256SUMS
```

Keep generated playable packages private.

## Versioned releases

Run **Bump version and tag** on `main` to increment all four Rust packages together. It uses the Releaser App to publish a GitHub-signed commit and a `vX.Y.Z` tag. The workflow requires `RELEASER_APP_ID` and `RELEASER_APP_PRIVATE_KEY`, distributed through the shared workflows repository. It checks the published signature and contributor trailers before creating the tag. If tagging fails after the commit succeeds, create the matching tag on that commit to resume; rerunning the bump would increment the version again.

A stable `vX.Y.Z` tag starts the release workflow. Tags must point to a commit on main and match every package version. Source checks, Windows and Linux patcher tests/builds, and the browser build must all succeed before publication. Manual dispatch defaults to a dry run: it builds and tests release artifacts on main without publishing. To retry publication, dispatch on the existing tag with `dry_run=false`.

Each release contains:

- `maiko-patcher-linux-x86_64.tar.gz` and `maiko-patcher-windows-x86_64.tar.gz`: standalone CLI, licence and usage instructions. Wasmtime is installed separately. The Linux CLI is statically linked with musl; the runtime kit has its own desktop and glibc requirements above.
- `maiko-browser.tar.gz`: the complete static builder, including its checked support kits.
- `maiko-runtime-kit-linux.zip`, `maiko-runtime-kit-windows.zip` and `maiko-runtime-kit-macos.zip`: the kits embedded in the browser build, with their upstream notices and corresponding source files. The Linux and Windows kits also work with the CLI's `--kit` option.
- `SHA256SUMS`, `LICENSE` and `README.md`.

The workflow stages all assets in a draft and publishes only after uploads succeed. Retrying preserves an already published release. After publication, Pages deploys the browser artifact from that release run. Main also continues to deploy development builds. Published kits remain pinned snapshots until `web/kits.json` is deliberately updated; a version bump does not regenerate them.

Workflows carry Orcastrate headers referencing `michidk/workflows` project templates. Update those templates alongside changes here so future synchronization preserves the change. Renovate extends the same shared preset used by Cyrus and redguard-preservation.
