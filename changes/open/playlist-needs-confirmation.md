# Playlist Needs-Confirmation Picker

**Mode:** Formal

## Intent

*(Parked — spun out of [[playlist-editor]].)*

The playlist engine's `resolve` can return `NeedsConfirmation { candidates }` — the descriptive fallback — when a track's file has both moved and had its audio changed (re-encoded/transcoded), so its content-identity hash no longer matches, yet a workspace file still matches closely by description and duration/size. The engine offers ranked candidates rather than guessing.

Deck currently treats that outcome as unavailable and never surfaces the candidates. Present the ranked candidates (path, tags, duration) for the operator to confirm one — the engine's `adopt_candidate` then updates the entry — or reject, leaving it unavailable.

Niche: a normal move (audio unchanged) resolves by hash via the workspace heal; this only triggers when the audio itself also changed. Until built, such an entry shows unavailable and can be re-added in the context-panel editor.

Also fold in the minor cosmetic tweaks to the context panel flagged during [[playlist-editor]] (to be specified).
