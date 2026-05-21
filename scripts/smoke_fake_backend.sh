#!/usr/bin/env sh
set -eu

ROOT_DIR=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
TMP_DIR=$(mktemp -d "${TMPDIR:-/tmp}/wattch-smoke.XXXXXX")
SOCKET="$TMP_DIR/wattch.sock"
STDOUT_LOG="$TMP_DIR/stdout.log"
STDERR_LOG="$TMP_DIR/stderr.log"
BUILD_LOG="$TMP_DIR/build.log"
SOURCES_OUT="$TMP_DIR/sources.out"
STREAM_OUT="$TMP_DIR/stream.out"
RUN_OUT="$TMP_DIR/run.out"
DAEMON_PID=""

cleanup() {
  if [ -n "$DAEMON_PID" ]; then
    kill "$DAEMON_PID" 2>/dev/null || true
    wait "$DAEMON_PID" 2>/dev/null || true
  fi
  rm -rf "$TMP_DIR"
}
trap cleanup EXIT INT TERM

cd "$ROOT_DIR"
cargo build -p rapl-wattchd -p wattch-cli >"$BUILD_LOG" 2>&1

WATTCH_SOCKET="$SOCKET" WATTCH_SOURCE_BACKEND=fake \
  cargo run --quiet -p rapl-wattchd --bin rapl-wattchd >"$STDOUT_LOG" 2>"$STDERR_LOG" &
DAEMON_PID=$!

index=0
while [ "$index" -lt 100 ]; do
  if [ -S "$SOCKET" ]; then
    break
  fi
  if ! kill -0 "$DAEMON_PID" 2>/dev/null; then
    exit 1
  fi
  index=$((index + 1))
  sleep 0.01
done

[ -S "$SOCKET" ]

WATTCH_SOCKET="$SOCKET" cargo run --quiet -p wattch-cli --bin wattch -- hello >/dev/null
WATTCH_SOCKET="$SOCKET" cargo run --quiet -p wattch-cli --bin wattch -- sources >"$SOURCES_OUT"
grep -q "fake:deterministic" "$SOURCES_OUT"
WATTCH_SOCKET="$SOCKET" cargo run --quiet -p wattch-cli --bin wattch -- stream --interval-ms 10 --duration 50ms --format csv >"$STREAM_OUT"
grep -q "fake:deterministic" "$STREAM_OUT"
WATTCH_SOCKET="$SOCKET" cargo run --quiet -p wattch-cli --bin wattch -- run --interval-ms 10 -- sh -c "sleep 0.05" >"$RUN_OUT"
grep -q "fake:deterministic" "$RUN_OUT"

printf '%s\n' "wattch smoke test passed"
