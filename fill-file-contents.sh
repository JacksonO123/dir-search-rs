#!/usr/bin/env bash

set -euo pipefail

COUNT="${1:-2000}"
DIR="data/file-contents"
NEEDLE="ZZNEEDLEZZ"

mkdir -p "$DIR"
rm -f "$DIR"/bench_*.txt

for i in $(seq 1 "$COUNT"); do
  if (( i % 40 == 0 )); then
    size=$(( (RANDOM % 1792 + 256) * 1024 ))
  else
    size=$(( (RANDOM % 64 + 1) * 1024 ))
  fi

  name="$DIR/bench_$(printf '%05d' "$i").txt"
  head -c "$size" /dev/urandom | base64 > "$name"

  if (( i % 10 == 0 )); then
    printf '%s\n' "$NEEDLE" >> "$name"
  fi
done

echo "Done."
echo "Files:      $(ls "$DIR"/bench_*.txt | wc -l | tr -d ' ')"
echo "Total size: $(du -sh "$DIR" | cut -f1)"
echo "Needle hits ($NEEDLE): $(grep -l "$NEEDLE" "$DIR"/bench_*.txt | wc -l | tr -d ' ')"
