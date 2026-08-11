//! Deck's implementation of the resilient playlist format (`.rpl`) — everything
//! in the format map except the content-identity hash, which comes from the
//! `resilient-playlists` crate. This is a pure engine: data model, file
//! resolution, and resilient writes. No UI; the playlist editor wires it in.
//!
//! Consumed by the editor (a later change); dead in the binary until then.
#![allow(dead_code)]

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use resilient_playlists::{content_hash, HASH_ALGORITHM, PAYLOAD_EXTRACTION_VERSION};
use serde::{Deserialize, Serialize};

/// A candidate whose duration is within this many seconds of the entry's is
/// eligible for hash confirmation (map: File Resolution).
const DURATION_TOLERANCE_SECS: f64 = 2.0;
/// Allowed file-size deviation for a candidate, as a fraction.
const FILE_SIZE_TOLERANCE: f64 = 0.01;
/// Most candidates the descriptive fallback offers. If neither description nor
/// duration puts the right file in the top few, a longer list won't help.
const MAX_FALLBACK_CANDIDATES: usize = 10;

// ---- Data model (map: Entry Structure) ----

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Identity {
    pub hash_algorithm: String,
    pub payload_extraction_version: u32,
    pub content_hash: String,
    #[serde(serialize_with = "write_duration_rounded")]
    pub duration_secs: f64,
}

