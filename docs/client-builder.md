# Build a package in your browser or terminal

The static GitHub Pages site accepts your owned `RGFX.EXE`, detects Windows or Linux with a manual override, and downloads a ZIP. A worker does the work entirely on your device. The page fetches static runtime kits; it never uploads your executable, generated assembly or package.

Both interfaces use the same Rust library for input validation and package creation. The browser uses an isolated worker; the CLI uses a private temporary directory.

The result is an experimental runtime snapshot, with the existing project's acceptance limits. Linux menu and original-function checks and Windows Wine checks do not establish full-game completion, hardware playback or real-Windows acceptance. The initial kits predate controller support on main.

## Use the local CLI

Install Rust and [Wasmtime](https://wasmtime.dev/), then build:

```sh
cargo build --locked --release -p maiko-patcher --bin maiko-patcher
```

Download the target's runtime kit from the release linked in `web/kits.json`, and run:

```sh
maiko-patcher build --input /path/to/RGFX.EXE --target linux \
  --kit linux.zip --output maiko-linux-x64.zip
```

Use `--target windows` with `windows.zip` for Windows x64. `--wasmtime PATH` selects a local Wasmtime installation. The CLI uses a private temporary directory, reads the original game without modifying it, and refuses to overwrite the output ZIP.

Extract the generated ZIP into a separate writable folder. Windows users open `RUN.cmd` and choose the original game folder. Linux users run `sh RUN.sh /path/to/Redguard`; the package includes SDL2, SDL2_mixer, GLU and OpenGL dispatch libraries. It needs a glibc 2.36 or newer desktop with X11/XWayland and a working OpenGL driver. Saves and settings go to the package's `user-data` directory.

## Runtime kits

The project release provides support libraries, build tools, launchers and licence/source files. It contains no original game executable, game assets or runnable game package. A recognized executable from an owned installation is required to create a package locally.

The builder checks the kit checksum, validates its input and verifies the resulting executable against the accepted runtime snapshot before offering a download. Changed or unrecognized inputs fail validation.

The Linux kit includes SDL2 2.30.11, SDL2_mixer 2.8.1, GLU and OpenGL dispatch libraries. The mixer supports WAVE playback. Corresponding notices and source/build files accompany the libraries. Keep those notices with any redistributed support libraries. Game files and generated playable packages must stay out of release uploads.

## Build and deploy the site

Use Bun, Rust's `wasm32-unknown-unknown` target and `wasm-bindgen-cli` 0.2.129:

```sh
bun install --frozen-lockfile
bun test web
gh release download client-runtime-20261113 --repo michidk/maiko \
  --pattern linux.zip --pattern windows.zip --dir .cache/site-kits
MAIKO_KIT_DIRECTORY=.cache/site-kits bun run build
```

`web/kits.json` pins Maiko release URLs and SHA-256 hashes. The site build checks the downloads and serves them from the same origin. The workflow downloads private release assets with its read-only Actions token before building; no credential reaches the static site or browser. For a local build, `MAIKO_KIT_DIRECTORY` can point to a directory with the same checked `linux.zip` and `windows.zip`. The content security policy restricts browser requests to the page's origin.

`.github/workflows/pages.yml` runs the source checks, builds Rust/WASM and the static page, uploads the Pages artifact, and deploys on main or manual dispatch. Set **Settings → Pages → Source → GitHub Actions** before deploying. The managed GitHub App needs Pages write permission to enable that setting through the API.
