# Rust port of Agente

> **Archived snapshot.** The canonical tracker for this effort moved to GitHub: map issue https://github.com/rohityin/Agente/issues/18 with children #1–#17. This file and `issues/` are the old local-markdown tracker, kept for history; do not edit them for tracker purposes.

Label: wayfinder:map

## Destination

A Rust implementation of Agente at full behavioral parity with the frozen Go implementation: same CLI and modes, providers, hand-rolled terminal UX, RPC v1, extension protocol v1, swarm, Telegram bot, skills, agentefile, and self-update. It lives in a root Cargo workspace alongside the Go code until cutover, targets windows/linux/darwin × amd64/arm64, and is verified against committed Go-generated golden fixtures.

## Notes

- Domain: a Go-to-Rust migration of a coding-agent harness. Use `grilling` + `domain-modeling` for decisions, `research` for external facts, `prototype` for TUI-fidelity questions.
- Execution is in scope for this effort (user decision): once the route is clear, tickets may carry porting work. Decisions come first.
- Frozen contract: persisted formats (sessions JSONL, auth.json, config.json, models.json, skill-pins.json) and wire protocols (RPC NDJSON v1, extension protocol v1) stay compatible. `ZOT_*` env aliases and `zotfile` names are dropped.
- Idiomatic Rust ecosystem beats footprint-for-footprint parity with Go's minimal-dependency stance; every dependency still needs its reason.
- The Go implementation is feature-frozen (bug fixes only) and is the oracle. Go is not installed on this machine yet; install via winget when fixture generation starts.
- Verification: golden fixtures generated once from the Go binary and committed, replayed by Rust tests; CI stays Go-free.
- Milestone shape: foundations first (ignore → core → provider → agent CLI → TUI), each crate landing with tests before any user-facing binary is claimed.
- The TUI rendering stack is the first risk attacked (user priority).
- Platform validation: Windows locally; macOS/Linux deferred to CI once a git remote exists. Targets remain all five.
- Tracking convention: local markdown under `.scratch/rust-port/` because the repo has no issue tracker configured. A ticket is claimed by `Status: claimed` and resolved by appending `## Answer` and setting `Status: resolved`.

## Decisions so far

- [Survey Rust TUI crates against Agente's renderer requirements](issues/01-tui-stack-survey.md): crossterm as the platform layer, ratatui viable only for the alt-screen history viewport; flow renderer, view builders, 256-color mapping, markdown/partial-JSON, and image/clipboard layers stay custom. Findings in `.scratch/rust-port/research/tui-stack-survey.md`.
- [Survey Rust async/networking/TLS crates for provider parity](issues/05-rust-net-stack-survey.md): tokio + reqwest with rustls-platform-verifier; hand-rolled SSE parser; smithy event-stream + SigV4 for Bedrock; proxy/timeout/cancellation divergences flagged. Findings in `.scratch/rust-port/research/rust-net-stack-survey.md`.
- [Choose the TUI rendering stack](issues/02-tui-render-stack.md): crossterm 0.29 as the platform/input layer plus a custom ported flow renderer; ratatui rejected entirely, alt-screen history included. Spike at `.scratch/rust-port/prototype/tui-spike/` (mid-stream resize without duplication, idle paints emit nothing, headless tests; 24 crates vs 64).
- [Define the executable parity contract](issues/03-parity-contract.md): E/S/B/D bars with the Go code as oracle; state files schema-exact round-trip; machine surfaces exact, prose faithful but behavioral; Go quirks reproduced wholesale (register of 20); deterministic intercept order; provider identity headers byte-exact, retry by outcome corpus; TUI protocol escapes exact; docs layout exact/content rewritten; fixtures pin a Go commit. Contract: `.scratch/rust-port/parity-contract.md`.

## Not yet specified

- Swarm parity: Unix-socket transport (already degraded on Windows), self-exec child contract, credential-over-stdin, event-log persistence, inbox semantics.
- Telegram bot and daemon lifecycle (long polling, state files, start/stop, in-TUI bridge).
- Skills discovery and parser parity (hand-rolled SKILL.md frontmatter subset, pins, shadowing diagnostics, built-in skills).
- Agentefile pack/inspect/verify/run and archive format.
- Self-update mechanism while the repo has no git remote.
- Image protocols, clipboard, and OSC 11 detection porting details beyond the survey.
- Provider-specific wire quirks (Bedrock SigV4 + event stream, Codex backend, Copilot token/host rewrite, Vertex JWT, Anthropic impersonation, Gemini image writes); graduate after the provider architecture lands.
- Model catalog generation/verification and discovery caching.
- Docs embedding, user documentation rewrite, and the AGENTS.md Rust workflow section.
- Platform matrix details: process control, file permissions, PowerShell tool, path handling, terminal resize.

## Out of scope

- `ZOT_*` env aliases, `zotfile` subcommand names, and other pre-rename back-compat (dropped at charting).
- New features or UX redesigns absent from the frozen Go implementation; this effort is parity-only.
- Keeping the Go implementation alive past cutover other than as the oracle.
