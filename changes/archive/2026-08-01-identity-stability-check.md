# Identity Stability Check

**Mode:** Formal

## Intent

A defensive check against byte-range extraction bugs — the fragile formats (MP3, AAC) especially. Whenever the application edits a track's tags/metadata, it computes the content-identity hash ([[content-identity-hashing]]) of the file immediately before and after the write. The hash must be invariant, because tag regions are excluded from identity by design; a difference means either our extraction wrongly included tag bytes, or the write corrupted the audio payload. Either is a critical fault.

This turns every real-world retag into a live conformance check on a real file, far wider coverage than the synthetic corpus, at negligible cost (hashing reads bytes without decoding).

On a mismatch, flag a critical error. Leaning **warn + loud log rather than undo**: a mismatch most likely indicates a bug in our own extraction, so undoing would destroy a legitimate edit to mask our defect, and undo needs a pre-edit copy anyway. The alarm worth surfacing to the operator is the data-integrity consequence — the track's identity in any playlist referencing it has just broken. Whether to also offer undo is an open question for the Approach.

Consumes the content-identity hasher; the check sits in the Metadata Editor's write path. Independent of the playlist editor, though related — could stand alone or fold into that work.


## Approach

### The check wraps the tag write

In the editor's save path, the file's content-identity hash (the `resilient-playlists` hasher) is computed immediately before and immediately after `write_tags`, on the same path before any rename — a rename moves the file but can't change its bytes. If both hashes compute and differ, the audio payload changed during a tag write, which must never happen: a critical fault.

### On a mismatch, assemble a self-contained incident folder; no auto-undo

All error artefacts live together under `~/.local/state/deck/identity-mismatches/` (`$XDG_STATE_HOME`, the correct home for diagnostic state — away from both the music folders and the app's config), created on first use. Before the tag write, the file is copied to a staging spot there. On a clean check (hashes match or unverifiable) the staging copy is deleted — no clutter, cost is a copy removed straight away. On a mismatch, an incident folder `<timestamp>-<stem>/` is created holding everything needed for analysis:

- `original.<ext>` — the pre-edit file (the staged copy),
- `edited.<ext>` — the post-edit result, copied in,
- `details.txt` — timestamp, source path, format, before/after hashes, and the audio-payload byte ranges each side reported.

So a fault yields both files to byte-diff plus the metadata, in one grabbable folder. A critical notification tells the operator the incident-folder path.

No *automatic* undo — a mismatch most likely means a bug in our own byte-range extraction, so silently reverting would hide the defect; preserving `original.<ext>` retains the evidence and a manual recovery path without that concealment. (Undo was the Intent's open question; resolved as this preservation rather than revert.) Cost: one file copy per tag edit, deleted immediately unless a fault is found; a crash between copy and delete could leave a stray staging file, not worth extra cleanup.

### Unverifiable edits are skipped, not flagged

If the hash cannot be computed before or after (an unsupported or unparseable file), the check is skipped rather than raising a false alarm — an unreadable identity is a different concern from a changed one, and not one this check exists to catch.


## Plan

- [x] `state_dir()` helper: `$XDG_STATE_HOME` (else `~/.local/state`) plus `/deck`.
- [x] Verified tag write: stage a copy, hash the file before and after `write_tags`; delete the staging copy on a clean or unverifiable check, or assemble an incident folder (`original.<ext>`, `edited.<ext>`, `details.txt` with timestamp/path/format/hashes/payload ranges) and return its path on a mismatch.
- [x] In the editor save path, call the verified write; on a returned incident, raise a critical notification naming the folder.
- [x] Bump Cargo patch (0.11.35 → 0.11.36).


## Log

- `write_tags_verified(path, fields)` stages a copy under `state_dir()/identity-mismatches/`, hashes before/after `write_tags` (crate `content_hash`), and returns `Ok(None)` clean / `Ok(Some(incident_dir))` on mismatch / `Err` on write failure. On mismatch it moves the staged copy to `original.<ext>`, copies the result to `edited.<ext>`, and writes `details.txt` (timestamp, path, before/after hashes, payload ranges for both). The editor save overrides its outcome notification with a critical alert naming the folder.
- Verified the happy path (`tag_write_preserves_identity`): a real lofty tag write on a corpus FLAC leaves the identity unchanged → no incident, no leftover staging. This guards against false positives spamming incidents on every edit. `XDG_STATE_HOME` pointed at a temp dir for the test.
- `state_dir()` is the reusable XDG-state helper; only this change's artefacts use it so far — panic.log/config relocation stays deferred (per the track-data-storage stub's note).
- Fault-injection hook (0.11.37) for exercising the alert without risk: `DECK_SIMULATE_IDENTITY_FAULT=1` forces the after-hash comparison to fail (it does *not* corrupt the file — an earlier byte-flip version was replaced as a footgun). Run the app with it to see the `⚠` notification + incident folder on any edited file, or the ignored `demo_identity_incident` test to print an incident. Zero cost when unset.
- Alert placement (0.11.38): when the browser is open (the usual edit context), the critical warning shows as a red banner across the browser header (`BrowserState.alert`, a 30 s-expiry message) rather than the transient global notification — closer to the eyes and long enough to read. Browser-closed edits (load-time rename offer) still use the global notification, now also 30 s.


## Conclusion

Shipped at 0.11.38. Every tag save now verifies content identity: the file is hashed (crate `content_hash`) before and after `write_tags`, and an unchanged identity is the silent normal case. A genuine change — a byte-range extraction bug or a corrupting write — raises a critical `⚠ IDENTITY CHANGED` alert (a 30 s red banner in the browser header when open, else the global notification) and assembles a self-contained incident folder under `~/.local/state/deck/identity-mismatches/<timestamp>-<stem>/` with `original.<ext>`, `edited.<ext>`, and `details.txt` (hashes, format, payload ranges). No auto-undo — the original is preserved for recovery and analysis.

Beyond the plan: a reusable `state_dir()` XDG-state helper (future config/state relocation can adopt it), and a safe fault-injection hook (`DECK_SIMULATE_IDENTITY_FAULT`) plus an ignored `demo_identity_incident` test for exercising the alert without touching files. Verified the happy path (`tag_write_preserves_identity`) so a normal edit never false-alarms. Spun off: [[track-data-storage]] (moving per-track BPM/cue/gain data into the user's filesystem). No map change — an internal safeguard.
