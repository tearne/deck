# Remove Re-key Converter

**Mode:** Formal

## Intent

*(Parked — hold until the re-key conversion has been run.)*

[[rekey-track-data-to-content-identity]] adds a hidden `--rekey-track-data` flag (the `rekey` module) that rewrites the existing `track-data.json` from decoded-PCM keys to content identities. It's a one-off: once the local database has been converted, the flag and the `rekey` module are dead weight — and `hash_mono`, whose only remaining caller is that module, dies with it (the load site no longer falls back to it).

Strip the flag, the `rekey` module, and `hash_mono` out.

Depends on [[rekey-track-data-to-content-identity]] having shipped and the conversion having been run. Same retire-after-use pattern as [[remove-cache-migration]].


## Approach

Straight deletion — every target is converter-only, confirmed by grep:

- The `rekey` module, the hidden `--rekey-track-data` flag, and its startup branch.
- `hash_mono` and the `TrackDatabase::overwrite_and_save` helper added for the converter — nothing else calls either.
- The `blake3` dependency, whose only use was `hash_mono`.


## Plan

- [x] Remove the `rekey` module, the `--rekey-track-data` flag, and its startup branch
- [x] Remove `hash_mono` and `TrackDatabase::overwrite_and_save`
- [x] Drop the now-unused `blake3` dependency


## Conclusion

Completed.
