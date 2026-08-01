# Re-key Track Data to Content Identity

**Mode:** Formal

## Intent

Per-track memory (BPM, offset, cue, gain) is keyed by a decoded-PCM hash (`hash_mono`) — computed by decoding the audio to mono samples. Playlists and the tag editor instead key on **content identity** ([[content-identity-hashing]]): a hash of the encoded audio payload with tags excluded, designed to be portable and shareable across machines.

Switch per-track memory to the same content-identity hash, so a track has one identity across the whole app — the hash its playlist entry already references — and its analysis/edits become portable and shareable the way playlists are.

The cost is a re-keying migration: existing entries are keyed the old way and their new key can't be derived without re-hashing each file, so this needs its own handling (re-key lazily on load, or a one-time pass).

Depends on [[track-data-storage]] having relocated per-track memory to its own file first.
