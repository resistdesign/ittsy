#!/usr/bin/env bash
set -euo pipefail

marker="/tmp/ittsy-input-verification-$$"
before_terminal="$(mktemp)"
after_terminal="$(mktemp)"
app_pid=""

cleanup() {
  rm -f "${marker}" "${before_terminal}" "${after_terminal}"
  if [[ -n "${app_pid}" ]]; then
    kill "${app_pid}" 2>/dev/null || true
  fi
}
trap cleanup EXIT

pgrep -x Terminal | sort > "${before_terminal}" || true

cargo build --release --locked
scripts/package-macos.sh target/release/ittsy dist >/dev/null
open -n dist/ittsy.app
sleep 3

app_pid="$(pgrep -n -f "${PWD}/dist/ittsy.app/Contents/MacOS/ittsy")"
test -n "${app_pid}"

pgrep -x Terminal | sort > "${after_terminal}" || true
diff -u "${before_terminal}" "${after_terminal}"

expected="abcdefghijklmnopqrstuvwxyz0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ"
osascript \
  -e 'tell application "System Events" to tell process "ittsy" to set frontmost to true' \
  -e 'delay 0.2' \
  -e "tell application \"System Events\" to keystroke \"printf ${expected} > ${marker}\"" \
  -e 'tell application "System Events" to key code 36'
sleep 2

test "$(cat "${marker}")" = "${expected}"
codesign --verify --deep --strict dist/ittsy.app
echo "macOS app launch and input verification passed"
