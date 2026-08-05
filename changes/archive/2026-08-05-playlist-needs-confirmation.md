# Playlist Needs-Confirmation Picker

**Mode:** Formal

## Intent

*(Spun out of [[playlist-editor]].)*

The playlist engine's `resolve` returns `NeedsConfirmation { candidates }` — the **descriptive fallback** — when a track's audio has changed (re-encoded/transcoded/re-ripped) so its content hash no longer matches *any* file, yet workspace files still match closely by description. Because the hash can't confirm identity, the engine can't be sure, so it offers ranked candidates rather than silently re-linking (the spec requires explicit user confirmation).

Deck currently treats that outcome as unavailable and never surfaces the candidates. Present the ranked candidates (path, tags, duration) for the operator to confirm one — the engine's `adopt_candidate` re-stamps the entry's identity from the chosen file — or reject, leaving it unavailable.

Scope is the descriptive fallback only. The other path to this outcome, an entry recorded under an older extraction method, should heal *automatically* per the spec, not via this picker — that's [[playlist-method-migration]].

Niche: a normal move (audio unchanged) resolves by hash via the workspace heal; this triggers only when the audio itself changed. Testable by corrupting an entry's `content_hash` in an `.rpl` (with a workspace set and the description intact). Until built, such an entry shows unavailable and can be re-added in the context-panel editor.

Also fold in the minor cosmetic tweaks to the context panel flagged during [[playlist-editor]] (to be specified).


## Approach

### Three-state entry status

The context panel's per-entry availability (currently `Found` vs not) becomes three-state: **Found / needs-confirmation / unavailable**, read from `resolve`'s outcome. A needs-confirmation entry renders distinctly (a `?` marker, not the dim "unavailable" style). Playback already steps over every non-`Found` entry, so a `?` entry is skipped until confirmed — no change to auto-advance or skip.

### Candidate ranking honours duration (spec conformance)

`descriptive_candidates` ranks by matching description fields only; the spec's Descriptive Fallback says candidates are "library files whose duration **and** description are similar". Bring the engine in line: filter candidates to within the duration tolerance (±2 s, as the library search already uses) and rank by description-field matches, then by closest duration. Duration is already probed for display, so this only tightens the existing scan. No spec change — the spec already prescribes this; the code was looser.

### Picker hosted in the context panel

Triggered from **Browse** by `Enter` on a `?` entry — it can't be loaded, so `Enter` offers the fix instead. The panel swaps the entry list for the candidate list, hosted like the in-panel tag editor (browser dims, distinct colour). Each candidate is a multi-line card — path, then artist/title/album, then duration — since there are few. `j/k` navigate, `Enter` confirms, `Esc` returns to Browse.

### Confirm adopts and persists; reject leaves it

Confirming calls the engine's `adopt_candidate` (the one sanctioned identity mutation): it re-stamps the entry's identity, hints, and description from the chosen file's `TrackFacts`, then the `.rpl` is written and any deck holding the playlist is synced. The entry re-resolves to `Found`. `Esc` cancels, leaving the entry unavailable.

### No workspace

A needs-confirmation status only arises with a workspace set (candidates come from searching it), so `?` entries can't exist without one. Pressing `Enter` on a plain unavailable entry with no workspace shows the existing "set a workspace (`@`)" nudge rather than an empty picker.


## Plan

- [x] Make the panel's per-entry status three-state (Found / needs-confirmation / unavailable) from `resolve`; mark `?` entries distinctly
- [x] Fix `descriptive_candidates` to filter and rank by duration (per spec), updating affected engine tests
- [x] Add the panel picker state: `Enter` on a `?` entry in Browse opens the candidate list (multi-line cards), hosted in the panel
- [x] Confirm → `adopt_candidate` + write `.rpl` + deck sync + re-resolve; `Esc` cancels
- [x] `Enter` on an unavailable entry with no workspace shows the set-workspace nudge


## Log

- `PlaylistPanel` availability became a three-state `EntryStatus` (Found / NeedsConfirmation / Unavailable); `?` entries render amber "? confirm".
- The picker is a `Panel::Confirm` state (holds the playlist buffer, the entry index, the ranked candidates, and a cursor), reusing the panel-hosting pattern; `adopt_candidate` runs on confirm, then `commit_playlist` writes and syncs a loaded deck, and status is recomputed.
- `descriptive_candidates` now filters to ±2 s duration and ranks by description then closest duration; existing fallback test still passes.
- Tags-refresh heal in the panel: `recompute_status` applies `resolve`'s `updated_entry` (relocated hints, refreshed tags) in place and persists on view/open, but not during transactional edit (`persist` flag) so the buffer isn't written mid-edit.
- The picker shows the original entry's recorded tags, hint path, and length; each candidate marks per-field matches (✓/· green/grey) including a duration delta, so the re-link decision uses more than the title.
- The picker freezes the original-entry header and scrolls the candidates in a stateful `List` (multi-line item per candidate), keeping the selection visible.
- Picker shows the candidate count and selected index in the header, and a scrollbar down the list, so off-screen candidates are obvious.
- The picker scrolls by line with `cursor` as a line offset, and the active card is the topmost one whose header line is still on screen (`confirm_active_card` — ceil of the offset, so a card yields to the next once its header scrolls off). `j/k` move one line; `Enter` adopts the active card.
- The playlist panel's entry `List` rendered without scroll state, so entries past the panel bottom were unreachable; now rendered stateful with the cursor selected so it scrolls to keep the cursor visible.
- A single Esc could cascade (deselect playlist → close browser) via a burst of events (kitty `Repeat`, or a phantom second `Press`). The phantom guard now slides: every Esc event (Press/Repeat/Release) stamps the window and only a Press after a gap longer than the window acts, so any burst collapses to one action. Overlaps [[guard-accidental-quit]].
- The picker header showed only the hint's relative path; `Hints` also carries `file_size_bytes`, now shown as a `size` line.
- All-green candidates were confusing: tags/duration match but confirmation is still needed because the content hash didn't (the actual trigger). Added a wrapped amber note ("Audio fingerprint changed — confirm the right file.") and a per-candidate `size` comparison line (`Candidate` now carries `file_size_bytes` from `cheap_probe`) — the cheap proxy for the changed audio payload, so a re-encode shows `·` and an identical file shows `✓`.
- Candidate paths were shown absolute while the hint was relative; candidates now display relative to the playlist dir (`relative_to` made `pub(crate)`) — same basis as the hint, and exactly what the hint becomes on adopt.
- Candidate cards are variable height: head, a line per tag field showing the value with a ✓/· match marker (album/year only when present), length with duration Δ, and the path hard-wrapped across as many lines as it needs. The renderer publishes each card's start line and the total into a shared `ConfirmLayout` (`Rc<RefCell>` on `Panel::Confirm`); the input's line-scroll reads it to clamp the offset and pick the active card, so input and render agree on the wrap-dependent layout without a fixed card height.


## Conclusion

Core landed per plan; the picker UX refinements added during testing are all in the Log, plus a playlist-panel scroll fix.

The Esc double-fire proved to be an input-architecture problem, not a picker detail — spun out to [[guard-accidental-quit]] with the analysis and an InputSource design; a partial guard remains in `main.rs`, to be removed there.

Map still has no playlist node — catch-up deferred, per [[playlist-editor]]. Shipped 0.15.17 (minor).
