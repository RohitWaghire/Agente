# Define per-crate definition of done and porting order

Type: grilling
Status: open
Blocked by: 03, 04

## Question

Define what "working" means for each crate given the foundations-first milestone shape (ignore → core → provider → agent CLI → TUI), and the order tickets land:

- Per-crate exit criteria without a user-facing binary: unit tests, fixture replay, differential probes?
- How the agent crate lands modes incrementally (print/stream/JSON/RPC before interactive).
- When the first runnable Rust binary appears and what it must do.
- How regression is caught while the TUI is still missing.

Conforms to 03 and 04.
