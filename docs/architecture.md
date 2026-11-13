# Runtime structure

Maiko provides native Windows and Linux services for an owned Redguard installation. The runtime reads game assets from that installation and stores saves and settings in a separate user directory.

## Rust workspace

| Crate | Path | Responsibility |
|---|---|---|
| `maiko-runtime` | `runtime/rust/` | Installation checks, files, storage, timing, input, audio, movies, memory, crash reports and the in-game overlay (egui, drawn before each buffer swap) |
| `maiko-glide` | `runtime/glide/` | Interface to the separately linked graphics renderer |
| `maiko-host` | `runtime/host/` | Startup and runtime service dispatch |
| `maiko-patcher` | `patcher/` | Local package builder shared by the CLI and browser |

SDL supplies windowing, input, timing and audio. OpenGLide-ng supplies the renderer. Platform bindings handle memory, folder selection and fatal error reporting. Dependency sources and licences are listed in [THIRD_PARTY.md](../THIRD_PARTY.md).

## Files and storage

The game installation is read-only. `cap-std` confines save and settings writes to the selected user directory. Installed movies can be read from a selected CD image, a GOG installation or supported loose movie files. Music is read from the audio tracks of the GOG disc image (`game.gog` and its `game.ins` cue sheet) by `runtime/rust/redbook.rs`, which implements the `AIL_redbook_*` calls the game uses and mixes the tracks into the audio output. Maiko does not distribute those files.

## Source layout

- `runtime/`: Rust services, interface headers and startup glue.
- `patcher/` and `web/`: local package creation and the static website.
- `tools/`, `linux/`, `glide/` and `audio/`: build and packaging support.
- `tests/`: independent fixtures, interface checks and runtime checks.
- `patches/`: documented changes to licensed upstream dependencies.

See [building.md](building.md) for source checks and [client-builder.md](client-builder.md) for package creation.
