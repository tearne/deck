# Error Capture Rethink

**Mode:** Formal

*(Spun out during message-log-file planning, superseding its error-report cross-reference question.)*

## Intent

Faults are currently preserved as dated files under `error_reports/` (identity-mismatch folders, identity-unhashable text files), with the panic log alongside. Now that a persistent message log exists, revisit the optimum way to capture errors: which faults still need standalone report files (preserved artefacts like original/edited file pairs plausibly do), which collapse into log lines, and how messages and reports should reference each other.

## Approach

### The dividing line: artefacts need files, text needs the log

A report earns its existence only by preserving something a log line cannot carry.

- **identity-mismatch** preserves file pairs — stays a report; its event already names the folder. Unchanged.
- **identity-unhashable** is path + error + boilerplate — collapses into its event ("Content identity unavailable (<error>) — not saved, unusable in playlists"), and the report kind is deleted. The error string travels from the load thread through the existing analysis channel.
- **panic.log** is written mid-crash when the stream can't help — stays.

Existing `identity-unhashable-*.txt` files on disk are left to age out by hand — no auto-deletion of user state.

### Crashes join the narrative

On startup, after seeding history: if the previous session's tail has a `started` line without a closing `deck quit`, emit a warning event (on the bar — a crash is worth one glance at launch) that the session ended abnormally, naming `panic.log` when one exists. No mtime heuristics — the quit bracket from event-log is the evidence.

### Map

Post-build catch-up: Error Reports drops to one kind; Track Database's detail pointer moves from report to log; Event Log gains the abnormal-end note. Beyond the line edits, the user wants a structural review: fault capture is currently described across four places (Error Reports, the Metadata Editor callout, Track Database detail, and now the Event Log) — assess whether the concept is too scattered and should be regrouped.

## Plan

- [x] Error detail through the analysis channel; the unhashable event carries it; the report kind and its writer deleted
- [x] Abnormal-end detection from the seeded history at startup; warning emitted, naming panic.log when present

## Log

- The analysis channel gained an optional fifth field for the identity error; the BPM-redetect channels share the tuple type and gained it as `None`.

- Detection scans only the most recent session's bracket, so an old crash followed by a clean session raises nothing. Unit-tested (45 tests total).

- The warning names panic.log purely on existence — a stale panic.log from an older crash would be named even if the recent abnormal end was, say, a kill. Accepted; the wording says "see", not "caused by".

## Conclusion

Completed at v0.22.1; minor bump confirmed. The map catch-up ran alongside, answering the scatter review: Error Reports became **Fault Capture**, telling the three-layer story (log, `error_reports/`, panic.log) in one node, with feature nodes keeping one-line pointers in. One structural tension noted and accepted: Fault Capture and Event Log are close cousins placed apart in the tree — revisit if it keeps chafing.
