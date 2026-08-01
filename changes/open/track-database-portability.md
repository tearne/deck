# Track Database Portability

**Mode:** Formal

## Intent

*(Under consideration — parked until wanted.)*

The [[track-data-storage]] change gave the track database (BPM, offset, cue, gain, keyed by audio hash) a well-defined XDG home in `~/.local/share/deck/`. That's tidy, but it sits outside the music collection — so migrating to another computer risks leaving the analysis and user edits behind while the music travels.

Consider options for keeping the track database with the music, so it survives a move between machines.

One idea: extend the notion of **workspace** — the workspace root would hold a copy of the track database, kept in sync with the canonical copy in `~/.local`. Other alternatives are open too (e.g. the database lives in the workspace and `~/.local` becomes a cache; sidecars; export/import).

Depends on [[track-data-storage]] (shipped). Relates to the [[rekey-track-data-to-content-identity]] question — portable per-track data is more useful once its key is the shareable content identity rather than the decoded-PCM hash.
