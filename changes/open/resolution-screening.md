# Resolution Screening

## Intent

*(Proposed by [[resolution-complexity-review]].)*

Step 2's candidate screen exists only to avoid hashing, and the hash confirm that follows is authoritative — so the screen can never prevent a wrong match, only lose a right one. A file it excludes is a track whose hash would have matched exactly, pushed into the confirmation picker for no reason.

Screen on file size alone and hash-confirm whatever passes. Dropping the duration screen also removes a file open and container parse per candidate, which is what makes the screen expensive in the first place.

Two things travel with it:

- The size tolerance needs a number that matches its stated purpose. `resilient-playlists/map.md:228` justifies 1% as accommodating "minor container rewrites that don't change the audio payload", but embedded artwork is exactly that and routinely exceeds it.
- The spec's "cheapest-first" wording in step 2 describes an ordering the implementation never had; with one screen left, the phrasing needs revisiting.
