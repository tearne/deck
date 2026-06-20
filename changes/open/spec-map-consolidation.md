# SPEC / Map Consolidation

**Mode:** Explore

## Intent

`deck` describes itself twice: a conceptual `map.md` and a `SPEC/` directory of detailed specifications. Maintaining both guarantees one gets neglected — and it already has, with SPEC still calling deck a "two-deck" player while the map correctly says three.

We want a single source of conceptual truth. Migrate SPEC's conceptual content into the map, close the map's gaps (notably the missing Tags feature and the dangling Spectrum Analyser link), and retire `SPEC/` entirely. The non-conceptual residue the map was never meant to hold — keybindings, project facts, the acceptance checklist, a thin layout reference, and the portable playlist contract — is rehomed as small standalone documents that, because they don't describe what the map describes, cannot rot against it.


## Approach

### Migration is live per-node, not pre-written here

Map edits are negotiated one node at a time during Build (MAP-GUIDANCE engagement rule). This Approach fixes *destinations*, not node prose — pre-staging node bodies is forbidden.

### Disposition of each SPEC document

| SPEC doc | Destination |
|----------|-------------|
| `architecture.md` | Map — threading folds into Application (or a small node) |
| `audio.md` | Map — Audio Pipeline (level/gain/filter/PFL) + Audio Latency node |
| `browser.md` | Map — Browser subtree (fills the TODO) |
| `cache.md` | Map — Settings subtree (fills the TODO) |
| `config.md` | Keybindings reference (layout, key-string format, config loading, `[display]` params); a conceptual Keymap node points to it |
| `deck.md` | Map — Deck subtree (reconcile with existing nodes) |
| `mixer.md` | Map — Audio Pipeline / Mixer |
| `overview.md` | README — project facts (stack, CLI, versioning); out-of-scope already in map root |
| `render.md` | Split — concepts to map (Detail Waveform subtree etc.); literal layout/colour/encoding tables to the layout reference |
| `tags.md` | Map — new Tags subtree |
| `verification.md` | Acceptance checklist — kept as its own doc |

### Keys live in one reference, never in map nodes

Map nodes name actions conceptually; literal keys go to one keybindings reference (MAP-GUIDANCE forbids mirroring config/literals, and keys churn independently). Migration strips keys from content as it moves.

### Residue homes are emergent; SPEC/ removed once emptied

The table's non-map destinations are categories, not filenames; actual homes (README, topical docs like `building.md`/`testing.md`, a keybindings or layout reference, or merges) are chosen per-doc during Build. The map may grow into a multi-file map; the playlist contract's home is settled in the playlist change. The README is owned by the open `readme.md` change, which draws keybindings from the new reference — so `SPEC/config.md` is deleted only once that reference exists, as is `SPEC/` once every doc has moved.

### Map repairs taken along the way

Create the Spectrum Analyser node (a dangling link from Filter), add Needle Drop, and fix two-vs-three-deck staleness as it surfaces — leaving these is the rot we are removing.


## Plan

**Topics**

- **Deck subtree** — reconcile `deck.md` into the Deck nodes (selection/swap, playback, beat detection, cue, metronome, nudge, beat jump, vinyl mode); add a Needle Drop node.

- **Audio Pipeline & Mixer** — fold `mixer.md` and `audio.md` precision into Filter / Level & Gain / PFL; raise the Audio Latency node from TODO.

- **Browser subtree** — raise from TODO using `browser.md` (navigation, workspace, fuzzy search, preview).

- **Settings subtree** — raise from TODO using `cache.md` (cached state, persistence).

- **Tags subtree** — new nodes for the tag/rename feature from `tags.md`.

- **Rendering** — reconcile `render.md` concepts into the Detail Waveform subtree; add the Spectrum Analyser node; route the literal layout/colour/encoding tables to the layout reference.

- **Threading** — fold `architecture.md` into Application.

- **Keybindings reference** — create it from `config.md`, mapping each config-action name (referenced by the map) to its key.

- **Layout reference** — create it from `render.md`'s literal tables (section ordering, info-bar layout, notification colour schemes).

- **Project facts** — route `overview.md` (stack, CLI, versioning) to the README via the `readme.md` change.

- **Verification checklist** — rehome `verification.md`.

