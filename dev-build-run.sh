#!/usr/bin/env bash
set -euo pipefail
incus exec dev -- bash -lc "cd ~/deck && cargo build --release"
rm -f ~/.config/deck/config.toml
incus file pull dev/root/deck/target/release/deck ./deck
./deck "$@"
