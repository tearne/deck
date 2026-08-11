# Deck Playlist Warning

**Mode:** Formal

## Intent

A deck's playlist can carry tracks that can't be located — most often because it was loaded before a workspace was set. Setting the workspace afterwards reports what it repaired, never what it couldn't, so the deck gives no sign until the operator reaches a missing track mid-mix.

Surface the problem on the deck itself:

- A warning status message on the deck whose playlist has unresolvable tracks.
- The `≡ x/y` playlist badge in a warning colour rather than its usual teal.

The badge says there is a problem; the browser says what it is.

## Approach

### A count of unplayable entries on the deck

The deck answers "is there a problem?", so it carries a number, not the per-entry status vector the panel keeps. Unplayable covers both unavailable and needs-confirmation entries — neither can play from the deck, and which it is, is the browser's answer.

### Counted only where resolution already runs

Opening a playlist on a deck, healing at workspace-set, and the panel committing an edit back to decks. No re-check on advance or on a timer — the badge reflects the last resolution. One screening serves the whole playlist, so resolving every entry costs no extra library pass.

### The panel hands its count to the deck

Committing an edit already copies the panel's entries into matching decks; the count travels with them, so re-linking a track in the browser clears the deck's warning without resolving again.

### The message reports the count

Today's nudge fires from a hint-path existence check, and only when no workspace is set. The real count replaces it: with a workspace, the message names how many entries are unplayable; without one, it keeps the "set a workspace" steer. A heal that leaves nothing unplayable stays silent on the deck — the badge returning to teal is the signal, and the global success message already covers it.

## Plan

- [x] Count a playlist's unplayable entries when it opens on a deck
- [x] Recount when setting a workspace heals a deck's playlist
- [x] Carry the panel's count to matching decks when an edit commits
- [x] Colour the playlist badge with the warning amber when the count is non-zero
- [x] Replace the hint-existence nudge with a message naming the count
- [x] Warn on each deck left with unplayable entries after a workspace-set heal
- [x] Cover in tests — entries counted at open, and the count cleared by a re-link

## Conclusion

No documentation impact. Playlists on a deck are unmapped, and mapping them was deferred as premature rather than overlooked — the badge and its count will need a home when that node is written.

Version 0.15.27 → 0.15.28.

## Log

- `heal_playlist` was deleted rather than extended. Resolving a playlist for the count already does everything it did, so the workspace-set pass and the open path now share one function, in two forms mirroring the panel's `recompute_status` / `recompute_status_against` split.
- The old open-time nudge tested hint-path existence only. A set whose tracks all sit where the hints say but fail the hash check previously stayed silent at open and now warns.
- Auto-advance and playlist skip carry the existing count forward rather than recomputing it.
- The deck's adoption of the panel count is a direct assignment in `commit_playlist`. The re-link test covers the panel side only — constructing a `Deck` needs a live audio player.
