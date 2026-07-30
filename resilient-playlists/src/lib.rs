//! Content-identity hashing for the resilient playlist format.
//!
//! The identity of a track is the Blake3 hash of its *encoded audio payload* —
//! the compressed audio bytes on disk, with tag regions and container overhead
//! excluded. Any conforming implementation reading the same file computes the
//! same hash byte-for-byte, so playlists stay valid across moves, renames, and
//! retags. See `playlist.md` for the specification.
//!
//! The extraction is deliberately plain (no Rust-specific idioms) so the C
//! reference port can mirror it directly, and is verified against a shared
//! test-vector corpus that defines conformance (see `map.md` and `corpus/`).

use std::ops::Range;

pub const HASH_ALGORITHM: &str = "blake3";

/// Version of the byte-range extraction rules this crate implements, recorded
/// per entry independently of `HASH_ALGORITHM`. Bumped only if a correction
/// changes which bytes are hashed, so an older implementation's entries are
/// recognisable and can be migrated (see `map.md`, Method Migration).
pub const PAYLOAD_EXTRACTION_VERSION: u32 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AudioFormat {
    Flac,
    Wav,
    OggVorbis,
    Opus,
    Mp3,
    Aac,
    M4a,
}

#[derive(Debug, PartialEq, Eq)]
pub enum IdentityError {
    UnknownFormat,
    Malformed(&'static str),
}

/// The hex Blake3 hash of a file's encoded audio payload.
pub fn content_hash(data: &[u8]) -> Result<String, IdentityError> {
    let format = detect_format(data).ok_or(IdentityError::UnknownFormat)?;
    let ranges = payload_ranges(data, format)?;
    let mut hasher = blake3::Hasher::new();
    for r in ranges {
        hasher.update(&data[r]);
    }
    Ok(hasher.finalize().to_hex().to_string())
}

/// The byte ranges of the encoded audio payload, in file order. Exposed so the
/// corpus can assert exact boundaries, not just the resulting hash.
pub fn payload_ranges(data: &[u8], format: AudioFormat) -> Result<Vec<Range<usize>>, IdentityError> {
    match format {
        AudioFormat::Wav => wav_payload_ranges(data),
        AudioFormat::Flac => flac_payload_ranges(data),
        AudioFormat::OggVorbis => ogg_payload_ranges(data, 3),
        AudioFormat::Opus => ogg_payload_ranges(data, 2),
        AudioFormat::M4a => m4a_payload_ranges(data),
        AudioFormat::Mp3 => framed_payload_ranges(data),
        AudioFormat::Aac => framed_payload_ranges(data),
    }
}

/// Identify the container from its leading bytes rather than the filename, so a
/// mislabelled file still hashes correctly. A leading ID3v2 tag (which some
/// taggers prepend even to FLAC) is skipped before sniffing.
pub fn detect_format(data: &[u8]) -> Option<AudioFormat> {
    let start = id3v2_end(data);
    let d = &data[start..];
    if d.len() >= 12 && &d[0..4] == b"RIFF" && &d[8..12] == b"WAVE" {
        return Some(AudioFormat::Wav);
    }
    if d.len() >= 4 && &d[0..4] == b"fLaC" {
        return Some(AudioFormat::Flac);
    }
    if d.len() >= 28 && &d[0..4] == b"OggS" {
        // The first packet body follows the page header + segment table; its
        // signature distinguishes the codec.
        let body = 27 + d[26] as usize;
        if d.len() >= body + 8 && &d[body..body + 8] == b"OpusHead" {
            return Some(AudioFormat::Opus);
        }
        if d.len() >= body + 7 && d[body] == 1 && &d[body + 1..body + 7] == b"vorbis" {
            return Some(AudioFormat::OggVorbis);
        }
    }
    if d.len() >= 8 && &d[4..8] == b"ftyp" {
        return Some(AudioFormat::M4a);
    }
    // MP3 and AAC-ADTS both begin with a 0xFF sync; the layer bits tell them
    // apart — ADTS carries layer 00, every MP3 layer is non-zero.
    if d.len() >= 2 && d[0] == 0xFF && (d[1] & 0xE0) == 0xE0 {
        let layer = (d[1] & 0x06) >> 1;
        if layer == 0 && (d[1] & 0xF0) == 0xF0 {
            return Some(AudioFormat::Aac);
        }
        return Some(AudioFormat::Mp3);
    }
    None
}

/// Byte offset just past a leading ID3v2 tag, or 0 if none. ID3v2 header is
/// "ID3" + 2 version bytes + 1 flags byte + a 4-byte synchsafe size (7 bits per
/// byte) covering the tag body; a footer-present flag adds 10 more bytes.
fn id3v2_end(data: &[u8]) -> usize {
    if data.len() < 10 || &data[0..3] != b"ID3" {
        return 0;
    }
    let flags = data[5];
    let size = ((data[6] as usize) << 21)
        | ((data[7] as usize) << 14)
        | ((data[8] as usize) << 7)
        | (data[9] as usize);
    let footer = if flags & 0x10 != 0 { 10 } else { 0 };
    (10 + size + footer).min(data.len())
}

// ---- WAV ----
//
// RIFF container: "RIFF" <u32 le size> "WAVE", then a sequence of chunks, each
// "<4-byte id> <u32 le size> <body>" with the body padded to an even length.
// The audio payload is the body of the "data" chunk only; every other chunk
// (`fmt `, `LIST`, `fact`, …) is excluded.

fn wav_payload_ranges(data: &[u8]) -> Result<Vec<Range<usize>>, IdentityError> {
    let mut pos = 12; // past "RIFF" <size> "WAVE"
    while pos + 8 <= data.len() {
        let id = &data[pos..pos + 4];
        let size = u32::from_le_bytes([data[pos + 4], data[pos + 5], data[pos + 6], data[pos + 7]]) as usize;
        let body_start = pos + 8;
        let body_end = body_start.saturating_add(size).min(data.len());
        if id == b"data" {
            return Ok(vec![body_start..body_end]);
        }
        // Chunks are padded to an even byte boundary.
        pos = body_start + size + (size & 1);
    }
    Err(IdentityError::Malformed("wav: no data chunk"))
}

// ---- FLAC ----
//
// "fLaC" marker, then a chain of METADATA_BLOCKs, each a 4-byte header (bit 7 of
// the first byte is the last-block flag, the low 7 bits are the type) followed
// by a 24-bit big-endian body length. The audio payload is every FRAME after
// the metadata chain — i.e. from the end of the chain to end of file.

fn flac_payload_ranges(data: &[u8]) -> Result<Vec<Range<usize>>, IdentityError> {
    let mut pos = id3v2_end(data);
    if data.len() < pos + 4 || &data[pos..pos + 4] != b"fLaC" {
        return Err(IdentityError::Malformed("flac: missing marker"));
    }
    pos += 4;
    loop {
        if pos + 4 > data.len() {
            return Err(IdentityError::Malformed("flac: truncated metadata"));
        }
        let last = data[pos] & 0x80 != 0;
        let len = ((data[pos + 1] as usize) << 16)
            | ((data[pos + 2] as usize) << 8)
            | (data[pos + 3] as usize);
        pos = pos + 4 + len;
        if last {
            break;
        }
    }
    if pos > data.len() {
        return Err(IdentityError::Malformed("flac: metadata exceeds file"));
    }
    Ok(vec![pos..data.len()])
}

// ---- Ogg (Vorbis / Opus) ----
//
// The file is a sequence of pages: "OggS", version, header-type, granule(8),
// serial(4), seqno(4), crc(4), page_segments(1), then a segment table of that
// many lacing values, then the body (sum of the lacing values). A lacing value
// < 255 ends a packet. The codec's header packets come first — three for Vorbis
// (identification, comment, setup), two for Opus (OpusHead, OpusTags) — and the
// encoder flushes them so audio always begins on a fresh page. The payload is
// every page from the first audio page to end of file, page headers included.

fn ogg_payload_ranges(data: &[u8], header_packets: usize) -> Result<Vec<Range<usize>>, IdentityError> {
    let mut pos = id3v2_end(data);
    let mut completed = 0usize;
    while pos + 27 <= data.len() {
        if &data[pos..pos + 4] != b"OggS" {
            return Err(IdentityError::Malformed("ogg: bad page capture"));
        }
        let page_segments = data[pos + 26] as usize;
        let table = pos + 27;
        if table + page_segments > data.len() {
            return Err(IdentityError::Malformed("ogg: truncated segment table"));
        }
        let mut body_len = 0usize;
        let mut packet_ends = 0usize;
        for i in 0..page_segments {
            let v = data[table + i];
            body_len += v as usize;
            if v < 255 {
                packet_ends += 1;
            }
        }
        let page_end = table + page_segments + body_len;
        completed += packet_ends;
        if completed >= header_packets {
            // Header packets end at a page boundary; audio is the next page on.
            return Ok(vec![page_end.min(data.len())..data.len()]);
        }
        pos = page_end;
    }
    Err(IdentityError::Malformed("ogg: no audio pages"))
}

// ---- M4A / MP4 ----
//
// A sequence of boxes, each "<u32 be size> <4-byte type> <body>". The audio
// payload is the body of the `mdat` box only; `ftyp`, `moov`, `free`, and the
// rest are container overhead. A size of 1 signals a 64-bit size in the eight
// bytes after the type; a size of 0 runs the box to end of file.

fn m4a_payload_ranges(data: &[u8]) -> Result<Vec<Range<usize>>, IdentityError> {
    let mut pos = 0usize;
    while pos + 8 <= data.len() {
        let size32 = u32::from_be_bytes([data[pos], data[pos + 1], data[pos + 2], data[pos + 3]]) as usize;
        let typ = &data[pos + 4..pos + 8];
        let (header_len, box_len) = if size32 == 1 {
            if pos + 16 > data.len() {
                return Err(IdentityError::Malformed("m4a: truncated 64-bit size"));
            }
            let big = u64::from_be_bytes(data[pos + 8..pos + 16].try_into().unwrap()) as usize;
            (16usize, big)
        } else if size32 == 0 {
            (8usize, data.len() - pos)
        } else {
            (8usize, size32)
        };
        if typ == b"mdat" {
            let body = pos + header_len;
            return Ok(vec![body..(pos + box_len).min(data.len())]);
        }
        pos += box_len;
    }
    Err(IdentityError::Malformed("m4a: no mdat box"))
}

// ---- MP3 / AAC (framed streams) ----
//
// No container — just a run of audio frames. The payload is everything between
// a leading ID3v2 tag and any trailing tags (ID3v1's 128-byte "TAG" block, and
// an APEv2 tag whose 32-byte footer begins "APETAGEX"). Every audio frame,
// including any encoder info frame, is part of the payload.

fn framed_payload_ranges(data: &[u8]) -> Result<Vec<Range<usize>>, IdentityError> {
    let start = id3v2_end(data);
    let mut end = data.len();
    // Trailing ID3v1 is the final 128 bytes beginning "TAG".
    if end >= start + 128 && &data[end - 128..end - 125] == b"TAG" {
        end -= 128;
    }
    // Trailing APEv2: a 32-byte footer "APETAGEX", version, then the tag size
    // (little-endian, covering items + footer). A separate 32-byte header may
    // precede the items; its presence is flagged in the footer flags.
    if end >= start + 32 && &data[end - 32..end - 24] == b"APETAGEX" {
        let tag_size = u32::from_le_bytes(data[end - 20..end - 16].try_into().unwrap()) as usize;
        let flags = u32::from_le_bytes(data[end - 12..end - 8].try_into().unwrap());
        let header = if flags & 0x8000_0000 != 0 { 32 } else { 0 };
        end = end.saturating_sub(tag_size + header);
    }
    if end < start {
        return Err(IdentityError::Malformed("framed: tags overlap"));
    }
    Ok(vec![start..end])
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Build a minimal valid WAV around `audio` and return (file, payload range).
    fn wav_with_payload(audio: &[u8]) -> Vec<u8> {
        let mut f = Vec::new();
        f.extend_from_slice(b"RIFF");
        f.extend_from_slice(&(36u32 + audio.len() as u32).to_le_bytes());
        f.extend_from_slice(b"WAVE");
        // fmt chunk (16 bytes): PCM, mono, 44100, 16-bit
        f.extend_from_slice(b"fmt ");
        f.extend_from_slice(&16u32.to_le_bytes());
        f.extend_from_slice(&1u16.to_le_bytes());      // PCM
        f.extend_from_slice(&1u16.to_le_bytes());      // channels
        f.extend_from_slice(&44100u32.to_le_bytes());  // sample rate
        f.extend_from_slice(&88200u32.to_le_bytes());  // byte rate
        f.extend_from_slice(&2u16.to_le_bytes());      // block align
        f.extend_from_slice(&16u16.to_le_bytes());     // bits
        // data chunk
        f.extend_from_slice(b"data");
        f.extend_from_slice(&(audio.len() as u32).to_le_bytes());
        f.extend_from_slice(audio);
        f
    }

    /// Conformance: every committed corpus file must hash to its target result,
    /// and its payload boundaries must match. This is the shared contract a second
    /// implementation (the C player) validates against the same corpus + targets.
    #[test]
    fn corpus_matches_target_results() {
        let dir = format!("{}/corpus", env!("CARGO_MANIFEST_DIR"));
        let targets: serde_json::Value =
            serde_json::from_slice(&std::fs::read(format!("{dir}/target_results.json")).unwrap()).unwrap();
        let entries = targets.as_object().unwrap();
        assert!(!entries.is_empty(), "target_results is empty");
        for (name, spec) in entries {
            let data = std::fs::read(format!("{dir}/{name}")).unwrap();
            let fmt = detect_format(&data).unwrap_or_else(|| panic!("{name}: format not detected"));
            let ranges = payload_ranges(&data, fmt).unwrap_or_else(|e| panic!("{name}: {e:?}"));
            let want_payload = spec["payload"].as_array().unwrap();
            assert_eq!(ranges[0].start, want_payload[0].as_u64().unwrap() as usize, "{name}: payload start");
            assert_eq!(ranges[0].end, want_payload[1].as_u64().unwrap() as usize, "{name}: payload end");
            assert_eq!(content_hash(&data).unwrap(), spec["hash"].as_str().unwrap(), "{name}: hash");
        }
    }

    fn corpus(name: &str) -> Vec<u8> {
        std::fs::read(format!("{}/corpus/{}", env!("CARGO_MANIFEST_DIR"), name)).unwrap()
    }

    /// A minimal ID3v2.4 tag of `body` zero bytes (10-byte header + body).
    fn id3v2(body: usize) -> Vec<u8> {
        let mut t = vec![b'I', b'D', b'3', 4, 0, 0];
        t.push(((body >> 21) & 0x7f) as u8);
        t.push(((body >> 14) & 0x7f) as u8);
        t.push(((body >> 7) & 0x7f) as u8);
        t.push((body & 0x7f) as u8);
        t.extend(std::iter::repeat(0).take(body));
        t
    }

    /// A 128-byte ID3v1 trailer.
    fn id3v1() -> Vec<u8> {
        let mut t = vec![b'T', b'A', b'G'];
        t.extend(std::iter::repeat(0).take(125));
        t
    }

    /// A minimal footer-only APEv2 tag (32-byte footer, no items, no header).
    fn apev2() -> Vec<u8> {
        let mut t = Vec::new();
        t.extend_from_slice(b"APETAGEX");
        t.extend_from_slice(&2000u32.to_le_bytes()); // version
        t.extend_from_slice(&32u32.to_le_bytes());   // tag size (footer only)
        t.extend_from_slice(&0u32.to_le_bytes());    // item count
        t.extend_from_slice(&0u32.to_le_bytes());    // flags: no header
        t.extend_from_slice(&[0u8; 8]);              // reserved
        t
    }

    /// Adding tags around a clean file must not change its identity — the whole
    /// point of hashing the audio payload rather than the file.
    #[test]
    fn tags_do_not_change_identity() {
        let flac = corpus("clean.flac");
        let flac_tagged = [id3v2(37), flac.clone()].concat();
        assert_eq!(content_hash(&flac_tagged).unwrap(), content_hash(&flac).unwrap());

        let mp3 = corpus("clean.mp3");
        let mp3_tagged = [id3v2(50), mp3.clone(), apev2(), id3v1()].concat();
        assert_eq!(content_hash(&mp3_tagged).unwrap(), content_hash(&mp3).unwrap());

        let aac = corpus("clean.aac");
        let aac_tagged = [id3v2(20), aac.clone(), id3v1()].concat();
        assert_eq!(content_hash(&aac_tagged).unwrap(), content_hash(&aac).unwrap());
    }

    /// Writes the tagged edge-case files into the corpus so the committed set
    /// (shared with other implementations) exercises the tag-stripping paths.
    /// Run manually: `cargo test write_tagged_corpus -- --ignored`.
    #[test]
    #[ignore]
    fn write_tagged_corpus() {
        let dir = format!("{}/corpus", env!("CARGO_MANIFEST_DIR"));
        std::fs::write(format!("{dir}/id3-prepended.flac"), [id3v2(37), corpus("clean.flac")].concat()).unwrap();
        std::fs::write(format!("{dir}/tagged.mp3"), [id3v2(50), corpus("clean.mp3"), apev2(), id3v1()].concat()).unwrap();
    }

    #[test]
    fn wav_detected_and_payload_isolated() {
        let audio = [1u8, 2, 3, 4, 5, 6, 7, 8];
        let file = wav_with_payload(&audio);
        assert_eq!(detect_format(&file), Some(AudioFormat::Wav));
        let ranges = payload_ranges(&file, AudioFormat::Wav).unwrap();
        assert_eq!(ranges.len(), 1);
        assert_eq!(&file[ranges[0].clone()], &audio);
    }

    #[test]
    fn wav_hash_ignores_surrounding_chunks() {
        // Same audio payload, different fmt-irrelevant trailer → identical hash
        // requires the hash to cover only the data body. Here we assert the hash
        // equals a direct Blake3 of the payload bytes.
        let audio = [9u8, 8, 7, 6, 5, 4, 3, 2, 1, 0];
        let file = wav_with_payload(&audio);
        let got = content_hash(&file).unwrap();
        let want = blake3::Hasher::new().update(&audio).finalize().to_hex().to_string();
        assert_eq!(got, want);
    }
}
