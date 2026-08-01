# Browser Tag Compliance

**Mode:** Formal

## Intent

A browser workflow for cleaning up badly-named tracks: flag files whose filename doesn't match their tags (non-compliant with the `Title - Artist` policy), highlight them in the listing, and let the operator fix them one after another. Sit in a folder and the loop is: scan flags the offenders → jump to the next → edit it → its marker clears → jump to the next.

Compliance is compared against tags, which means opening and probing each file (via `propose_rename_stem`), so the check runs as a **background scan** — streaming results into the listing as they arrive, cancelling and restarting when the operator navigates, and caching so a revisit is instant. Non-compliant entries get a marker (and a count); a command jumps to the next/previous flagged entry, and an edit re-checks the file so a fix clears its flag.

Scope: the current directory's listing (a bounded set), not a whole-workspace scan.

Depends on [[browser-file-operations]] — fixing a flagged file in place is the in-browser edit operation that change introduces.


## Approach

### Compliance is a browser toggle

`T` (for Tags) in command mode turns compliance checking on for the browser; off by default. When on, the current directory's audio files are scanned and non-compliant ones marked, with a count; off, there is no scanning or marking. It is a deliberate "cleanup mode" rather than always-on, so ordinary browsing to load a track never pays the scan cost. The flag rides on `BrowserState` (preserved across navigation like the mode).

### Compliance = filename vs tags, the same check as the rename offer

A file is non-compliant when its stem differs from `propose_rename_stem` — exactly the test the load-time rename offer uses. That opens and probes the file for tags, so it is I/O-heavy.

### A background worker scans; results stream into a cache

Scanning runs on a `thread::spawn` worker fed the current directory's unscanned audio paths, returning `(path, non_compliant)` over an `mpsc` channel — the UI thread, which is drawing waveforms for any playing deck, never blocks on a file probe. Results land in a per-session cache keyed by path, so revisiting a directory is instant and only new files are scanned. A shared `AtomicBool` cancels the worker when the operator navigates away; a fresh scan starts for the new directory's unscanned files.

### Entries carry their result; a marker and count show it

`BrowserEntry` gains a compliance field (unknown / compliant / non-compliant), populated each frame from the cache. Non-compliant entries render with a marker, and the browser shows a count of how many need attention. Editing a file invalidates its cache entry, so a fix re-checks it and clears the marker.

### Jump between flagged entries

`n` / `N` in command mode move the cursor to the next / previous non-compliant entry, so the operator works through them in order: `n` to the next, `e` to fix it, marker clears, `n` onward.


## Plan

- [x] Add `compliance_on` to `BrowserState` (preserved across navigation) and a `compliance: Option<bool>` to `BrowserEntry`; `T` toggles it in command mode.
- [x] Background compliance worker: scans a directory's unscanned audio paths via `propose_rename_stem`, streams `(path, non_compliant)` over a channel, cancellable with an `AtomicBool`.
- [x] Per-session cache keyed by path; each frame, drive the scan for the current directory (cancel/restart on navigation) and populate entry compliance from the cache.
- [x] Invalidate a file's cache entry on edit so a fix re-checks it.
- [x] Render non-compliant entries with a marker and show a count while compliance is on.
- [x] `n` / `N` jump the cursor to the next / previous non-compliant entry.
- [x] Bump Cargo patch (0.11.29 → 0.11.30).


## Log

- Scan: `spawn_compliance_scan` runs `is_non_compliant` (`propose_rename_stem(path) != stem`) over a directory's unscanned audio paths on a thread, streaming `(path, flagged)` over an mpsc channel with an `AtomicBool` cancel. `drive_compliance_scan` runs each frame the browser is open — drains results into a session `HashMap` cache, marks entries from it, and cancels/restarts the scan on directory change. When off, it drops the scan and clears markers.
- Entry compliance rides on `BrowserEntry.compliance`; the `T` toggle and `n`/`N` jumps live in browser command mode; non-compliant entries render `⚠ name` in amber with a `tags: N ⚠ (n/N jump)` status indicator when on. `jump_flagged` wraps; unit-tested.
- Edit invalidation: `handle_tag_editor_key` removes the old and target paths from the cache on save, so a fix (tags-only or rename) re-scans and the marker clears.
- Incidental fix: `navigate_to` now preserves `target_deck` (and `compliance_on`); previously navigating into a directory reset the load target to deck 1 — a latent bug from the deck-independent-browser change.
- Hand-back refinement (0.11.31): editing a flagged file now auto-advances the cursor to the next flagged entry below. Only a save advances (a cancel leaves the cursor put — handles skips); with auto-advance covering the loop, the explicit `n`/`N` jump keys were removed (`jump_flagged` stays, driving the advance); reach the first flagged file with `j`/`k`.
- Auto-advance anchor fix (0.11.32): anchoring on the *fixed file's* new position jumped to the top whenever the fix re-sorted the file below the remaining flagged ones. Now anchors on the *neighbour that was below the fixed file* (captured at edit-open, stable across the rename since only the edited file is renamed): resume there if flagged, else the next flagged below. `handle_tag_editor_key` now returns whether a save occurred; main promotes the pre-captured anchor on save, discards it on cancel.
- Visual unification (0.11.33): the deck's load-time rename offer now uses the browser's non-compliant amber `Rgb(230,170,60)` (was red) and a `⚠` marker, fading to grey after 10 s as before — so the "non-conforming tags" signal reads the same in the browser and on the deck.


## Conclusion

Shipped at 0.11.33. A browser "cleanup mode" (`T`) that flags files whose name doesn't match their tags: a background worker scans the current directory (via `propose_rename_stem`, off the UI thread), results stream into a per-session path-keyed cache, and non-compliant entries show a `⚠` amber marker with a count. The workflow is `j`/`k` to the first flagged, `e` to fix, and the cursor auto-advances to the next flagged below — anchored on the fixed file's former neighbour so a rename's re-sort doesn't offset it, and only on a save so cancels leave you put. The deck's rename offer was recoloured to the same amber so the signal is unified.

Simplified during hand-back: the explicit `n`/`N` jump keys were dropped once auto-advance covered the loop. Incidental fix carried along: `navigate_to` now preserves `target_deck` (a latent reset-to-deck-1 bug from the previous change). Map catch-up pending: a Tag Compliance node under Browser.
