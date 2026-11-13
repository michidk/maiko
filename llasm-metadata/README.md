# Original-game translation analysis

These files translate Redguard through SR's CPU-state LLasm backend. The Mac port remains experimental. Emitting LLasm does not establish that the resulting game runs correctly.

`instruction_flags.sci` records inspected producer/consumer flag contracts. Its texture-size dispatch entries preserve carry and zero across shared CMP branches. Other entries preserve zero, signed comparison flags or CMPSB carry. They do not suppress all unknown-flag errors. The translator must still stop at unresolved regions.

`instruction_replacements.sci` contains the five overlapping Watcom TEST expansions, all replacements from the native host and the startup book/clock changes. Each entry carries its own documentation and original-byte hash. `instruction_checks.json` independently inventories their addresses and lengths. `tests/check_patch_tables.py` checks the annotations, and translation checks the original bytes. Unsupported system instructions call the Rust host's fatal `rg_translation_trap`; they never fall back to an interpreter.

`tests/llasm_opcodes.py` and `tests/llasm_execution.c` execute the overlap expansion from this table at both entry points. Every AX value is checked for the defined TEST flags and preservation of unrelated flags and registers. Rust instruction helpers also have comparisons against x86 hardware for MMX operations, byte rotates, variable shifts and comparison flags.

From the repository root, with the independently built patched OUT_LLASM translator:

```sh
python3 -m tools.translate_llasm \
  --install /path/to/Redguard \
  --sr /path/to/patched/SR.exe
```

This regenerates the metadata and mixed code/data LE header overrides from the owned, hash-checked original image. It records the translator and metadata hashes and runs SR under a timeout. Output stays under ignored `artifacts/`. Exit2 means the recorded translation remains blocked. The emitted source includes the shared numeric host services and Glide wrappers. Rust enters guest code through an LLasm C wrapper, and each CPU state lives in the offset heap because floating helpers can return pointers into it.

Cross-link a successful translation using `python3 -m tools.check_macos_backend --translation artifacts/<timestamp>-llasm-analysis`. This also runs the synthetic instruction checks. Cross-linking does not establish execution on macOS; reports keep runtime and full-game verification separate from compilation.
