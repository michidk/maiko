# Launching Maiko on macOS

The runtime requires macOS 11 or newer on Apple Silicon or Intel.
The package contains a universal `redguard-native` executable, its renderer and
SDL frameworks. It contains no Redguard installation, game assets or disc media.
You must supply your own copy of Redguard.

## First launch

1. Extract the whole runtime ZIP. Keep `redguard-native`,
   `libredguard-glide.dylib` and `Frameworks/` together.
2. If macOS blocks this trusted test build, dismiss the warning with **Done**
   and run this in Terminal, adjusting the extracted folder path:

   ```sh
   xattr -dr com.apple.quarantine "$HOME/Downloads/Maiko Mac Runtime"
   ```

   This removes quarantine from that package and its libraries. The build is
   not notarized. Use this command only for a package you trust.
3. Open `redguard-native` directly. No command launcher is needed. If no game
   installation is found beside it, a folder picker asks for the owned folder
   containing `RGFX.EXE`, `SYSTEM.INI` and `FXART`.

The executable runs natively on both Mac architectures. It checks `game/`
beside itself, then its own directory, then the remembered installation before
opening the picker. Only recognized original `RGFX.EXE` images are accepted.
Use `--choose-game-dir` to choose a different installation, or
`--game-dir /path/to/Redguard` to select one explicitly.

Saves and settings go to `~/Library/Application Support/Maiko/user-data/`,
separate from the installation. The selected game folder is remembered under
`~/Library/Application Support/Maiko/`. Original disc media is optional; Maiko
reads `game.gog` and `game.ins` from your installation or its parent when present.
Do not copy those files into a runtime download.

## Logs and troubleshooting

Every launch automatically writes a new log under `~/Library/Logs/Maiko/`.
The newest seven session logs are kept. Set `MAIKO_LOG_DIR` to override that
location. If that directory cannot be opened, Maiko tries `Maiko-logs/` in the
system temporary directory and reports the fallback. No shell redirection is needed. The log includes build revision,
platform, architecture, CPU backend, arguments, installation checks, runtime
messages, exit reason/status and fatal diagnostics. It includes local paths;
check those before sharing it.

For a problem report, attach the newest log and describe what happened just
before the window closed. A normal `event=exit` with status zero records a clean
exit; `BLOCKED` or `event=panic` indicates a fatal failure. A clean exit can still
be unintended, so include what you were doing. Pressing Escape/F10 or requesting
a window close is logged to help establish that sequence.

To try without the intro, run `./redguard-native --skip-intro` in the package
folder. If Terminal reports `permission denied`, use `chmod +x redguard-native`.
If a `dyld` error occurs before Maiko starts, no session log may be created;
include that loader error and ensure the complete framework directories and
symlinks were extracted.

## Verification

Both architectures cross-link. The same CPU-state backend has rendered the menu
and opening ship on Linux with software OpenGL and dummy audio. The owner has
also supplied Mac runtime output showing movie/audio activity and a clean exit
path. That output alone does not establish hardware quality or full-game
completion. The longer Linux movie probe still times out at its final Quit step.
