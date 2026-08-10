//! Deck's music library over the `@` workspace, supplying the playlist engine's
//! resolution access ([`playlist::Library`]): the candidate files to search and
//! the cheap/full reads it needs to confirm a match.
//!
//! With no workspace set there are no candidates, so resolution falls back to the
//! hinted path alone — the operator sets a workspace to enable relocation.

use std::path::{Path, PathBuf};

use crate::audio::probe_duration_secs;
use crate::browser::is_audio;
use crate::playlist::{Description, Library};
use crate::tags::read_tags_for_editor;

pub(crate) struct WorkspaceLibrary {
    root: Option<PathBuf>,
}

impl WorkspaceLibrary {
    pub(crate) fn new(workspace: Option<&Path>) -> Self {
        Self { root: workspace.map(Path::to_path_buf) }
    }
}

impl Library for WorkspaceLibrary {
    fn candidates(&self) -> Vec<PathBuf> {
        let mut found = Vec::new();
        if let Some(root) = &self.root {
            collect_audio_files(root, &mut found);
        }
        found
    }

    fn cheap_probe(&self, path: &Path) -> Option<(f64, u64)> {
        let size = std::fs::metadata(path).ok()?.len();
        let duration = probe_duration_secs(path)?;
        Some((duration, size))
    }

    fn read_bytes(&self, path: &Path) -> Option<Vec<u8>> {
        std::fs::read(path).ok()
    }

    fn read_description(&self, path: &Path) -> Option<Description> {
        let [artist, title, album, year, ..] = read_tags_for_editor(path)?;
        Some(Description { artist, title, album, year })
    }
}

fn collect_audio_files(dir: &Path, found: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else { return };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_audio_files(&path, found);
        } else if is_audio(&path) {
            found.push(path);
        }
    }
}
