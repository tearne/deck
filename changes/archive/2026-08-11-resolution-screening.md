# Resolution Screening

**Mode:** Formal

## Intent

*(Proposed by [[resolution-complexity-review]], which recommended the opposite screen — see Approach.)*

When a track isn't where the playlist expects it, Deck searches the library, skipping anything that isn't about the right length *and* about the right file size. Adding cover art changes a file's size but not how long it plays — so a track that was re-organised and re-tagged gets skipped, and the operator is asked to identify by hand what Deck could have recognised outright.

Deck also searches the library even when every track is exactly where it should be, which costs time for nothing.

Check length only, and search only when something is actually missing.

## Approach

### Same size first, then similar length

A file that was only moved keeps its size exactly; one that was also re-tagged keeps only its length. Trying the same-size files first settles the ordinary case in a single read, and widening to similar length when none of them match means a re-tagged track is still found rather than handed to the operator. The review proposed size alone, which is fast but leaves that track unfound.

### Size must match exactly, not within a band

The current 1% allowance was calibrated to absorb "minor container rewrites", which it does not do — cover art blows past it easily. Nothing between exact and same-length is worth expressing: an untouched file matches to the byte, and anything else is the widened pass's problem.

### The library is searched only when something is missing

The search is prepared on first use rather than in advance, so a playlist whose tracks are all present does no searching at all — restoring what was lost in [[resolution-scan-cost]].

### The spec drops the size rule

The format's search step describes a size screen that will no longer exist.

## Plan

- [x] Confirm same-size candidates before any others
- [x] Widen to similar-length candidates only when no same-size candidate confirms, skipping those already read
- [x] Prepare the library search on first use, so a playlist with nothing missing never searches
- [x] Replace the size tolerance in the format's search step
- [x] Cover both journeys in tests — a moved file found from the same-size pass, a moved and re-tagged file found from the widened pass
- [x] Cover a playlist with nothing missing performing no search

## Conclusion

The review recommended screening on size alone; this change does close to the opposite, because size is the property that changes when a file is re-tagged. Anyone reading the archived review should treat its proposal as superseded here.

The format spec grew rather than shrank — the search is now two ordered attempts. That is the cost of not losing a track sitting in plain sight.

Also fixed the eager library scan introduced by [[resolution-scan-cost]], where a playlist with nothing missing searched anyway.

## Log

- An existing test, `offers_are_capped`, began failing because its library contained a file byte-identical to the missing entry. The old size screen had excluded that file, so the test reached the fallback by accident; the widened pass now finds it, which is correct. The fixture was wrong, not the change.
- Making the search lazy removed the need to pass the library alongside it: the search holds the library, so `resolve` and its callers lost a parameter rather than gaining one.
- The screening cost is now paid only by playlists that need it. A playlist whose hints are all good walks nothing, which is what the test `a_playlist_with_nothing_missing_never_searches` pins.
- Version 0.15.25 → 0.15.26.
