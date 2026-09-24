# Extension host parity and Rust SDK scope

Type: grilling
Status: open
Blocked by:

## Question

Decide the Rust extension host's behavior and scope:

- Handshake (`hello`/`hello_ack`), readiness (explicit `ready` frame vs 250 ms idle heuristic, shared 3 s deadline).
- Intercept-chain semantics, including today's fail-open behavior on timeout (5 s).
- Outbound queue overflow behavior, panel frames, command/tool registration precedence.
- Process cleanup escalation and stderr log handling.
- Whether a Rust extension SDK ships; the Go SDK and protocol v1 stay regardless.

Intentional divergences must be recorded here, not discovered accidentally later.
