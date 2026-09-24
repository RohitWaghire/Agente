# Rust net stack survey for provider parity (ticket 05)

Date: 2026-09-24. Method: Go claims cite files in this repo; Rust claims cite crates.io
registry metadata, docs.rs, and crate source at pinned tags/branches (raw.githubusercontent.com).
No paid or credentialed API calls were made. Version/downloads figures are from the crates.io
API read on 2026-09-24.

## 0. Go baseline (the behavior that must be preserved)

| Concern | Go implementation | Location |
|---|---|---|
| Client contract | `Stream(ctx, req) (<-chan Event, error)`; setup errors returned directly, runtime errors arrive as `EventDone{Err}`, channel closed after `EventDone` | `packages/provider/provider.go:201-282` |
| SSE parser | `bufio.Scanner`, initial buffer 64 KiB, **max token 10 MiB**; blank line flushes; `:` comment skipped; split on first `:`; one optional leading space stripped; multiple `data:` lines joined with `\n` (no trailing LF); only `event`/`data` are read (`id`/`retry` ignored) | `packages/provider/sse.go:17-59` |
| Retry | 2 silent retries before stream headers; fixed 250 ms then 750 ms; statuses 429/500/502/503/504/524/529; `retry-after-ms`/`Retry-After` honored, capped at 60 s; terminal-quota 429 bodies not retried; 4 KiB body drain; request rebuilt by `newReq()` closure (JSON body pre-marshaled) | `packages/provider/retry.go:15-252` |
| HTTP client | per-provider `http.Client{Timeout: 0}`; `--insecure` clones `DefaultTransport` and sets `InsecureSkipVerify` only on that client; auth/discovery use 30 s clients | `packages/provider/httpclient.go:12-29`, `auth/oauth.go:261`, `google_vertex.go:192` |
| Bedrock | bearer token or hand-rolled SigV4 (env/`~/.aws/credentials`); hand-rolled AWS event-stream framing, CRCs deliberately not validated | `packages/provider/amazon_bedrock.go:1-56, 816-917, 952-1065` |
| Vertex | RS256 JWT (`rsa.SignPKCS1v15`, SHA-256) exchanged at `oauth2.googleapis.com/token`, in-memory cache; URL rewrite via RoundTripper | `packages/provider/google_vertex.go:195-333` |
| OAuth | PKCE S256; loopback callback on fixed port; JSON-or-form token body; extra provider args; `id_token` base64-decoded only (never signature-verified) | `packages/provider/auth/oauth.go`, `auth/callback.go:39-61` |
| Credentials | `auth.json`, atomic write, mode 0600 | `packages/provider/auth/store.go:301-315` |
| Images | provider: decode png/jpeg/gif, Catmull-Rom downscale, re-encode jpeg q85 / png (gif→png) | `packages/provider/anthropic_image.go:31-131` |
| Images (clipboard) | decode png/jpeg/gif + tiff/webp (`golang.org/x/image`), caps 32 MiB / 25 M px / 64 MiB, normalize to PNG | `packages/tui/clipboard_image.go:14-64` |
| Module deps | only `golang.org/x/image` for image formats | `go.mod` |

Go runtime facts (go1.25.0 source):

- `http.DefaultTransport` = `Proxy: ProxyFromEnvironment`, dial timeout 30 s, keepalive 30 s,
  `ForceAttemptHTTP2: true`, `MaxIdleConns: 100`, `MaxIdleConnsPerHost: 2`,
  `IdleConnTimeout: 90 s`, `TLSHandshakeTimeout: 10 s`, `ExpectContinueTimeout: 1 s`
  (`src/net/http/transport.go:45-60`).
- Automatic gzip: adds `Accept-Encoding: gzip` itself (skipped for Range requests or an explicit
  header) and transparently decompresses only when it added the header
  (`transport.go:2762-2782, 2378-2380`).
- Redirects: max 10 hops (`len(via) >= 10` error) and sensitive headers
  (`Authorization`, `WWW-Authenticate`, `Cookie`, `Cookie2`, `Proxy-Authorization`,
  `Proxy-Authenticate`) stripped when the redirect changes domain (`src/net/http/client.go:820-822, 809-810, 684-688`).
