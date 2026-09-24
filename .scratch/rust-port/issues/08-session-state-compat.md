# Session and state-file compatibility strategy

Type: grilling
Status: open
Blocked by: 07

## Question

Decide how Rust reads and writes existing state at exact compatibility:

- Sessions JSONL: meta/message/compaction/usage/rename rows; hydration by field presence; compaction replay; orphan tool-pair repair; fork/reparent; usage deltas; `PruneEmptySessions`/`ListSessions`/`RenameSession` semantics; path scheme `sha256(cwd)[:8]`.
- auth.json: 0600 atomic writes, OAuth token fields, credential lookup/refresh.
- config.json: ~45 keys, tri-state booleans, precedence rules.
- models.json and skill-pins.json.

State what is byte-exact vs tolerant, and how golden fixtures verify each. This conforms to the parity contract (03) and the JSON strategy (07).
