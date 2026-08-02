# Remove Cache Migration

**Mode:** Formal

## Intent

[[track-data-storage]] adds startup migration that reads the old `~/.config/deck/cache.json`, distributes its fields into the new XDG data/state files, and retires it — plus the legacy flat-HashMap read path it depends on. This code earns its keep only while users still have an un-migrated `cache.json`; once existing installs have started up once, it's dead weight.

After a suitable interval, strip the migration and the legacy read path out, leaving only the direct XDG-located load/save.

Depends on [[track-data-storage]] having shipped. Hold until existing installs have had time to migrate.


## Approach

### Delete the migration code and its startup call

Remove `migrate_legacy_cache`, `read_legacy`, and `LegacyCacheFile` from the cache module and the `migrate_legacy_cache()` call at startup. The two stores load directly from their XDG files; nothing else references the removed code. `write_json_atomic` and `default_art_bright_idx` stay — the live stores use them.

### No orphan cleanup

A stray `~/.config/deck/cache.json` on an install that never ran v0.11.40 is left untouched, not deleted. The hold is lifted on the premise that every install has already migrated (which deletes the file), so there is nothing to clean; adding cleanup would just re-add the coupling this change removes.


## Plan

- [x] Remove `migrate_legacy_cache`, `read_legacy`, and `LegacyCacheFile` from the cache module
- [x] Remove the `migrate_legacy_cache()` startup call


## Conclusion

Completed.
