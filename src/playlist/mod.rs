//! Deck's implementation of the resilient playlist format (`.rpl`) — everything
//! in the format map except the content-identity hash, which comes from the
//! `resilient-playlists` crate. This is a pure engine: data model, file
//! resolution, and resilient writes. No UI; the playlist editor wires it in.
//!
//! Consumed by the editor (a later change); dead in the binary until then.
#![allow(dead_code)]

use std::path::{Path, PathBuf};

use resilient_playlists::{content_hash, HASH_ALGORITHM, PAYLOAD_EXTRACTION_VERSION};
use serde::{Deserialize, Serialize};

/// A candidate whose duration is within this many seconds of the entry's is
/// eligible for hash confirmation (map: File Resolution).
const DURATION_TOLERANCE_SECS: f64 = 2.0;
/// Allowed file-size deviation for a candidate, as a fraction.
const FILE_SIZE_TOLERANCE: f64 = 0.01;

// ---- Data model (map: Entry Structure) ----

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Identity {
    pub hash_algorithm: String,
    pub payload_extraction_version: u32,
    pub content_hash: String,
    pub duration_secs: f64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct Description {
    #[serde(default)]
    pub artist: String,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub album: String,
    #[serde(default)]
    pub year: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Hints {
    pub relative_path: String,
    pub file_size_bytes: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Entry {
    pub identity: Identity,
    pub description: Description,
    pub hints: Hints,
    /// Reserved by the spec, currently always empty. Held as raw JSON so any
    /// future content round-trips untouched.
    #[serde(default = "empty_object")]
    pub settings: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Playlist {
    pub version: u32,
    pub entries: Vec<Entry>,
}

fn empty_object() -> serde_json::Value {
    serde_json::Value::Object(serde_json::Map::new())
}

impl Playlist {
    pub fn empty() -> Self {
        Playlist { version: 1, entries: Vec::new() }
    }
}

// ---- Building an entry from a located track (map: Entry Structure, Identity) ----

/// The content-derived and tag-derived parts of a track, supplied by the caller
/// (Deck decodes for duration and reads tags; the filesystem gives size).
pub struct TrackFacts {
    pub bytes: Vec<u8>,
    pub duration_secs: f64,
    pub file_size_bytes: u64,
    pub description: Description,
}

/// Build a fresh entry for a track at `path`, relative to `playlist_dir`, stamped
/// with the current hashing method.
pub fn entry_from_track(
    path: &Path,
    playlist_dir: &Path,
    facts: &TrackFacts,
) -> Result<Entry, resilient_playlists::IdentityError> {
    Ok(Entry {
        identity: Identity {
            hash_algorithm: HASH_ALGORITHM.to_string(),
            payload_extraction_version: PAYLOAD_EXTRACTION_VERSION,
            content_hash: content_hash(&facts.bytes)?,
            duration_secs: facts.duration_secs,
        },
        description: facts.description.clone(),
        hints: Hints {
            relative_path: relative_to(playlist_dir, path),
            file_size_bytes: facts.file_size_bytes,
        },
        settings: empty_object(),
    })
}

// ---- Injected access (map: File Resolution needs library + tags) ----

/// The library and file access resolution needs, injected so the engine stays
/// Deck-independent and unit-testable. Deck supplies the real implementation
/// over its `@` workspace; tests supply fakes.
pub trait Library {
    /// Every audio file under the library root.
    fn candidates(&self) -> Vec<PathBuf>;
    /// Cheap metadata for pre-filtering — duration and size without decoding.
    /// `None` if the file is missing or unreadable.
    fn cheap_probe(&self, path: &Path) -> Option<(f64, u64)>;
    /// Full file bytes, for the hash-confirm step. `None` if unreadable.
    fn read_bytes(&self, path: &Path) -> Option<Vec<u8>>;
    /// Current tags as a description, for tag-refresh and candidate display.
    fn read_description(&self, path: &Path) -> Option<Description>;
}

// ---- Resolution (map: File Resolution, Descriptive Fallback, Tags Refresh) ----

#[derive(Debug, Clone, PartialEq)]
pub struct Candidate {
    pub path: PathBuf,
    pub description: Description,
    pub duration_secs: f64,
    pub file_size_bytes: u64,
}

#[derive(Debug, PartialEq)]
pub enum Resolution {
    /// File located (hint or search). `updated_entry` is `Some` when hints or
    /// description changed and the caller should persist.
    Found { path: PathBuf, updated_entry: Option<Entry> },
    /// No hash match; the descriptive fallback offers these for confirmation.
    NeedsConfirmation { candidates: Vec<Candidate> },
    /// No match and nothing to confirm — kept, shown unavailable.
    Unavailable,
}

/// Locate the file for `entry`, per the map's File Resolution steps. Read-only:
/// it never writes, returning any updated entry for the caller to persist.
pub fn resolve(entry: &Entry, playlist_dir: &Path, library: &dyn Library) -> Resolution {
    // Step 1: the hinted path, confirmed by hash.
    let hinted = playlist_dir.join(&entry.hints.relative_path);
    if entry_matches_file(entry, &hinted, library) {
        let updated = refresh_description(entry, &hinted, library);
        return Resolution::Found { path: hinted, updated_entry: updated };
    }

    // Step 2: search the library, cheapest-first, then confirm by hash.
    for path in library.candidates() {
        let Some((duration, size)) = library.cheap_probe(&path) else { continue };
        if !within_tolerance(entry, duration, size) {
            continue;
        }
        if entry_matches_file(entry, &path, library) {
            let mut updated = entry.clone();
            updated.hints.relative_path = relative_to(playlist_dir, &path);
            updated.hints.file_size_bytes = size;
            if let Some(desc) = changed_description(&updated, &path, library) {
                updated.description = desc;
            }
            return Resolution::Found { path, updated_entry: Some(updated) };
        }
    }

    // Step 3: descriptive fallback — rank by description similarity.
    let candidates = descriptive_candidates(entry, library);
    if candidates.is_empty() {
        Resolution::Unavailable
    } else {
        Resolution::NeedsConfirmation { candidates }
    }
}

/// Whether `entry`'s stored hash confirms the file at `path`. Only attempted
/// when the entry was made by a method this build can reproduce; otherwise it
/// is unresolvable by hash (map: Identity, Method Migration) and returns false.
fn entry_matches_file(entry: &Entry, path: &Path, library: &dyn Library) -> bool {
    if entry.identity.hash_algorithm != HASH_ALGORITHM
        || entry.identity.payload_extraction_version != PAYLOAD_EXTRACTION_VERSION
    {
        return false;
    }
    let Some(bytes) = library.read_bytes(path) else { return false };
    match content_hash(&bytes) {
        Ok(h) => h == entry.identity.content_hash,
        Err(_) => false,
    }
}

fn within_tolerance(entry: &Entry, duration: f64, size: u64) -> bool {
    let duration_ok = (duration - entry.identity.duration_secs).abs() <= DURATION_TOLERANCE_SECS;
    let want = entry.hints.file_size_bytes as f64;
    let size_ok = want == 0.0 || (size as f64 - want).abs() <= want * FILE_SIZE_TOLERANCE;
    duration_ok && size_ok
}

/// A description refresh for a *located* entry (map: Tags Refresh), or `None` if
/// unchanged. Returns a full updated entry so step 1 can persist a pure refresh.
fn refresh_description(entry: &Entry, path: &Path, library: &dyn Library) -> Option<Entry> {
    changed_description(entry, path, library).map(|desc| {
        let mut updated = entry.clone();
        updated.description = desc;
        updated
    })
}

fn changed_description(entry: &Entry, path: &Path, library: &dyn Library) -> Option<Description> {
    match library.read_description(path) {
        Some(desc) if desc != entry.description => Some(desc),
        _ => None,
    }
}

/// Library files similar to the entry in *both* duration and description, for the
/// fallback: filtered to within the duration tolerance, ranked by description-field
/// matches, then by closest duration.
fn descriptive_candidates(entry: &Entry, library: &dyn Library) -> Vec<Candidate> {
    let target = entry.identity.duration_secs;
    let mut scored: Vec<(u32, f64, Candidate)> = library
        .candidates()
        .into_iter()
        .filter_map(|path| {
            let (duration, size) = library.cheap_probe(&path)?;
            let duration_delta = (duration - target).abs();
            if duration_delta > DURATION_TOLERANCE_SECS {
                return None;
            }
            let description = library.read_description(&path)?;
            let score = description_similarity(&entry.description, &description);
            (score > 0).then_some((score, duration_delta, Candidate { path, description, duration_secs: duration, file_size_bytes: size }))
        })
        .collect();
    scored.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal)));
    scored.into_iter().map(|(_, _, c)| c).collect()
}

/// A deliberately simple similarity: count of matching non-empty fields. The map
/// leaves ranking implementation-defined.
fn description_similarity(a: &Description, b: &Description) -> u32 {
    let field = |x: &str, y: &str| (!x.is_empty() && x.eq_ignore_ascii_case(y)) as u32;
    field(&a.artist, &b.artist)
        + field(&a.title, &b.title)
        + field(&a.album, &b.album)
        + field(&a.year, &b.year)
}

/// Adopt a confirmed descriptive-fallback candidate: overwrite identity from the
/// new file (the one sanctioned identity mutation), rewrite hints, refresh
/// description. The caller has already obtained user confirmation.
pub fn adopt_candidate(
    entry: &mut Entry,
    path: &Path,
    playlist_dir: &Path,
    facts: &TrackFacts,
) -> Result<(), resilient_playlists::IdentityError> {
    entry.identity = Identity {
        hash_algorithm: HASH_ALGORITHM.to_string(),
        payload_extraction_version: PAYLOAD_EXTRACTION_VERSION,
        content_hash: content_hash(&facts.bytes)?,
        duration_secs: facts.duration_secs,
    };
    entry.description = facts.description.clone();
    entry.hints = Hints {
        relative_path: relative_to(playlist_dir, path),
        file_size_bytes: facts.file_size_bytes,
    };
    Ok(())
}

// ---- Relative paths ----

/// Express `target` relative to `base_dir` using `../` as needed. Both are
/// treated as-is (callers pass canonical absolute paths); falls back to the
/// target's string form when no relation can be computed.
pub(crate) fn relative_to(base_dir: &Path, target: &Path) -> String {
    use std::path::Component;
    let base: Vec<Component> = base_dir.components().collect();
    let targ: Vec<Component> = target.components().collect();
    let common = base.iter().zip(&targ).take_while(|(a, b)| a == b).count();
    if common == 0 {
        return target.to_string_lossy().into_owned();
    }
    let ups = base.len() - common;
    let mut rel = PathBuf::new();
    for _ in 0..ups {
        rel.push("..");
    }
    for c in &targ[common..] {
        rel.push(c.as_os_str());
    }
    rel.to_string_lossy().into_owned()
}

// ---- Resilient writes (map: Resilient Writes, Backup Scheme) ----

/// Write `playlist` to `path` per the map's Write Procedure: serialise, validate
/// by re-parse, rotate backups, then atomically rename a same-directory temp
/// file over the primary.
pub fn write_playlist(path: &Path, playlist: &Playlist) -> std::io::Result<()> {
    let json = serde_json::to_vec_pretty(playlist)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
    if serde_json::from_slice::<Playlist>(&json).is_err() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "serialised playlist did not re-parse",
        ));
    }

    let dir = path.parent().unwrap_or_else(|| Path::new("."));
    let tmp = dir.join(format!(
        ".{}.tmp",
        path.file_name().unwrap_or_default().to_string_lossy()
    ));
    std::fs::write(&tmp, &json)?;

    rotate_backups(path);
    std::fs::rename(&tmp, path)
}

