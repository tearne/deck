# Browser Jump to Loaded

**Mode:** Formal

## Intent

When browsing away to find the next track, getting back to where a currently-loaded track lives means navigating there by hand. Add a quick jump: the browser still opens at the last-visited directory, but pressing `` ` `` rotates the browser through the directories of the tracks currently loaded on the decks — landing in each one with that track highlighted — and loops back round to the directory it opened at. With several decks loaded, repeated presses cycle between their locations.


## Approach

### `` ` `` cycles, but the loop owns the rotation

`` ` `` in command mode returns a `CycleLocation` result; the player loop handles it, since only it knows the decks' track paths. In search mode `` ` `` is filter text, so this is command-mode only.

### Locations = the opening directory plus each loaded deck's track

The cycle is `[opening directory]` followed by each deck that has a track loaded, in deck order — each as `(directory, track file)`. A rotation index, reset when the browser opens, advances and wraps on each press. Empty decks contribute nothing; with none loaded the cycle is just the opening directory, so `` ` `` is a no-op. The index simply rotates — manual navigation between presses doesn't reset it.

### Landing highlights the track and names the stop

A browser `go_to(directory, highlight, label)` navigates (preserving mode, target deck, and compliance state), places the cursor on the track's entry, and records a label naming the stop: the opening directory is "Working directory" (no track highlight), each deck stop is "Deck N directory". The browser shows the label prominently at the top of the panel, cleared on the next manual navigation so it reflects a deliberate jump rather than wherever you wander after.


## Plan

- [x] Add `BrowserResult::CycleLocation`; `` ` `` in command mode returns it.
- [x] Add a browser `go_to(dir, highlight: Option<&Path>, label)` that navigates, positions the cursor, and stores the label; navigation clears the label.
- [x] Render the location label prominently at the top of the browser while set.
- [x] In the loop, track a rotation index reset on browser open; on `CycleLocation`, build the location list (opening directory + loaded decks' track dir/file with labels), advance, and `go_to` the next.
- [x] Bump Cargo patch (0.11.33 → 0.11.34).


## Log

- `` ` `` in command mode returns `BrowserResult::CycleLocation`; the loop builds `[opening dir] + loaded decks' (dir, track)`, advances a `location_cycle` index (reset on open, `% len` wrap), and calls `bs.go_to`. `go_to` navigates (via `navigate_to`, preserving mode/target/compliance), highlights the track by path, and stores the label; `navigate_to` clears the label on any manual move, so it only reflects a deliberate jump.
- Label shown as a chip `◈ <label>` at the top-left of the content row (over the mode hint): "Working directory" for home, "Deck N directory" for decks. No decks loaded → cycle is just home, so `` ` `` re-affirms it.
- Hand-back tweaks (0.11.35): chip recoloured from the bold mode-accent to a subtle blue (`fg 170,195,225 / bg 38,50,78`, no bold) so it's less shouty; the label now shows on first open too ("Working directory"), not only after a `` ` `` press.


## Conclusion

Shipped at 0.11.35. `` ` `` in the browser's command mode rotates through the directories of the tracks currently loaded on the decks — highlighting the track in each — and loops back to the working directory it opened at. A subtle blue chip at the top names the stop ("Working directory" / "Deck N directory"), shown from first open and cleared on manual navigation. Empty decks are skipped; with none loaded the cycle is just the working directory.

The player loop owns the rotation (it knows the deck paths) via a new `BrowserResult::CycleLocation` and a `BrowserState::go_to`. Map catch-up pending: a note on the Load Target node (or a small Browser addition) for the `` ` `` cycle.
