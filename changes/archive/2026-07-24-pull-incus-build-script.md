# Pull Incus Build Script

**Mode:** Formal

## Intent

Provide a `dev-build-run.py` script (POS style, per `ADDITIONAL/POS.md`) that takes one argument — the name of an incus container — and:

1. Auto-discovers the project's location inside that container, based on the git project name.
2. Pulls the container's `target/release` build into the current working directory.
3. Runs the pulled build with a new `deck` flag that points config resolution at the CWD instead of `~/.config/deck`, so it starts fresh without ever touching the tester's real config.
4. Runs the pulled application.

This replaces the ad hoc `dev-build-run.sh` (which hardcodes the `dev` container and project path) with a reusable script that works against any named incus container — taking its name as well as its role.

## Approach

### Project name is a constant in the script, not derived from a local git checkout

`PROJECT_NAME = "deck"` is hardcoded rather than read via `git rev-parse --show-toplevel`: the script is deck-specific tooling shipped inside this repo, and the whole point is to fetch a build without needing a local checkout at all — a `git rev-parse` at runtime fails outright when run from an arbitrary directory (e.g. `~/Downloads`), which is a real usage pattern this script exists to support.

### Container-path discovery: try `~/<project_name>` first, fall back to `find`

Matches the common case `dev-build-run.sh` already hardcodes (`~/deck`) without a filesystem-wide search on every run, but still works if a container keeps the checkout somewhere else via a bounded `find` fallback.

### New `deck --local-config` flag, rather than a script-side workaround

Both the adjacent-config bypass and a backup-and-restore dance were considered and dropped: the former skips the app's real first-run behavior (auto-create-with-notice), the latter adds fragile cleanup logic (temp files, `atexit`, crash recovery) to the script for something the app itself can express directly. Instead, `deck` gains a `--local-config` flag: when passed, config resolution (`resolve_config()`, `src/config/mod.rs:197-224`) uses `std::env::current_dir()` as the config directory instead of `~/.config/deck`, going through the exact same exists-or-auto-create logic (including the creation notice) just rooted elsewhere. This is independent of the existing adjacent-to-binary check (which stays as-is) — `--local-config` is an explicit, discoverable override rather than relying on the coincidence that the script happens to pull the binary into the CWD. The tester's real config is never touched, and the app takes the same code path a genuine first run does.

`main.rs` parses `--local-config` out of `std::env::args()` before treating the remaining first argument as the track/directory path (`args.get(1)` today), and threads the resulting bool through `tui_loop()` into `load_config()`.

### Replaces `dev-build-run.sh` — same name, new implementation

The shell script is deleted; `dev-build-run.py` takes over its name and its behavior (including forwarding extra CLI args to the launched app via `sys.argv`, matching the shell script's `"$@"` — e.g. a track/directory path per the README's `deck [path]` usage), generalised to accept any container instead of a hardcoded one.

## Plan

- [x] Add a `--local-config` flag to `deck`: parse it out in `main.rs` ahead of the positional path argument, thread it through `tui_loop()` into `load_config()`/`resolve_config()` in `src/config/mod.rs` so config resolution roots at the CWD instead of `~/.config/deck`
- [x] Write `dev-build-run.py` (POS style): derive the project name from the local git checkout, discover its path in the named container, pull `target/release/<project_name>`, and launch it with `--local-config` plus any passthrough args
- [x] Delete `dev-build-run.sh`
- [x] Replace the manual `--local-config` arg scan with `clap` (derive), gaining proper `-h`/`--help` and `--version` for free

## Log

- Version: 0.10.2 → 0.11.0 (minor, new `--local-config` functionality) → 0.11.1 (patch, clap re-test).
- `dev-build-run.py`: argument parsing and `local_project_name()` verified directly. Incus-dependent steps (container discovery, build, pull, launch) untested — no `incus` in this sandbox.
- Argument parsing switched from an ad hoc `--local-config` scan to `clap` (derive), adding `-h`/`--help` and `-V`/`--version`.
- Bug: `local_project_name()` crashed (`git rev-parse --show-toplevel` exits 128) when run outside a git checkout — a real case, since the script is meant to work from anywhere. Replaced with a hardcoded `PROJECT_NAME = "deck"` constant. Version 1.0.0 → 1.0.1.
- Bug: `cargo build --release` in the container failed with `cargo: command not found` — misdiagnosed first as a PATH-sourcing issue (fixed by sourcing `~/.cargo/env`, version 1.0.1 → 1.0.2) but that didn't fix it either.
- Real cause: `incus exec` runs as root by default, while the discovered checkout belongs to a different user (`ubuntu`) who has `cargo` installed. Fixed by running the build via `su - <user> -c '...'`, where `<user>` is derived from the discovered path (`home_dir_owner()`). Version 1.0.2 → 1.0.3.

## Conclusion

`deck` ships `--local-config` at `v0.11.1` (clap-based CLI parsing, replacing the manual arg scan). `dev-build-run.py` reached `v1.0.3` after three real-world fixes surfaced by testing against an actual incus container: no local git checkout required (hardcoded `PROJECT_NAME`), and running the remote build as the checkout's owning user rather than root (the `~/.cargo/env` sourcing fix was a misdiagnosis en route to that). `dev-build-run.sh` is deleted. End-to-end tested successfully against a real container.
