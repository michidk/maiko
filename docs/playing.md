# Playing Maiko

The reference behind the [README](../README.md): every option, the controller layout, movies, saves, logs and the optional version 5 fixes.

## Command-line options

```
redguard-native [--game-dir FOLDER | --choose-game-dir] [--user-dir FOLDER] [--cd-image PATH]
                [--fullscreen | --window-size WIDTHxHEIGHT]
                [--skip-intro] [--skip-outro] [--load-save N]
                [--texture-filter original|smooth|sharp]
```

These are the options of the game itself, `redguard-native`, which the launcher starts with the settings from Game Options. You can also start it directly. Every option is optional. With none, the program finds the game and starts the original book menu. An unknown option, or a missing or invalid value, makes the program print this usage line and exit with code 1. On Windows, to use options without a terminal, make a shortcut to `redguard-native.exe` and add them after the path in the shortcut's **Target**; from a terminal, add them to the command, for example:

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
| `--texture-filter MODE` | How textures are filtered. `original` (the default) does what the game asks: bilinear filtering, the soft look of the 3dfx original, with point sampling where the game requests it. `smooth` filters every texture bilinearly; `sharp` point-samples every texture for crisp pixels. The launcher offers the same choice under **Textures**. |
| `--load-save N` | Load save slot `N` (0 to 999; the three-digit `SAVEGAME\SAVEGAME.NNN` folder in your saves, for example `SAVEGAME.001` for slot 1) instead of showing the book at startup. Startup stops with a message if that save does not exist. |

The renderer picks a graphics backend by itself. To choose one, set **Renderer** in the launcher's Game Options, or set the `WGPU_BACKEND` environment variable to `vulkan`, `dx12` (Windows), `metal` (macOS) or `gl` before starting `redguard-native`. The log names the backend and graphics driver in use (`Renderer: …`).

Nothing is written to your game files. If your `RGFX.EXE` is the second recognized build, the outro is always skipped because that file already contains that change.

The program also reads `REDGUARD_DIR`, the game folder; `--game-dir` takes precedence over it. Two variables help when reporting problems: `RG_FLASH_LOG=1` makes the program log every frame that is a single flat colour, and `RG_CAPTURE_DIR=FOLDER` makes it save a still of the game picture there from time to time. They are not needed to play.

## Game controllers

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

## Overlay

