#!/usr/bin/env bash
# Regenerates the per-container import-test fixtures under
# crates/core/tests/fixtures/import/. Requires ffmpeg.
#
# The fixtures are committed; this script documents their provenance and
# lets them be rebuilt if a container needs to change. Source signal is a
# 2-second 440 Hz sine, stereo, 44.1 kHz — chosen so tests can assert the
# decoded length, level, and zero-crossing rate after the 16 kHz downmix.
set -euo pipefail

cd "$(dirname "$0")/.."
out="crates/core/tests/fixtures/import"
mkdir -p "$out"

SRC="sine=frequency=440:duration=2"

gen() { # gen <output-file> [extra ffmpeg args...]
  local f="$1"
  shift
  ffmpeg -y -hide_banner -loglevel error \
    -f lavfi -i "$SRC" -af "volume=12dB" -ac 2 -ar 44100 "$@" "$out/$f"
  echo "  $out/$f"
}

gen tone.wav -c:a pcm_s16le
gen tone.mp3 -c:a libmp3lame -b:a 96k
gen tone.m4a -c:a aac -b:a 96k
gen tone.mp4 -c:a aac -b:a 96k
gen tone.mov -c:a aac -b:a 96k
# ffmpeg's native vorbis encoder is marked experimental but is fine for a
# test tone (libvorbis is not part of the stock Homebrew ffmpeg build).
gen tone.ogg -c:a vorbis -strict -2
gen tone.flac -c:a flac
gen tone.mkv -c:a vorbis -strict -2

# A chained OGG (two complete streams concatenated) — legal per the OGG spec
# but unsupported in v1; the importer must fail typed, not half-import.
cat "$out/tone.ogg" "$out/tone.ogg" > "$out/chained.ogg"
echo "  $out/chained.ogg"

# A FLAC whose frame bodies are corrupted but whose sync codes survive:
# probing succeeds and packets parse, every frame fails its CRC, decode
# yields zero samples.
python3 - "$out" <<'EOF'
import sys, pathlib
out = pathlib.Path(sys.argv[1])
data = bytearray((out / "tone.flac").read_bytes())
assert data[:4] == b"fLaC"
# Walk the metadata blocks (1-byte flags+type, 3-byte big-endian length) to
# find the first audio frame.
pos = 4
while True:
    last = data[pos] & 0x80
    length = int.from_bytes(data[pos + 1 : pos + 4], "big")
    pos += 4 + length
    if last:
        break
# Find each frame sync (0xFF 0xF8) and zero a span inside the frame body,
# far enough past the header that the sync scan still works.
syncs = [i for i in range(pos, len(data) - 1) if data[i] == 0xFF and data[i + 1] == 0xF8]
assert syncs, "no FLAC frame syncs found"
for s in syncs:
    for i in range(s + 16, min(s + 64, len(data))):
        data[i] = 0x00
(out / "corrupt.flac").write_bytes(data)
print(f"  {out}/corrupt.flac ({len(syncs)} frames corrupted)")
EOF

echo "fixtures written to $out/"