/// Rotate `.bak1`→`.bak2`→`.bak3` (dropping the oldest) and the current primary
/// into `.bak1`. Missing files are skipped.
fn rotate_backups(path: &Path) {
    let _ = std::fs::rename(backup_path(path, 2), backup_path(path, 3));
    let _ = std::fs::rename(backup_path(path, 1), backup_path(path, 2));
    if path.exists() {
        let _ = std::fs::copy(path, backup_path(path, 1));
    }
}

/// Hidden sibling backup path `.<name>.bakN`.
fn backup_path(path: &Path, slot: u8) -> PathBuf {
    let name = path.file_name().unwrap_or_default().to_string_lossy();
    path.with_file_name(format!(".{name}.bak{slot}"))
}

/// Read a playlist, falling back through the backups in slot order when the
/// primary won't parse. Returns the parsed playlist and whether a backup was
/// used (so the caller can surface the recovery).
pub fn read_playlist(path: &Path) -> std::io::Result<(Playlist, bool)> {
    if let Ok(bytes) = std::fs::read(path) {
        if let Ok(pl) = serde_json::from_slice::<Playlist>(&bytes) {
            return Ok((pl, false));
        }
    }
    for slot in 1..=3 {
        if let Ok(bytes) = std::fs::read(backup_path(path, slot)) {
            if let Ok(pl) = serde_json::from_slice::<Playlist>(&bytes) {
                return Ok((pl, true));
            }
        }
    }
    Err(std::io::Error::new(
        std::io::ErrorKind::InvalidData,
        "playlist and all backups failed to parse",
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn wav(payload: &[u8]) -> Vec<u8> {
        let mut f = Vec::new();
        f.extend_from_slice(b"RIFF");
        f.extend_from_slice(&(36u32 + payload.len() as u32).to_le_bytes());
        f.extend_from_slice(b"WAVE");
        f.extend_from_slice(b"fmt ");
        f.extend_from_slice(&16u32.to_le_bytes());
        f.extend_from_slice(&1u16.to_le_bytes());
        f.extend_from_slice(&1u16.to_le_bytes());
        f.extend_from_slice(&44100u32.to_le_bytes());
        f.extend_from_slice(&88200u32.to_le_bytes());
        f.extend_from_slice(&2u16.to_le_bytes());
        f.extend_from_slice(&16u16.to_le_bytes());
        f.extend_from_slice(b"data");
        f.extend_from_slice(&(payload.len() as u32).to_le_bytes());
        f.extend_from_slice(payload);
        f
    }

    #[derive(Default)]
    struct FakeLibrary {
        files: HashMap<PathBuf, (Vec<u8>, f64, Description)>,
    }
    impl FakeLibrary {
        fn add(&mut self, path: &str, bytes: Vec<u8>, duration: f64, desc: Description) {
            self.files.insert(PathBuf::from(path), (bytes, duration, desc));
        }
    }
    impl Library for FakeLibrary {
        fn candidates(&self) -> Vec<PathBuf> {
            self.files.keys().cloned().collect()
        }
        fn cheap_probe(&self, path: &Path) -> Option<(f64, u64)> {
            self.files.get(path).map(|(b, d, _)| (*d, b.len() as u64))
        }
        fn read_bytes(&self, path: &Path) -> Option<Vec<u8>> {
            self.files.get(path).map(|(b, _, _)| b.clone())
        }
        fn read_description(&self, path: &Path) -> Option<Description> {
            self.files.get(path).map(|(_, _, d)| d.clone())
        }
    }

    fn desc(artist: &str, title: &str) -> Description {
        Description { artist: artist.into(), title: title.into(), ..Default::default() }
    }

    fn entry_for(bytes: &[u8], duration: f64, size: u64, rel: &str, description: Description) -> Entry {
        Entry {
            identity: Identity {
                hash_algorithm: HASH_ALGORITHM.into(),
                payload_extraction_version: PAYLOAD_EXTRACTION_VERSION,
                content_hash: content_hash(bytes).unwrap(),
                duration_secs: duration,
            },
            description,
            hints: Hints { relative_path: rel.into(), file_size_bytes: size },
            settings: empty_object(),
        }
    }

    #[test]
    fn rpl_round_trips_and_tolerates_unknown_fields() {
        let pl = Playlist {
            version: 1,
            entries: vec![entry_for(&wav(&[1, 2, 3]), 3.0, 100, "a.wav", desc("A", "T"))],
        };
        let json = serde_json::to_string(&pl).unwrap();
        assert_eq!(serde_json::from_str::<Playlist>(&json).unwrap(), pl);

        // An entry carrying a future unknown field still parses.
        let with_extra = json.replacen("\"settings\":{}", "\"settings\":{},\"future\":42", 1);
        assert!(serde_json::from_str::<Playlist>(&with_extra).is_ok());
    }

    #[test]
    fn resolves_via_hint_when_file_present() {
        let bytes = wav(&[10, 20, 30]);
        let entry = entry_for(&bytes, 5.0, bytes.len() as u64, "song.wav", desc("A", "T"));
        let mut lib = FakeLibrary::default();
        lib.add("/lib/song.wav", bytes, 5.0, desc("A", "T"));
        // playlist_dir is /lib so the hint resolves directly.
        match resolve(&entry, Path::new("/lib"), &lib) {
            Resolution::Found { path, updated_entry } => {
                assert_eq!(path, PathBuf::from("/lib/song.wav"));
                assert_eq!(updated_entry, None); // exact hit, tags unchanged
            }
            other => panic!("expected Found, got {other:?}"),
        }
    }

    #[test]
    fn relocates_via_library_search_and_rewrites_hints() {
        let bytes = wav(&[7, 7, 7, 7]);
        // Hint points at a stale path that isn't in the library.
        let entry = entry_for(&bytes, 8.0, bytes.len() as u64, "old/moved.wav", desc("A", "T"));
        let mut lib = FakeLibrary::default();
        lib.add("/lib/new/moved.wav", bytes, 8.0, desc("A", "T"));
        match resolve(&entry, Path::new("/lib"), &lib) {
            Resolution::Found { path, updated_entry } => {
                assert_eq!(path, PathBuf::from("/lib/new/moved.wav"));
                let e = updated_entry.expect("relocation should update hints");
                assert_eq!(e.hints.relative_path, "new/moved.wav");
            }
            other => panic!("expected Found, got {other:?}"),
        }
    }

    #[test]
    fn refreshes_description_when_tags_differ() {
        let bytes = wav(&[3, 1, 4]);
        let entry = entry_for(&bytes, 2.0, bytes.len() as u64, "t.wav", desc("Old", "Name"));
        let mut lib = FakeLibrary::default();
        lib.add("/lib/t.wav", bytes, 2.0, desc("New", "Name"));
        match resolve(&entry, Path::new("/lib"), &lib) {
            Resolution::Found { updated_entry: Some(e), .. } => {
                assert_eq!(e.description, desc("New", "Name"));
            }
            other => panic!("expected refreshed description, got {other:?}"),
        }
    }

    #[test]
    fn descriptive_fallback_when_no_hash_match() {
        let entry = entry_for(&wav(&[1]), 100.0, 999, "gone.wav", desc("Artist", "Song"));
        let mut lib = FakeLibrary::default();
        // A re-encode: different bytes (so no hash match) but same description.
        lib.add("/lib/reencoded.wav", wav(&[9, 9, 9, 9, 9]), 100.5, desc("Artist", "Song"));
        match resolve(&entry, Path::new("/lib"), &lib) {
            Resolution::NeedsConfirmation { candidates } => {
                assert_eq!(candidates.len(), 1);
                assert_eq!(candidates[0].path, PathBuf::from("/lib/reencoded.wav"));
            }
            other => panic!("expected NeedsConfirmation, got {other:?}"),
        }
    }

    #[test]
    fn unavailable_when_nothing_matches() {
        let entry = entry_for(&wav(&[1]), 100.0, 999, "gone.wav", desc("Artist", "Song"));
        let lib = FakeLibrary::default();
        assert_eq!(resolve(&entry, Path::new("/lib"), &lib), Resolution::Unavailable);
    }

    #[test]
    fn unreproducible_version_is_unresolvable_by_hash() {
        let bytes = wav(&[5, 5, 5]);
        let mut entry = entry_for(&bytes, 4.0, bytes.len() as u64, "v.wav", desc("A", "T"));
        entry.identity.payload_extraction_version = 999; // a method this build can't reproduce
        let mut lib = FakeLibrary::default();
        lib.add("/lib/v.wav", bytes, 4.0, desc("A", "T"));
        // The file is right there, but the version mismatch blocks hash confirm,
        // so it degrades to the descriptive fallback rather than a silent match.
        match resolve(&entry, Path::new("/lib"), &lib) {
            Resolution::NeedsConfirmation { .. } => {}
            other => panic!("expected fallback, got {other:?}"),
        }
    }

    #[test]
    fn resilient_write_rotates_backups_and_reads_back() {
        let dir = std::env::temp_dir().join(format!("deck-rpl-{}", std::process::id()));
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join("set.rpl");

        let v1 = Playlist { version: 1, entries: vec![] };
        write_playlist(&path, &v1).unwrap();
        let v2 = Playlist {
            version: 1,
            entries: vec![entry_for(&wav(&[1]), 1.0, 50, "x.wav", desc("A", "B"))],
        };
        write_playlist(&path, &v2).unwrap();

        // Primary reads back as the latest; a backup now holds the prior version.
        let (read, from_backup) = read_playlist(&path).unwrap();
        assert_eq!(read, v2);
        assert!(!from_backup);
        assert!(backup_path(&path, 1).exists());

        // Corrupt the primary: recovery falls through to the backup.
        std::fs::write(&path, b"{ not json").unwrap();
        let (recovered, from_backup) = read_playlist(&path).unwrap();
        assert!(from_backup);
        assert_eq!(recovered, v1);

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn relative_to_computes_dotdot_paths() {
        assert_eq!(relative_to(Path::new("/a/b"), Path::new("/a/b/c/t.wav")), "c/t.wav");
        assert_eq!(relative_to(Path::new("/a/b"), Path::new("/a/x/t.wav")), "../x/t.wav");
    }
}
