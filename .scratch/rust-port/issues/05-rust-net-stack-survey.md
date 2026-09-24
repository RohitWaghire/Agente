# Survey Rust async/networking/TLS crates for provider parity

Type: research
Status: resolved
Blocked by:

## Question

Survey the Rust ecosystem against the provider layer's needs and report facts, not decisions:

- Async runtime options (tokio vs std threads with blocking HTTP) and their consequences for streaming and cancellation.
- HTTP clients and HTTP/2; SSE support or the need for a custom parser (multi-line `data:`, 10 MiB lines, keep-alive comments, `[DONE]`).
- TLS backends (rustls vs native-tls) for the Windows certificate store and corporate proxies; proxy env handling; request-body replay for retries.
- AWS SigV4 and binary event-stream framing (crates vs custom); RS256 JWT; image decode/scale crates.
- How Go's `http.Transport` defaults map onto the candidates.

Deliverable: a dense findings file at `.scratch/rust-port/research/rust-net-stack-survey.md` citing primary sources. This feeds ticket 06.

## Answer

Resolved by research subagent. Findings: `.scratch/rust-port/research/rust-net-stack-survey.md`.

- Base stack: tokio 1.53 + reqwest 0.13 (defaults rustls + aws-lc-rs + rustls-platform-verifier, HTTP/2, system proxy). Hyper-util/ureq alternatives rejected: ureq is HTTP/1.1-only and misses platform roots by default.
- SSE: no maintained crate matches Go's parser semantics (multi-line `data:`, comments, 10 MiB line cap, byte-preserving behavior); hand-roll (~100 LOC). eventsource-stream/reqwest-eventsource are stale.
- Retry: capture `Bytes` request bodies for replay, mirroring Go's `newReq()`. reqwest defaults differ from Go's transport (no connect timeout; 15 s keepalive; unlimited idle conns vs Go 30 s/30 s/2).
- Proxy: reqwest/hyper-util honor `ALL_PROXY` and OS-level settings Go ignores; parity may require explicit proxy configuration.
- Cancellation: dropping a `JoinHandle` detaches; cancellation needs explicit tokens (`CancellationToken`) threaded into stream reads.
- Bedrock: `aws-smithy-eventstream` + `aws-sigv4` (or hand-rolled signer) rather than full AWS SDKs. Vertex: `jsonwebtoken` (rust_crypto backend) + `gcp_auth`/yup-oauth2.
- Images: `image` 0.25 + `fast_image_resize` 6.1 (CatmullRom). Avoid dead crates: async-sse, sse, oidc, eventsource-stream.
- Context7 spot-check (2026-09-24) confirmed the reqwest async builder knobs and `CancellationToken` child-token semantics; use the async client (blocking defaults to a 30 s total timeout). Three source-level follow-ups remain open in §11 of the findings file and do not change the stack. See §12.
