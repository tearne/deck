# Track Data Storage

**Mode:** Formal

## Intent

The app's persistent data all lands in `~/.config/deck/`, but most of it isn't config. `cache.json` holds per-track memory (BPM, offset, cue, gain, keyed by audio hash) and global session state (last browser directory, workspace, latency, vinyl mode, art brightness); `panic.log` is a diagnostic log. None of these belong under `~/.config`.

Store each kind where the XDG Base Directory standard says it belongs, so the data sits where users and tooling expect and `~/.config` holds only actual config:

- **Per-track memory** → `$XDG_DATA_HOME` (`~/.local/share/deck/`) — it carries irreplaceable user edits (cue points, gain trim), so it's data, not a regenerable cache.
- **Global session state** and **`panic.log`** → `$XDG_STATE_HOME` (`~/.local/state/deck/`), joining the identity-mismatch dumps the tag editor already writes there.
- **`config.toml`** stays in `~/.config/deck/`.

Existing files under `~/.config/deck/` are migrated to their new homes on startup so current users lose nothing.

(Earlier framing — moving per-track data out into the music library as sidecars — is dropped. The data stays in the app's own directories; this change only corrects which directory.)


## Approach

### Single XDG base-directory resolver

One module resolves the three base directories — config, data, state — each honouring its `XDG_*_HOME` override with the standard `~/.config` · `~/.local/share` · `~/.local/state` fallback, suffixed `/deck`. It replaces the resolution now duplicated across `cache_path`, config, and the existing `state_dir`, and fixes `panic_log_path`, which hardcodes `~/.config`. `config.toml` stays in config; `panic.log` moves to state, joining the identity-mismatch dumps already there.

### Split the cache into two owners

The single `Cache` becomes two independently-persisted values: the **track database** (hash-keyed per-track entries) at `~/.local/share/deck/track-data.json`, and **session state** (last browser path, workspace, latency, vinyl mode, art brightness) at `~/.local/state/deck/session.json`. Each keeps the existing dirty-flag / idle-flush / atomic-rename save, so the per-file crash-safety guarantee holds. Both load at startup.

### Migrate existing files on startup

If `~/.config/deck/cache.json` exists, its fields are distributed into the two new files and the old file deleted. The legacy flat-HashMap read path is kept to read it. Current users lose nothing.


## Plan

- [x] Add an XDG base-directory resolver for the config, data, and state homes, subsuming `state_dir` and the duplicated `home_dir` helpers
- [x] Route `config.toml` and `panic.log` through the resolver, moving `panic.log` to the state dir
- [x] Split the cache into a track database (`~/.local/share/deck/track-data.json`) and session state (`~/.local/state/deck/session.json`), updating all call sites
- [x] Migrate an existing `cache.json` on startup into the two files, then delete it


## Log

- Resolver lives in a new `xdg` module (`config_dir` / `data_dir` / `state_dir`).
- The split keeps the module name `cache` and the type `CacheEntry`; the single `Cache` became `TrackDatabase` + `SessionState`. Renaming module/type deferred as out of scope — flagged for map catch-up.
- `track-data.json` serialises as a plain `{hash: entry}` map, which is byte-identical to the oldest pre-wrapper cache format.
- Cache flowed through three function boundaries, not one: `build_deck` and `service_deck_frame` needed only the track database; the main `tui_loop` took `&mut Cache` and now takes both stores.


## Conclusion

Completed as planned. The code keeps the module name `cache` and the type `CacheEntry` while the concept split into `TrackDatabase` + `SessionState`; renaming was left out of scope.

Documentation impact: the map's **Cache** node describes one `~/.config/deck/cache.json` holding two kinds of content — now false. It needs catching up to the two-store split (track database in the data dir, session state in the state dir) and the panic.log relocation. Handled as a per-node map negotiation, not in this change.
