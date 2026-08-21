# Session Restore

**Mode:** Formal

## Intent

On startup, pressing some key could reload the last known tracks — the decks as they were when the player last quit — instead of starting empty and re-browsing for each deck.

2026-08-21: fresh motivation — accidentally exiting loses all session data, which is irritating enough that a general exit confirmation was considered and rejected as more friction. Automatic session save (with restore on startup) is the better shape; the design should treat protection against accidental exit as a primary goal, not just startup convenience.

## Approach

### The session snapshot rides in Session State

Each deck's snapshot — track path, position, attached playlist and index, speed setting, and session-only mixer state (level, pitch, filter, PFL) — joins the existing session file alongside the browser directory and workspace, with the selected deck. Same dirty-on-mutation, idle-flush, flush-on-quit discipline: the snapshot is always current, so an accidental exit, a crash, or a pulled plug all lose nothing. No new store, no new save path.

### Position is written on a timer, not per frame

Playhead position changes continuously; marking the session dirty every frame would flush every second forever. The snapshot takes position on a coarse cadence (every 10 s and at every pause, seek, load, and quit) — a restore lands within seconds of where the set was, which is what "as they were" needs.

### Restore is asked for, never automatic

On startup with a snapshot present, the empty-deck panels offer it in one line (`Alt+r: restore last session`) and a dedicated action (`session_restore`, default `Alt+r`) performs it; the offer stands while every deck is empty and withdraws once any track is loaded. Starting a fresh set must not mean waiting through three decodes of yesterday's tracks, and the offer costs nothing to ignore. Restored decks come back **paused** at their saved positions with mixer state applied — playback never starts on its own.

### Missing tracks are skipped, not fatal

A deck whose file has gone is reported in the message stream and left empty; the others restore. The snapshot is replaced by the new state as soon as the restored decks begin saving.

### Command-line path still wins deck 1

Today's "a path on the command line loads onto deck 1" keeps its meaning; it simply counts as a loaded track, so the restore offer withdraws. Restore and a startup path are alternatives, not a merge.

## Plan

- [x] Session file gains a `decks` snapshot: per slot the track path, position (seconds), playlist path and index, speed setting (BPM or playback speed per mode), level, pitch, filter, PFL; plus the selected deck
- [x] Snapshot refreshed on load, unload, pause, seek, playlist step, any mixer or speed change, and quit; position additionally every 10 s while playing
- [x] `session_restore` action, default `Alt+r`, in config, keymap table, and help overlay
- [x] Restore: while all decks are empty, start a load per saved slot, carrying the saved playlist attachment and a deferred state (position, speed, mixer) applied when the deck finishes building; decks come up paused; selected deck restored
- [x] Missing or unreadable files: message per slot, deck left empty, others proceed
- [x] ~~Empty-deck panels carry the offer~~ → the offer joins the startup "No track loaded" hint in the message bar
- [x] Hand back: quit with decks loaded (playing and paused), restart, `Alt+r`; check positions, speeds, mixer, playlist badge, a deleted file, and a command-line path start

## Log

- Transport (position, speed) is applied when the load-time grid result arrives, not at build: the BPM and the per-track mode land there, and the Beat-mode cue seek would otherwise overwrite the restored position. Mixer state is applied at build.
- Snapshot recording starts only once the restore offer is withdrawn (any deck loaded or loading); otherwise the first empty frame would wipe the saved decks before `Alt+r` could use them.
- A slot mid-load keeps its previous snapshot rather than reading as empty.
- Quit writes positions unconditionally (three quit paths), bypassing the 10 s cadence.
- Playlist re-attachment trusts the saved index; the file is re-read and resolved for the unplayable count, but the saved track is loaded directly rather than through the playlist's resolver.
- 0.30.0 for first hand-back.
- Hand-back: the per-panel offer replaced by a single clause on the existing startup hint ("…, Alt+r restores last session"), since three identical lines said nothing more than one. The ghost-playhead toggle is now remembered in the session file too. 0.30.1.
- Restore hint was hidden behind the "config created" notice, which fires on every dev-script launch. 0.30.2 special-cased it; 0.30.4 instead removed the pre-existing rule that suppressed the startup hint whenever a config notice fired — the bar's existing precedence (events outrank hints) sequences them. Esc now dismisses only what the bar shows, revealing a hint behind a message rather than dropping both.
- Startup hint dismissed on browser open and on restore. 0.30.3.
- 0.30.4 for hand-back.

## Conclusion

Shipped at 0.30.4 (minor bump confirmed). Two departures from the Plan: the restore offer is a clause on the startup hint rather than a line on each empty panel, and — in the course of making that visible — the old rule suppressing the startup hint behind a config notice was removed, so the bar's documented precedence now carries startup sequencing alone. The ghost-playhead toggle joined the session file. Map catch-up pending: Session State (deck snapshot: what is kept, the 10 s position cadence, `Alt+r` restore, offer withdrawn once anything loads; ghosts toggle), Keymap (`session_restore`), and a Hints note in Messages (startup hint leaves on browser open or restore). A follow-up Intent is filed: [Beat Mode Needs a Grid](beat-mode-needs-a-grid.md).
