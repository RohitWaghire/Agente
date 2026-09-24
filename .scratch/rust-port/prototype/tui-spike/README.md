# tui-spike — TUI rendering stack spike (PROTOTYPE)

Throwaway artifact for wayfinder ticket `.scratch/rust-port/issues/02-tui-render-stack.md`.
It answers one question: **can a crossterm-only renderer (crossterm as the
platform/input layer, a ported custom flow renderer for output) reproduce
agente's inline streaming UX, and can it be verified without a real
terminal?**

This is not production code. It is deliberately small, unpolished, and
will be deleted once the decision it feeds is recorded.

## Run

```sh
cd .scratch/rust-port/prototype/tui-spike
cargo run -- demo     # interactive: streaming turn, tool box, partial JSON
cargo run -- script   # headless scripted session + emulator report
cargo test            # assertions, incl. resize without duplicate transcript
```

`demo` keys: resize the real window mid-stream (the case under test);
`r` cycles synthetic widths; `h` toggles an alt-screen history view;
space starts another turn when idle; `q` quits.

## What to watch in `demo`

1. Assistant text streams in place; committed rows scroll into the
   terminal's native scrollback (scroll up mid-stream).
2. The live tool box grows from partial JSON as the `write` args arrive
   (only `content` is extracted; the box caps at 14 preview rows).
3. **Resize mid-stream**: the transcript is rebuilt from raw model state
   at the new width, then cleared and repainted once. No duplicated
   history, no stale narrow frame.
4. Idle: the renderer emits nothing when neither buffer nor cursor
   changed, so the terminal's own cursor blink is not reset every tick.
5. `h`: the alt-screen history view is a separate full-frame path; the
   custom renderer covers it without a framework.

## Headless results (Windows, 2026-09-24)

`cargo run -- script` — 80x24, resize to 60x20 mid-stream after the tool
box completed, while a later assistant delta was still arriving:

```
frames painted: 104 (full repaints: 2)
bytes emitted across paints: 22273; idle repaint bytes (unchanged state, 2 draws): 0
scrollback rows at resize: 9; scrollback purges: 3; final scrollback rows: 20
sentinel occurrences in scrollback+screen (1 = no duplicate):
  "addressable": 1  "reflows": 1  "throttle": 1  "partial": 1  "wrote 9 lines": 1
```

`cargo test` asserts the same properties (idle no-op emits zero bytes,
append patches less than a full repaint, post-resize transcript contains
each sentinel exactly once, partial-JSON extraction matches the Go
extractor's escape handling).

The scripted session runs the renderer against a `Vec<u8>` and feeds the
bytes through a small ANSI emulator, so none of the assertions need a
terminal. That is the intended shape for the real port's renderer tests.

## Dependency cost

`cargo tree` on this spike (Windows, default features): **1 direct
dependency (crossterm 0.29), 24 unique crates** including the spike.
A throwaway probe with `ratatui 0.30` default features resolves to
**64 unique crates** (ratatui + its backend, palette, layout, text,
unicode-width/segmentation, etc.), sitting on the same crossterm.

## Deliberate simplifications (do not trust these areas)

- Width math is `chars().count()`, not cell width; no ANSI-aware
  truncation, no grapheme/VS16/CJK handling.
- No markdown, syntax highlighting, images, clipboard, or OSC 11.
- No input editor; the bottom band is a status line + fake prompt.
- The reflow-rebase branch of `render.go:702-753` (chat height changing
  above the viewport) is omitted; the spike relies on full repaint for
  that class of change.
- Tool box/side rendering is a rough visual mimic of `view.go`, not a
  fidelity port (no selection, collapse, images, per-row styles).
- The scripted emulator implements only the escape subset the renderer
  emits; it is not a general terminal model.
