# Wattch Agent Instructions

## Project Identity

Wattch is a minimal local energy measurement tool.

This repository contains only:

1. Rust daemon
2. Rust CLI
3. Shared Rust crates needed by the daemon and CLI

Do not add clients in other languages.

Do not add:

- Python client
- Node client
- JavaScript client
- web dashboard
- VSCode extension
- MCP server
- gRPC service
- HTTP API
- JSON wire protocol
- database
- report generator
- AI integration
- profiler integration
- dynamic plugin loader

The MVP is intentionally small.

---

## Architecture

Expected repository structure:

```text
wattch/
  Cargo.toml
  README.md
  AGENTS.md
  proto/
    wattch.proto
  crates/
    wattch-proto/
    wattch-core/
    rapl-wattchd/
    wattch-cli/
```

Crate responsibilities:

```text
wattch-proto
  generated protobuf types only

wattch-core
  shared errors
  protobuf framing
  powercap source discovery
  energy delta math
  validation helpers
  monotonic timing helpers

rapl-wattchd
  Unix socket server
  request handling
  global stream state
  sampling loop

wattch-cli
  minimal CLI commands:
    hello
    sources
    stream
```

Keep boundaries strict.

The daemon should not contain CLI formatting logic.

The CLI should not contain source discovery logic.

The protocol crate should not contain daemon behavior.

## Protocol

Use protobuf via `prost`.

Use this wire frame:

```text
[4-byte little-endian uint32 payload_length][protobuf payload]
```

Maximum frame size:

```text
1 MiB
```

Do not use:

- gRPC
- HTTP
- JSON
- newline-delimited messages

Streaming samples are sent as protobuf `Response` messages.

Samples must reuse the `request_id` from the original `StartStreamRequest`.

## Socket

Default config file:

```text
/etc/wattch/wattch.conf
```

Default service socket path:

```text
/run/wattch/wattch.sock
```

Environment overrides:

```text
WATTCH_CONFIG
WATTCH_SOCKET
```

Socket permissions:

```text
0600
```

The daemon is expected to run as root when powercap permissions require it.

The CLI is expected to run without root.

When the daemon is started through `sudo`, it must use `SUDO_UID` and `SUDO_GID` to hand the root-created socket to the invoking user while keeping socket mode `0600`.

For system services not launched through `sudo`, the config file may provide numeric `socket_uid` and `socket_gid` values.

## Powercap Path Requirement

Use the Linux powercap sysfs root:

```text
/sys/devices/virtual/powercap
```

Do not use:

```text
/sys/class/powercap
```

Do not use:

```text
/sys/device/virtual/device
```

The source discovery implementation must accept an injectable root path so tests can use a fake temporary powercap tree.

The daemon production default is:

```text
/sys/devices/virtual/powercap
```

The daemon test override is:

```text
WATTCH_POWER_CAP_ROOT
```

Rationale: `/sys/devices/virtual/powercap` is the canonical powercap sysfs tree shown by the kernel docs. `energy_uj` is the cumulative energy counter and `max_energy_range_uj` is the counter range; watts should be computed from deltas rather than read as instantaneous RAPL power.

## Powercap Source Discovery

Search recursively under:

```text
<powercap_root>/intel-rapl
```

A valid energy source directory must contain:

```text
energy_uj
max_energy_range_uj
name
```

Convert:

```text
energy_j = energy_uj / 1_000_000.0
max_energy_j = max_energy_range_uj / 1_000_000.0
```

Each discovered source becomes:

```text
source_id: incrementing u32 starting from 1
name: rapl:<zone-name>
kind: rapl
unit: joule
available: true
```

Incomplete source directories must be ignored, not fatal.

If no sources exist, daemon must still run.

Make source ordering deterministic by sorting directories by path before assigning IDs.

## Sampling

Use monotonic time only.

Minimum interval:

```text
1_000_000 ns
```

Default interval:

```text
100_000_000 ns
```

Reject lower intervals with:

```text
INTERVAL_TOO_LOW
```

Delta computation:

```text
if current >= previous:
    delta_j = current - previous
    counter_wrap = false
else:
    delta_j = (max_energy_j - previous) + current
    counter_wrap = true
```

Power computation:

```text
power_w = delta_j / (interval_ns / 1_000_000_000.0)
```

Do not claim exact process-level or function-level energy attribution in this MVP.

## Daemon Behavior

The daemon must support:

