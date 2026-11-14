# Maiko

Maiko runs *The Elder Scrolls Adventures: Redguard* (1998) natively on Windows and Linux, without DOSBox. It provides native graphics, input, sound, file and timing services for your own installation.

Maiko contains no game files. **You need your own copy of Redguard.** The GOG release is the supported one.

## Features

- **Native, no DOSBox.** The original game code runs as a native program with its own graphics, input, sound, file and timing services.
- **Windows, Linux and macOS.** 64-bit Windows and Linux; macOS 11 or newer. Intel and Apple Silicon Macs use one universal package.
- **Hardware rendering.** The Glide version of the game is drawn through OpenGL, in a window of any size or borderless fullscreen.
- **Original sound, music and movies.** CD music and the intro play from your GOG `game.gog` disc image.
- **Game controllers.** SDL2 gamepads work alongside keyboard and mouse.
- **Finds your game itself.** It looks in the usual GOG, Wine, Heroic and Lutris folders and only asks if it finds nothing. Your installation is never modified.
- **Separate saves and settings**, kept in a `user-data` folder next to the program, plus session logs for bug reports.
- **Startup shortcuts:** `--skip-intro`, `--skip-outro` and `--load-save N`.
- **Optional version 5 fixes.** Build with the bug fixes of the later build 5.0 executable from the 1998 demo; see [Version 5 fixes](#version-5-fixes).
- **Nothing leaves your device.** You build the program from your own `RGFX.EXE` in your browser or with a CLI. Maiko ships no game code or assets.

## Setup

You need:

- A Redguard installation containing `RGFX.EXE`, the Glide version of the game executable. The GOG release is the supported one and includes it. Maiko recognizes three builds of this file and refuses any other, so it never runs code it was not checked against: the GOG and CD build 1.0, a variant of it, and build 5.0 from the 1998 demo.
- 64-bit Windows 10 or newer, or a Linux desktop (glibc 2.36 or newer, X11 or XWayland), with a graphics driver that supports OpenGL.

Maiko is built from your own executable, which stays on your device. There are two ways to do it.

### Option 1: the browser (easiest)

1. Open the [Maiko builder](https://michidk.github.io/maiko/).
2. Select your `RGFX.EXE` (in the `Redguard` folder of your installation). The page detects your system; change it if needed.
3. Wait for the build. The download starts automatically.
4. Extract the ZIP into its **own** writable folder and continue with [Play on Windows](#play-on-windows) or [Linux](#linux).

Nothing is uploaded: the build runs in a worker in your browser. It takes a few minutes on a current desktop browser.

### Option 2: the command line

Use this for scripts, a machine without a graphical browser, or to check what you are running.

1. From the [latest release](https://github.com/michidk/maiko/releases/latest) download `maiko-patcher-windows-x86_64.tar.gz` or `maiko-patcher-linux-x86_64.tar.gz`, and the runtime kit for your system (`maiko-runtime-kit-windows.zip` or `maiko-runtime-kit-linux.zip`). `SHA256SUMS` lists the checksums.
2. Install [Wasmtime](https://wasmtime.dev/), which the patcher uses to run the build tools. Or build the patcher yourself with `cargo build --locked --release -p maiko-patcher --bin maiko-patcher`.
3. Build the package:

```sh
maiko-patcher build --input /path/to/Redguard/RGFX.EXE --target linux \
  --kit maiko-runtime-kit-linux.zip --output maiko-linux-x64.zip
```

Use `--target windows` with the Windows kit for a Windows package, and `--wasmtime PATH` to pick a specific Wasmtime. Add `--v5-fixes` for the [version 5 fixes](#version-5-fixes). The patcher reads your game without modifying it, works in a private temporary folder and refuses to overwrite an existing output file. The macOS package is only available from the browser builder.

Details, including how the kits are made, are in [docs/client-builder.md](docs/client-builder.md). [Source checks](docs/building.md) are available for contributors.

### Version 5 fixes

The 1998 demo ships a later build of `RGFX.EXE`: version 1.0, build 5.0, where the GOG and CD executable is build 1.0. Maiko can build your executable with that build's fixes. Tick **Version 5 fixes** in the browser builder or pass `--v5-fixes` to the patcher. They are off by default. With them:

- a dying player no longer grabs a rope or hook;
- a fall off a ledge starts clear of the edge instead of stuttering on it;
- object flats bind their texture when they are created and no longer stop the game with `Bad Texture Index`;
- with speech on, dialogue lines without a voice recording are shown as timed subtitles, even with subtitles turned off; build 1.0 never shows them;
- a missing dialogue line plays a stock line instead of stopping the game;
- inventory models use full-resolution textures;
- animation selection skips empty slots.

The demo's island lock, its `[RTX_PATCH]` table and its debug overlay are not included.

If you have the build 5.0 `RGFX.EXE` itself, Maiko accepts it directly. It runs with the full game's data, where the island lock stays off, and the option is not offered because the fixes are already in it. Maiko translates it through `build5-metadata/addresses.sci`, which gives the build 5.0 address of every build 1.0 address its patches use. The [Redguard Preservation demo page](https://michidk.github.io/redguard-preservation/demo.html#executable) describes every change; each patch is documented in `native-metadata/v5_fixes.sci`.

## Play on Windows

1. Extract the ZIP into **its own folder**, anywhere you can write to. Do not extract it into the game folder. Keep the extracted files together: `redguard-native.exe`, `SDL2.dll`, `SDL2_mixer.dll`, `redguard-glide.dll`, `libwinpthread-1.dll` and the licence notices.
2. Double-click `redguard-native.exe`. It finds the game on its own when it can: the folder it remembered from an earlier start, `REDGUARD_DIR` if you set it, its own folder and the folders around it, and the standard GOG folders such as `C:\GOG Games\Redguard`. Only if none of those holds the game does it ask you to choose the Redguard folder (the one with `RGFX.EXE`, or the GOG folder around it), and it remembers your choice.
3. The original book menu opens in a window. Choose **New Game** to see the intro and start on the ship, or **Continue** to load your last save.

Nothing is copied into or changed in your game folder. The controls are the original ones from the game's `KEYS.INI`, and the original options menu works.

### Fullscreen and window size

By default the game opens in a window at its own size. `--fullscreen` and `--window-size` change that; see [Command-line options](#command-line-options).

### Game controllers

SDL2-compatible gamepads work on Windows and Linux. Connect one before starting or plug it in while playing. The layout uses the button positions below, with Xbox names as a reference. Multiple connected controllers can provide input.

| Control | Original key | Action with the default game settings |
|---|---|---|
| Left stick or D-pad | Arrow keys | Move forward/backward and turn; navigate menus |
| A / bottom face button | Space | Jump |
| X / left face button | Left Ctrl | Attack or interact |
| B / right face button | Escape | Back or cancel |
| Y / top face button | S | Draw or sheathe sword |
| Left shoulder | Left Shift | Walk modifier |
| Right shoulder | Left Alt | Alt modifier |
| Start / Menu | Escape | Open the book menu or go back |
| Back / View | I | Inventory |
| Left-stick click | Enter | Confirm menu selection |

The stick acts as digital directions, with a 50% activation threshold and a 37.5% release threshold to avoid drift and repeated changes near the edge. Controller presses use the same keyboard path as physical keys, so changing those keys in the game's settings also changes their controller actions. Keyboard and mouse remain usable alongside the controller. Unplugging a controller releases its inputs; switching away from the window releases all game inputs, and held controller inputs resume when focus returns.

The right stick, triggers, vibration and controller-specific remapping are not implemented. Inventory and other screens that require a mouse still need one.

### Command-line options

```
redguard-native [--game-dir FOLDER | --choose-game-dir] [--user-dir FOLDER] [--cd-image PATH]
                [--fullscreen | --window-size WIDTHxHEIGHT]
                [--skip-intro] [--skip-outro] [--load-save N]
```

Every option is optional. With none, the program finds the game and starts the original book menu. An unknown option, or a missing or invalid value, makes the program print this usage line and exit with code 1. On Windows, to use options without a terminal, make a shortcut to `redguard-native.exe` and add them after the path in the shortcut's **Target**; from a terminal, add them to the command, for example:

```bat
redguard-native.exe --fullscreen --skip-intro --load-save 1
```

| Option | Effect |
|---|---|
| `--game-dir FOLDER` | Use `FOLDER`, the one containing `RGFX.EXE`, as the game folder. Nothing is searched, asked or remembered, and a folder that does not hold a recognized game is an error (exit code 1) instead of a question. |
| `--choose-game-dir` | Ask for the game folder (a folder dialog on Windows, a prompt in the terminal on Linux) instead of using the remembered one. Cannot be combined with `--game-dir`. |
| `--user-dir FOLDER` | Keep saves and settings in `FOLDER` instead of the `user-data` folder beside the program. |
| `--cd-image PATH` | Where the movies and CD music come from instead of the installed `game.gog` or the Redguard disc: a disc image (`game.gog`, an ISO or a raw 2352-byte `.bin`), its cue sheet (`.cue`, needed for the music; one `.bin` for the disc or one per track), or a folder holding the disc's files and perhaps a rip of it. |
| `--fullscreen` | Borderless fullscreen at your desktop resolution. |
| `--window-size WIDTHxHEIGHT` | A window of this size, from 320x240 to 8192x8192, for example `--window-size 1600x1200`. The game still renders at its own resolution and is scaled to the window, and the mouse follows the scaling. Without either window option the window has the game's own size. |
| `--skip-intro` | New Game starts on the ship without the intro movie. |
| `--skip-outro` | Quitting from the book skips the book-closing animation. |
| `--load-save N` | Load save slot `N` (0 to 999; the three-digit `SAVEGAME\SAVEGAME.NNN` folder in your saves, for example `SAVEGAME.001` for slot 1) instead of showing the book at startup. Startup stops with a message if that save does not exist. |

Nothing is written to your game files. If your `RGFX.EXE` is the second recognized build, the outro is always skipped because that file already contains that change.

The program also reads `REDGUARD_DIR`, the game folder; `--game-dir` takes precedence over it. Two variables help when reporting problems: `RG_FLASH_LOG=1` makes the program log every frame that is a single flat colour, and `RG_CAPTURE_DIR=FOLDER` makes it save a still of the game picture there from time to time. They are not needed to play.

### Overlay

Press the backtick key (`` ` ``, left of `1` on a US keyboard) to open or close a small overlay with the current world number, Cyrus's position and which skip options are active. While it is open the game receives no keyboard, mouse or controller input.

### Movies

On a GOG installation the intro and other movies play automatically from the game's `game.gog` file, which Maiko finds inside the game folder or the folder above it. If your copy keeps movies as loose `.SMK` files, put them in the game folder or in a `MOVIES`, `ANIMS` or `VIDEO` folder inside it. A damaged or ambiguous `game.gog` stops startup with a message in the log.

An installation from the original CD reads the movies from the Redguard disc, as the original did. Maiko looks for a folder holding the disc's `INTRO.SMK`: inside a Wine prefix, the drive letter the installer wrote into `SYSTEM.INI` (`animation_drive`); a copy of the disc in a folder inside the game folder (such as the `PlayCD` folder a DOSBox setup mounts as the CD drive); on Windows that drive letter; on Linux `/media/USER`, `/run/media/USER`, `/media` and `/mnt`; on macOS `/Volumes`. A cue sheet in that folder, for example a Redump rip with one `.bin` per track, also gives the music. Otherwise `--cd-image` takes an image, a cue sheet or a folder.

### Saves and settings

Saves and settings are kept in a `user-data` folder next to `redguard-native.exe`, separate from the original game. The first time you run, any existing saves and settings from your game folder are copied there once; after that the two stay independent. The game folder is remembered separately, in your user profile (`%APPDATA%\maiko\config\install.json` on Windows, `~/.config/maiko/install.json` on Linux). Delete that file or start with `--choose-game-dir` to pick another. To start fresh, delete `user-data`.

### If something goes wrong

If the game cannot start, it shows a message. To see everything it prints, start it from a Command Prompt in its folder and keep the output:

```bat
redguard-native.exe > maiko-log.txt 2>&1
```

Typical causes:

| Message or symptom | What to do |
|---|---|
| "The game folder could not be used" | The chosen folder does not contain a recognized `RGFX.EXE`. Choose the inner Redguard folder; if it is right, your `RGFX.EXE` is not one of the builds Maiko recognizes, so report its SHA-256 (the log prints it) and where you got it. |
| A black window or no window | Check that your graphics driver supports OpenGL, and include the log in a report. |
| The log says `Native audio device failed` | The game could not open a sound device. Check your default Windows playback device and include the log if it persists. |
| A message beginning `BLOCKED` | The game reached an instruction Maiko cannot run. This is a bug: report it with the log and what you were doing. |

### Reporting problems

Every launch writes a session log under `%LOCALAPPDATA%/Maiko/logs` on Windows,
`$XDG_STATE_HOME/maiko/logs` or `~/.local/state/maiko/logs` on Linux, and
`~/Library/Logs/Maiko/` on macOS. The newest seven logs are kept. Set
`MAIKO_LOG_DIR` to choose a different directory.

Open an issue and attach the newest session log. Say what you were doing, which Redguard release you own (GOG, CD, other) and your Windows version. A short description of where in the game it happened, such as "opening ship" or "tavern, after talking to the barman", makes bugs much easier to find.

## Linux

The [client-side builder](docs/client-builder.md) creates Linux x86-64 packages. They include SDL2, SDL2_mixer, GLU and OpenGL dispatch libraries and require a glibc 2.36 or newer desktop with X11/XWayland and a working OpenGL driver. Extract the package and run `./redguard-native` in a terminal; it looks for the game as on Windows (`~/GOG Games/Redguard`, a Wine prefix's `GOG Games/Redguard`, the default Heroic and Lutris folders and the other standard folders), asks in the terminal if it finds nothing and remembers the answer; or pass `--game-dir /path/to/Redguard`. The options above work the same way.

## macOS

The browser builder creates a universal Mac package (Intel and Apple Silicon) from your own executable. Extract it into its own folder and open `redguard-native`; it finds the game as on the other systems. Follow the [launch instructions](platforms/macos/LAUNCH.md) for macOS's quarantine warning, saves and logs.

## Known limitations

- **CD music needs a disc image with its cue sheet.** The music is read from the disc's audio tracks: the GOG `game.gog` with the `game.ins` beside it, or for the CD release a rip of the disc with its `.cue` (one `.bin` for the disc or one per track) beside the disc's files or given with `--cd-image`. A disc in a drive or an ISO holds no audio tracks Maiko can read, so the game then runs without music.
- **Only recognized `RGFX.EXE` builds run.** Other builds and patched executables are not supported.

## Legal

Maiko is released under the [MIT licence](LICENSE). It is an independent fan project and is not affiliated with or endorsed by Bethesda Softworks or ZeniMax. *The Elder Scrolls* and *Redguard* belong to their owners. This repository contains no game code or assets, and generated packages contain content derived from your own installation and must not be redistributed. You must own Redguard to use Maiko. Third-party components and their licences are listed in [THIRD_PARTY.md](THIRD_PARTY.md).

## Related projects

Maiko runs the original game. Two other projects work from the same game files:

- [Redguard Preservation](https://github.com/michidk/redguard-preservation): the reverse-engineered file format, engine and configuration documentation ([published docs](https://michidk.github.io/redguard-preservation/)) and the `rgpre` tool that reads and converts the game's files.
- [Cyrus](https://github.com/michidk/cyrus) ([site](https://michidk.github.io/cyrus/)): a rebuild of Redguard in Rust and Bevy, which is early in development, and a finished [asset viewer](https://michidk.github.io/cyrus/viewer/) that opens the game's models, worlds, textures, fonts, animations, scripts and audio.

## For developers

- [docs/building.md](docs/building.md): source checks.
- [docs/architecture.md](docs/architecture.md): runtime structure.
- [AGENTS.md](AGENTS.md): repository workflow and rules for contributors and coding agents.
