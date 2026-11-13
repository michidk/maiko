# Build a package in your browser or terminal

The static GitHub Pages site accepts your owned `RGFX.EXE`, detects Windows or Linux with a manual override, and downloads a ZIP. A worker does the work entirely on your device. The page fetches static runtime kits; it never uploads your executable, generated assembly or package.

Both interfaces use the same Rust library for input validation and package creation. The browser uses an isolated worker; the CLI uses a private temporary directory.

The result is an experimental runtime snapshot, with the existing project's acceptance limits. Linux menu and original-function checks and Windows Wine checks do not establish full-game completion, hardware playback or real-Windows acceptance. The initial kits predate controller support on main.

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

`web/kits.json` pins Maiko release URLs and SHA-256 hashes. The site build checks the downloads and serves them from the same origin. The workflow downloads the public, pinned kits and verifies their hashes before building. For a local build, `MAIKO_KIT_DIRECTORY` can point to a directory with the same checked `linux.zip` and `windows.zip`. The content security policy restricts browser requests to the page's origin.

`.github/workflows/pages.yml` calls the reusable source-check and browser-build workflows and deploys on main or manual dispatch. Set **Settings → Pages → Source → GitHub Actions** before deploying. The managed GitHub App needs Pages write permission to enable that setting through the API.


## Versioned releases

Run **Bump version and tag** on `main` to increment all four Rust packages together. It uses the Releaser App to publish a GitHub-signed commit and a `vX.Y.Z` tag. The workflow requires `RELEASER_APP_ID` and `RELEASER_APP_PRIVATE_KEY`, distributed through the shared workflows repository. It checks the published signature and contributor trailers before creating the tag. If tagging fails after the commit succeeds, create the matching tag on that commit to resume; rerunning the bump would increment the version again.

A stable `vX.Y.Z` tag starts the release workflow. Tags must point to a commit on main and match every package version. Source checks, Windows and Linux patcher tests/builds, and the browser build must all succeed before publication. Manual dispatch defaults to a dry run: it builds and tests release artifacts on main without publishing. To retry publication, dispatch on the existing tag with `dry_run=false`.

Each release contains:

- `maiko-patcher-linux-x86_64.tar.gz` and `maiko-patcher-windows-x86_64.tar.gz`: standalone CLI, licence and usage instructions. Wasmtime is installed separately. The Linux CLI is statically linked with musl; the runtime kit has its own desktop and glibc requirements above.
- `maiko-browser.tar.gz`: the complete static builder, including its checked support kits.
- `maiko-runtime-kit-linux.zip` and `maiko-runtime-kit-windows.zip`: the exact kits embedded in that browser build, including their upstream notices and corresponding source files. Pass these filenames to the CLI's `--kit` option.
- `SHA256SUMS`, `LICENSE` and `README.md`.

The workflow stages all assets in a draft and publishes only after uploads succeed. Retrying preserves an already published release. After publication, Pages deploys the browser artifact from that release run. Main also continues to deploy development builds. Published kits remain pinned snapshots until `web/kits.json` is deliberately updated; a version bump does not regenerate them.

Workflows carry Orcastrate headers referencing `michidk/workflows` project templates. Update those templates alongside changes here so future synchronization preserves the change. Renovate extends the same shared preset used by Cyrus and redguard-preservation.