- `HelloRequest`
- `ListSourcesRequest`
- `StartStreamRequest`
- `StopStreamRequest`

Protocol version:

```text
1
```

Daemon version:

```text
0.1.0
```

Only one active stream is allowed globally.

Multiple clients may connect, but only one can own the stream.

If a second stream starts while another is active, return:

```text
STREAM_ALREADY_RUNNING
```

If `StopStreamRequest` is sent while no stream exists, return:

```text
STREAM_NOT_RUNNING
```

Invalid source IDs return:

```text
SOURCE_NOT_FOUND
```

## Error Codes

Use stable numeric error codes:

```text
1 UNKNOWN
2 BAD_REQUEST
3 UNSUPPORTED_VERSION
4 SOURCE_NOT_FOUND
5 SOURCE_UNAVAILABLE
6 STREAM_ALREADY_RUNNING
7 STREAM_NOT_RUNNING
8 INTERVAL_TOO_LOW
9 INTERNAL
```

Do not renumber existing codes.

Add new codes only when necessary.

## CLI

Only implement:

```text
wattch-cli hello
wattch-cli sources
wattch-cli stream --interval-ms 100
```

CLI output should be plain text.

Do not add JSON output yet.

Do not add report generation.

Do not add benchmark orchestration.

## Testing Requirements

Tests are mandatory.

Do not scaffold code without tests.

Tests must not require:

- root access
- real RAPL
- real `/sys/devices/virtual/powercap`
- specific CPU model
- system-specific power zones

Use fake powercap directories with `tempfile`.

Testing environment variables:

```text
WATTCH_SOCKET
WATTCH_POWER_CAP_ROOT
```

Use these to isolate tests.

## Explicit Test Matrix

The scaffold must force tests from the first commit.

Use this test matrix.

```text
Unit tests

framing:
  frame_roundtrip_request
  frame_roundtrip_response
  frame_rejects_too_large_payload
  frame_rejects_truncated_payload

powercap math:
  powercap_delta_without_wrap
  powercap_delta_with_wrap
  powercap_microjoules_to_joules

powercap discovery:
  powercap_discovers_fake_sources
  powercap_ignores_incomplete_source_dirs
  powercap_orders_sources_deterministically
  powercap_handles_missing_root

validation:
  validate_interval_accepts_10ms
  validate_interval_rejects_below_10ms
  validate_source_ids_accepts_existing_ids
  validate_source_ids_rejects_missing_ids

daemon protocol:
  hello_accepts_protocol_v1
  hello_rejects_unsupported_protocol
  start_stream_rejects_invalid_source
  start_stream_rejects_low_interval

Integration tests

daemon_hello_roundtrip_over_unix_socket
daemon_list_sources_over_unix_socket_with_fake_powercap_root
daemon_start_stream_emits_samples_with_fake_powercap_root
daemon_rejects_second_active_stream
daemon_stop_without_stream_returns_error
cli_hello_smoke_test
cli_sources_smoke_test_with_fake_daemon_or_fake_powercap

Fake sysfs fixture:

tempdir/
  intel-rapl/
    intel-rapl:0/
      name
      energy_uj
      max_energy_range_uj
      intel-rapl:0:0/
        name
        energy_uj
        max_energy_range_uj
    intel-rapl:1/
      name
      energy_uj
      max_energy_range_uj

Expected fixture values:

intel-rapl:0/name = package-0
intel-rapl:0/energy_uj = 1000000
intel-rapl:0/max_energy_range_uj = 262143000000

intel-rapl:0/intel-rapl:0:0/name = core
intel-rapl:0/intel-rapl:0:0/energy_uj = 500000
intel-rapl:0/intel-rapl:0:0/max_energy_range_uj = 262143000000

Expected discovered sources:

1 rapl:package-0 rapl joule true
2 rapl:core      rapl joule true

Make source ordering deterministic by sorting directories by path before assigning IDs.

## Quality Gate

Before considering a task complete, run:

```sh
cargo fmt --all
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo build --workspace
```

If a check fails, fix the code.

Do not hide failing tests.

Do not weaken tests to make them pass.

## Code Style

Prefer:

- small modules
- explicit structs
- clear error types
- deterministic tests
- simple async code

Avoid:

- premature generic abstractions
- macros unless needed
- large framework dependencies
- clever lifetime-heavy designs
- hidden global mutable state except for explicit daemon stream ownership

The correct implementation is boring, small, and easy to measure.
