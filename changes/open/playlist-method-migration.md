# Playlist Method Migration

**Mode:** Formal

## Intent

*(Parked — latent until `payload_extraction_version` is bumped.)*

The `.rpl` spec's **Method Migration** ([[playlist-format]], `resilient-playlists/map.md`) heals an entry forward when the byte-range rules change (a bump to `payload_extraction_version`, or a new `hash_algorithm`): resolve the file using the entry's *stated, older* version, and on a hash match, re-stamp the entry's identity to the current method — **automatically, no user confirmation**. The spec warns that skipping this degrades every migrated entry to the descriptive fallback needlessly.

Deck doesn't implement it. `resolve`'s `entry_matches_file` rejects a version mismatch outright (without trying the older rules), so a migrated entry sitting exactly where its hint says would wrongly fall to the descriptive fallback / confirmation picker. Harmless today because only version `1` exists — but wrong the moment the extraction rules are corrected.

Two pieces:

- The `resilient-playlists` crate must expose hashing under a *specified* extraction version (retain older rules), which it doesn't today — currently `content_hash` only produces the current version.
- `resolve` uses the entry's stated version for the hash-confirm and, on match, rewrites `content_hash` + `payload_extraction_version` (and `hash_algorithm`) forward — opportunistic and per entry (a missing file keeps its older version until it reappears).

Distinct from [[playlist-needs-confirmation]], which handles the genuine re-encode case (audio changed, needs confirmation).

## Notes

- 2026-08-09: Taken up, then re-parked. A `superseded` hash-history array was proposed alongside it (retain superseded identities so an implementation at an older version can still hash-resolve an entry that a newer one has migrated), plus a payload-keyed rule for whether a confirmed re-link clears that history. All deferred.
- The reason for deferring: the format already fails safe. `entry_matches_file` refuses to hash under a method it can't reproduce, so the entry degrades to the descriptive fallback and the operator confirms — not silent, not lossy. Migration is an optimisation on that path, not a correctness fix.
- `corpus/target_results.json` pins the extraction contract with expected hashes and byte ranges, so a `payload_extraction_version` bump means the corpus was wrong rather than that the rules were revised. That is rarer than this machinery assumes.
- Revisit if a real bump lands, at which point what actually changed will be known. See [[resolution-complexity-review]].