- Proxy env: only `HTTP_PROXY`/`http_proxy`, `HTTPS_PROXY`/`https_proxy`,
  `NO_PROXY`/`no_proxy`; refuses `HTTP_PROXY` in CGI (`REQUEST_METHOD` set); **no `ALL_PROXY`**
  and no OS-level proxy settings (`src/vendor/golang.org/x/net/http/httpproxy/proxy.go:84-95, 133-134`).
- TLS: client default minimum TLS 1.2; only TLS 1.2/1.3 exist (`src/crypto/tls/common.go:1178`).

## 1. Runtime and HTTP

| Crate | Version | Last release | Status | License |
|---|---|---|---|---|
| tokio | 1.53.1 | 2026-07-20 | active (releases Jul 2026) | MIT |
| reqwest | 0.13.5 | 2026-09-08 | active; 190 M recent downloads | MIT/Apache-2.0 |
| hyper | 1.11.1 | 2026-08-28 | active | MIT |
| hyper-util | 0.1.20 | 2026-02-02 | active | MIT |
| ureq | 3.4.2 | 2026-09-13 | active | MIT/Apache-2.0 |
| ureq-proto | 0.6.4 | 2026-09-16 | active | MIT/Apache-2.0 |

- **reqwest 0.13.5 defaults** (`Cargo.toml` @ tag v0.13.5): features
  `["default-tls", "charset", "http2", "system-proxy"]`; `default-tls = ["rustls"]`
  which enables rustls + aws-lc-rs + `rustls-platform-verifier`. HTTP/2 is on by default
  (`h2 0.4` + `hyper/http2`); HTTP/3 behind `http3`. Streaming: `Response::chunk()`
  (new in 0.13, no feature gate) and `Response::bytes_stream()` (requires optional `stream`
  feature) (`src/async_impl/response.rs:310-353`).
- **ureq 3.4.2** is blocking, std-thread friendly, "uses blocking I/O instead of async I/O"
  (README). Default features `rustls` + `gzip`; rustls uses `ring` + static `webpki-roots`
  (Mozilla bundle), not the OS store (README "Root certificates" / "TLS (https)").
  **HTTP/1.1 only** — `ureq-proto` is described as "First class HTTP/1.1 protocol implementation"
  (docs.rs header); the ureq README/docs never mention HTTP/2. Committed request/response
  phases have separate timeouts: `Timeout::{Global, PerCall, Resolve, Connect, SendRequest,
  SendBody, RecvResponse, RecvBody}` (docs.rs `ureq::Timeout`).
- **Consequences for `<-chan Event`**: the closest async shape is
  `tokio::sync::mpsc::Receiver<Event>` plus a spawned producer; `send().await` gives the
  backpressure an unbuffered Go channel gives, and `send` returns `SendError` when the
  receiver is dropped, which cleanly terminates the producer (docs.rs `mpsc::Sender::send`).
  A `Stream<Item = Event>` (`futures-core::Stream`) is the idiomatic alternative and composes
  with `select!`. Setup errors map naturally to `Result<Receiver, Error>`; runtime errors stay
  an `EventDone`-style enum variant.
