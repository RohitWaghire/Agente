# Define the executable parity contract

Type: grilling
Status: resolved
Blocked by:

## Progress

Claimed, inventory gathered across six exploration passes (CLI/config, wire protocols, persisted state, peripheral subsystems, terminal/tools/platform, provider HTTP), then resolved in a HITL grilling round. Contract table written to `.scratch/rust-port/parity-contract.md`.

## Question

Enumerate every externally observable surface of the Go implementation and, for each, state the parity bar (exact vs behavioral) and how it is verified. At minimum:

- CLI flags, exit codes, stdout/stderr for every mode.
- `--json` event schema and RPC v1 frames and behaviors.
- Extension protocol v1 and host lifecycle.
- Session JSONL; auth.json; config.json; models.json; skill-pins.json.
- Skills discovery and SKILL.md parsing; docs installation layout.
- Environment variables; file permissions and atomic writes.
- Platform-specific behaviors (clipboard, process control, terminal).

Output is a contract table that later tickets and the fixture harness consume. Decisions here bind the port.

## Answer

Resolved in a HITL grilling session. The contract table is `.scratch/rust-port/parity-contract.md` and is binding; later-discovered surfaces are added by amendment, never shipped outside it.

Headline decisions:

- **Bars**: E (exact bytes/streams), S (schema-exact), B (behavioral), D (recorded divergence). Oracle is the frozen Go code; docs are corrected to match it.
- **State files**: schema-exact round-trip in both directions, matching Go's serialization conventions per file (indent, trailing newline, row/field order); volatile values (ids, timestamps, paths) normalized in fixtures.
- **Text**: exact for exit codes, stream routing, RPC/`--json` frames, print/stream payloads, digest lines; human wording copied faithfully (no cleanups) but verified behaviorally, not byte-locked.
- **Quirks**: reproduced wholesale from a 20-item register (payload-less compact events, no `done` after compact, pre-auth continuation, usage==cumulative writes, dead `panel_resize`, etc.), each with a regression test.
- **Determinism**: extension intercept chains use deterministic load order (D1); other Go nondeterminism is preserved as behavior.
- **Provider wire**: identity headers (Claude Code, codex_cli_rs, Copilot) byte-exact; schemas exact; SSE behavior exact; retry parity by classification outcome corpus (D7); transport differences documented (D6).
- **TUI**: protocol escapes exact (fencing, alt-screen counts, cursor visibility, 3J rules, OSC 7/12, image headers); layout/highlighting behavioral via headless ANSI-emulator goldens; PNG encode bytes implementation-local (D8).
- **Docs/version**: `$AGENTE_HOME/docs` layout exact, content rewritten (D3); Rust reports its own version in the exact `agente <v>` shape (D4).
- **Freeze**: fixtures pin a Go commit SHA; the Go tree is committed as the freeze baseline before generation (user-sanctioned); the broken `session_environment_test.go` must be fixed first so `go test ./...` is green.

Facts carried into the contract: `config.json` has 31 top-level keys (not ~45); `packages/agent/session_environment_test.go:52` references an undefined `setAgenteSessionEnvironment` (baseline broken); ~15 doc-vs-code drifts are recorded for correction during the docs rewrite.
