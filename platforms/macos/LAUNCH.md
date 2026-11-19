# Launching Maiko on macOS

The runtime requires macOS 11 or newer on Apple Silicon or Intel.
The package contains the universal `maiko` launcher and `redguard-native` game
executables. It contains no Redguard installation, game assets or disc media.
You must supply your own copy of Redguard.

## First launch

1. Extract the whole runtime ZIP. Keep `maiko`, `redguard-native` and
   `Frameworks/` together.
2. If macOS blocks the downloaded package, dismiss the warning with **Done**
   and run this in Terminal, adjusting the extracted folder path:

   ```sh
   xattr -dr com.apple.quarantine "$HOME/Downloads/Maiko Mac Runtime"
   ```

   This removes quarantine from that package and its libraries. The build is
   not notarized. Use this command only for a package you trust.
3. Open `maiko`, the launcher. If it finds no game installation, it asks for
   the owned folder containing `RGFX.EXE`, `SYSTEM.INI` and `FXART`. It then
   shows Redguard's original start window, drawn from the `REDGUARD.EXE` in that
   installation. **Game Options** sets fullscreen, the window size and skipping
   the intro; **Play Redguard** starts the game. You can also open
   `redguard-native` directly.

The executable runs natively on both Mac architectures. It uses the shared installation search and opens a folder picker when needed. Only recognized original `RGFX.EXE` images are accepted.
For direct executable options, see [Advanced use](#advanced-use).

The game keeps its saves and settings in the game folder, shared with the
GOG/DOSBox version. The selected game folder is remembered under
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

Use the launcher's **Game Options** to skip the intro.
If a `dyld` error occurs before Maiko starts, no session log may be created;
include that loader error and ensure the complete framework directories and
symlinks were extracted.

## Advanced use

From Terminal in the extracted package folder, run the game executable directly when you need command-line options:

```sh
./redguard-native --choose-game-dir
```

Use `--game-dir /path/to/Redguard` to select a folder explicitly, or `--skip-intro` to skip the intro. These arguments belong to `redguard-native`, not `maiko`, the launcher. If Terminal reports `permission denied` for the game executable, use `chmod +x redguard-native`.

See the [full command-line reference](../../docs/playing.md#command-line-options).

<a id="verification"></a>
## Developer reference

Backend build instructions and the scope of platform verification belong in the [macOS backend guide](README.md).
