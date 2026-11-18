# Third-party components

Maiko's own source is MIT licensed (see LICENSE). Upstream components retain their own licenses and notices. The build fetches runtime and development components into ignored .cache directories. The website fonts are bundled under `web/fonts/` with their license notices.

| Component | Source / revision | License / use |
|---|---|---|
| M-HT/SR | https://github.com/M-HT/SR/tree/ac690ddf3010bc3d2c875cb1b5e8da4f53a8d5b3 | MIT; build tool and runtime support |
| browser-d / DMD | https://github.com/live-codes/browser-d/tree/6a9b3a8e83c6c79248a7f7957c07280009f250fe | MIT driver; DMD, druntime and Phobos BSL-1.0; browser LLasm compiler |
| LLVM / LLD 19.1.0 | https://github.com/llvm/llvm-project/tree/llvmorg-19.1.0 | Apache-2.0 WITH LLVM-exception; browser optimizer, code generators and Mach-O linker |
| WASI SDK 29 / wasi-libc | https://github.com/WebAssembly/wasi-sdk/tree/wasi-sdk-29 | Apache-2.0 WITH LLVM-exception, Apache-2.0, MIT and upstream notices; browser compiler support |
| LLVM WASI compatibility changes | https://github.com/breeze833/clang-wasi/tree/b284fc9fb092f7ffec71a17163b8c585a4763548 | Apache-2.0; documented LLVM-only patch in `patches/llvm-wasi.patch` |
| rustyglide 0.1.0 | https://github.com/michidk/rustyglide | MIT OR Apache-2.0; statically linked renderer |
| wgpu, naga 30.0.2 | https://github.com/gfx-rs/wgpu | MIT OR Apache-2.0; graphics backends (Vulkan, Direct3D 12, Metal, OpenGL) under rustyglide, statically linked |
| MinGW-w64 / GCC runtime | Debian toolchain packages | Package-specific licenses and GCC runtime exceptions |
| egui, egui_glow, glow 0.33.3 / 0.16.0 | https://github.com/emilk/egui/tree/0.33.3, https://github.com/grovesNL/glow | MIT/Apache-2.0; launcher interface. The bundled egui fonts carry their own licenses (Ubuntu Font Licence, OFL, MIT/Bitstream Vera) |
| egui, egui-wgpu 0.36.2 | https://github.com/emilk/egui/tree/0.36.2 | MIT/Apache-2.0; in-game overlay |
| Cinzel / Crimson Pro | https://github.com/google/fonts/tree/main/ofl/cinzel, https://github.com/google/fonts/tree/main/ofl/crimsonpro | SIL Open Font License 1.1; locally served website typography. Notices in `web/fonts/` |
| Rust standard library 1.95.0 | https://github.com/rust-lang/rust/tree/1.95.0/library | MIT/Apache-2.0 with dependency notices; statically linked host services |

Patches apply to the pinned upstream sources and retain their notices. Runtime packaging gathers the SR and compiler/runtime notices; the Windows package also gathers the notices of every Rust crate in the locked dependency graph (`tools/rust_licenses.py`), which covers rustyglide, wgpu, naga and egui-wgpu. Preserve the applicable notices when distributing built libraries. The license here does not grant rights to Redguard code or assets, including locally generated playable packages. No game data is included in Git.
