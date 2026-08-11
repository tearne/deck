# Deck Playlist Warning

## Intent

Loading a playlist before a workspace is set means its tracks can't be located yet. Setting the workspace afterwards repairs what it can and reports *"Relocated moved tracks in open playlists"* — but it says only what it fixed, never what it couldn't. A deck can be carrying a set with missing tracks and give no sign until the operator reaches one mid-mix.

Surface the problem on the deck itself:

- A warning status message on the deck whose playlist has unresolvable tracks.
- The `≡ x/y` playlist badge in a warning colour rather than its usual teal.

The badge is an indicator, not a diagnosis — the operator opens the playlist in the browser to see which tracks and why. It answers "is there a problem in this set?", and the browser answers "what is it?".

Depends on [[resolution-scan-cost]], which makes resolving every entry cheap enough to do for decks as well as the panel.
