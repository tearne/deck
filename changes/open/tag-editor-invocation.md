# Tag Editor / Renamer Invocation

## Intent

Today the tag editor and the file-renamer it drives can only be reached one way: loading a track whose filename doesn't conform to `Title - Artist` raises a rename offer, and `y` opens the editor. Two things are worth revisiting:

- **No on-demand invocation** — a file whose name already conforms, or that you simply want to retag, can't be edited at all. The editor only appears for non-conforming names at load.
- **The entry key `y` is hardcoded** — it isn't a configurable action like most of the player's controls.

This proposal is a placeholder to discuss whether to add an on-demand way to open the editor/renamer, and whether its invocation should move into the configurable keymap.
