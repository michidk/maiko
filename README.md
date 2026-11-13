# Maiko

Maiko runs *The Elder Scrolls Adventures: Redguard* (1998) natively on Windows and Linux, without DOSBox. It provides native graphics, input, sound, file and timing services for your own installation.

Maiko contains no game files. **You need your own copy of Redguard.** The GOG release is the supported one.

## What you need

- Windows 10 or newer, 64-bit, with a graphics driver that supports OpenGL.
- A Redguard installation that contains `RGFX.EXE`, the Glide version of the game executable. The GOG release includes it. Maiko recognizes two builds of this file and refuses any other, so it never runs code it was not checked against.
- A runtime ZIP built from your own executable using the [client-side browser builder](https://michidk.github.io/maiko/) or [Rust CLI](docs/client-builder.md). Your executable stays on your device. [Source checks](docs/building.md) are available for contributors.

## Play on Windows

1. Extract the ZIP into **its own folder**, anywhere you can write to. Do not extract it into the game folder. Keep the extracted files together:
   `redguard-native.exe`, `SDL2.dll`, `SDL2_mixer.dll`, `redguard-glide.dll`, `libwinpthread-1.dll`, `RUN.cmd`, and the licence notices.
2. Open `RUN.cmd` in a text editor and set `GAME_DIR` to the folder that contains `RGFX.EXE`, for example:

   ```bat
   set "GAME_DIR=C:\GOG Games\Redguard\Redguard"
   ```

   Leave it empty and a folder chooser opens each time you start the game. You can also run `RUN.cmd "C:\path\to\Redguard"`.
3. Double-click `RUN.cmd`. The original book menu opens in a window. Choose **New Game** to see the intro and start on the ship, or **Continue** to load your last save.

Nothing is copied into or changed in your game folder. The controls are the original ones from the game's `KEYS.INI`, and the original options menu works.

### Fullscreen and window size

By default the game opens in a window at its own size. In `RUN.cmd`, set `DISPLAY_ARGS` to `--fullscreen` for a borderless fullscreen window at your desktop resolution, or to `--window-size 1600x1200` (any width and height from 320x240 to 8192x8192) for a window of that size. The game still renders at its own resolution and is scaled to the window, and the mouse follows the scaling. The same options work when you start `redguard-native.exe` yourself.

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

### Startup options

Set `OPTIONS` in `RUN.cmd`, or pass the options after `--game-dir` when you start `redguard-native.exe` yourself:

```bat
set "OPTIONS=--skip-intro --skip-outro --load-save 1"
```

On Linux, add them after the game folder: `sh RUN.sh /path/to/Redguard --skip-intro --load-save 1`.

| Option | Effect |
|---|---|
| `--skip-intro` | New Game starts on the ship without the intro movie. |
| `--skip-outro` | Quitting from the book skips the book-closing animation. |
| `--load-save N` | Loads save slot `N` (the three-digit `SAVEGAME\SAVEGAME.NNN` folder in your saves, for example `SAVEGAME.001` for slot 1) instead of showing the book at startup. Startup stops with a message in the log if that save does not exist. |

Nothing is written to your game files. If your `RGFX.EXE` is the second recognized build, the outro is always skipped because that file already contains that change.

### Overlay

Press the backtick key (`` ` ``, left of `1` on a US keyboard) to open or close a small overlay with the current world number, Cyrus's position and which skip options are active. While it is open the game receives no keyboard, mouse or controller input.

### Movies

On a GOG installation the intro and other movies play automatically from the game's `game.gog` file, which Maiko finds inside the game folder or the folder above it. If your copy keeps movies as loose `.SMK` files, put them in the game folder or in a `MOVIES`, `ANIMS` or `VIDEO` folder inside it. A damaged or ambiguous `game.gog` stops startup with a message in the log.

### Saves and settings

Saves and settings are kept in a `user-data` folder next to `RUN.cmd`, separate from the original game. The first time you run, any existing saves and settings from your game folder are copied there once; after that the two stay independent. To keep them somewhere else, change `USER_DIR` in `RUN.cmd`. To start fresh, delete `user-data`.

### If something goes wrong

`RUN.cmd` writes everything the game printed to `native-test.log` in the same folder and shows it when the game exits. Typical causes:

| Message or symptom | What to do |
|---|---|
| The log says `Game image validation or installation selection failed` | `GAME_DIR` does not point at a folder containing a recognized `RGFX.EXE`. Check the folder; if it is right, your `RGFX.EXE` is not one of the two builds Maiko recognizes, so report its SHA-256 and where you got it. |
| A black window or no window | Check that your graphics driver supports OpenGL, and include `native-test.log` in a report. |
| The log says `Native audio device failed` | The game could not open a sound device. Check your default Windows playback device and include the log if it persists. |
| A message beginning `BLOCKED` | The game reached an instruction Maiko cannot run. This is a bug: report it with `native-test.log` and what you were doing. |

### Reporting problems

Open an issue and attach `native-test.log`. Say what you were doing, which Redguard release you own (GOG, CD, other) and your Windows version. A short description of where in the game it happened, such as "opening ship" or "tavern, after talking to the barman", makes bugs much easier to find.

## Linux

The [client-side builder](docs/client-builder.md) creates Linux x86-64 packages. They include SDL2, SDL2_mixer, GLU and OpenGL dispatch libraries and require a glibc 2.36 or newer desktop with X11/XWayland and a working OpenGL driver.

## Known limitations

The experimental Mac test has separate [launch instructions](macos/LAUNCH.md),
including how to handle macOS's download quarantine warning. It has not yet been
verified on a Mac and is outside the supported Windows/Linux packages.

- **CD music needs the GOG disc image.** The music is read from the audio tracks of `game.gog`, using the `game.ins` cue sheet beside it (the GOG release has both next to the game folder). Without them the game runs without music.
- **Only recognized `RGFX.EXE` builds run.** Other builds and patched executables are not supported.
- **macOS is not supported.**

## Legal

Maiko is released under the [MIT licence](LICENSE). It is an independent fan project and is not affiliated with or endorsed by Bethesda Softworks or ZeniMax. *The Elder Scrolls* and *Redguard* belong to their owners. This repository contains no game code or assets, and generated packages contain content derived from your own installation and must not be redistributed. You must own Redguard to use Maiko. Third-party components and their licences are listed in [THIRD_PARTY.md](THIRD_PARTY.md).

## For developers

- [docs/building.md](docs/building.md): source checks.
- [docs/architecture.md](docs/architecture.md): runtime structure.
- [AGENTS.md](AGENTS.md): repository workflow and rules for contributors and coding agents.
