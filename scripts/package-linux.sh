#!/usr/bin/env bash
set -euo pipefail

binary="${1:-target/release/ittsy}"
output_dir="${2:-dist}"

test -x "$binary" || {
  echo "missing executable: $binary" >&2
  exit 1
}

mkdir -p "$output_dir"
install -m 755 "$binary" "$output_dir/ittsy"
