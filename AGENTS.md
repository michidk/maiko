# Agent notes for Maiko

Maiko is a static recompilation of *The Elder Scrolls Adventures: Redguard*. It translates the original DOS engine to native Windows and Linux code and runs it on a Rust host. Read [README.md](README.md), then [docs/architecture.md](docs/architecture.md) and [docs/building.md](docs/building.md).

## Branches and publishing

- **All work lands directly on `main`.** Commit validated, completed work to `main` and push it. Do not open pull requests and do not keep long-lived feature branches. This is a standing instruction from the owner for this repository and replaces the usual "feature branch, no direct push to `main`" rule.
- **macOS is the one exception.** Mac work stays on `feat/macos-backend` and its draft PR. Do not merge it into `main`, do not rebase it onto `main` and do not touch it unless the owner asks. Mac development is on hold.
- Pull `main` before starting and again before every commit. Reconcile with changes from other sessions; never overwrite peer work and never force-push.
- Publish through the GitHub API as the managed App so GitHub signs the commit: GraphQL `createCommitOnBranch` on `main` with `expectedHeadOid`. Put `Co-authored-by: Michael Lohr <michael@lohr.dev>` and the agent's own `Co-authored-by` line in `message.body`, then read the commit back and confirm a valid signature and both trailers.
- Use Conventional Commits (`type(scope): description`). Do not commit game files, CD images, generated translations, build output, saves or `.cache/` and `artifacts/`.
- There is no CI in this repository. Run the source checks in [docs/building.md](docs/building.md) before committing.

## What belongs in the repository

Only our own work: source, tests, build tooling, documentation for users and contributors, and the documented patches to pinned upstream sources. No game code or assets, no copies of upstream code beyond the patches in `patches/`, no research logs, plans or feasibility studies, and no machine-specific paths or private references. Record the evidence for a change in its commit message.

## Rust first

New first-party code is Rust. Do not add production C, C++ or Python. The existing build and test tooling in `tools/` and `tests/` is Python and the ABI and original-function tests are C and C++; these are to be ported to Rust over time, starting with the build, package and verify tool, then the probes. Vendored upstream code (SR, OpenGLide-ng, SDL) and the generated translated engine assembly stay as they are. `python3 -m tools.audit_runtime --check` rejects owned production C/C++ in the runtime directories.

## Validation rules

- State only what was observed. A Wine or Linux run is not real-Windows acceptance, and dummy-audio or software-GL runs do not establish hardware playback or rendering.
- Never rewrite a test or gate so a failing build passes, unless the owner has said the requirement itself is wrong.
- Every patch is documented next to the code that implements it, not in a separate document: byte replacements in `native-metadata/instruction_replacements.sci` and `metadata/instruction_replacements.sci` carry a comment block (Original, Replacement, Effect, Why, Evidence) and a `# sha256:` of the original bytes above each entry, the build rejects an entry without them, and `python3 tests/check_patch_tables.py` checks the tables without the game. The text hooks that edit the generated assembly are documented in `tools/pipeline.py` and each upstream patch carries a header at the top of its `.patch` file. A new or changed patch updates its own documentation in the same commit.
- Original game bytes are hash-checked at every patch site. Guard a null or invalid read before the first x87 push so the floating-point stack stays balanced.

## Test environment

Docker images and `/tmp` do not survive a server restart; home-directory tools such as `~/.cargo` can disappear too (`rustup default stable` restores Rust).

- Build and test commands are in [docs/building.md](docs/building.md). Docker bind mounts must resolve below the repository's parent folder or `MAIKO_MOUNT_ROOT`.
- Run one game or build process at a time. Start long jobs with `setsid nohup ... & disown` and wait on the process id; waiting on the launching shell returns immediately.
- The F12 console is useful for placing the player and loading worlds in probes. Type each key separately (about 90 ms down, 220 ms apart, and 1.5 s after F12); `xdotool type` drops characters.
- Door activation fires only while the on-screen prompt is visible, which means facing the door; the action key is shared with attacking, so anywhere else it draws the sword.
- The owner's `RGFX.EXE` is one recognized build and the GOG copy is the other. Both must work.
