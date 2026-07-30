#!/usr/bin/env bash
# Regenerates the clean corpus files. Provenance only — the committed files are
# the pinned contract; this documents how they were made. Synthetic source (a
# tone), so no third-party audio. `-bitexact -map_metadata -1` strips encoder
# version strings and metadata for a minimal, deterministic baseline.
#
# Requires ffmpeg. Run from the repository root.
set -euo pipefail
cd "$(dirname "$0")"

SRC='sine=frequency=440:duration=0.08:sample_rate=44100'
common=(-y -bitexact -f lavfi -i "$SRC" -ac 1 -map_metadata -1)

ffmpeg "${common[@]}"                                             clean.wav
ffmpeg "${common[@]}" -c:a flac                                  clean.flac
ffmpeg "${common[@]}" -c:a libvorbis                             clean.ogg
ffmpeg "${common[@]}" -c:a libopus                               clean.opus
ffmpeg "${common[@]}" -c:a libmp3lame -write_xing 0 -id3v2_version 0  clean.mp3
ffmpeg "${common[@]}" -c:a aac -f adts                           clean.aac
ffmpeg "${common[@]}" -c:a aac                                   clean.m4a

# Tag-placement edge cases are derived from the clean files by the test
# `playlist::tests::write_tagged_corpus` (run: cargo test write_tagged_corpus -- --ignored),
# which wraps them in synthetic ID3v2 / ID3v1 / APEv2 tags.
echo "Clean corpus regenerated. Update target_results.json if hashes change."
