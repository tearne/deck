# Browser Arrow Repeat

**Mode:** Formal

## Intent

Holding an arrow key in the browser doesn't scroll — the browser accepts only key presses and discards repeat events, so navigating a long list means hammering the key. Held arrows should repeat.


## Approach

### Repeats treated as presses, browser-wide

The browser's key handler accepts repeat events identically to presses for all keys, not just arrows. Terminals without key-release reporting already deliver repeats as presses, so this makes behaviour uniform across terminals instead of special-casing navigation keys; any key where auto-repeat mattered would misbehave in those terminals today, and none does.


## Plan

- [x] Browser key handler accepts `Repeat` events as presses.
- [x] Bump Cargo patch (0.11.9 → 0.11.10).


## Conclusion

Completed.
