# Placeholder BPM Persisted

**Mode:** Formal

## Intent

A new track with no BPM set opens in Playback mode, as intended. On reopening it later, it comes up in Beat mode with a BPM of 120 — the placeholder — though the operator never set one. Somewhere between load and the next open, the placeholder is being saved as if it were a real tempo (and/or Beat mode is being remembered for a track that never had a grid). A track without a tempo the operator chose should keep opening in Playback mode with no BPM.

Observed 2026-08-21 while testing ghost playheads (0.29.14); the track had been opened and played, but no tap, detection, or manual adjust was used.
