# Key Hint Overload

**Mode:** Formal

## Intent

Keep relevant key hints visible and readable for the current keyboard context, with enough space for current state and active inputs. Minimising hints is a possible future change.

## Approach

Replace the separate pane instruction footers with one full-width hint area at the bottom of the screen. Label it with the current keyboard context, such as Browser · Search or Tags · Editing.

Group hints by task: navigation, actions, and switching panes or focus. Wrap between complete key-and-description pairs, adding rows as needed. Keep the group order consistent. Reserve the required height before laying out the content; on very short terminals, preserve active inputs and confirm/cancel controls first.

Choose hints from the actual input context: browser command, search, move or playlist naming; playlist editing or candidate selection; tag editing; messages; or deck focus. Temporary tasks take precedence. Show applicable controls, including commands missing from today's footers. Use configured bindings for configurable actions.

Keep pane names and local state with their content. Give browser mode, workspace, tag-warning count and location a dedicated status line; keep search and playlist-name inputs there, with room to wrap. Put their key instructions in the shared hint area.

When focus returns to the decks, show a compact set of view/focus controls and the key for the existing keyboard reference. The reference remains the full deck-control guide.

Check each context at wide, paired-pane and single-pane widths, plus short heights. Review the rendered layout together before concluding.


## Worklist

- [x] Inventory the controls available in each input context, including temporary prompts and configured bindings.
- [x] Build the shared hint area with grouped controls, wrapping and height allocation.
- [x] Replace pane instruction footers; retain local state and active inputs beside their content.
- [x] Connect context selection to focus, mode and temporary tasks, including messages and deck focus.
- [x] Verify hint selection and layout across wide, paired-pane, single-pane and short terminals; run relevant checks.
- [x] Review the rendered result with the user, update affected documentation, and conclude after acceptance.

## Conclusion

Pane-switch hints returned to the headers. Browser mode moved beside its chip; search and move inputs moved to the top. Empty Tags panes cannot take focus. Duplicate hints were removed from the main keyboard reference.

## Log

- 2026-09-15: Intent, Approach and Worklist approved; Build started. Existing working-tree changes belong to earlier work and are preserved.
- 2026-09-15: Shared contextual hints implemented; browser state and wrapping input retained inside its frame. Layout tests cover all hint contexts, configured bindings, narrow/short screens and browser input visibility. Workspace tests: 66 passed, two existing ignored. keybindings.md updated. Visual acceptance and map catch-up (Bottom Pane / Browser) remain; build lock retained. Version-bump choice requested and pending.
- Final short-screen check: tag editor now scrolls to keep the active cursor or rename toggle visible as hints grow; regression test covers every field. Release build passes; interactive visual acceptance is still pending. Rendered browser previews: `/tmp/deck-hints-preview.txt`.
- User clarified that approval covers version 0.38.0 only; visual review has not happened. Applying the approved version and rebuilding for testing.
- Release 0.38.0 built successfully for user testing (`target/release/deck`); visual review remains pending.
- Testing feedback: omit empty hint groups, including the bare “Decks:” row when no view can take focus. Patch bumped to 0.38.1 for hand-back.
- Testing feedback for 0.38.2: restored configured pane-switch hints to destination headers and removed them from the shared area, including the tag-editor duplicate. Omitted redundant arrow alternatives while retaining arrows needed for search and text editing.
- Testing feedback for 0.38.3: pane-specific hint groups now reuse the existing browser mode colours, playlist green and tags violet, including their actions/editing groups. Candidate/confirmation hints use attention amber; shared switching and view controls remain neutral.
- Testing feedback for 0.38.4: pane-right keeps browser focus and layout unchanged for folders, playlists, empty selections and unreadable tracks. Tags focus is entered only after its editor opens successfully; the Tags header is quiet and has no navigation hint without editable track metadata.
- Testing feedback for 0.38.5: removed dot separators from pane/context labels (for example, “Browser Command”); separators between individual key hints remain.
- Testing feedback for 0.38.6: renamed the browser command-mode navigation group to “Navigate”, alongside “Actions”; pane ownership is conveyed by colour.
- Testing feedback for 0.38.7: audited every hint-group label. Groups now consistently describe controls (Navigate, Actions, Edit, Switch, Views), with pane identity supplied by colour. Search, naming and candidate controls are grouped accordingly; explicit confirmation questions retain their task wording.
- Testing feedback for 0.38.8: moved persistent browser status into its bottom border so it cannot be mistaken for a file. Only active input or a move destination reserves list-body space, with a distinct background.
- Testing feedback for 0.38.9: removed redundant “workspace set” from browser status; the top-border `@` and workspace path already carry that information.
- Testing feedback for 0.38.10: removed mode from the bottom border. Command leaves the Browser chip bare; SEARCH and MOVE appear beside the chip. Empty bottom-border status no longer renders a blank title.
- Testing feedback for 0.38.11: moved the search input and move destination above the file list, inside the top of the browser frame. Playlist naming uses the same input area.
- Approved keyboard-reference cleanup for 0.38.12: removed pane-switch/width hints, Tab focus and Brows/Help/Art/Msgs cells from the main keyboard diagram and its documentation. Deck selection, restore, performance controls and art brightness remain. Bindings are unchanged.
- User accepted the completed build at 0.38.12. Remaining closeout: Bottom Pane and Browser map updates, then Conclude approval and archive.
- Conclusion explicitly approved. Map tidy checked both project maps: no unresolved links; 29 nodes exceed 800 characters (no approver exemptions). No map edits made. Bottom Pane and Browser map catch-up remains unapproved for a subsequent Map Review; keybindings.md is current. Archived at 0.38.12.
