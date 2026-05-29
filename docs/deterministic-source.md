# Deterministic Source Backend

Wattch can run `rapl-wattchd` with a deterministic in-memory source backend. This is intended for repeatable tests, local CLI smoke checks, and development on machines without readable Linux RAPL powercap files.

It is not a real energy measurement backend.

## Configuration

Set `WATTCH_SOURCE_BACKEND` on the daemon process:

```sh
WATTCH_SOURCE_BACKEND=fake
```

Supported values:

```text
unset        use the default Linux powercap/RAPL backend
powercap     use the Linux powercap/RAPL backend
rapl         alias for the Linux powercap/RAPL backend
fake         use the deterministic in-memory backend
```

Invalid values make `rapl-wattchd` exit during startup.

The fake backend is selected by the daemon only. The CLI does not read `WATTCH_SOURCE_BACKEND`; it talks to whichever daemon is listening on `WATTCH_SOCKET`.

Use `WATTCH_SOCKET` to keep local fake-backend runs isolated from the system daemon:

```sh
export WATTCH_SOCKET=/tmp/wattch-fake.sock
```

`WATTCH_POWER_CAP_ROOT` is only used by the `powercap`/`rapl` backend. The fake backend ignores powercap sysfs paths and does not require root.

## Source Behavior

The fake backend exposes one available source:

```text
ID  NAME                KIND  UNIT   AVAILABLE
1   fake:deterministic  fake  joule  yes
```

The source starts at `100.0 J` and advances by `5.0 J` on every daemon read. When a stream starts, the daemon first takes an internal baseline reading, then emits samples after each requested interval:

```text
first emitted sample   energy_j=105.0  delta_j=5.0
second emitted sample  energy_j=110.0  delta_j=5.0
third emitted sample   energy_j=115.0  delta_j=5.0
```

Power is still computed by the normal sampler:

```text
power_w = delta_j / (interval_ns / 1_000_000_000.0)
```

For example, a `10 ms` interval produces `5 J / 0.01 s = 500 W` samples. A `100 ms` interval produces `50 W` samples.

## Manual Usage

Build the daemon and CLI:

```sh
cargo build -p rapl-wattchd -p wattch-cli
```

Start the daemon in one terminal:

```sh
export WATTCH_SOCKET=/tmp/wattch-fake.sock
rm -f "$WATTCH_SOCKET"
WATTCH_SOURCE_BACKEND=fake cargo run --quiet -p rapl-wattchd --bin rapl-wattchd
```

Use the CLI from another terminal with the same socket path:

```sh
export WATTCH_SOCKET=/tmp/wattch-fake.sock

cargo run --quiet -p wattch-cli --bin wattch -- hello
cargo run --quiet -p wattch-cli --bin wattch -- sources
cargo run --quiet -p wattch-cli --bin wattch -- stream --source 1 --interval-ms 10 --duration 50ms --format csv
cargo run --quiet -p wattch-cli --bin wattch -- run --source 1 --interval-ms 10 -- sh -c "sleep 0.05"
```

Stop the daemon with `Ctrl-C` when finished.

## Smoke Test

The repository includes an end-to-end fake-backend smoke script:

```sh
scripts/smoke_fake_backend.sh
```

The script:

- builds `rapl-wattchd` and `wattch`
- starts the daemon with `WATTCH_SOURCE_BACKEND=fake`
- uses an isolated temporary Unix socket
- runs `wattch hello`
- verifies `wattch sources`
- verifies `wattch stream`
- verifies `wattch run`

On success it prints exactly:

```text
wattch smoke test passed
```
