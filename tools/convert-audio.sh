#!/usr/bin/env bash
# Convert Soulscale's WAV masters into web formats for the Discord client.
#
#   tools/convert-audio.sh <source-dir> [output-dir]
#
# <source-dir> holds music/*.wav and sfx/*.wav. Output (default client/public/audio):
#   <name>.ogg  Ogg Opus (Chrome/Discord desktop/Android; gapless loops)
#   <name>.m4a  AAC fallback (Safari / iOS Discord WebView)
#   index.json  every asset with its exact duration and sample count (for loop points)
# The hand-edited mapping of game events -> sounds lives in <output-dir>/manifest.json
# and is never overwritten by this script.
# Requires ffmpeg + ffprobe on PATH. Safe to re-run; outputs are rebuilt each time.
set -euo pipefail
SRC="${1:?usage: tools/convert-audio.sh <source-dir> [output-dir]}"
OUT="${2:-client/public/audio}"
command -v ffmpeg >/dev/null || { echo "ffmpeg not found on PATH" >&2; exit 1; }
command -v ffprobe >/dev/null || { echo "ffprobe not found on PATH" >&2; exit 1; }

entries=()
for kind in music sfx; do
  [ -d "$SRC/$kind" ] || continue
  mkdir -p "$OUT/$kind"
  if [ "$kind" = music ]; then opus=128k; aac=160k; else opus=96k; aac=128k; fi
  shopt -s nullglob
  for wav in "$SRC/$kind"/*.wav "$SRC/$kind"/*.WAV; do
    name="$(basename "${wav%.*}")"
    case "$name" in *[!A-Za-z0-9._-]*) echo "skip (rename without spaces/symbols): $wav" >&2; continue;; esac
    ffmpeg -hide_banner -loglevel error -y -i "$wav" -map_metadata -1 -vn \
      -c:a libopus -b:a "$opus" -vbr on -application audio -ar 48000 "$OUT/$kind/$name.ogg"
    ffmpeg -hide_banner -loglevel error -y -i "$wav" -map_metadata -1 -vn \
      -c:a aac -b:a "$aac" -ar 48000 -movflags +faststart "$OUT/$kind/$name.m4a"
    probe() { ffprobe -v error -select_streams a:0 -show_entries "stream=$1" -of default=nw=1:nk=1 "$wav"; }
    rate="$(probe sample_rate)"; chans="$(probe channels)"; samples="$(probe duration_ts)"
    dur="$(ffprobe -v error -show_entries format=duration -of csv=p=0 "$wav")"
    entries+=("{\"id\":\"$kind/$name\",\"kind\":\"$kind\",\"duration\":$dur,\"rate\":$rate,\"samples\":$samples,\"channels\":$chans}")
    echo "  $kind/$name  (${dur}s)"
  done
done
{
  echo '{"version":1,"assets":['
  printf '%s\n' "$(IFS=,; echo "${entries[*]}")" | sed 's/},{/},\n{/g'
  echo ']}'
} > "$OUT/index.json"
echo "wrote ${#entries[@]} assets to $OUT (index.json updated; manifest.json untouched)"
