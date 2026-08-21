# Drop Detection

**Mode:** Formal

## Intent

*(Captured as an aside during 30-grid-anchors map catch-up.)*

Automatic BPM detection has never worked very well, and establishing the tempo manually is more fun anyway — tap plus grid refinement covers the job. Remove detection: the action, its keys, the confirmation flow, the background analysis pass, and the stratum-dsp dependency if nothing else uses it. Cache lookup, tap, manual adjust, and refinement remain the BPM sources.

## Approach

Detection owned more than its key: the `pending_bpm` confirmation prompt (its only feeder — tap never confirms), the 15 s auto-reject, the `y`-intercept, and the redetect state (`redetecting`, `background_rx`, `redetect_saved_hash`) all go with it, along with `detect_bpm()` and the stratum-dsp dependency. The load-time pass survives — it hashes and looks up the cache, no detection involved. The BPM confirmation prompt ceases to exist as a concept, so the Messages map node's prompt examples change too.

## Plan

- [x] Action, keys, and handler removed
- [x] pending-BPM prompt and redetect state removed
- [x] `detect_bpm` and stratum-dsp removed
- [x] Docs, help overlay, and map updated (Beat Grid, root/Track Loading wording, Messages prompts, keybindings)

## Conclusion

Completed at v0.28.0; minor bump confirmed. The map's "BPM analysis" phrasing in the root and Track Loading nodes had always meant the hash-and-cache-lookup pass — reworded to say so now that no analysis exists.
