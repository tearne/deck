# Browser H Updir

**Mode:** Formal

## Intent

In browser command mode, `h` goes up one directory, joining `Backspace`/`Left`. Move mode already uses `h` this way, so this aligns the two vocabularies. `h` is unbound in command mode and the panel-side `h` uses intercept before the browser sees the key — no clashes. Note: `l` stays as-is (playlist panel), so there's no full vim h/l symmetry. (2026-08-25)
