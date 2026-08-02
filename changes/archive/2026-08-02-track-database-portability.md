# Track Database Portability

**Mode:** Formal

## Intent

*(Under consideration — parked until wanted.)*

The [[track-data-storage]] change gave the track database (BPM, offset, cue, gain, keyed by audio hash) a well-defined XDG home in `~/.local/share/deck/`. That's tidy, but it sits outside the music collection — so migrating to another computer risks leaving the analysis and user edits behind while the music travels.

Consider options for keeping the track database with the music, so it survives a move between machines.

One idea: extend the notion of **workspace** — the workspace root would hold a copy of the track database, kept in sync with the canonical copy in `~/.local`. Other alternatives are open too (e.g. the database lives in the workspace and `~/.local` becomes a cache; sidecars; export/import).

Depends on [[track-data-storage]] (shipped). Relates to the [[rekey-track-data-to-content-identity]] question — portable per-track data is more useful once its key is the shareable content identity rather than the decoded-PCM hash.


## Approach

### Local working database with a workspace mirror

`~/.local/share/deck/track-data.json` stays the runtime database — every read and write, as now. When a workspace is set, the database is additionally mirrored to a copy inside it, so the analysis and edits travel with the music.

### Sync on attach; the workspace copy wins

Whenever a workspace becomes active — at startup if one is already configured, and when the operator sets or changes it — the two copies are reconciled and both written at once: identities absent locally are added, and on any identity present in both, the workspace entry overwrites the local one (the carried library is authoritative, per the operator's choice). Local-only entries are kept and pushed out to the workspace copy immediately, so attaching a library synchronises both directions there and then rather than at the next save.

### Mirror on every save

While a workspace is set, each persist (idle flush and quit) writes the workspace copy — `<workspace>/track-data.json`, a visible file in the library root using the same filename as the canonical `.local` copy so moving one to the other is a plain copy — alongside `.local`. The copy is the whole database, not scoped to the library; keys are content identities, so extra entries for out-of-workspace tracks are harmless. Because import runs before any save, a library edited on another machine is adopted first, then mirrored back out — keeping A→B→A round-trips consistent.

### A detached library is simply no workspace

If the configured workspace path no longer exists (the library isn't attached), it resolves to no workspace — so nothing mirrors or imports until the library returns. Edits made while detached land in `.local` only.


## Plan

- [x] Give the track database a workspace-mirror path and an import-merge where workspace entries win; `save` also writes the mirror when one is set
- [x] On startup, when a workspace is already configured, point the mirror at it and import its copy
- [x] When the workspace is set or cleared in the browser, update the mirror and import on set


## Log

- `set_mirror` guards against the workspace resolving to the canonical data dir (mirror == `.local` path), so that degenerate case never double-writes or self-imports.
- Attach is a two-way `sync_with_mirror`: it adopts the workspace copy's entries (they win) and then writes both copies immediately, so at startup and on set the workspace copy is reconciled at once — local-only entries are pushed out even when the incoming copy is empty. This is what keeps A→B→A round-trips consistent.
- Added a `cache` unit test covering the merge (workspace-wins, local-only kept, workspace-only added) and the dual-write.


## Conclusion

Completed as planned, minor bump to v0.12.2. One correction mid-build: the attach step was first written as import-only (deferring the mirror write to the next save), which didn't push local-only data out on startup; it became a two-way `sync_with_mirror` that writes both copies immediately.

Documentation impact — map catch-up: the **Track Database** node could gain a line that, when a workspace is set, the database mirrors to `<workspace>/track-data.json` and reconciles (workspace-wins) on attach, so it travels with the music.
