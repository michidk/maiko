# Maiko

Maiko runs *The Elder Scrolls Adventures: Redguard* (1998) natively on Windows and Linux, without DOSBox. It provides native graphics, input, sound, file and timing services for your own installation.

Maiko contains no game files. **You need your own copy of Redguard.** The GOG release is the supported one.

## What you need

- Windows 10 or newer, 64-bit, with a graphics driver that supports OpenGL.
- A Redguard installation that contains `RGFX.EXE`, the Glide version of the game executable. The GOG release includes it. Maiko recognizes two builds of this file and refuses any other, so it never runs code it was not checked against.
- A runtime ZIP built from your own executable using the [client-side browser builder](https://michidk.github.io/maiko/) or [Rust CLI](docs/client-builder.md). Your executable stays on your device. [Source checks](docs/building.md) are available for contributors.

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
redguard-native [--game-dir FOLDER | --choose-game-dir] [--user-dir FOLDER] [--cd-image FILE]
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
| `--cd-image FILE` | Read the intro and other movies from this CD image (`game.gog`, an ISO or a raw 2352-byte image) instead of the installed `game.gog` or loose movie files. Rarely needed. |
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
| "The game folder could not be used" | The chosen folder does not contain a recognized `RGFX.EXE`. Choose the inner Redguard folder; if it is right, your `RGFX.EXE` is not one of the two builds Maiko recognizes, so report its SHA-256 (the log prints it) and where you got it. |
| A black window or no window | Check that your graphics driver supports OpenGL, and include the log in a report. |
| The log says `Native audio device failed` | The game could not open a sound device. Check your default Windows playback device and include the log if it persists. |
| A message beginning `BLOCKED` | The game reached an instruction Maiko cannot run. This is a bug: report it with the log and what you were doing. |

### Reporting problems

Open an issue and attach the log (see above). Say what you were doing, which Redguard release you own (GOG, CD, other) and your Windows version. A short description of where in the game it happened, such as "opening ship" or "tavern, after talking to the barman", makes bugs much easier to find.

## Linux

The [client-side builder](docs/client-builder.md) creates Linux x86-64 packages. They include SDL2, SDL2_mixer, GLU and OpenGL dispatch libraries and require a glibc 2.36 or newer desktop with X11/XWayland and a working OpenGL driver. Extract the package and run `./redguard-native` in a terminal; it looks for the game as on Windows (`~/GOG Games/Redguard`, a Wine prefix's `GOG Games/Redguard`, the default Heroic and Lutris folders and the other standard folders), asks in the terminal if it finds nothing and remembers the answer; or pass `--game-dir /path/to/Redguard`. The options above work the same way.

## Known limitations

- **CD music needs the GOG disc image.** The music is read from the audio tracks of `game.gog`, using the `game.ins` cue sheet beside it (the GOG release has both next to the game folder). Without them the game runs without music.
- **Only recognized `RGFX.EXE` builds run.** Other builds and patched executables are not supported.
- **macOS is not supported.**

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
