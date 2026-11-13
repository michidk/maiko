# Maiko

Maiko runs *The Elder Scrolls Adventures: Redguard* (1998) natively on Windows and Linux, without DOSBox. It translates the original DOS engine to native code and replaces the DOS services the engine relied on (graphics, input, sound, files, timers) with a native runtime. The game itself is still the original: the same code, levels, scripts, art and sound.

**Maiko is experimental.** The opening of the game, the island and many interiors run, but nobody has played the whole game through yet. See [Known limitations](#known-limitations) before relying on it.

Maiko contains no game files. **You need your own copy of Redguard.** The GOG release is the tested one.

## What you need

- Windows 10 or newer, 64-bit, with a graphics driver that supports OpenGL.
- A Redguard installation that contains `RGFX.EXE`, the Glide version of the game executable. The GOG release includes it. Maiko recognizes two builds of this file and refuses any other, so it never runs code it was not checked against.
- Maiko's runtime ZIP. It contains the translated game engine, so it is built from your own copy of Redguard and not downloaded: [docs/building.md](docs/building.md) lists the steps ("Build the Windows runtime from your own copy"). It needs Linux with Docker to build, and the result runs on Windows.

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

A Linux build exists and is used to test the game automatically, but there is no packaged Linux release yet. If you are comfortable building from source, see [docs/building.md](docs/building.md).

## Known limitations

- **Not a finished port.** Playing the opening ship, arriving on the island, entering the harbour buildings and the catacombs, and loading each of the game's 29 maps work in automated tests. A complete playthrough, quests and the end of the game are untested.
- **CD music is not implemented.** Sound effects, speech and movie audio work. Music that the original played from CD tracks is silent.
- **Windows testing is limited.** Most checking is automated on Linux and under Wine. An early Windows tester reported that one tap of Jump made Cyrus rise forever, that the game started too fast, that movies did not play and that distant sea fog showed artifacts. Fixes are in this version but have not been confirmed on a real Windows machine; please say if you still see any of them.
- **Only recognized `RGFX.EXE` builds run.** Other builds and patched executables are not supported.
- **macOS is not supported.**
- **Twelve code paths stop the game** with a `BLOCKED` message if reached: unsupported 16-bit and privileged instructions in the engine's system-level code. None was reached in testing.

## Legal

Maiko is released under the [MIT licence](LICENSE). It is an independent fan project and is not affiliated with or endorsed by Bethesda Softworks or ZeniMax. *The Elder Scrolls* and *Redguard* belong to their owners. This repository contains no game code or assets, and the runtime Maiko builds from your own copy contains translated game code and is for your own use. You must own Redguard to use Maiko. Third-party components and their licences are listed in [THIRD_PARTY.md](THIRD_PARTY.md).

## For developers

- [docs/building.md](docs/building.md): build, test and package.
- [docs/architecture.md](docs/architecture.md): how the translation and the runtime fit together.
- [AGENTS.md](AGENTS.md): repository workflow and rules for contributors and coding agents.

The format and engine documentation that this port relies on is maintained in [redguard-preservation](https://github.com/michidk/redguard-preservation).
