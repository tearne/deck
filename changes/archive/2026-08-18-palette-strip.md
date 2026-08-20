# Palette Strip

**Mode:** Formal

## Intent

`p` (palette_cycle) is half-wired: the handler cycles PALETTE_SCHEMES, but only one scheme exists, so the key visibly does nothing and the help advertises a dead feature. Strip the action, key, handler, and doc mentions; the schemes table stays (it defines the one palette). Git history keeps the machinery for when palettes return.

## Plan

- [x] Action, config key, handler, and scheme_idx removed
- [x] Help overlay and keybindings.md entries removed (legend alignment held)

## Conclusion

Completed at v0.26.3; patch bump. The map never claimed palette cycling, so no map impact.
