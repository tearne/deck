# Backup Rotation On Edit

## Intent

*(Proposed by [[resolution-complexity-review]].)*

Recomputing a playlist's status persists the file whenever resolution heals anything, and every write rotates the backup slots. So merely opening a playlist that heals one stale hint discards the oldest backup; three such opens retire every backup a corruption would have needed to recover from.

Backups should track operator edits, not incidental healing. Healing writes should either not rotate, or rotate into a slot reserved for them.
