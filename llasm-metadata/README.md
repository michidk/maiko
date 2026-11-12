# Original-game translation analysis

These files support an incomplete LLasm translation investigation. They are not a runnable game target. The native NASM instruction replacements have not yet been ported wholesale.

`instruction_flags.sci` records inspected producer/consumer flag contracts. Its texture-size dispatch entries preserve carry and zero across shared CMP branches. Other entries preserve zero, signed comparison flags or CMPSB carry. They do not suppress all unknown-flag errors. The translator must still stop at unresolved regions.

`instruction_replacements.sci` contains the five overlapping Watcom TEST expansions, twelve fatal system-boundary placeholders and two returns replacing the unused DOS keyboard interrupt epilogues, as in the established native host. Replacement addresses and lengths have checked original-byte hashes in `instruction_checks.json`. The overlap expansions still need independent execution tests. `rg_translation_trap` is deliberately unresolved; no runnable host implementation is supplied by this analysis target.

From the repository root, with the independently built patched OUT_LLASM translator:

```sh
python3 -m tools.translate_llasm \
  --install /path/to/Redguard \
  --sr .cache/sr-llasm/SR/SR.exe
```

This regenerates the metadata and mixed code/data LE header overrides from the owned, hash-checked original image. It records the translator and metadata hashes and runs SR under a timeout. Output stays under ignored `artifacts/`. Exit2 means the recorded translation remains blocked. Emission would still require LLasm compilation, a CPU-state host adapter and runtime checks before any game support claim.
