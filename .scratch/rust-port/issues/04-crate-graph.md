# Define the Rust workspace crate graph

Type: grilling
Status: open
Blocked by: 03

## Question

Define the Cargo workspace: crate list and names (mirroring `core`/`provider`/`agent`/`tui`/`ignore`), dependency direction, where shared protocol/state types live, feature flags, the binary crate, and the public API each crate exposes.

Constraints: root Cargo workspace with Go staying until cutover; respect the ownership map's separation (provider quirks must not leak into core; terminal handling only in tui); conform to the parity contract (03).
