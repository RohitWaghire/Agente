# Design interactive dialogs and slash commands

Type: grilling
Status: open
Blocked by:

## Question

The TUI stack is settled ([Choose the TUI rendering stack](02-tui-render-stack.md)): crossterm 0.29 plus a custom flow renderer, with a bottom band on the main screen and an alt-screen history view. Now pin down the interactive-surface layer at parity with the frozen Go implementation:

- Inventory the dialogs in the Go tree (model picker, session picker/fork, theme picker, settings, swarm manager, agentefile picker, permission prompts, and whatever else `packages/agent/modes/interactive.go` actually opens) and decide how each composes with the custom renderer: live bottom band vs alt-screen overlay vs inline chat block.
- Input routing: editor keys vs dialog shortcuts vs permission confirmations, and how a dialog suspends and restores flow-renderer state on open/close (and on resize while open).
- Slash-command surface: registration and discovery, help/autocomplete behavior, argument parsing, mode-specific commands, and how commands map onto dialogs vs direct actions.
- Flag which pieces are appearance/behavior questions that deserve a `prototype` spike before building (e.g. pickers) instead of being decided on paper.

Deliverable: a decision on the dialog/overlay architecture and the slash-command surface, expressed against the custom renderer paths, plus any prototype-spike tickets it surfaces.
