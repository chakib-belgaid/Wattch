# Wattch Archive Record

Status: frozen and archived.

Archive date: 2026-06-04.

Reason: Wattch achieved its intended goal: validate that a minimal Rust daemon
and Rust CLI can discover Linux RAPL powercap sources, read cumulative
`energy_uj` counters, compute energy/power deltas, and expose those samples over
a local protobuf Unix-socket protocol.

## Frozen Scope

The archived project scope is:

- Rust daemon: `rapl-wattchd`
- Rust CLI: `wattch`
- Shared Rust crates: `wattch-core` and `wattch-proto`
- Protobuf length-prefixed Unix-socket wire protocol
- RAPL powercap discovery under `/sys/devices/virtual/powercap`
- CLI commands: `hello`, `sources`, `stream`, and `run`
- Deterministic tests using fake powercap trees and fake daemons

The archived project intentionally does not include other clients, dashboards,
HTTP/gRPC APIs, databases, report generation, AI integrations, profiler
integrations, or plugin systems.

## Maintenance Policy

Do not expand the product surface while the project is archived. Acceptable
future changes are limited to:

- Critical correctness fixes
- Security or dependency maintenance
- Build or CI fixes needed to keep the archived project reproducible
- Documentation clarifications that do not change scope

Any feature work should first explicitly unarchive the project and update this
record.

## Verification Gate

Before treating the archived state as valid, run:

```sh
cargo metadata --locked --no-deps --format-version 1 > /dev/null
cargo fmt --all --check
cargo clippy --locked --workspace --all-targets -- -D warnings
cargo test --locked --workspace
cargo build --locked --workspace
```
