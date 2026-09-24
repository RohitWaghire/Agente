# Choose the TUI rendering stack

Type: prototype
Status: resolved
Blocked by: 01

## Question

Given the survey findings (01), decide how the Rust TUI is built: a framework (e.g. ratatui + crossterm), crossterm + a ported custom renderer, or a hybrid (framework primitives + custom flow renderer). Raise fidelity with a rough spike demonstrating the riskiest behavior: inline flow rendering of a streaming assistant turn with a tool box and a live partial-JSON preview, resized mid-stream.

Decision criteria: behavior parity with `packages/tui`, testability without a real terminal, dependency cost, and Windows input limitations. This is HITL: the user reacts to the spike. Record the decision and where the spike lives.

## Progress

Spike built at `.scratch/rust-port/prototype/tui-spike/` (read its README for the run/reaction guide). `cargo test` green on Windows: idle paints emit zero bytes, append diffs less than a full repaint, and a mid-stream resize (80x24 -> 60x20, after 9 rows had scrolled) repaints from raw state with each transcript sentinel appearing exactly once; the scripted session runs against a `Vec<u8>` + a small ANSI emulator, so it asserts without a terminal. Dependency cost measured: crossterm-only resolves to 24 unique crates; a `ratatui 0.30` probe resolves to 64. Awaiting the user's reaction to the demo before recording the decision.

## Answer

Resolved in a HITL session: the user reacted to the spike and chose option (b), with ratatui dropped entirely.

**Decision: crossterm 0.29 as the platform/input layer plus a custom ported flow renderer.** No framework and no hybrid — the spike's full-frame alt-screen path covers the history view that would have been ratatui's only remaining role.

Evidence: the spike reproduced the riskiest behavior — a streaming assistant turn with a live tool box and partial-JSON preview, mid-stream resize (80x24 -> 60x20 after 9 rows had scrolled) rebuilding every row from raw model state, repaint with each transcript sentinel appearing exactly once, idle paints emitting zero bytes — with 1 direct dependency / 24 unique crates versus 64 for a `ratatui 0.30` probe. The renderer was verified headlessly against `Vec<u8>` plus a small ANSI emulator, which fixes the intended test shape for the real port (no terminal required).

Assets: spike at `.scratch/rust-port/prototype/tui-spike/` (`cargo run -- demo` to reproduce, `cargo run -- script` for the headless report, `cargo test` 6/6 green). Simplified areas and known gaps are listed in the spike README.

Follow-ups created: [#16 Design interactive dialogs and slash commands](16-dialog-slash-command-architecture.md), [#17 Decide the xterm modifyOtherKeys fallback](17-modifyotherkeys-fallback.md).
