# Identity Stability Check

**Mode:** Formal

## Intent

A defensive check against byte-range extraction bugs — the fragile formats (MP3, AAC) especially. Whenever the application edits a track's tags/metadata, it computes the content-identity hash ([[content-identity-hashing]]) of the file immediately before and after the write. The hash must be invariant, because tag regions are excluded from identity by design; a difference means either our extraction wrongly included tag bytes, or the write corrupted the audio payload. Either is a critical fault.

This turns every real-world retag into a live conformance check on a real file, far wider coverage than the synthetic corpus, at negligible cost (hashing reads bytes without decoding).

On a mismatch, flag a critical error. Leaning **warn + loud log rather than undo**: a mismatch most likely indicates a bug in our own extraction, so undoing would destroy a legitimate edit to mask our defect, and undo needs a pre-edit copy anyway. The alarm worth surfacing to the operator is the data-integrity consequence — the track's identity in any playlist referencing it has just broken. Whether to also offer undo is an open question for the Approach.

Consumes the content-identity hasher; the check sits in the Metadata Editor's write path. Independent of the playlist editor, though related — could stand alone or fold into that work.
