# Track Data Storage

**Mode:** Formal

## Intent

*(Stub — needs design thought, not yet scoped.)*

The per-track memory — detected BPM, phase offset, cue point, gain trim, keyed by a content hash — currently lives hidden in the app's `cache.json`. Consider storing this data in the user's filesystem instead, so it travels with the music library and is user-visible and controllable rather than buried in an app directory.

Open questions for the design:

- **Where** — alongside each track, a sidecar per directory, or one file at a user-chosen library root?
- **Format** — human-readable and durable, resilient-write like the playlist format?
- **Identity** — could it key on the same content-identity hash as [[content-identity-hashing]] (encoded-payload) rather than the current decoded-PCM hash, so per-track data becomes portable and shareable the way playlists are?
- **Migration** — folding in the existing `cache.json` per-track entries.

Separate from the app's config/state directory tidy-up (config.toml, panic.log placement), which can proceed independently.
