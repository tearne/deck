# Pull Incus Build Script

**Mode:** TBD

## Intent

Provide a `pull_incus_build.py` script (POS style, per `ADDITIONAL/POS.md`) that takes one argument — the name of an incus container — and:

1. Auto-discovers the project's location inside that container, based on the git project name.
2. Pulls the container's `target/release` build into the current working directory.
3. Purges the local user config (`~/.config/deck/config.toml`) so the pulled build picks up fresh defaults rather than a stale config.
4. Runs the pulled application.

This generalises the ad hoc `dev-build-run.sh` (which hardcodes the `dev` container and project path) into a reusable script that works against any named incus container.
