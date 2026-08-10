# Unreadable Tags Are Not Empty

**Mode:** Wander

## Intent

*(Grew out of [[resolution-complexity-review]], originally proposed as a guard inside playlist resolution.)*

Reading a file's tags reports failure as success. `read_tags_for_editor` returns empty strings when the file can't be opened or its container can't be parsed — indistinguishable from a file that genuinely carries no tags.

Four consumers take that at face value: resolution's description refresh replaces a good description with blanks; newly built playlist entries are minted with an empty description; the browser preview shows a tagged track as untagged; and the tag editor opens with every field blank, one save away from writing those blanks over the file's real tags.

Report the failure instead of hiding it, and let each consumer decide what to do with it.

## Conclusion

The change arrived as a playlist fix — guard the description refresh against being emptied — and became a tags-layer fix instead. Reading the code showed the reader already knew it had failed and was discarding that fact, reporting an unopenable file as one carrying no tags. Guarding downstream would have inferred that failure back from emptiness, added a rule to an area a review had just judged over-wired, and left the same defect in the three other consumers.

Fixing at source needed no engine change: `changed_description` already declined a `None`, and `Library::read_description` already had the channel to carry one.

`None` is handled per caller rather than by central policy, because the four consumers want genuinely different responses — propagate, render empty, or refuse to open and warn. The browser preview is the weakest of these: an unreadable file now shows as empty rather than as an untagged track, which is less wrong but not yet informative.

## Log

- The engine needed no change at all. `changed_description` already treats `None` as "no refresh", and `Library::read_description` already returned `Option` — `WorkspaceLibrary` simply never used that channel, wrapping the all-empty array in `Some`.
- The four consumers wanted different answers, so `None` is handled per caller rather than centrally: resolution and `track_facts` propagate it, the browser preview falls back to `Preview::Empty`, and both tag-editor open sites decline to open and warn.
- The tag editor was the sharpest case — `for_track` seeded its fields directly from the reader, so a failed read opened a blank editor over a fully tagged file, one save from overwriting it. Now returns `Option<Self>`.
- Version 0.15.22 → 0.15.23.
