# RPC server concurrency and parity approach

Type: grilling
Status: open
Blocked by:

## Question

Decide the Rust RPC server design:

- NDJSON framing with 16 MiB lines, one turn at a time.
- Busy responses for `clear`/`set_model`/`set_reasoning` while a turn runs.
- In-flight wait when stdin closes; process exit semantics.
- Token auth handshake (`AGENTE_RPC_TOKEN`).
- Event schema shared with `--json`; prompt `started` response and `done` emission.

Protocol v1 is additive-only: nothing existing may change. Conforms to the parity contract (03).
