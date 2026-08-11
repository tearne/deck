# Backup Rotation On Edit

**Mode:** Wander

## Intent

*(Proposed by [[resolution-complexity-review]].)*

Deck keeps three backups of each playlist so a damaged file can be recovered. It takes a fresh backup on every save — including the saves it makes by itself when it quietly repairs track locations.

Re-organise your music, open a playlist a few times, and all three backups have been pushed out by those automatic saves. The copy from before your last real edit — the one you would actually want back — is gone.

Take a backup when the operator changes something, not when Deck tidies up.

## Conclusion

Repairs write without touching the backups at all, rather than rotating into a slot reserved for them as the Intent offered. A reserved slot would preserve the pre-repair state, but that state is rarely what anyone wants back — the useful recovery point is the playlist as the operator last chose it, and not rotating produces exactly that with no extra machinery.

The cost accepted: if a repair ever relocates an entry wrongly, the backups won't hold the pre-repair version, so recovery goes back to before the last edit. A wrong relocation needs a hash collision, which the identity scheme makes vanishingly unlikely; losing edit history to routine housekeeping was happening by design.

Two named save functions rather than a flag, so each call site states which kind of write it is.

The spec gained a sentence on what backups are for, so a later implementer doesn't read the exception as an optimisation and drop it.

## Log

- Five save sites: one operator edit (committing an edited playlist), one creating a new empty playlist, and three repairs. Only the first two rotate now.
- Creating a new playlist still rotates, which costs nothing — there is no prior state to push out.
- Spec amended in two nodes: the write procedure makes rotation conditional, and the backup scheme now states what backups are for, so the exception doesn't read as an optimisation a later implementer could drop.
- Version 0.15.26 → 0.15.27.
