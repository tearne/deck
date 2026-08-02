//! One-off converter: re-keys the track database from decoded-PCM hashes to
//! content identities. Removed once the local database has been converted.
//!
//! The database only stores the old PCM hash per entry, so the bridge to each
//! file's content identity is rebuilt by scanning the workspace: every audio
//! file is decoded to recompute its PCM hash, and the ones matching an existing
//! entry are re-emitted under the file's content identity.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicUsize;
use std::sync::Arc;

use crate::audio::decode_audio;
use crate::browser::is_audio;
use crate::cache::{hash_mono, SessionState, TrackDatabase};

pub(crate) fn run() {
    let Some(workspace) = SessionState::load().workspace().map(Path::to_path_buf) else {
        eprintln!("deck: no workspace set — open the app and set one (@) before converting");
        return;
    };

    let mut database = TrackDatabase::load();
    let old_entries = database.entries_snapshot();
    let old_count = old_entries.len();
    println!("Scanning {} for {old_count} tracked entries…", workspace.display());

    let wanted: HashSet<String> = old_entries.keys().cloned().collect();
    let pcm_to_identity = map_pcm_to_identity(&workspace, &wanted);

    let mut rekeyed = HashMap::new();
    for (pcm_hash, entry) in old_entries {
        if let Some(identity) = pcm_to_identity.get(&pcm_hash) {
            rekeyed.insert(identity.clone(), entry);
        }
    }

    let converted = rekeyed.len();
    let dropped = old_count - converted;
    database.overwrite_and_save(rekeyed);
    println!("Converted {converted} entries; dropped {dropped} whose files were not found under the workspace.");
}

/// For each workspace audio file whose PCM hash matches a wanted entry, the
/// file's content identity. Decoding is the cost — the PCM hash is the only
/// reliable bridge from an old key to a file.
fn map_pcm_to_identity(workspace: &Path, wanted: &HashSet<String>) -> HashMap<String, String> {
    let mut mapping = HashMap::new();
    for file in audio_files_under(workspace) {
        let Some(pcm_hash) = pcm_hash_of(&file) else { continue };
        if wanted.contains(&pcm_hash) {
            if let Some(identity) = content_identity_of(&file) {
                mapping.insert(pcm_hash, identity);
            }
        }
    }
    mapping
}

fn pcm_hash_of(file: &Path) -> Option<String> {
    let progress = Arc::new(AtomicUsize::new(0));
    let (mono, _stereo, _rate, _channels) =
        decode_audio(&file.to_string_lossy(), Arc::clone(&progress), progress).ok()?;
    Some(hash_mono(&mono))
}

fn content_identity_of(file: &Path) -> Option<String> {
    let bytes = std::fs::read(file).ok()?;
    resilient_playlists::content_hash(&bytes).ok()
}

fn audio_files_under(root: &Path) -> Vec<PathBuf> {
    let mut found = Vec::new();
    collect_audio_files(root, &mut found);
    found
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