- **Cancellation**: `ctx.Context` maps to `tokio::select!` ("returning when the first branch
  completes, cancelling the remaining branches", docs.rs) or a
  `tokio_util::CancellationToken`; dropping `JoinHandle` **detaches** the task, so the handle
  must be aborted explicitly or the task must watch a token (docs.rs `tokio::task::JoinHandle`).
  With blocking `ureq`, an in-flight body read cannot be interrupted: cancellation can only be
  approximated with `Timeout::RecvBody` / closing the reader, and the thread lingers; this is
  the main reason tokio+reqwest preserves Go semantics better than std threads + ureq.
- **Footprint**: tokio is feature-gated (`net`, `time`, `sync`, `rt-multi-thread`, `macros`
  are the relevant set); reqwest already requires it. ureq avoids the runtime but loses
  external cancellation and HTTP/2.

## 2. SSE

Go parser requirements, restated precisely: multi-line `data:` joined by `\n`, comment lines
skipped, blank line flush, a 10 MiB max line, no trailing-newline semantics, no `id`/`retry`,
`[DONE]` handled by the provider decoder (OpenAI) rather than the parser, and byte-preserving
behavior (Go strings tolerate invalid UTF-8; JSON parsing fails later).

| Crate | Version | Last release | Status | Verdict |
|---|---|---|---|---|
| eventsource-stream | 0.2.3 | 2022-02-17 | stale: repo last push 2024-08-15, 8 open issues, not archived; 9.5 M recent downloads | spec parser over `Stream` |
| reqwest-eventsource | 0.6.0 | 2024-03-29 | stale: repo last push 2024-06-13, 16 open issues | reqwest + eventsource-stream + reconnection |
| sse-stream | 0.3.0 | 2026-09-18 | active (repo push 2026-09-18, 0 open issues, 11 stars) | codec (decode+encode+keep-alive) |
| eventsource-client | 0.18.0 | 2026-08-10 | active (LaunchDarkly, Apache-2.0) | full SSE client, not a parser |
| async-sse | 5.1.0 | 2021-09-17 | dead | — |
| sse | 0.2.0 | 2018-11-08 | dead | — |

Evidence per candidate:

- **eventsource-stream** implements the WHATWG algorithm: `data` lines append value + one LF
  and the final LF is popped at dispatch; `event` sets the type; blank line completes; comments
  ignored; `id`/`retry` handled; event type defaults to `"message"`; an event whose data buffer
  is empty is not dispatched (`src/event_stream.rs:44-113`). Differences from Go: requires valid
  UTF-8 (`Utf8StreamError`, `src/event_stream.rs:188-207`), no line-size cap anywhere, and the
  empty-data+`event:` case is dropped where Go emits it. Core deps are tiny
  (`futures-core`, `nom 7.1`, `pin-project-lite`), but the release/project are frozen.
- **reqwest-eventsource** wraps reqwest with eventsource-stream and adds reconnection
  (`LastEventId`). Useful only if spec reconnection is wanted; adds another stale layer.
- **sse-stream 0.3.0** is a runtime-independent codec over `Buf`: BOM handling, comment/BOM/CRLF,
  fragmented lines, multi-line data, UTF-8 validation of data values, plus an encoder and
  `KeepAlive`. It compacts retained capacity to `MAX_RETAINED_CAPACITY = 64 KiB` but has no hard
  event/line cap (`src/stream/parser/data.rs:6, 57-68`). Deps: `bytes`, `futures-util`,
  `http-body`, `http-body-util`, `pin-project-lite`, `thiserror` (+optional `memchr`,
  `simdutf8`, `tracing`). Young/low-bus-factor but the only maintained decoder.
- **eventsource-client** is a full hyper-based client; adopting it would replace reqwest for
  streaming providers and bypass the per-request control the Go layer has.

**Evidence for hand-rolling**: the Go parser is 59 lines. No maintained crate replicates the
exact contract (byte-preserving, 10 MiB cap, empty-`data`-with-`event` emission, no `[DONE]`
special case). A Rust port of `readSSE` (BufRead + `memchr`/`split`, ~80-120 lines) is
straightforward and testable against the existing Go tests; `sse-stream` is the fallback if a
maintained dependency is preferred, accepting spec-semantic drift. `reqwest-eventsource` is not
recommended (two stale layers).

## 3. TLS

| Crate | Version | Last release | Status | License |
|---|---|---|---|---|
| rustls | 0.23.45 (0.24.0-dev.1 pre-release) | 2026-09-14 | active | Apache-2.0 OR ISC OR MIT |
| rustls-platform-verifier | 0.7.0 | 2026-04-12 | active | MIT/Apache-2.0 |
| native-tls | 0.2.18 | 2026-02-18 | maintained | MIT/Apache-2.0 |
| webpki-roots | 1.0.9 | 2026-07-18 | active | MPL-2.0 (root data) / ISC |
| aws-lc-rs | 1.18.1 | 2026-09-01 | active | ISC AND (Apache-2.0 OR ISC) |
| ring | 0.17.14 | 2025-03-11 | slower cadence | Apache-2.0 AND ISC |

- **rustls-platform-verifier** (README): Windows uses the platform certificate store through
  the Windows API (revocation supported); macOS 10.14+ uses `Security.framework` (revocation
  supported); Linux uses the system CA bundle via `rustls-native-certs` + `openssl-probe`
  (no revocation fallback); WASM uses webpki roots. It exists precisely for enterprise CA /
  corporate-proxy trust, matching what Go's `crypto/x509` system pool does on Windows/macOS.
- **native-tls** delegates to Schannel (Windows, including cert store), Security.framework
  (macOS), OpenSSL (Linux) — the direct analog of Go's platform verification.
- **reqwest 0.13.5 defaults**: `default-tls` is now rustls (+aws-lc-rs) with
  `rustls-platform-verifier` (`Cargo.toml`). In reqwest **0.12.x `default-tls` was
  `native-tls`** (`v0.12.28/Cargo.toml:41`), so guidance written for 0.12 is wrong for 0.13.
  Client defaults: hostname verification on, cert verification on, SNI on (`tls_sni: true`,
  `src/async_impl/client.rs:292-299`); `danger_accept_invalid_certs(true)` /
  `danger_accept_invalid_hostnames` exist for the `--insecure` path; `native-tls` and
  `use_preconfigured_tls` remain available.
- **ureq 3.4.2 defaults**: rustls with `ring` and static `webpki-roots`; the OS store is
  **not** used unless the `platform-verifier` feature is enabled and
  `RootCerts::PlatformVerifier` is configured (README "Root certificates"). For corporate
  TLS interception this is a real parity gap versus Go; `native-tls` is optional and never
  selected implicitly.
- SNI: rustls sends SNI for DNS hostnames; reqwest `tls_sni` defaults true; native-tls/SChannel
  likewise. No parity concern.
- TLS versions: rustls is 1.2/1.3 only; Go is 1.2/1.3 only (default min 1.2) — parity.

## 4. Proxy env handling

| Env var | Go `ProxyFromEnvironment` | reqwest 0.13 (default) | ureq 3.4 (default Agent) |
|---|---|---|---|
| `HTTP_PROXY`/`http_proxy` | yes | yes | yes |
| `HTTPS_PROXY`/`https_proxy` | yes | yes | yes |
| `NO_PROXY`/`no_proxy` | yes | yes (curl-like matcher) | yes (exact/suffix/wildcard/`*`) |
| `ALL_PROXY`/`all_proxy` | **no** | yes | yes |
| CGI `REQUEST_METHOD` guard | yes | hyper-util records `is_cgi` | no evidence found |
| OS settings | no | Windows registry + macOS SystemConfiguration (default `system-proxy` feature) | Windows registry only, opt-in `win-system-proxy` feature |

- reqwest 0.13: `ClientBuilder::new()` sets `auto_sys_proxy: true`, pushing
  `ProxyMatcher::system()` (`src/async_impl/client.rs:310, 417-418`), which is reqwest's wrapper
  around `hyper_util::client::proxy::matcher::Matcher::from_system()` (`src/proxy.rs:514-517`).
  `from_system` = env vars first (`ALL_PROXY` > `HTTPS_PROXY`/`HTTP_PROXY` by scheme, plus
  `NO_PROXY`) and then platform config when `client-proxy-system` is compiled in
  (`hyper-util/src/client/proxy/matcher.rs:227-249, 242-247`).
- ureq 3.4.2: "ureq automatically reads proxy configuration from environment variables when
  creating a default `Agent`" — order `ALL_PROXY`, `HTTPS_PROXY`, `HTTP_PROXY` with lowercase
  variants, plus `NO_PROXY` patterns (README "Proxying"); `Proxy::try_from_env()` is the API
  (`src/proxy.rs:208-241`).
- **Parity risks**: (a) `ALL_PROXY` may redirect traffic that Go would send direct; (b) the
  Windows registry proxy (reqwest default, ureq opt-in) is invisible to Go; (c) NO_PROXY
  matcher semantics differ (curl-like vs Go's). To pin reqwest to Go's exact set, either
  configure proxies manually from the three env vars and call `.no_proxy()`, or accept and
  document the wider behavior.

## 5. Retry replay / request-body replay

- Go never reuses an `*http.Request`; `doStreamWithRetry` takes a `newReq()` closure and every
  attempt rebuilds the request from a captured `[]byte` JSON payload
  (`packages/provider/retry.go:57-66, 82-86`). Only pre-stream failures are retried.
- Rust: keep the serialized payload as `Bytes` and rebuild the `RequestBuilder` per attempt.
  `RequestBuilder::try_clone()`/`Request::try_clone()` return `Option` and only succeed for
  explicitly reusable bodies (`Bytes`/`String`); streamed bodies return `None`
  (`reqwest v0.13.5 src/async_impl/request.rs:145-147, 542-551`; test `try_clone_stream`
  asserts the failure case). `http_body_util::Full<Bytes>` (0.1.5) is `Clone` and is the
  standard replayable body.
- reqwest 0.13 ships a built-in retry layer: default policy retries only protocol NACKs
  (`Classifier::ProtocolNacks`), max 2 retries per request, no budget, scoped per host or
  custom (`src/retry.rs:84-199`); custom classifiers can match method + status, which is the
  hook for the Go status set including nonstandard 524/529. Tuning `Retry-After` parsing
  (including `retry-after-ms`) stays custom either way.
- `reqwest-middleware` 0.5.2 + `reqwest-retry` 0.9.1 (MIT/Apache-2.0; 2026 releases; both depend
  on `reqwest ^0.13.1`) provide `RetryTransientMiddleware` and `retry-policies` backoff, but
  pull tower/async-trait/getrandom machinery for behavior the manual loop already covers.
- Recommendation implied by evidence: manual retry loop over a `Bytes` payload mirrors Go
  exactly and is smaller than reqwest-retry; reqwest's built-in retry is viable but its default
  classifier and budget semantics differ from the Go one.

## 6. AWS SigV4 + binary event-stream framing

| Crate | Version | Last release | License | Key deps (normal) |
|---|---|---|---|---|
| aws-sigv4 | 1.6.0 | 2026-09-22 | Apache-2.0 | aws-credential-types, aws-smithy-http, aws-smithy-runtime-api, aws-smithy-types, hmac 0.13, sha2 0.11, hex, time, tracing, bytes (+optional aws-smithy-eventstream, http 0.2/1.x, p256, zeroize) |
| aws-smithy-eventstream | 0.61.4 | 2026-09-15 | Apache-2.0 | aws-smithy-types, bytes, crc32fast |
| aws-sdk-bedrockruntime | 1.146.0 | 2026-09-22 | Apache-2.0 | full SDK (aws-config/aws-runtime/aws-http/…) |

- `aws-sigv4` default features are `http1` + `sign-http`; `sign-eventstream` is optional
  (docs.rs features page). It signs `http` 1.x requests with credentials including session
  tokens, is sync and runtime-agnostic; `aws_sigv4::http_request` and `aws_sigv4::sign` are the
  relevant modules (docs.rs). It is released weekly with the rest of smithy-rs, so maintenance
  is not a concern.
- `aws-smithy-eventstream` is small and self-contained; `frame::MessageFrameDecoder` decodes
  frames incrementally from a `Buf` and returns `Ok(None)` until a full frame is available
  (docs.rs). It depends on `crc32fast`; the Go parser deliberately does **not** validate CRCs
  (`packages/provider/amazon_bedrock.go:26-29, 826-827`), so a stricter decoder is a behavior
  difference to test with synthetic frames (docs.rs does not document CRC rejection explicitly).
- Licensing: all smithy-rs crates are Apache-2.0 only (not dual MIT) — compatible but note it.
- Footprint judgment: `aws-sigv4` drags `aws-smithy-runtime-api`/`aws-smithy-http` (enterprise
  sized source, no tokio); `aws-sdk-bedrockruntime` drags the whole SDK stack and is
  disproportionate for two endpoints. The Go code is ~115 lines of SigV4 and ~100 lines of
  framing; Rust equivalents using `hmac` + `sha2` + `hex` + `crc32fast` are small, but
  `aws-sigv4` + `aws-smithy-eventstream` (without the SDK) is the maintained middle ground.

## 7. RS256 JWT and Google service-account token exchange

- **jsonwebtoken 11.1.0** (MIT; 2026-09-16; MSRV 1.88, edition 2024): default feature is only
  `use_pem`; RS256 signing requires opting into a crypto backend — `aws_lc_rs` (pulls
  aws-lc-rs 1.18.1, C/asm) or `rust_crypto` (pure Rust `rsa` + `sha2` + `p256`/`p384` etc.)
  (`v11.1.0/Cargo.toml:69-82`). API supports `EncodingKey::from_rsa_pem`, `Algorithm::RS256`,
  claims validation (docs.rs). Active repo (2,095 stars, pushed 2026-09-16).
- Hand-rolled alternative: `rsa` 0.9.10 (stable; Apache-2.0/MIT; 0.10.0-rc is pre-release) +
  `sha2` + `base64` + `serde_json` reproduces the Go function
  (`google_vertex.go:199-218`, 20 lines) with no JWT-claim machinery.
- **Token exchange**: `yup-oauth2` 12.1.2 (MIT/Apache-2.0; 2026-01-07; 245 stars) provides
  `ServiceAccountAuthenticator` and refresh flows; core deps are hyper 1 + hyper-util +
  tokio + http-body-util, with rustls/hyper-rustls/native-tls optional (crates.io deps). It
  duplicates HTTP plumbing already present with reqwest.
- `gcp_auth` 0.12.7 (MIT; 2026-06-22; MSRV 1.85) covers default and custom service accounts
  (crates.io description) and is a lighter option than yup-oauth2.
- `oauth2` 5.0.0 (MIT/Apache-2.0; 2025-01-21) covers generic authorization-code/refresh/device
  flows; agente's provider quirks (state = PKCE verifier, JSON token body, extra token args,
  manual copy-code redirect) do not map cleanly onto it and are only a few lines by hand.
- Loopback callback: Go uses a fixed-port `net/http` server (`auth/callback.go:39-61`); Rust
  options are hyper's server (already transitively present) or a minimal `std::net::TcpListener`
  HTTP parse. `tiny_http` 0.12.0 was last released 2022-10-06 — treat as stale.
- OpenAI `id_token` handling needs no JWT verification (base64 decode only,
  `auth/oauth.go:212-235`), so no crypto dependency is implied by the Codex flow.

## 8. Image decode/scale

| Concern | Go | Rust candidate | Notes |
|---|---|---|---|
| PNG | `image/png` | `image` 0.25.10 via `png` 0.18.1 | decode+encode |
| JPEG | `image/jpeg` | `image` via `zune-jpeg` 0.5.15 | decode+encode |
| GIF | `image/gif` | `image` via `gif` 0.14.2 | decode; Go provider re-encodes to PNG |
| TIFF | `golang.org/x/image/tiff` | `image` via `tiff` 0.11.3 (MIT) | clipboard decode path only |
| WebP | `golang.org/x/image/webp` | `image` via `image-webp` 0.2.4 | decoder: lossy+lossless+alpha+animation; encoder: **lossless only** (README) |
| Catmull-Rom scale | `golang.org/x/image/draw` `CatmullRom.Scale` | `image::imageops::FilterType::CatmullRom`; `fast_image_resize` 6.1.0 `FilterType::CatmullRom` | pure-Rust vs SIMD |
| Limits | explicit 32 MiB / 25 M px / 64 MiB caps | `image::Limits` (allocation guards) | set them explicitly |

- `image` 0.25.10 (2026-03-10): default `default-formats` include GIF/JPEG/PNG/TIFF/WebP etc.
  (README format table); `codecs::webp` exposes `WebPEncoder`/`WebPDecoder`.
- `fast_image_resize` 6.1.0 (MIT/Apache-2.0; MSRV 1.87; 2026-07-21; 462 stars): `FilterType`
  includes `Box`, `Bilinear`, `Hamming`, `CatmullRom`, `Mitchell`, `Gaussian`, `Lanczos3`,
  `Custom`; `CatmullRom` maps to `catmul_filter` (source `src/convolution/filters.rs:108, 169`).
  Optional `image` integration feature exists; it is SIMD-accelerated.
- WebP encoding caveat: `image-webp` cannot emit lossy WebP (README "Future possibilities").
  This does not break parity because the Go provider never re-encodes WebP (it only sniffs it
  and leaves oversized WebP untouched) and the clipboard path normalizes to PNG.
- `resize` 0.8.9 (2026-02-07, 191 k recent downloads) is not competitive with the two options
  above; it is only a dev-dependency of fast_image_resize.

## 9. Go `http.Transport` default → Rust equivalent

| Go default | Rust equivalent | Parity notes |
|---|---|---|
| `Client.Timeout: 0` (agente) / no client timeout | reqwest `timeout: None` (default), `read_timeout: None` | match; set `timeout()`/`read_timeout()` only where Go did (auth 30 s) |
| Dial timeout 30 s | reqwest `connect_timeout: None` (default) | **must set 30 s to match**; ureq `Timeout::Connect` |
| TLS handshake timeout 10 s | no separate reqwest knob; folds into connect timeout | fold into connect timeout or add separate `tls` timeout via hyper connector |
| `ExpectContinueTimeout: 1 s` | none; reqwest/hyper do not send `Expect: 100-continue` | benign for JSON APIs |
| Keep-alive 30 s | reqwest `tcp_keepalive: 15 s` (default) | set 30 s for parity |
| `IdleConnTimeout: 90 s` | reqwest `pool_idle_timeout: 90 s` (default) | match |
| `MaxIdleConnsPerHost: 2` | reqwest `pool_max_idle_per_host: usize::MAX` (default) | **differs**; set 2 for parity |
| `MaxIdleConns: 100` | no direct global cap | document |
| `ForceAttemptHTTP2: true` | reqwest `http2` feature (default on) | match; ureq is HTTP/1.1-only |
| Auto gzip request+decode | reqwest `gzip` feature (opt-in, not default); ureq `gzip` (default) | enable `gzip`/`brotli`/`zstd`/`deflate` as needed; reqwest only sets `Accept-Encoding` when enabled |
| Redirects: max 10, strip sensitive headers cross-domain, add Referer | reqwest `Policy::limited(10)` (default), strips sensitive headers on host/port/scheme change, `referer: true` (default) | close match (`src/redirect.rs:3-4, 160-163, 239-242, 339-343`) |
| Proxy from env only (3 vars, CGI guard) | reqwest/hyper-util adds `ALL_PROXY` + OS settings; ureq adds `ALL_PROXY` + opt-in registry | **widest parity risk**, see §4 |
| `InsecureSkipVerify` per client | reqwest `danger_accept_invalid_certs(true)` on a dedicated `Client` | match; do not mutate a process-wide config |
| TLS 1.2 min, 1.2/1.3 only | rustls 1.2/1.3 only; reqwest rustls default | match |
| System trust store (Windows/macOS/Linux) | reqwest 0.13 default = platform verifier; ureq needs `platform-verifier` feature | ureq default (`webpki-roots`) diverges; reqwest 0.13 matches |
| `ctx` cancellation | `select!`/`CancellationToken`/task abort | blocking ureq cannot interrupt a body read |
| `[]byte` body replay | `Bytes` + rebuild / `try_clone` | stream bodies cannot be cloned |

## 10. Crates flagged as stale / unusable

- **async-sse 5.1.0** (2021-09-17): dead.
- **sse 0.2.0** (2018-11-08): dead.
- **eventsource-stream 0.2.3** (2022-02-17): stale (repo quiet since 2024-08-15); usable code but
  frozen, no line cap, UTF-8-strict.
- **reqwest-eventsource 0.6.0** (2024-03-29): stale and layers on eventsource-stream + reqwest.
- **tiny_http 0.12.0** (2022-10-06): stale; only relevant to the OAuth loopback server.
- **oidc 0.3.0** (2019-09-22): dead.
- **ring 0.17.14** (2025-03-11): not dead but slow; reqwest's rustls default uses aws-lc-rs,
  ureq's uses ring.
- No maintained crate reproduces the Go SSE parser's exact semantics; that is the strongest
  candidate for a hand-rolled module.

## 11. Unverified / follow-ups

- Whether `aws-smithy-eventstream` rejects CRC mismatches is not documented on its docs.rs
  pages (only the `crc32fast` dependency is visible); confirm from source before relying on it.
- `gcp_auth`'s HTTP/TLS stack was not inspected beyond metadata.
- ureq's default per-phase timeout values were not read from source (only that the phases and
  configuration exist).

## 12. Context7 spot-check (2026-09-24)

- reqwest async builder confirmed to expose every parity knob this survey relies on: `timeout` (no default), `read_timeout` (no default), `connect_timeout` (default `None`), `pool_idle_timeout`, `pool_max_idle_per_host`, `tcp_keepalive*`, `redirect`, `proxy`/`no_proxy`, `gzip`/`brotli`/`zstd`/`deflate`, TLS options, and a built-in `retry` policy. Nuance: the **blocking** reqwest client has a 30 s default total timeout while the async client has none — provider clients must use the async client to match Go's `Timeout: 0`.
- tokio-util `CancellationToken` semantics confirmed: `cancel()` cancels the token and all derived child tokens and wakes waiters; cancelling a child does not cancel the parent; cancellation is not atomic across children (a concurrent observer may still see a child uncancelled until `cancel()` returns). `TaskTracker` + `with_cancellation_token_owned` is the documented graceful-shutdown pattern.
- The three unverified follow-ups above are source-level behaviors absent from crate docs; context7 cannot close them. None change the recommended stack, and they can be checked at implementation time.
