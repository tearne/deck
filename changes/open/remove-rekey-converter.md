# Remove Re-key Converter

**Mode:** Formal

## Intent

*(Parked — hold until the re-key conversion has been run.)*

[[rekey-track-data-to-content-identity]] adds a hidden `--rekey-track-data` flag (the `rekey` module) that rewrites the existing `track-data.json` from decoded-PCM keys to content identities. It's a one-off: once the local database has been converted, the flag and the `rekey` module are dead weight — and `hash_mono`, whose only remaining caller is that module, dies with it (the load site no longer falls back to it).

Strip the flag, the `rekey` module, and `hash_mono` out.

Depends on [[rekey-track-data-to-content-identity]] having shipped and the conversion having been run. Same retire-after-use pattern as [[remove-cache-migration]].
