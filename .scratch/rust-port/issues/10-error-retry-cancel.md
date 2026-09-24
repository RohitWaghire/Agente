# Error, retry, and cancellation parity model

Type: grilling
Status: open
Blocked by: 06

## Question

Decide how Rust represents lifecycle semantics Go expresses implicitly:

- Retry classification (currently string-matched on Go error text) and synthesized HTTP errors after exhausted retries.
- Terminal-quota detection and `retry-after` handling.
- Cancellation propagation into streams and tools (Go `context.Context` → Rust primitive).
- Panic recovery (`recover()` → `catch_unwind` or structured errors).
- Tool status values (`completed`/`failed`/`blocked`/`cancelled`/`timed_out`) and confirm-gate semantics.

Goal is behavior parity, not API mimicry. Record intentional Rust divergences and why parity still holds.
