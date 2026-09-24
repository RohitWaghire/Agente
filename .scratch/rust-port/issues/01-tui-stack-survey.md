# Survey Rust TUI crates against Agente's renderer requirements

Type: research
Status: resolved
Blocked by:

## Question

Which Rust crates can reproduce the behavior of `packages/tui` and the interactive mode's use of it, and which gaps force custom code? Cover at minimum:

- Flow-mode inline rendering with terminal scrollback (not alt-screen), bottom-aligned diff repaints, `\x1b[3J` suppression under VS Code, synchronized-output fencing, scroll regions, DECSC/DECRC.
- Streaming markdown rendering (headings, lists, GFM tables, inline code/bold/italic), partial-JSON tool previews, 16 ms redraw throttle, typewriter pacer.
- Syntax highlighting parity with chroma v2 (monokai-derived style, terminal256, 18 token overrides, lexer selection by filename/extension) — evaluate syntect and alternatives, including style/token-taxonomy mapping effort.
- Terminal input: raw mode, resize, bracketed paste, kitty CSI-u / modifyOtherKeys, SGR mouse wheel, 50 ms Esc peek, Ctrl+V clipboard.
- Unicode display width and ANSI-aware wrapping/truncation (runewidth + uniseg behavior); candidate crates and known divergences (VS16/ZWJ/CJK).
- Image protocols (kitty `a=T,f=100,C=1`, iTerm2 OSC 1337 `doNotMoveCursor`), PNG re-encoding, footprint math.
- Per-OS clipboard text/image (AppleScript, wl-paste/xclip/xsel, PowerShell STA) — crate vs command-based.
- Auto light/dark detection via OSC 11.

Deliverable: a dense findings file at `.scratch/rust-port/research/tui-stack-survey.md` citing primary sources (crate docs and source), with a capability matrix and an explicit list of gaps that must be custom. This feeds the stack decision in ticket 02.

## Answer

Resolved by research subagent. Findings: `.scratch/rust-port/research/tui-stack-survey.md`.

- Platform layer: crossterm 0.29 covers raw mode, size/resize, bracketed paste, kitty CSI-u, SGR mouse wheel, synchronized update, purge (3J), alt-screen, DECSC/DECRC, DECSCUSR. It does **not** parse xterm modifyOtherKeys (`CSI 27;~`), so that fallback needs custom handling.
- ratatui 0.30 is viable only for the alt-screen history viewport; its inline viewport is not a drop-in for flow mode and scroll regions are unsupported on Windows. The flow renderer stays custom (~900–1,300 LOC), plus view/timeline builders ported from `view.go` (~2,500–3,500 LOC).
- Highlighting: syntect 5.3 + two-face 0.5 with a custom 24-bit→256 mapper (syntect has no 256-color formatter; chroma uses compuphase redmean). Default Onig backend needs a C toolchain on Windows; two-face excludes PowerShell/Sass under fancy-regex. Scope-mapping chroma's monokai + 18 overrides is ~300–500 LOC.
- Width: unicode-width 0.2 + unicode-segmentation 1.13 close most of the gap; VS16/ZWJ/CJK divergences remain explicit test targets.
- Other custom gaps: markdown + partial-JSON streaming (~600–900 LOC), clipboard/image platform layer incl. kitty/iTerm2 encoders (~600–950 LOC). termion is excluded (Unix-only). iTerm2 `doNotMoveCursor` is undocumented upstream (Agente already defaults iTerm2 off).
- Context7 spot-check (2026-09-24) confirmed ratatui `insert_before` scrollback behavior; verdicts unchanged. See §7 of the findings file.
