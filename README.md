# Maiko

Maiko runs *The Elder Scrolls Adventures: Redguard* (1998) natively on Windows, Linux and macOS, without DOSBox. You build it from your own copy of the game; Maiko contains no game files.

Join the [Discord](https://discord.com/invite/xjCCyqbxkp) and discuss Maiko in `#redguard-maiko-recomp`.

**Disclaimer:** AI was used heavily in this reverse engineering project; without AI it would not have been possible.

## Name

Maiko is named after [Maiko](https://en.uesp.net/wiki/Redguard:Maiko), the cartographer in *Redguard*. The name reflects how the project maps the original DOS executable to native code for modern systems.

## Features

- **Native, no DOSBox:** the original game code runs as a native 64-bit program.
- **Hardware rendering** by [rustyglide](https://github.com/michidk/rustyglide) through Vulkan, Direct3D 12, Metal or OpenGL, in a resizable window or fullscreen.
- **The original launcher**, drawn from your installation's `REDGUARD.EXE`, with options for fullscreen, window size, renderer, textures and skipping movies.
- **Original sound, CD music and movies** from your GOG `game.gog` or the Redguard disc.
- **Controller support:** Xbox, PlayStation and other gamepads on Windows, Linux and macOS, plugged in at any time and used alongside keyboard and mouse ([button layout](docs/playing.md#game-controllers)).
- **Finds your game itself** and shares its saves and settings with the GOG/DOSBox version: the game reads and writes them in its own folder.
- **Optional [version 5 fixes](docs/playing.md#version-5-fixes)** from the later build 5.0 executable.

## Setup

Watch the video guide first: it shows building, installing and playing.

[![Video guide: playing Redguard with Maiko on Windows, Mac and Linux](https://img.youtube.com/vi/EP6OcLl9wWo/hqdefault.jpg)](https://www.youtube.com/watch?v=EP6OcLl9wWo)

You need the **GOG release** of Redguard (or the original CD): its `RGFX.EXE` is the Glide executable Maiko is built from. The Steam release ships only the software-rendered `RG.EXE` and does not work. Maiko recognizes three builds of `RGFX.EXE` and refuses any other.

**In the browser (easiest):** open the [Maiko builder](https://michidk.github.io/maiko/), select your `RGFX.EXE` and wait a few minutes. The build runs in your browser; nothing is uploaded.

**On the command line:** download `maiko-patcher` and the runtime kit for your system from the [latest release](https://github.com/michidk/maiko/releases/latest), install [Wasmtime](https://wasmtime.dev/), then:

```sh
maiko-patcher build --input /path/to/Redguard/RGFX.EXE --target linux \
  --kit maiko-runtime-kit-linux.zip --output maiko-linux-x64.zip
```

Use `--target windows` with the Windows kit, and `--v5-fixes` for the version 5 fixes. Mac packages come from the browser builder only. See [docs/client-builder.md](docs/client-builder.md) for details.

## Play

Extract the ZIP into **its own writable folder** (not the game folder) and start the launcher: `maiko.exe` on Windows, `./maiko` on Linux and macOS. It finds your game in the usual GOG, Wine, Heroic and Lutris folders and only asks if it finds nothing. Click **Play Redguard**.

- **Windows:** 64-bit Windows 10 or newer.
- **Linux:** x86-64 with glibc 2.36 or newer, X11 or XWayland, and a Vulkan or OpenGL driver.
- **macOS:** 11 or newer, Intel or Apple Silicon. Follow the [launch instructions](platforms/macos/LAUNCH.md) for the quarantine warning.

The game can also be started directly as `redguard-native` with options such as `--fullscreen`, `--skip-intro` or `--load-save 1`. [docs/playing.md](docs/playing.md) lists every option, the controller layout, movies, saves and logs.

## Problems

If the screen stays black, choose another **Renderer** in the launcher's Game Options. When the game crashes, the launcher saves a crash report and offers to open an [issue](https://github.com/michidk/maiko/issues); attach it and say what you were doing. See [troubleshooting](docs/playing.md#if-something-goes-wrong) and [known limitations](docs/playing.md#known-limitations).

## Related projects

- [rustyglide](https://github.com/michidk/rustyglide): the 3dfx Glide API in Rust on wgpu, and Maiko's renderer. Use it as a Rust library or as a drop-in `glide2x.dll` / `libglide2x.so` for any game that needed a Voodoo card.
- [Redguard Preservation](https://github.com/michidk/redguard-preservation): file format, engine and configuration [documentation](https://michidk.github.io/redguard-preservation/) and the `rgpre` asset tool.
- [Cyrus](https://github.com/michidk/cyrus): Redguard rebuilt in Rust and Bevy, with an [asset viewer](https://michidk.github.io/cyrus/viewer/).

## For developers

[docs/building.md](docs/building.md) (source checks), [docs/architecture.md](docs/architecture.md) (runtime structure) and [AGENTS.md](AGENTS.md) (workflow and rules for contributors and coding agents).

## Legal

Maiko is released under the [MIT licence](LICENSE). It is an independent fan project, not affiliated with or endorsed by Bethesda Softworks or ZeniMax; *The Elder Scrolls* and *Redguard* belong to their owners. Generated packages contain content derived from your own installation and must not be redistributed. Third-party licences are listed in [THIRD_PARTY.md](THIRD_PARTY.md).
