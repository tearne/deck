# Tags Refresh Guard

## Intent

*(Proposed by [[resolution-complexity-review]].)*

Locating a file refreshes the entry's stored description from its current tags whenever they differ. The comparison accepts any difference, so a file whose tags read as empty replaces a populated description with an empty one.

That description is the only record of what a track is once its file goes missing, and the key the descriptive fallback matches on — so an empty refresh discards the recovery data at the moment recovery becomes necessary.

Decline a refresh that would empty a populated description. This is deliberately not a judgement about tag quality, which a playlist has no way to make; it distinguishes only "the file says something different" from "the file says nothing".
