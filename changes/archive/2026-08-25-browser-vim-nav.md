# Browser Vim Nav

**Mode:** Wander

## Intent

In browser command mode, `h` goes up one directory, joining `Backspace`/`Left`. Move mode already uses `h` this way, so this aligns the two vocabularies. `h` is unbound in command mode and the panel-side `h` uses intercept before the browser sees the key — no clashes. Note: `l` stays as-is (playlist panel), so there's no full vim h/l symmetry. (2026-08-25)

## Conclusion

`h` joined `Backspace`/`Left` going up; `l` descends into a highlighted directory and nothing else — loading stays on Enter, and playlists keep the panel (which consumes `l` first). Both also work while picking tracks in playlist-edit browser focus, by normal fall-through. Legend reads `h/j/k/l move`. Shipped as 0.33.13. Map impact: the Browser node's command-mode sentence.
