
# wattch

Wattch is a minimal local energy measurement daemon and CLI.

This scaffold intentionally contains only Rust code:

- `rapl-wattchd`: local RAPL Unix socket server and sampling loop
- `wattch`: user-facing CLI built by the `wattch-cli` crate
- `wattch-core`: shared framing, validation, time, and powercap helpers
- `wattch-proto`: protobuf types generated with `prost`

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

## Verification

```sh
cargo fmt
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo build --workspace
```
