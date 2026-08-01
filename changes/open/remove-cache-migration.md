# Remove Cache Migration

**Mode:** Formal

## Intent

[[track-data-storage]] adds startup migration that reads the old `~/.config/deck/cache.json`, distributes its fields into the new XDG data/state files, and retires it — plus the legacy flat-HashMap read path it depends on. This code earns its keep only while users still have an un-migrated `cache.json`; once existing installs have started up once, it's dead weight.

After a suitable interval, strip the migration and the legacy read path out, leaving only the direct XDG-located load/save.

Depends on [[track-data-storage]] having shipped. Hold until existing installs have had time to migrate.
