# One Esc, One Action

**Mode:** Formal

## Intent

The terminal delivers a single physical Esc press as two events, so one tap acts twice — cascading two levels, deselecting the playlist and then closing the browser. One Esc per level is the wanted behaviour; the duplicate is not.

## Approach

### Ask the terminal to encode Esc properly

`DISAMBIGUATE_ESCAPE_CODES` joins the pushed keyboard flags. The duplicate is Esc's *release*: `REPORT_EVENT_TYPES` alone asks Kitty to report releases while leaving Esc on a legacy encoding that can't express one, so it arrives as an identical second `Press`. Disambiguated, it becomes a labelled `Release`, which every handler already discards. No normalisation layer is needed — this is the whole fault.

### Esc acts on press only

Panel and browser handlers accept `Press | Repeat`. A held Esc now repeats where before it didn't, so Esc narrows to `Press` — otherwise a hold cascades the levels the duplicate used to.

### Quit confirmation is left alone

`pending_quit` keeps confirming only when a deck is playing. The unguarded-quit worry was a symptom of the double Esc; once one press yields one action, quitting with nothing playing costs nothing and needs no prompt.

## Plan

- [x] Add `DISAMBIGUATE_ESCAPE_CODES` to the pushed keyboard enhancement flags
- [x] Confirm from a trace whether a held Esc reports `Repeat`
- [x] Narrow Esc to `Press` in the panel and browser handlers
- [x] Delete `ESC_PHANTOM_WINDOW`, its sliding-window guard, and the comments asserting Esc sends no Release
- [x] Remove the Esc trace diagnostic
- [x] Verify Esc in the browser search field, tag editor, and playlist edit panel, in Kitty and one non-kitty terminal

## Conclusion

Completed. Verified in Kitty and GNOME Terminal.

The fault was a missing terminal capability flag, not an input-handling problem — so the fix is a flag and a two-line kind check, and the input-normalisation component the Intent proposed was never built. Shipped at 0.15.21.

Documentation impact: the Keymap node still describes the deleted 200 ms debounce and needs catching up. Separately, `keybindings.md` lists quit as "with confirmation", which holds only while a deck is playing — pre-existing, but this change confirmed the behaviour rather than widening it.

## Log

- The duplicate event was Esc's `Release`. `REPORT_EVENT_TYPES` asks Kitty to report releases, but without `DISAMBIGUATE_ESCAPE_CODES` Esc keeps a legacy bare-`\x1b` encoding that carries no event type, so the release arrived as an identical second `Press`.
- The old guard's "two Press events ~100 ms apart" was wrong. The measured gap was the key's hold duration — 56 ms to 376 ms across traces — which is why no fixed window separated it from a deliberate re-press.
- Held Esc traces as `Press`, ~500 ms, then `Repeat` every ~30 ms, then `Release`. The narrowing to `Press` is load-bearing: without it one hold races the whole cascade.
- The Esc kind check uses `continue` rather than the old guard's `continue 'tui`, so the frame's remaining queued events are no longer discarded.
- Narrowing is applied to Esc alone in the event loop, not to the panel and browser `Press | Repeat` gates, which other keys need for held-key scrolling.
