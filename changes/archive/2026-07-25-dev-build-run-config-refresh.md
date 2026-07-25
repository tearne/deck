# Dev-Build-Run Config Refresh

**Mode:** Formal

## Intent

`dev-build-run.py` always launches the pulled binary with `--local-config`, and the app never refreshes an existing `config.toml`. When a build changes the default keybindings, the stale local file silently overlays old bindings onto the new seeds — rebound keys stop working while the hardcoded help overlay shows the new layout.

Delete the local config each time the script runs, so testing always starts from the defaults the build ships with.


## Approach

### Delete just before launch

The script removes `./config.toml` — the file `--local-config` resolves in the working directory — after a successful build and pull, immediately before launching. The freshly pulled binary recreates it from its embedded defaults (and prints its "config created" notice). Deleting on the launch path only means a failed build or pull leaves the existing config untouched.


## Plan

- [x] Remove `./config.toml` on the launch path in `dev-build-run.py`, after a successful pull.
- [x] Bump the script's `VERSION` patch (1.0.3 → 1.0.4).


## Conclusion

Completed.