Press the backtick key (`` ` ``, left of `1` on a US keyboard) to open or close a small overlay with the current world number, Cyrus's position and which skip options are active. While it is open the game receives no keyboard, mouse or controller input.

## Movies

On a GOG installation the intro and other movies play automatically from the game's `game.gog` file, which Maiko finds inside the game folder or the folder above it. If your copy keeps movies as loose `.SMK` files, put them in the game folder or in a `MOVIES`, `ANIMS` or `VIDEO` folder inside it. A damaged or ambiguous `game.gog` stops startup with a message in the log.

An installation from the original CD reads the movies from the Redguard disc, as the original did. Maiko looks for a folder holding the disc's `INTRO.SMK`: inside a Wine prefix, the drive letter the installer wrote into `SYSTEM.INI` (`animation_drive`); a copy of the disc in a folder inside the game folder (such as the `PlayCD` folder a DOSBox setup mounts as the CD drive); on Windows that drive letter; on Linux `/media/USER`, `/run/media/USER`, `/media` and `/mnt`; on macOS `/Volumes`. A cue sheet in that folder, for example a Redump rip with one `.bin` per track, also gives the music. Otherwise `--cd-image` takes an image, a cue sheet or a folder.

## Saves and settings

Saves and settings are kept in a `user-data` folder next to `redguard-native.exe`, separate from the original game. The first time you run, any existing saves and settings from your game folder are copied there once; after that the two stay independent. The game folder is remembered separately, in your user profile (`%APPDATA%\maiko\config\install.json` on Windows, `~/.config/maiko/install.json` on Linux). Delete that file or start with `--choose-game-dir` to pick another. To start fresh, delete `user-data`.

## If something goes wrong

If the game cannot start, it shows a message. To see everything it prints, start it from a Command Prompt in its folder and keep the output:

```bat
redguard-native.exe > maiko-log.txt 2>&1
```

Typical causes:

| Message or symptom | What to do |
|---|---|
| "The game folder could not be used" | The chosen folder does not contain a recognized `RGFX.EXE`. Choose the inner Redguard folder; if it is right, your `RGFX.EXE` is not one of the builds Maiko recognizes, so report its SHA-256 (the log prints it) and where you got it. |
| A black window or no window | Choose another **Renderer** in the launcher's Game Options, check that your graphics driver supports Vulkan or OpenGL, and include the log in a report. |
| The log says `Native audio device failed` | The game could not open a sound device. Check your default Windows playback device and include the log if it persists. |
| A message beginning `BLOCKED` | The game reached an instruction Maiko cannot run. This is a bug: report it with the log and what you were doing. |

## Reporting problems

Every launch writes a session log under `%LOCALAPPDATA%/Maiko/logs` on Windows,
`$XDG_STATE_HOME/maiko/logs` or `~/.local/state/maiko/logs` on Linux, and
`~/Library/Logs/Maiko/` on macOS. The newest seven logs are kept. Set
`MAIKO_LOG_DIR` to choose a different directory. The log records scene and menu changes;
`MAIKO_TRACE=1` adds the detailed state samples the developer tests use.

When the game was started from the launcher and crashes, freezes and has to be closed, or fails
to start, the launcher saves `Maiko-crash-DATE_TIME.zip` (UTC) in the `crash-reports` folder
inside the log folder and offers to open an issue. It holds the run's log, the last screenshot, the system details (OS, CPU, graphics
driver, Maiko version, the names and sizes of the files in the game folder), the launcher
settings, your INI files and your newest save; never any game files. **Technical Support ›
Create report** makes the same ZIP at any time. A game that stops drawing for 15 seconds logs
`event=stall` with where it was, so a freeze leaves evidence too.

Open an issue and attach the report, or the newest session log. Say what you were doing, which Redguard release you own (GOG, CD, other) and your Windows version. A short description of where in the game it happened, such as "opening ship" or "tavern, after talking to the barman", makes bugs much easier to find.

## Known limitations

- **CD music needs a disc image with its cue sheet.** The music is read from the disc's audio tracks: the GOG `game.gog` with the `game.ins` beside it, or for the CD release a rip of the disc with its `.cue` (one `.bin` for the disc or one per track) beside the disc's files or given with `--cd-image`. A disc in a drive or an ISO holds no audio tracks Maiko can read, so the game then runs without music.
- **Only recognized `RGFX.EXE` builds run.** Other builds and patched executables are not supported.

## Version 5 fixes

The 1998 demo ships a later build of `RGFX.EXE`: version 1.0, build 5.0, where the GOG and CD executable is build 1.0. Maiko can build your executable with that build's fixes. Tick **Version 5 fixes** in the browser builder or pass `--v5-fixes` to the patcher. They are off by default. With them:

- a dying player no longer grabs a rope or hook;
- a fall off a ledge starts clear of the edge instead of stuttering on it;
- object flats bind their texture when they are created and no longer stop the game with `Bad Texture Index`;
- with speech on, dialogue lines without a voice recording are shown as timed subtitles, even with subtitles turned off; build 1.0 never shows them;
- a missing dialogue line plays a stock line instead of stopping the game;
- inventory models use full-resolution textures;
- animation selection skips empty slots.

The demo's island lock, its `[RTX_PATCH]` table and its debug overlay are not included.

If you have the build 5.0 `RGFX.EXE` itself, Maiko accepts it directly. It runs with the full game's data, where the island lock stays off, and the option is not offered because the fixes are already in it. Maiko translates it through `metadata/build5/addresses.sci`, which gives the build 5.0 address of every build 1.0 address its patches use. The [Redguard Preservation demo page](https://michidk.github.io/redguard-preservation/demo.html#executable) describes every change; each patch is documented in `metadata/native/v5_fixes.sci`.