- **Retire SPEC/** — delete the directory once every doc has moved.

**Done when** every `SPEC/` doc has migrated into the map or rehomed to a residue doc, `map.md` has no dangling links and no two-vs-three-deck staleness, and `SPEC/` is gone.


## Log

- Key-handling policy (governs the whole migration): map nodes keep stable config-action identifiers (e.g. `bpm_tap`, `redetect_bpm`) as the cross-reference join to the keybindings reference; literal keys never enter the map — they live only in the reference, keyed by action name. When folding SPEC content written in literal keys, translate to action names. (Reverses an initial "strip identifiers" idea — the identifier is the join, not noise.)

- Cross-reference convention: a node that references config-action names carries `**See also** → [Keymap](#keymap)`; the Keymap node holds the single outward link to the keybindings reference / project config file. Indirection through the hub, not per-node file links, so a rename is fixed in one place.

- Deck subtree reconciled from `deck.md` — new nodes Deck Selection, Track Loading, Needle Drop; folds into Beat Grid (constant-tempo), Transport (end-of-track), Mode (vinyl suppresses analysis), Metronome (activation beat). Its render/UI residue (BPM spinner, confirmation countdown, metadata display, vinyl % readout, cue/tick markers) goes to the layout reference in the Rendering topic — so `deck.md` is not deletable until then.

- PFL cross-checked against `src/audio/mod.rs` + `src/main.rs` + `src/config/mod.rs`: `audio.md` is **stale** (claims per-deck `Space+x`/`Space+v` PFL keys); code and `mixer.md` agree PFL acts on the **selected** deck via un-suffixed actions (`pfl_on_off`, `pfl_up/down/reset`), whereas level/gain/filter are per-deck (`deckN_*`). New PFL Monitor node under Mixer; corrected the Deck Selection node, which had wrongly listed PFL among the directly-addressed controls.

- Consistency sweep owed before wrap, two conventions: (1) every node that names config-actions carries the `[Keymap](#keymap)` See-also; (2) every node describing persisted state carries a `[Cache](#cache)` See-also (the on-disk cache is described there). Nodes predating these (Speed Control, Cue Point, Mode, Level & Gain gain, Beat Grid, Audio Latency) still need one or both.

- Settings node renamed to **Cache** (single node) and built from `cache.md`. Cross-check vs `src/cache/mod.rs`: code also persists `art_bright_idx` (cover-art brightness) and per-track `offset_established`, neither in `cache.md`. Updated the two `[Settings]` See-alsos to `[Cache]`. `cache.md` is fully conceptually migrated (no render residue) — a deletion candidate once SPEC retires.

- Tags cross-check: `tags.md`'s `h`-opens-editor is **stale** (no `h` binding; in-code comment also stale) — the editor is reachable only via the rename offer's `y`. Tag/editor keys are hardcoded, not config actions → keybindings reference needs a "fixed keys" section. The discovered gaps (no on-demand invocation; `y` hardcoded) are parked as a new proposal `tag-editor-invocation.md`, out of scope here.

- Tags subtree built under Deck: Renaming (parent) + Metadata Editor (child, renamed from "Tag Editor" — rename is the motivation, metadata is the tool). `tags.md` residue: fixed keys → keybindings reference "fixed keys" section; rename-offer styling + modal rendering → layout reference; `rename_roundtrip` test → verification checklist. Not deletable until those land.

- Threading folded into the Application root (not a separate node — cross-cutting infra, not a feature box). `architecture.md` fully migrated, no residue — a clean deletion candidate.

- **Resume pointer (as of pause before Rendering):** 6 topics complete — Deck subtree, Audio Pipeline & Mixer, Browser, Cache, Tags, Threading. **Next: Rendering**, then Keybindings reference + Keymap node, Layout reference, Project facts → README, Verification checklist, the consistency sweep, and finally Retire SPEC/. No node is mid-edit; safe interruption point.

- Browser subtree built from `browser.md` (Browser parent + Search, Preview). Verified against code: workspace prompt/key is `@` (not the SPEC's `~`), preview starts at 20%/30 s fallback, load targets the selected deck, playing-deck open is guarded. `browser.md` residue: its key table → keybindings reference; browser-title format and directory styling → layout reference. Not deletable until those exist.

- Audio Latency cross-checked against `src/main.rs`: `audio.md`'s claim that latency adjustment "compensates `offset_ms` by the opposite amount" is **stale** — the handlers only change `audio_latency_ms` and save. Also confirmed compensation applies during playback only (zeroed when paused) and cue-play offsets its target by the latency. Built the Audio Latency node from TODO without the fictional offset-compensation.

- Audio Pipeline & Mixer topic done (PFL Monitor added; Filter slope, Level & Gain, Pitch Shift folded; Audio Latency built). `mixer.md`/`audio.md` conceptual content is migrated, but their render residue (info-bar gain glyph, PFL cyan readout, spectrum/filter shading) goes to the layout reference + Spectrum Analyser node in the Rendering topic — so neither is deletable until then.
