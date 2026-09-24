# Provider architecture and model catalog representation

Type: grilling
Status: open
Blocked by: 06

## Question

Decide the provider trait/module architecture and the catalog:

- Client trait shape (async stream handle vs channel) and how per-provider backends are organized.
- Ownership of request/response conversion; how the model router selects by API type.
- Representation of the ~731-model builtin catalog (generate Rust from the Go literals, embed JSON, or build-time codegen) plus discovery, caching, merge, and user-model precedence.
- Reasoning-level mapping and retry classification boundaries.

Constraints: provider quirks stay inside this crate; identity-impersonation headers (Claude Code, Codex CLI, Copilot) must remain byte-exact; conform to 03/06/07.
