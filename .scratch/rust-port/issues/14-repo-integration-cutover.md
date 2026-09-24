# Plan repo integration, build, and Go cutover

Type: grilling
Status: open
Blocked by: 04

## Question

Plan how the Rust workspace integrates with the repo and tooling:

- Makefile/CI targets; binary naming and version injection (replace `-ldflags` with `build.rs`/env).
- Docs embedding and `$AGENTE_HOME/docs`; README and AGENTS.md updates (Rust dev workflow section).
- Cross-platform build and verification strategy (Windows local, CI when a remote exists).
- Final cutover criteria and removal of the Go implementation; fate of Go-only tooling such as `make test`.

Conforms to the crate graph (04) and parity contract (03).
