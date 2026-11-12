# Agent notes for Maiko

Maiko is a static recompilation of *The Elder Scrolls Adventures: Redguard*. It translates the original DOS engine to native Windows and Linux code and runs it on a Rust host. Read [README.md](README.md) first, then [docs/development/RUST_PLAN.md](docs/development/RUST_PLAN.md), [PLATFORM.md](docs/development/PLATFORM.md) and [VALIDATION.md](docs/development/VALIDATION.md).

## Branches and publishing

- **All work lands directly on `main`.** Commit validated, completed work to `main` and push it. Do not open pull requests and do not keep long-lived feature branches. This is a standing instruction from the owner for this repository and replaces the usual "feature branch, no direct push to `main`" rule.
- **macOS is the one exception.** Mac work stays on `feat/macos-backend` and its draft PR. Do not merge it into `main`, do not rebase it onto `main` and do not touch it unless the owner asks. Mac development is on hold.
- Pull `main` before starting and again before every commit. Reconcile with changes from other sessions; never overwrite peer work and never force-push.
- Publish through the GitHub API as the managed App so GitHub signs the commit: GraphQL `createCommitOnBranch` on `main` with `expectedHeadOid`. Put `Co-authored-by: Michael Lohr <michael@lohr.dev>` and the agent's own `Co-authored-by` line in `message.body`, then read the commit back and confirm a valid signature and both trailers.
- Use Conventional Commits (`type(scope): description`). Do not commit game files, CD images, generated translations, build output or `.cache/` and `artifacts/`.

## Rust first

New first-party code is Rust. Do not add production C, C++ or Python. The one-time inventory of what is still not Rust, and the order in which it should move, is in [RUST_PLAN.md](docs/development/RUST_PLAN.md). Vendored upstream code (SR, OpenGLide-ng, SDL) and the generated translated engine assembly stay as they are. `python3 -m tools.audit_runtime --check` rejects owned production C/C++ in the runtime directories.

## Validation rules

- State only what was observed. A Wine or Linux run is not real-Windows acceptance, and dummy-audio or software-GL runs do not establish hardware playback or rendering.
- Record exact hashes, commands and limits in `VALIDATION.md` and a row in `docs/research/decisions.tsv`. Keep failures; do not edit them away.
- Never rewrite a test or gate so a failing build passes, unless the owner has said the requirement itself is wrong.
- Original game bytes are hash-checked at every patch site. Patches live in `native-metadata/` (`instruction_replacements.sci` and `instruction_checks.json` must stay in step). Guard a null or invalid read before the first x87 push so the floating-point stack stays balanced.

## Test environment

Docker images and `/tmp` do not survive a server restart; home-directory tools such as `~/.cargo` can disappear too (`rustup default stable` restores Rust).

- Build and test: `tools.pipeline translate --native`, then `tools.build_linux` or `tools.build_native`, then `tools.package_native` and `tools.verify_native_package`. The README lists the flags.
- Probes run in a container from the saved image: `docker load -i .cache/startup-timing/playtest-image.tar.gz`, then `docker run -d --name maiko-timing --network none --memory 3g -e SDL_AUDIODRIVER=dummy -v <run dir>:/work -v tests:/tests:ro -v <install>:/game:ro maiko-playtest:bookworm sleep infinity`, and `docker exec ... xvfb-run -a python3 /tests/<probe>`.
- Run one game or build process at a time. Start long jobs with `setsid nohup ... & disown` and wait on the process id; waiting on the launching shell returns immediately.
- Console input via `xdotool` needs 150 ms per key and 1.5 s after F12, or digits are dropped.
- The owner's install is the GOG copy under `~/workspaces/cyrus/magic/gog/extracted/app/Redguard`; the owner's own `RGFX.EXE` is variant 1 and the GOG one is variant 0. Both must work.
