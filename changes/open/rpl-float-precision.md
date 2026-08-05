# RPL Float Precision

**Mode:** Formal

## Intent

*(Parked — captured as an aside during [[playlist-needs-confirmation]].)*

`.rpl` entries record `duration_secs` at full f64 precision, e.g. `198.86666666666667`. It's verbose and noisy in a human-readable, hand-editable file. Duration only feeds candidate pre-filtering at ±2 s tolerance ([[playlist-format]], File Resolution), so a couple of decimal places is ample.

Round `duration_secs` to a sensible precision (e.g. 2–3 dp) when writing `.rpl` files. It's not part of the content hash, so this doesn't touch identity. Points to settle:

- Chosen precision, and whether any other float fields have the same issue.
- Whether the `resilient-playlists` conformance/corpus expects an exact duration serialisation (round in Deck's writer vs. in the shared format rules).
- Migration: existing files rewrite to the rounded form on next resilient write, or left as-is.
