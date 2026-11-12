# Native Glide backend

The prototype builds OpenGLide-ng revision `9f8a2cb73af3b6d26e9ec67bb9e1873d3e211726` as a Windows x64 DLL. It uses the Windows OpenGL backend. No DOS overlay or DOSBox forwarding port is involved.

```sh
python3 build_glide.py --install /path/to/Redguard
```

The script exports the pinned Git tree into a temporary directory, applies `patches/openglide-x64.patch`, builds in an offline 4 GiB / 2 CPU container, then runs standalone and guest-interface tests under Wine/Xvfb/Mesa. Output stays ignored in `.cache/glide/`. The current host needs Python, Git, tar, patch and Docker. The image supplies MinGW C/C++, Wine, Xvfb and Mesa.

The source patch fixes GCC's old `=A` timestamp-counter constraint for x64, where EAX and EDX must be captured separately. It also keeps default options when the wrapper cannot create its config in a read-only current directory, checks failed config reads and avoids recursion when an incompatible config cannot be removed. The generated SDK header describes Windows x64's actual type widths. The DLL needs the MinGW `libwinpthread-1.dll`, copied beside it. The build retains a separate DLL and import library.

`backend_smoke.cpp` uploads a red texture, draws a triangle, reads RGB565 pixels inside and outside it, saves a PPM and swaps buffers. `bridge_test.cpp` exercises the guest stack argument decoder with a green triangle. It checks the 20-byte guest texture and framebuffer structures, pointer-free vertex layout, low-address framebuffer staging and a sentinel after the returned guest structure. These checks run against the real renderer. The standalone triangle also runs with a read-only current directory; that regression reproduced the original integration crash before the wrapper fix.

`imports.py` reads the 124-entry table from the supported original executable and verifies every five-byte call stub and decorated stdcall argument length. It generates SR external mappings, assembly wrappers with callee stack cleanup, and API identifiers. It does not copy translated game code into source files.

`bridge.cpp` implements the 38 entry points identified by the existing Redguard Glide research. Unimplemented imports fail explicitly. Texture descriptors convert 32-bit data pointers into native pointers. Framebuffer locks use bounded low-address staging allocations because the backend's returned pointer can exceed 4 GiB. Hardware configuration and vertex layouts have explicit size checks. Runtime coverage of all 38 functions is not established by the triangle tests; framebuffer write modes beyond RGB565/555/1555 remain unsupported.

The private Cyrus archive provided the dispatcher and input references. The public API summary is [Redguard's Glide interface](../../../docs/engine/glide.md). The archive is research, not a complete game source tree. Its historical decompiled game functions are not imported into this project.

OpenGLide is LGPL-2.1. Preserve its source, notices and local patch when packaging a backend; the legacy SDK headers also carry 3dfx notices. MinGW's pthread runtime needs its notice. Packaging and redistribution review remain part of the menu ZIP task. No menu or native Windows acceptance follows from either smoke test.
