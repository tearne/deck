# Message Log File

**Mode:** Formal

*(Split from the original message-log change. Builds on [message-stream](../archive/2026-08-11-message-stream.md), landed at v0.16.1.)*

## Intent

Past messages should survive the session. Append every message from the message stream to a log file in the state dir, alongside the panic log and error reports, so issues can be diagnosed after the fact — e.g. hash corruptions, whose reports currently sit in `error_reports/` with no surrounding narrative. Error-report writes emit a message naming the report, making the log the chronological index into `error_reports/`.

## Approach

Settled during the original change's planning: human-readable lines (not JSONL), everything logged regardless of severity, a generous configurable retention.

### File and format

`messages.log` in the state dir, beside the panic log and `error_reports/`. One line per message: `YYYY-MM-DD HH:MM:SS <severity> <source>  <text>` — local time, matching the history overlay's clock; source as `deck1`/`playlist`/`tags`/`files`/`app`. Message text is kept single-line at the sink (newlines sanitised), so the file parses back line-per-message.

### Write discipline

Opened for append at startup; each emit writes and flushes its line. Messages number dozens per session, so a flush per line costs nothing and survives a crash — which is exactly when the log matters.

### Retention

Age-based, `retention_days` (default 90) under a new `[messages]` config section. At startup, lines older than the cutoff are dropped. One self-pruning file rather than rotated suffixes: still bounded, and a single file stays grep-able and readable.

### Previous sessions in the view

On startup, after pruning, the log file is parsed back into messages and seeds the in-memory log — the history view then scrolls seamlessly into previous sessions. Seeding never touches the global bar; only fresh emits display. Unparseable lines (from before any format change) are skipped, not fatal.

### Session delimiter

A startup message (`deck v… started`, Info/app) is emitted through the normal path — the first line of each session in both file and view, so current and previous sessions are visually separable.

### Error reports unchanged here

How faults are captured is being rethought now a log exists (see error-capture-rethink); this change leaves report writing exactly as it is.

### History header names the file

The overlay's header row gains the log file's path, dim, after the scroll hints.

## Plan

- [x] Log-line format on the message type — serialise to `YYYY-MM-DD HH:MM:SS <severity> <source>  <text>` and parse back; text sanitised single-line at the sink
- [x] Append-and-flush writer opened at startup, wired into every emit
- [x] Startup prune of lines older than the retention cutoff, atomic rewrite
- [x] `[messages] retention_days` config parsing, default 90
- [x] Seed the in-memory log from the pruned file at startup, skipping unparseable lines; seeded entries don't display on the bar
- [x] Startup `deck v… started` message
- [x] History header names the log file path

Added after first hand-back (stale key names in hints):

- [x] Startup hint names the key actually bound to `open_browser` instead of the stale "press z"
- [x] Empty deck rows drop the stale "Alt+D to open the file browser"
- [x] Ephemeral hints — bar-only display path (`show_hint`), no history or file entry; startup hint moves onto it

## Log

- The no-track startup hint used to fire when the bar was empty; the started message now always occupies the bar first, so the hint keys on the config notice's absence instead. Net behaviour for the operator is unchanged.

- The calendar helper `civil_from_days` is shared from error_reports; its inverse (`days_from_civil`) lives in messages for parsing timestamps back.

- Pruning rewrites the file from parsed messages only, so malformed lines are dropped from the file as well as skipped for seeding.

- Unit tests added: serialise/parse roundtrip across zone offsets, newline sanitising, malformed-line rejection, and prune behaviour (44 tests total).

- Both startup hints named keys that no longer do what they claimed ("press z", "Alt+D"). The startup hint now reverse-looks-up the key bound to `open_browser` (chord form preferred); empty deck rows just say "no track".

- Guidance is a third display kind alongside prompts and messages: hints show on the bar (Info-styled, below real messages in precedence, Esc-dismissable) but enter neither history nor file. The startup hint is the first user.

## Conclusion

Completed at v0.18.3; minor bump confirmed. Two hand-back rounds added scope beyond the plan: the stale key-name hints ("press z", "Alt+D") were fixed — the startup hint now reverse-looks-up the bound key — and ephemeral hints became a third display kind, shown but never recorded. The Log carries the details.

Map catch-up is now due across the whole message trilogy. The user wants the node(s) to explain the three kinds with examples: **prompts** (await a key, live beside their subject — BPM confirm, rename offer, quit/load confirm), **messages** (displayed on the global bar, remembered in history, written to `messages.log` — move outcomes, playlist warnings, identity alerts), **hints** (displayed only, never recorded — the no-track startup hint). Related open proposals: event-log, error-capture-rethink, deck-row-condense.
