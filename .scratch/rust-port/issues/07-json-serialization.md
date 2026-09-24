# Choose the JSON and serialization strategy

Type: grilling
Status: open
Blocked by:

## Question

Decide how Rust models Go's dynamic JSON surfaces:

- `provider.Content` sum type: untagged enums, or tagged by field presence?
- `json.RawMessage` tool args and schemas: serde `RawValue` or `Value`?
- `map[string]any` RPC/extension frames: typed structs with `#[serde(flatten)]` extras, or `Value`?
- Unknown-field tolerance, number fidelity, insertion order where it matters.
- Permissive parsing behavior (Go silently skips malformed SSE chunks; JSON round-trip loses concrete types).
- serde vs hand-rolled parsing for the compatibility surfaces.

This binds the session/state strategy (08), provider architecture (09), extension host (11), and RPC (12).
