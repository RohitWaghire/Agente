# Choose async runtime, HTTP client, and TLS stack

Type: grilling
Status: open
Blocked by: 05

## Question

Decide the provider-layer stack: runtime/concurrency model (tokio vs threads), streaming channel type (mpsc vs broadcast vs callbacks), cancellation primitive, HTTP client and TLS backend, and how request construction and retry replay work.

Weigh behavioral parity risks (proxy support, Windows certificates, streaming edge cases) against dependency weight. This decision feeds the provider architecture (09), the error/cancellation model (10), and the fixture harness (13).
