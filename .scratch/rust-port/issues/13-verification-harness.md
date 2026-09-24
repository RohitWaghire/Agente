# Design the golden-fixture verification harness

Type: grilling
Status: open
Blocked by: 03

## Question

Design the verification harness per the parity contract (03):

- Install Go (winget) and confirm `go test ./...` is green as the baseline.
- Fixture inventory and generation commands: RPC transcripts, `--json` event streams, session files, provider wire captures against fake HTTP servers, extension host exchanges, exit codes/stdout/stderr.
- Where fixtures live in-repo and how Rust tests replay them; how SSE/retry cases become deterministic.
- When live differential probing against the Go binary is used vs committed goldens.

This is the backbone of every later port ticket.

Note (from resolving 03): baseline prerequisites — `packages/agent/session_environment_test.go:52` calls an undefined `setAgenteSessionEnvironment`, so `go test ./agent` will not compile; fix it before confirming the green baseline. Fixture generation pins the Go commit SHA in the fixture manifest, and the Go tree is committed as the freeze baseline first (user-sanctioned in 03). Contract: `.scratch/rust-port/parity-contract.md`.
