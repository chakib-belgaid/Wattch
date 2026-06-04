
# wattch

[![Rust CI](https://github.com/chakib-belgaid/Wattch/actions/workflows/ci.yml/badge.svg)](https://github.com/chakib-belgaid/Wattch/actions/workflows/ci.yml)

Status: v0.1 exploratory prototype.

Wattch currently demonstrates a minimal Rust RAPL daemon/CLI pipeline,
protobuf framing over Unix sockets, and a deterministic fake backend for
repeatable tests. The current version is not a production profiler and does
not claim process-level or function-level energy attribution.
Current work is focused on validation, reproducibility, and a cleaner
protocol-first design before adding more hardware backends.

- `rapl-wattchd`: local RAPL Unix socket server and sampling loop
- `wattch`: user-facing CLI built by the `wattch-cli` crate
- `wattch-core`: shared framing, validation, time, and powercap helpers
- `wattch-proto`: protobuf types generated with `prost`

## Status

This repository is the original Wattch v0 prototype.

It validated the feasibility of a local Rust-based energy measurement daemon
and CLI using RAPL, protobuf framing, and Unix sockets.

Active work is moving toward a production-oriented rewrite focused on a
backend-independent protocol, deterministic validation, and reproducible
measurement harnesses.

## v0.1-alpha scope

v0.1-alpha is focused on the local Rust daemon, Rust CLI, protobuf framing over Unix sockets, RAPL source discovery, and deterministic tests.

It intentionally excludes non-Rust clients, service APIs, dashboards, databases, report generation, AI integration, profiler integration, and plugin systems.

## Release packages

GitHub Releases build Linux x86_64 packages for:

- Debian stable (`.deb`)
- Ubuntu 24.04 LTS (`.deb`)
- Fedora latest (`.rpm`)
- Rocky Linux 9 / RHEL-compatible systems (`.rpm`)
- openSUSE Tumbleweed (`.rpm`)

The packages install:

- `wattch` to `/usr/bin/wattch`
- `rapl-wattchd` to `/usr/sbin/rapl-wattchd`

## Protocol

The daemon and CLI use Unix domain sockets. Every protobuf message is framed as:

```text
[4-byte little-endian uint32 payload_length][protobuf payload]
```

The maximum payload size is 1 MiB. There is no gRPC, HTTP, JSON wire protocol, database, persistent report system, plugin system, or profiler integration in this MVP.

## Runtime

Default config file:

```text
/etc/wattch/wattch.conf
```

Default service socket path:

```text
/run/wattch/wattch.sock
```

The daemon discovers Linux RAPL powercap zones under:

```text
/sys/devices/virtual/powercap/intel-rapl
```

`rapl-wattchd` is expected to run as root when powercap permissions require it. When started through `sudo`, it uses `SUDO_UID` and `SUDO_GID` to hand the root-created socket to the invoking user with mode `0600`, so `wattch` can run without root.

Example config:

```ini
# /etc/wattch/wattch.conf
socket_path = "/run/wattch/wattch.sock"
socket_mode = 0600

# Optional for system services not launched through sudo:
# socket_uid = 1000
# socket_gid = 1000
```

For deterministic tests and local experiments:

- `WATTCH_CONFIG` overrides the config file path.
- `WATTCH_SOCKET` overrides the socket path.
- `WATTCH_POWER_CAP_ROOT` overrides the powercap root.
- `WATTCH_SOURCE_BACKEND` selects the daemon source backend:
  - unset, `powercap`, or `rapl`: discover real Linux powercap/RAPL sources.
  - `fake`: use one deterministic in-memory source named `fake:deterministic`.

The fake backend is configured only on the daemon process. The CLI only needs to point at the same Unix socket. The fake source does not require root or a Linux powercap sysfs tree.

Full details are in [docs/deterministic-source.md](docs/deterministic-source.md).

## Commands

```sh
cargo build -p rapl-wattchd -p wattch-cli
sudo ./target/debug/rapl-wattchd
./target/debug/wattch hello
./target/debug/wattch sources --format table
./target/debug/wattch sources --format csv
./target/debug/wattch stream --interval-ms 100 --duration 5s --format table
./target/debug/wattch run -- cargo test
./target/debug/wattch run --format csv -- cargo test
```

`wattch run -- <command>` is the command wrapper for energy measurement. It
uses the same daemon connection settings as the other CLI commands: first the
default `/etc/wattch/wattch.conf` or `WATTCH_CONFIG`, then `WATTCH_SOCKET` as an
environment override for the socket path. The wrapper reports RAPL energy
observed while the child command runs; it does not provide function-level or
process-exclusive attribution.

Deterministic fake backend example:

```sh
cargo build -p rapl-wattchd -p wattch-cli

export WATTCH_SOCKET=/tmp/wattch-fake.sock
rm -f "$WATTCH_SOCKET"
WATTCH_SOURCE_BACKEND=fake cargo run --quiet -p rapl-wattchd --bin rapl-wattchd
```

In another terminal:

```sh
export WATTCH_SOCKET=/tmp/wattch-fake.sock
cargo run --quiet -p wattch-cli --bin wattch -- sources
cargo run --quiet -p wattch-cli --bin wattch -- stream --source 1 --interval-ms 10 --duration 50ms --format csv
cargo run --quiet -p wattch-cli --bin wattch -- run --source 1 --interval-ms 10 -- sh -c "sleep 0.05"
```

Or run the scripted end-to-end smoke check:

```sh
scripts/smoke_fake_backend.sh
```

## Quality gate

```sh
cargo metadata --locked --no-deps --format-version 1 > /dev/null
cargo fmt --all --check
cargo clippy --locked --workspace --all-targets -- -D warnings
cargo test --locked --workspace
cargo build --locked --workspace
```
