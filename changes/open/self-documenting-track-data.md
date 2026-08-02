# Self-documenting Track Data

**Mode:** Formal

## Intent

*(Parked — captured as an aside during [[track-database-portability]].)*

Once the track database travels with the music and sits as a visible file in the library root ([[track-database-portability]]), a stranger opening `track-data.json` has no idea what it is. Move to a comment-tolerant "human JSON" format so the file can carry a header explaining itself: what it's for, which application it belongs to (Deck), and the locations it lives in (`~/.local/share/deck/` canonical, plus a copy in the workspace root).

Format options to weigh: JSONC/JSON5 (comments, needs a tolerant reader on load), or a `_comment`/`_about` field kept inside plain JSON (no parser change, less elegant). Applies to both the canonical and workspace copies, so it's independent of the portability sync design.