/// Duration is only ever compared within the ±2 s resolution tolerance, never for
/// equality (format map: Identity), so full f64 precision — `198.86666666666667` —
/// is pure noise in a file meant to be read and hand-edited. Two decimal places sit
/// well inside the spec's bound and are finer than anything displayed.
fn write_duration_rounded<S: serde::Serializer>(secs: &f64, serializer: S) -> Result<S::Ok, S::Error> {
    serializer.serialize_f64((secs * 100.0).round() / 100.0)
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

/// The library screened once, ready to resolve many entries against. Holds only
/// what screening needs; descriptions cost a tag read each and are fetched later,
/// for the few candidates the fallback actually ranks.
pub struct LibrarySnapshot {
    files: Vec<(PathBuf, f64, u64)>,
}

impl LibrarySnapshot {
    /// Walk and probe the library. Costly in proportion to library size, which is
    /// why callers take one snapshot per operation and resolve every entry against
    /// it rather than repeating this per entry.
    pub fn probe(library: &dyn Library) -> Self {
        let files = library
            .candidates()
            .into_iter()
            .filter_map(|path| {
                let (duration, size) = library.cheap_probe(&path)?;
                Some((path, duration, size))
            })
            .collect();
        LibrarySnapshot { files }
    }
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

/// Locate the file for `entry`, per the map's File Resolution steps, searching the
/// already-probed `snapshot`. Read-only: it never writes, returning any updated
/// entry for the caller to persist.
pub fn resolve(
    entry: &Entry,
    playlist_dir: &Path,
    library: &dyn Library,
    snapshot: &LibrarySnapshot,
) -> Resolution {
    // Step 1: the hinted path, confirmed by hash.
    let hinted = playlist_dir.join(&entry.hints.relative_path);
    if entry_matches_file(entry, &hinted, library) {
        let updated = refresh_description(entry, &hinted, library);
        return Resolution::Found { path: hinted, updated_entry: updated };
    }

    // Step 2: search the library, cheapest-first, then confirm by hash.
    for (path, duration, size) in &snapshot.files {
        if !within_tolerance(entry, *duration, *size) {
            continue;
        }
        if entry_matches_file(entry, path, library) {
            let mut updated = entry.clone();
            updated.hints.relative_path = relative_to(playlist_dir, path);
            updated.hints.file_size_bytes = *size;
            if let Some(desc) = changed_description(&updated, path, library) {
                updated.description = desc;
            }
            return Resolution::Found { path: path.clone(), updated_entry: Some(updated) };
        }
    }

    // Step 3: descriptive fallback — rank by description similarity.
    let candidates = descriptive_candidates(entry, library, snapshot);
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

/// Library files within the duration tolerance, ranked by how alike their
/// descriptions are and then by closest duration. Description only orders the
/// offers — a file that matches no tag at all is still offered, because a
/// re-encode is usually retagged too and the operator confirms every re-link.
fn descriptive_candidates(
    entry: &Entry,
    library: &dyn Library,
    snapshot: &LibrarySnapshot,
) -> Vec<Candidate> {
    let target = entry.identity.duration_secs;
    let mut scored: Vec<(f64, f64, Candidate)> = snapshot
        .files
        .iter()
        .filter_map(|(path, duration, size)| {
            let duration_delta = (duration - target).abs();
            if duration_delta > DURATION_TOLERANCE_SECS {
                return None;
            }
            // Only now is a tag read worth paying for — the duration screen has
            // already cut the library down to a handful.
            let description = library.read_description(path)?;
            let score = description_similarity(&entry.description, &description);
            Some((score, duration_delta, Candidate { path: path.clone(), description, duration_secs: *duration, file_size_bytes: *size }))
        })
        .collect();
    scored.sort_by(|a, b| {
        b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal)
            .then(a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal))
    });
    scored.truncate(MAX_FALLBACK_CANDIDATES);
    scored.into_iter().map(|(_, _, c)| c).collect()
}

/// Weighted field similarity in 0.0..=1.0. Title and artist identify the track;
/// album identifies only its release, so it counts weakly. Year is excluded
/// entirely — every track of the same year matches it perfectly, which dilutes
/// the fields that actually discriminate. The map leaves ranking
/// implementation-defined.
fn description_similarity(a: &Description, b: &Description) -> f64 {
    0.4 * field_similarity(&a.title, &b.title)
        + 0.4 * field_similarity(&a.artist, &b.artist)
        + 0.2 * field_similarity(&a.album, &b.album)
}

/// Proportion of tokens two fields share. An empty field on either side scores
/// zero: a missing tag is not evidence of a match.
fn field_similarity(a: &str, b: &str) -> f64 {
    let (a, b) = (comparison_tokens(a), comparison_tokens(b));
    if a.is_empty() || b.is_empty() {
        return 0.0;
    }
    a.intersection(&b).count() as f64 / a.union(&b).count() as f64
}

/// Comparison tokens for a field: lowercased, accent-folded, split on anything
/// non-alphanumeric, with a leading "the" dropped. Comparing sets of these rather
/// than whole strings is what lets `01 - Closer` and `Closer`, or `The Beatles`
/// and `Beatles`, recognise each other.
fn comparison_tokens(field: &str) -> BTreeSet<String> {
    let folded: String = field.to_lowercase().chars().map(fold_accent).collect();
    let mut tokens = folded
        .split(|c: char| !c.is_alphanumeric())
        .filter(|t| !t.is_empty())
        .peekable();
    if tokens.peek() == Some(&"the") {
        tokens.next();
    }
    tokens.map(str::to_string).collect()
}

/// Common accented forms to their ASCII base, so `Beyoncé` and `Beyonce` compare
/// equal. Deliberately small — the accents that turn up in artist names, not a
/// general Unicode normalisation.
fn fold_accent(c: char) -> char {
    match c {
        'á' | 'à' | 'â' | 'ä' | 'ã' | 'å' => 'a',
        'é' | 'è' | 'ê' | 'ë' => 'e',
        'í' | 'ì' | 'î' | 'ï' => 'i',
        'ó' | 'ò' | 'ô' | 'ö' | 'õ' | 'ø' => 'o',
        'ú' | 'ù' | 'û' | 'ü' => 'u',
        'ñ' => 'n',
        'ç' => 'c',
        'ý' | 'ÿ' => 'y',
        other => other,
    }
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
        /// Counts the costly operations, so tests can assert the library is
        /// screened once per operation rather than once per entry.
        walks: std::cell::Cell<usize>,
        probes: std::cell::Cell<usize>,
    }
    impl FakeLibrary {
        fn add(&mut self, path: &str, bytes: Vec<u8>, duration: f64, desc: Description) {
            self.files.insert(PathBuf::from(path), (bytes, duration, desc));
        }
    }
    impl Library for FakeLibrary {
        fn candidates(&self) -> Vec<PathBuf> {
            self.walks.set(self.walks.get() + 1);
            self.files.keys().cloned().collect()
        }
        fn cheap_probe(&self, path: &Path) -> Option<(f64, u64)> {
            self.probes.set(self.probes.get() + 1);
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
    fn duration_is_written_rounded_but_still_resolves() {
        let bytes = wav(&[1, 2, 3]);
        let noisy = 198.86666666666667;
        let entry = entry_for(&bytes, noisy, bytes.len() as u64, "a.wav", desc("A", "T"));
        let json = serde_json::to_string(&Playlist { version: 1, entries: vec![entry] }).unwrap();
        assert!(json.contains("\"duration_secs\":198.87"), "{json}");

        // The rounded value still screens the real file in: rounding error is
        // orders of magnitude inside the tolerance.
        let reread: Playlist = serde_json::from_str(&json).unwrap();
        let mut lib = FakeLibrary::default();
        lib.add("/lib/a.wav", bytes, noisy, desc("A", "T"));
        match resolve(&reread.entries[0], Path::new("/lib"), &lib, &LibrarySnapshot::probe(&lib)) {
            Resolution::Found { .. } => {}
            other => panic!("expected Found, got {other:?}"),
        }
    }

    #[test]
    fn resolves_via_hint_when_file_present() {
        let bytes = wav(&[10, 20, 30]);
        let entry = entry_for(&bytes, 5.0, bytes.len() as u64, "song.wav", desc("A", "T"));
        let mut lib = FakeLibrary::default();
        lib.add("/lib/song.wav", bytes, 5.0, desc("A", "T"));
        // playlist_dir is /lib so the hint resolves directly.
        match resolve(&entry, Path::new("/lib"), &lib, &LibrarySnapshot::probe(&lib)) {
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
        match resolve(&entry, Path::new("/lib"), &lib, &LibrarySnapshot::probe(&lib)) {
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
        match resolve(&entry, Path::new("/lib"), &lib, &LibrarySnapshot::probe(&lib)) {
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
        match resolve(&entry, Path::new("/lib"), &lib, &LibrarySnapshot::probe(&lib)) {
            Resolution::NeedsConfirmation { candidates } => {
                assert_eq!(candidates.len(), 1);
                assert_eq!(candidates[0].path, PathBuf::from("/lib/reencoded.wav"));
            }
            other => panic!("expected NeedsConfirmation, got {other:?}"),
        }
    }

    #[test]
    fn retagged_re_encode_ranks_above_unrelated_tracks() {
        let entry = entry_for(&wav(&[1]), 100.0, 999, "gone.flac", desc("The Beatles", "Come Together"));
        let mut lib = FakeLibrary::default();
        // The re-encode: same track, retagged in a different house style.
        lib.add("/lib/right.mp3", wav(&[9, 9]), 100.4, desc("Beatles", "01 - Come Together"));
        // Decoys of near-identical length with unrelated tags.
        lib.add("/lib/wrong-a.mp3", wav(&[8, 8]), 100.1, desc("Miles Davis", "So What"));
        lib.add("/lib/wrong-b.mp3", wav(&[7, 7]), 99.9, desc("Portishead", "Roads"));
        match resolve(&entry, Path::new("/lib"), &lib, &LibrarySnapshot::probe(&lib)) {
            Resolution::NeedsConfirmation { candidates } => {
                assert_eq!(candidates[0].path, PathBuf::from("/lib/right.mp3"));
                // The decoys are still offered — the operator confirms either way.
                assert_eq!(candidates.len(), 3);
            }
            other => panic!("expected NeedsConfirmation, got {other:?}"),
        }
    }

    #[test]
    fn candidates_are_offered_when_no_tag_matches_at_all() {
        let entry = entry_for(&wav(&[1]), 100.0, 999, "gone.flac", desc("Aphex Twin", "Xtal"));
        let mut lib = FakeLibrary::default();
        lib.add("/lib/relabelled.mp3", wav(&[9, 9]), 100.2, desc("", "track 07"));
        match resolve(&entry, Path::new("/lib"), &lib, &LibrarySnapshot::probe(&lib)) {
            Resolution::NeedsConfirmation { candidates } => {
                assert_eq!(candidates.len(), 1);
            }
            other => panic!("expected an offer despite no tag overlap, got {other:?}"),
        }
    }

    #[test]
    fn offers_are_capped() {
        let entry = entry_for(&wav(&[1]), 100.0, 999, "gone.flac", desc("A", "T"));
        let mut lib = FakeLibrary::default();
        for i in 0..25 {
            lib.add(&format!("/lib/f{i}.mp3"), wav(&[i as u8]), 100.0, desc("Someone", "Something"));
        }
        match resolve(&entry, Path::new("/lib"), &lib, &LibrarySnapshot::probe(&lib)) {
            Resolution::NeedsConfirmation { candidates } => {
                assert_eq!(candidates.len(), MAX_FALLBACK_CANDIDATES);
            }
            other => panic!("expected NeedsConfirmation, got {other:?}"),
        }
    }

    #[test]
    fn near_miss_forms_score_as_matches() {
        // Leading article, punctuation, accents and track-number prefixes are noise.
        assert_eq!(field_similarity("The Beatles", "beatles"), 1.0);
        assert_eq!(field_similarity("Beyoncé", "Beyonce"), 1.0);
        assert_eq!(field_similarity("Sigur Rós", "sigur ros"), 1.0);
        assert_eq!(field_similarity("Closer", "closer!"), 1.0);
        // Extra tokens cost, but still score far above unrelated text.
        assert!(field_similarity("01 - Come Together", "Come Together") > 0.5);
        assert_eq!(field_similarity("Come Together", "So What"), 0.0);
        // A missing tag is not evidence of a match.
        assert_eq!(field_similarity("", ""), 0.0);
    }

    #[test]
    fn many_entries_screen_the_library_once() {
        let mut lib = FakeLibrary::default();
        for i in 0..20 {
            lib.add(&format!("/lib/f{i}.wav"), wav(&[i as u8]), 50.0 + i as f64, desc("A", "T"));
        }
        // Entries whose hints all miss — the worst case, where every one of them
        // would previously have walked and probed the whole library twice.
        let entries: Vec<Entry> = (0..15)
            .map(|i| entry_for(&wav(&[200 + i as u8]), 300.0, 999, "gone.wav", desc("Nobody", "Nothing")))
            .collect();

        let snapshot = LibrarySnapshot::probe(&lib);
        let (walks_after_probe, probes_after_probe) = (lib.walks.get(), lib.probes.get());
        assert_eq!(walks_after_probe, 1);
        assert_eq!(probes_after_probe, 20);

        for entry in &entries {
            let _ = resolve(entry, Path::new("/lib"), &lib, &snapshot);
        }

        // Resolving 15 entries adds no walk and no probe of its own.
        assert_eq!(lib.walks.get(), walks_after_probe);
        assert_eq!(lib.probes.get(), probes_after_probe);
    }

    #[test]
    fn unavailable_when_nothing_matches() {
        let entry = entry_for(&wav(&[1]), 100.0, 999, "gone.wav", desc("Artist", "Song"));
        let lib = FakeLibrary::default();
        assert_eq!(resolve(&entry, Path::new("/lib"), &lib, &LibrarySnapshot::probe(&lib)), Resolution::Unavailable);
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
        match resolve(&entry, Path::new("/lib"), &lib, &LibrarySnapshot::probe(&lib)) {
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
