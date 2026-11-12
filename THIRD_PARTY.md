# Third-party components

The project-specific source uses the MIT license in LICENSE. Upstream components retain their own licenses and notices. The build fetches these components into ignored .cache directories.

| Component | Source / revision | License / use |
|---|---|---|
| M-HT/SR | https://github.com/M-HT/SR/tree/ac690ddf3010bc3d2c875cb1b5e8da4f53a8d5b3 | MIT; translator, runtime support, heap and audio |
| OpenGLide-ng | https://github.com/eugenenuke/openglide-ng/tree/9f8a2cb73af3b6d26e9ec67bb9e1873d3e211726 | LGPL, as stated in upstream source; separately linked renderer |
| SDL2 2.30.11 | https://github.com/libsdl-org/SDL/tree/release-2.30.11 | zlib; input, windowing, timing and audio |
| SDL2_mixer 2.8.1 | https://github.com/libsdl-org/SDL_mixer/tree/release-2.8.1 | zlib; audio mixing; optional codecs retain their licenses |
| MinGW-w64 / GCC runtime | Debian toolchain packages | Package-specific licenses and GCC runtime exceptions |

Patches apply to the pinned upstream sources and retain their notices. Runtime packaging gathers the SR, compiler/runtime and SDL notices and provides renderer source and build files separately. Preserve the applicable notices when distributing built libraries. The license here does not grant rights to Redguard code or assets, including generated translations of the game. No game data is included in Git.
