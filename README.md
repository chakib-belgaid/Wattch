
# wattch

Energy profiling infrastructure for developers and AI coding agents.

## Problem

Most developers cannot measure the energy impact of code changes in a practical way. Existing tools are often deprecated, require root access, are hard to integrate into developer workflows, or assume clean benchmarking machines.

## Goal

Build an open-source protocol and toolchain for collecting, analyzing, and reporting software energy measurements across languages and developer tools.

## Initial MVP

- RAPL-based local energy measurement
- Rust CLI
- Python profiling integration
- pyinstrument-compatible reporting
- simple benchmark runner
- machine-readable JSON output
- first report format for humans and AI agents

## Long-term direction

- VSCode extension
- MCP server for coding agents
- plugin system for multiple languages
- energy regression detection
- AI-assisted report generation

## Why this matters

AI coding agents will generate more code faster. Developers need feedback loops that make performance and energy impact visible before inefficient software silently compounds.

## Current status

Early architecture and MVP design phase.

## Next milestone

Produce a minimal working local measurement demo:
`run command -> collect RAPL data -> output JSON report`.
