# Self-documenting Track Data

**Mode:** Formal

## Intent

*(Parked — captured as an aside during [[track-database-portability]].)*

Once the track database travels with the music and sits as a visible file in the library root ([[track-database-portability]]), a stranger opening `track-data.json` has no idea what it is. Move to a comment-tolerant "human JSON" format so the file can carry a header explaining itself: what it's for, which application it belongs to (Deck), and the locations it lives in (`~/.local/share/deck/` canonical, plus a copy in the workspace root).

Format options to weigh: JSONC/JSON5 (comments, needs a tolerant reader on load), or a `_comment`/`_about` field kept inside plain JSON (no parser change, less elegant). Applies to both the canonical and workspace copies, so it's independent of the portability sync design.

## Approach

A wrapper object with an `_about` field, staying valid JSON — a `.json` file that isn't parseable JSON is a trap for outside tools, and self-documentation is for strangers. No new dependency. Reading tries the wrapper form, falling back to the legacy flat map, so existing files upgrade silently at their next save; both copies share the one writer, so mirror merging is untouched. Rider: entries move to a BTreeMap so file output is deterministically ordered (no diff churn in a versioned library).

## Plan

- [x] Wrapper form with `_about`, legacy fallback on read
- [x] Deterministic entry order

## Log

- Tests added: header roundtrip (leads the file, stays valid JSON) and legacy fallback; the existing mirror-sync test exercises the fallback path too since it writes a headerless mirror (46 tests total).

## Conclusion

Completed at v0.23.0; minor bump confirmed. Human-JSON formats (Hjson/JSON5) were passed over for plain valid JSON — outside tools keep working — so entries nest under `tracks` beside the `_about` header, legacy files upgrading via a one-time read fallback. Track Database's map detail updated alongside.
