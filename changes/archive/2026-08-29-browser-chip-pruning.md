# Browser Chip Pruning

**Mode:** Wander

## Intent

The browser's location chip earns its keep only where the caption explains something not self-evident. Keep it for the rename offer and new-playlist arrivals; drop it from the `` ` `` jump and the passive follow (the highlight is the feedback). The caption becomes `Option<String>` on `go_to`, fixing an existing artifact where empty-string captions rendered a stub chip. (2026-08-29)

## Log

- Extended at hand-back: the open-time "Working directory" chip (fresh open and session restore) dropped too — same self-evidence rule.

## Conclusion

`go_to`'s caption is `Option<String>`; chips remain only where arrival isn't self-evident (rename offer, new playlist). Dropped from the `` ` `` jump, passive follow, playlist-edit landings (fixing their empty-string stub chips), and both open-time "Working directory" sites. Shipped as 0.34.7. Map impact: the chip sentence in Jump to Loaded.
