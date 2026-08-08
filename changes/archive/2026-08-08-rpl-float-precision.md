# RPL Float Precision

**Mode:** Wander

## Intent

`.rpl` entries record `duration_secs` at full f64 precision, e.g. `198.86666666666667`. It's verbose and noisy in a human-readable, hand-editable file. Duration only feeds candidate pre-filtering at ±2 s tolerance ([[playlist-format]], File Resolution), so a couple of decimal places is ample.

Round `duration_secs` to a sensible precision when writing `.rpl` files. It's not part of the content hash, so this doesn't touch identity.

## Conclusion

Rounding sits at the serialisation boundary — `serialize_with` on `Identity.duration_secs` — rather than at the two identity-stamping sites. Construction-time rounding would have left a single file holding both precisions depending on when each entry was added, which reads as arbitrary rather than merely verbose; normalising at the writer heals legacy files on their next write with no migration step.

That rewrites a field inside `identity`, which the spec declared immutable outside a confirmed re-link. Rather than carve out precision as a special case, the spec now separates the track an entry *denotes* (immutable except on re-link) from how that identity is *encoded* (normalisable forward). This absorbed a pre-existing contradiction: Method Migration already rewrote encoding fields on a plain resolve, which the old "only sanctioned mutation" wording didn't account for.

The spec states a bound — rounding error an order of magnitude below the resolution tolerance — not a precision. 2 dp is Deck's choice and stays out of the portable format.

Follow-on: Deck's `map.md` has no playlist node, so nothing on the Deck side records the 2 dp choice. Pre-existing gap, not opened here.

## Log

- Agreed shape at kickoff: 2 dp, rounded at the serialisation boundary in `src/playlist/mod.rs` so every write normalises and legacy files heal on next write.
- Write-time normalisation rewrites a field inside `identity`, which `resilient-playlists/map.md:75` and `:239` declare immutable outside a confirmed re-link. Spec amendment agreed as part of this change: distinguish the track an entry *denotes* (immutable except on re-link) from how that identity is *encoded* (normalisable forward).
- `:239`'s "only sanctioned mutation of identity" was already contradicted by Method Migration (`:266`), which rewrites `content_hash`, `payload_extraction_version` and `hash_algorithm` on a plain resolve. The same amendment resolves both.
- The sentence at `:239` sits in the **Descriptive Fallback** node, not **File Resolution** as first recorded.
- `duration_secs` has a third reader beyond the two resolution paths: `src/render/mod.rs:2092` displays it at `{:.0}`, whole seconds. Rounding to 2 dp is two orders of magnitude finer than anything shown, so the display is unaffected.
- The spec bound is expressed as "rounding error at least an order of magnitude below the resolution tolerance" rather than a fixed precision. `resilient-playlists` is a portable spec, so Deck's 2 dp is not named in it.
- Spec amended in `resilient-playlists/map.md`: **Identity** (denotes/encodes split, rounding bullet, See also retarget) and **Descriptive Fallback** (closing sentence). **Method Migration** needed no change — it only rewrites encoding fields, which the amended Identity node now licenses.
- Implemented as `#[serde(serialize_with)]` on `Identity.duration_secs`; the in-memory value keeps full precision.
- Deck's own `map.md` has no playlist node, so the 2 dp choice is recorded nowhere on the Deck side. Pre-existing coverage gap, not opened by this change.
- Version 0.15.21 → 0.15.22.
