# Decide the xterm modifyOtherKeys fallback

Type: grilling
Status: open
Blocked by:

## Question

crossterm 0.29 is now the locked platform/input layer ([Choose the TUI rendering stack](02-tui-render-stack.md)), and it cannot parse xterm modifyOtherKeys (`CSI 27;<mod>;<code>~`): its CSI `~` dispatcher only accepts first params 1-8/11-34, so 27 yields a parse error (survey §1 row 13, `parse.rs`). Agente enables modifyOtherKeys as a fallback so modified keys (e.g. Shift+Enter) work in terminals without the kitty keyboard protocol (`terminal.go:116-117`).

Decide how the port handles it:

- (a) accept the loss: modified keys work only on kitty-protocol terminals (Ghostty, Kitty, recent xterm.js/VS Code);
- (b) patch or fork crossterm: keeps one parser but adds a maintenance/release burden contrary to the dependency policy;
- (c) keep agente's own byte-level input parser for the fallback path alongside crossterm's event stream.

If (c), settle how it coexists with the 50 ms bare-Esc peek (`input.go:169`) and bracketed-paste payload handling without double-consuming bytes. Deliverable: the chosen approach plus a verification strategy driven by fixture-fed byte sequences (no real terminal required).
