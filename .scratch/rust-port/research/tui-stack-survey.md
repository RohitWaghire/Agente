# Rust TUI stack survey vs Agente's renderer requirements

Ticket: `.scratch/rust-port/issues/01-tui-stack-survey.md`. All crate facts below were fetched on 2026-09-24 from docs.rs / crate repositories / protocol specs; version numbers are those docs.rs resolves `latest` to at that date. No live provider or credentialed calls were made. Where a source could not be fetched it is called out inline.

## 0. Ground truth in the Go tree (what is actually being reproduced)

Measured (`Get-ChildItem` + line counts, tests excluded unless stated):

| Unit | Lines | Notes |
|---|---:|---|
| `packages/tui` non-test (28 files) | 8,447 | includes `view.go` 2,676 and `render.go` 926 |
| `packages/tui` tests (33 files) | 3,053 | behavioral parity corpus |
| `packages/agent/modes/interactive.go` | 7,232 | owns startup/teardown escapes, pacer, throttle, clipboard wiring |

Behavior anchors used repeatedly below (local refs):

- Flow renderer: `Renderer.DrawLog` (main-screen scrollback flow), `DrawHistory` (alt-screen history browser), `Draw` (bottom-aligned fixed frame) — `packages/tui/render.go:322,437,463`.
- Logical-buffer diff + viewport rebase when rows scroll above the viewport: `render.go:628-885`.
- VS Code `\x1b[3J` suppression policy keyed on `TERM_PROGRAM` — `render.go:54-97`; policy re-applied on resize/clear (`render.go:137-156,177-196`).
- Synchronized-output fencing `\x1b[?2026h/l` wraps every paint — `render.go:350,425,526,877`; kitty image deletion `\x1b_Ga=d\x1b\\` per repaint — `render.go:380,531` (`terminal.go:108`).
- ANSI-aware truncation using per-rune `runewidth.RuneWidth` — `render.go:264-320`; ANSI-aware wrap using rune widths — `editor.go:953-1057`; visible width via `runewidth.StringWidth(stripANSI(s))` — `editor.go:917-942`.
- Input: byte-at-a-time CSI parser with SS3, Alt-bytes, SGR mouse wheel (64/65), bracketed paste, kitty CSI-u, xterm `CSI 27;mod;code~` modifyOtherKeys, 50 ms Esc peek — `input.go:71-453`; peek via `unix.Select` on fd (`resize_unix.go:36-58`).
- Startup/teardown escape set actually emitted by interactive mode: `\x1b[?2004h` + `\x1b[>1u\x1b[>4;2m` + `\x1b[r` + kitty delete + `\x1b[2J` + `\x1b[3J` + home (`interactive.go:783`), teardown mirrors it (`interactive.go:795`). Resize callback at `interactive.go:807`; `DrawHistory`/`DrawLog` toggle at `interactive.go:1860-1862`.
- `SeqMouseOn/Off`, `SeqSaveCursor/RestoreCursor`, `SetScrollRegion`, `SeqEraseToEnd` are **defined but never referenced** outside tests in the current tree (`terminal.go:122-134,181-187`; repo-wide non-test grep). Mouse wheel parsing exists in `input.go` but mouse reporting is never enabled. DECSTBM/DECSC are therefore compile-time surface, not live behavior.
- Streaming: 16 ms redraw throttle (`interactive.go:888`), 120 ms animation tick (`interactive.go:881`), typewriter pacer 6 runes / 16 ms (`interactive.go:7581-7639`).
- Clipboard: Ctrl+V is parsed as `KeyPasteClipboard` (`input.go:96,417`) and resolved through `ReadClipboardText`/`ReadClipboardImagePNG` (`interactive.go:2198,2658,2952,2973`); platform code is command-based on all three OSes (`clipboard_text_{darwin,linux,windows}.go`, `clipboard_{darwin,linux,windows}.go`, `clipboard_image_command.go`).
- Image protocol detection + encoders: `image.go:39-226`; PNG-only kitty payload re-encode (`image.go:144-157`); `a=T,f=100,C=1` kitty header and 4096-byte chunks (`image.go:169-226`); iTerm2 OSC 1337 with `doNotMoveCursor=1` (`image.go:116-136`); iTerm2 is deliberately disabled by default (`image.go:76-84`). Footprint math with `AGENTE_CELL_ASPECT` (`image.go:238-302`).
- Highlights: chroma v2.23.1, `styles.Get("monokai")` + 18 token overrides + `Background bg:` (`highlight.go:196-237`), lexer map ~50 extensions + 10 filename specials (`highlight.go:266-338`), `terminal256` formatter + post-hoc background-ANSI stripping (`highlight.go:40-57,124-177`), 512-entry memo keyed by style+lang+src (`highlight.go:66-105`).
- Markdown: hand-rolled subset renderer (headings, GFM tables with alignment/fit/wrap, lists, blockquotes, inline code/bold/italic, partial fence flush) — `packages/tui/markdown.go:23-141`.
- Partial JSON: custom streaming extractor for top-level `"field"` and last `"newText"` — `packages/tui/partialjson.go:20-154`.
- Themes: `TerminalColor` 256/ANSI/RGB, JSON theme files with dark/light + shared overrides, syntax overrides, spinner data, path discovery — `theme.go`, `theme_loader.go`.
- OSC 11 background detection before raw TUI: BEL-terminated query, 1–4 hex digit channels, Rec.709 luma, 80 ms timeout, `AGENTE_THEME` override — `detect_bg.go:26-154`.

## 1. Capability matrix

Status legend: **native** = crate API covers it; **glue** = crate covers the primitive, port writes the policy/composition; **custom** = no crate does it, port writes it.

| # | Requirement | Candidate(s) | Status | Concrete notes / evidence |
|---|---|---|---|---|
| 1 | Main-screen flow rendering, scrollback, bottom-aligned diff, reflow above viewport | ratatui `Viewport::Inline` + `Terminal::insert_before`; else custom | **custom** (ratatui: glue at best) | ratatui inline viewport anchors to the cursor and supports `insert_before` without clearing the viewport when `scrolling-regions` is on (`Terminal` docs, Inline Viewport section, docs.rs/ratatui 0.30.2). It does **not** model agente's logical-buffer diff, viewport-top tracking, or the "rebase on height change above viewport" logic (`render.go:702-753`). Also ratatui's cell-buffer diffing is the opposite data model (styles in cells vs pre-styled strings). |
| 2 | `\x1b[3J` suppression under VS Code | none | **custom** | Policy is app logic; crossterm exposes `ClearType::Purge` = "All plus history" (`docs.rs/crossterm/.../ClearType`) but no policy. |
| 3 | Synchronized-output fencing `?2026h/l` | crossterm `BeginSynchronizedUpdate`/`EndSynchronizedUpdate`; ratatui | **glue** | crossterm terminal module lists both commands (0.29.0 docs). ratatui's crossterm backend does **not** wrap `Terminal::draw` in them (no reference in `ratatui-crossterm/src/lib.rs`, fetched from `main`). |
| 4 | Scroll regions (DECSTBM) | ratatui `scrolling-regions` feature; crossterm `ScrollUp/ScrollDown` only | **glue with caveat** | ratatui's backend emits `CSI r;r` + `CSI nS/T` + reset, but `execute_winapi` returns `Unsupported` on Windows for scroll-region commands (`ratatui-crossterm` source). crossterm itself has no DECSTBM command. Go only ever emits the region *reset* today. |
| 5 | DECSC/DECRC | crossterm `SavePosition`/`RestorePosition` = `ESC7`/`ESC8` | **native** | `crossterm/src/cursor.rs` (master) writes `"\x1B7"`/`"\x1B8"`; ratatui backend wraps them (source + tests). Unused in Go today. |
| 6 | Streaming markdown (headings, lists, GFM tables, inline styling) | pulldown-cmark (parse), custom renderer | **glue → custom** | `Options::ENABLE_TABLES`/`ENABLE_STRIKETHROUGH`/`ENABLE_TASKLISTS`; GFM blockquote tags behind `ENABLE_GFM` (docs.rs/pulldown-cmark 0.13.4, `Options`). Terminal-specific width-fitting/wrapping of table cells and partial-fence behavior (`markdown.go:134-140`) have no crate equivalent. |
| 7 | Partial-JSON tool previews | none (serde_json is all-or-nothing) | **custom** | ~200-line scanner; no evaluated crate specializes in tolerant partial string extraction. |
| 8 | 16 ms redraw throttle / 120 ms animation / typewriter pacer | none needed | **custom (trivial)** | Plain timer/event-loop logic; Rust equivalent ~150 lines using `tokio`/`std::thread` or crossterm `EventStream` polling. |
| 9 | Syntax highlighting parity (chroma v2 monokai + 18 overrides, terminal256, lexer by filename/ext) | syntect 5.3.0 (+ two-face for missing syntaxes), ansi mapping custom | **glue + custom** | See §2. Deep parity is bounded by taxonomy differences. |
| 10 | Raw mode, size, resize | crossterm `enable_raw_mode`, `size`, `window_size`, `Event::Resize` | **native** | crossterm 0.29 docs; Unix resize via signal-hook dependency (features page). |
| 11 | Bracketed paste | crossterm `EnableBracketedPaste` + `Event::Paste` (default feature) | **native** | `bracketed-paste` is a default feature (`docs.rs/crate/crossterm/latest/features`); paste payload preserves embedded ESC sequences (`parse.rs` test). |
| 12 | Kitty CSI-u keyboard protocol | crossterm `PushKeyboardEnhancementFlags` + CSI-u parser | **native** | Supports codepoints, modifier masks, event kinds, alternate keys, functional key codepoints (`parse.rs::parse_csi_u_encoded_key_code`). |
| 13 | xterm modifyOtherKeys `CSI 27;mod;code~` fallback | crossterm | **custom required** | crossterm dispatches `CSI ... ~` to `parse_csi_special_key_code`, which only accepts first params 1–8/11–34; `27` yields `could_not_parse_event_error` (`parse.rs`). Go maps it explicitly (`input.go:345-359`). |
| 14 | 50 ms bare-Esc peek | crossterm `event::poll(Duration)` + `read()`; internal parser emits Esc only when no further input is buffered | **glue** | `parse.rs` emits `Esc` iff `input_available == false`; caller's poll timeout defines the feeling. Go uses an explicit 50 ms `select()` peek (`input.go:169`, `resize_unix.go:36`). |
| 15 | SGR mouse wheel | crossterm `EnableMouseCapture` + `MouseEventKind::ScrollUp/Down` | **native** | SGR parser exists with wheel kinds (`parse_csi_sgr_mouse`); note Go never enables mouse today. |
| 16 | Ctrl+V clipboard | app-level; crossterm reports Ctrl+V as `KeyCode::Char('v')` + CONTROL | **native/trivial** | `parse_event` control-byte mapping; handler is custom on both sides. |
| 17 | Unicode display width | unicode-width 0.2.2; unicode-segmentation 1.13.3 | **native + glue** | See §3. unicode-width does string-level ligature handling; grapheme iteration for cursor/truncate boundaries needs unicode-segmentation. |
| 18 | ANSI-aware wrapping/truncation | no crate; strip-ANSI is app convention | **custom** | Go's `stripANSI` handles only CSI (`editor.go:921-942`); port ~250–400 lines. |
| 19 | Kitty image protocol (`a=T,f=100,C=1`, chunking, PNG-only) | ratatui-image (protocol encoding), viuer; else custom | **glue or custom** | `C=1` = "no cursor movement" and 4096-byte chunk rule are in the kitty spec. ratatui-image's `Picker`/`Protocol` does encoding but is widget/buffer-oriented and by default links `chafa` dynamically (`chafa-dyn` default feature) — poor fit for a line-string renderer. |
| 20 | iTerm2 OSC 1337 + `doNotMoveCursor` | viuer / ratatui-image (render only) | **custom for exact bytes** | iTerm2's current official inline-images page documents `name,size,width,height,preserveAspectRatio,inline`; **`doNotMoveCursor` is not documented there** (page fetched 2026-09-24). Go already defaults iTerm2 off because `doNotMoveCursor` "is not reliable across iTerm versions" (`image.go:76-84`). Treat iTerm2 support as best-effort. |
| 21 | PNG re-encode / footprint math | image 0.25.10 (decode/encode) | **native + custom math** | `image` default-formats include png/jpeg/gif/webp/tiff (`docs.rs/crate/image/latest/features`), but defaults also pull AVIF/EXR/rayon — recommend `default-features = false` + explicit formats. Footprint/aspect math (`AGENTE_CELL_ASPECT`) stays custom (`image.go:238-302`). |
| 22 | Clipboard text+image per OS | arboard 3.6.1; commands as in Go | **glue** | arboard supports Win (`clipboard-win`), macOS (`objc2-app-kit`), Linux X11 (`x11rb`) and Wayland (`wl-clipboard-rs`), with image data behind an optional `image ^0.25` dep (`docs.rs/arboard` deps). `ImageData` is raw RGBA8 pixels (`docs.rs/arboard/.../ImageData`), so PNG conversion is app code. Command fallbacks (`pbpaste`, `wl-paste`/`xclip`/`xsel`, PowerShell STA) remain valid; Linux command path avoids requiring an X11/Wayland client library. |
| 23 | OSC 11 auto light/dark | termbg 0.6.2; else custom | **glue** | termbg provides `theme()`, `rgb()`, `latency()` and an `EventReader` trait (`docs.rs/termbg`), but is pinned to crossterm **0.28** (dep list), creating a second crossterm major in the tree. Go path is 150 lines with `select()`; either accept the skew or port directly. |
| 24 | Alternate screen history browsing | crossterm `EnterAlternateScreen`/`LeaveAlternateScreen`; ratatui fullscreen Terminal | **native/glue** | Go's `DrawHistory` uses `?1049h/l` plus its own fixed-frame diff (`render.go:437-457`). A ratatui Terminal for alt-screen history is plausible but must not diff while the main-screen renderer is live. |
| 25 | OSC 7 cwd, OSC 12 cursor color, DECSCUSR cursor shape | none of the candidates (crossterm has `SetCursorStyle`; ratatui none) | **custom/glue** | crossterm `SetCursorStyle` covers DECSCUSR; OSC 7/12 are hand-written strings today (`terminal.go:137-179`, `layout.go:85-94`). |

### Candidate verdicts

- **crossterm 0.29.0** — primary platform layer. Covers raw mode, size/window size, resize events, bracketed paste, kitty CSI-u, SGR mouse, alternate screen, synchronized update, `ClearType::Purge`, DECSC/DECRC, DECSCUSR. Gaps: modifyOtherKeys, OSC 7/11/12, image protocols, no DECSTBM setter. It already depends on **rustix 1.x** for Unix termios, so "rustix-based approach" is effectively crossterm's foundation; going direct to rustix (`tcgetattr`/`tcsetattr`/`tcgetwinsize`/`isatty`, feature `termios`, docs.rs/rustix 1.1.5) buys nothing but input/output work.
- **ratatui 0.30.2** — optional, only for the alt-screen history viewport or a settings dialog. Its inline viewport (`Viewport::Inline`, `Terminal::insert_before`, `scrolling-regions`) is the closest crate feature to agente flow mode, but it maintains a cell buffer, not styled lines; adopting it for the main chat path means rewriting `view.go` output into `Line`/`Span` and losing byte-level control over per-row escapes and image lines. Windows scroll-region commands are unsupported.
- **termion 4.0.6** — excluded. README: "Supports Redox, Mac OS X, BSD, and Linux"; no Windows path. No advantage over crossterm.
- **unicode-width 0.2.2 / unicode-segmentation 1.13.3** — both needed; see §3.
- **syntect 5.3.0 (+ two-face 0.5.2)** — the only mature Rust Sublime-grammar highlighter; see §2.
- **image 0.25.10** — decode/encode + PNG normalization; trim features.
- **arboard 3.6.1** — text on all three OSes; image as RGBA. Keep Linux command fallback if an X11/Wayland client dep is unwanted.
- **ratatui-image 11.1.0 / viuer 0.11.0** — protocol encoders exist but neither targets "escape embedded inside a styled line string": ratatui-image is a ratatui widget with font-size querying and default chafa linking; viuer prints at coordinates via `termcolor`. Cheaper to port `image.go`'s ~300 lines than to bend either. (viuer 0.11.0 docs.rs build failed; details taken from its crates.io README/0.9.2 docs.)
- **termbg 0.6.2** — works, but crossterm 0.28 pin; likely custom.
- **ansi_colours 1.2.3** — RGB↔ANSI256 conversion (`ansi256_from_rgb`, `rgb_from_ansi256`, docs.rs). **License: LGPL-3.0-or-later**, not permissive; and its algorithm is not chroma's. For parity, reimplement chroma's mapping (see §2).

## 2. Syntax highlighting: syntect evidence and chroma-parity plan

### 2.1 Engine viability

- syntect 5.3.0 features (source `Cargo.toml`): `default = ["default-onig"]`; `default-onig` pulls `onig` (Oniguruma C library). The pure-Rust path is `default-fancy` (fancy-regex). Onig features: `plist-load` for `.tmTheme`, `yaml-load` for `.sublime-syntax`, `default-syntaxes`/`default-themes` from embedded dumps, `dump-load`/`dump-create` for binary dumps, `regex-onig` / `regex-fancy`.
- Onig availability risk: README states fancy-regex exists because "the `onig` crate ... requires building and linking the Oniguruma C library. Many users experience difficulty building the `onig` crate, especially on Windows and Webassembly." Agente's product constraint is one portable binary with a small footprint: shipping a C toolchain requirement in CI and cross-compiling `make release` is the blocker.
- fancy-regex cost/risk: README — "about half the speed of the default Oniguruma engine"; "absurdly slow in debug mode"; and feature unification means any transitive dep enabling onig wins. `two-face` adds a hard parity consequence: several syntaxes are **excluded** under `fancy` because the definitions use regex features fancy-regex cannot compile — its docs mark ARM Assembly, JavaScript (Babel), LiveScript, **PowerShell**, Sass, Salt State `*` as excluded. Agente's `extLang` maps `ps1 → powershell` (`highlight.go:292`), so fancy-regex mode loses highlighting for a mapped language. `two-face`'s docs also note onig default includes these.
- Startup cost: syntect README numbers — **~138 ms** to load+link all default syntax definitions raw, **~23 ms** from the internal binary dump with lazy regex compilation, ~250 ms end-to-end for `syncat` "mostly spent loading". Loading is one-time; the Rust port should load a dump (or ship a pre-built dump) at startup and keep it in an `Arc`. `two-face` adds ~961 KiB of syntax assets (onig; 937 KiB fancy) and ~61 KiB of themes (docs.rs/two-face "Embedded Asset Sizes").
- Theme loading: `ThemeSet::load_defaults()` ships `base16-ocean.dark`, `base16-eighties.dark`, `base16-mocha.dark`, `base16-ocean.light`, `InspiredGitHub`, `Solarized (dark)`, `Solarized (light)`; `get_theme(path)` / `load_from_folder` / `discover_theme_paths` load `.tmTheme` files (`ThemeSet` docs). Fully loadable themes: yes, but only `.tmTheme` (plist); agente's user-facing theme format is its own JSON (`theme_loader.go`) so no compat expectation is broken.
- Terminal output: syntect's only terminal helper is `util::as_24_bit_terminal_escaped` ("Formatted with 24-bit color terminal escape codes. Meant for debugging and testing."); there is no 256-color formatter in-crate (`syntect::util` docs). Chroma's `terminal256` formatter maps each style colour to the nearest entry of a hardcoded 256-entry xterm table using `Colour.Distance` (compuphase redmean; comment: "not as accurate as LAB ... but *vastly* simpler") and clears the style's `Background` before formatting (`formatters/tty_indexed.go`, `colour.go` at tag v2.23.1). Porting parity therefore means: custom nearest-palette mapping (~60 lines + table) or 24-bit output (visual difference and terminal support assumptions change).

### 2.2 Mapping the chroma style

- Base style: `styles.Get("monokai")` with 18 explicit token overrides plus `Background bg:` to clear the background (`highlight.go:196-237`). Style-entry grammar (chroma `StyleEntry`): `#rrggbb`, `bold`, `italic`, `underline`, `noinherit`, `bg:#...` (`style.go::ParseStyleEntry`, pkg.go.dev chroma/v2#StyleEntry). Rust equivalent: parse the 18 strings into `Color::RGB` + `FontStyle` bitflags feeding `Theme`/`Settings`.
- Token→scope taxonomy mismatch is the real labor (not code volume):
  - `Keyword` → `keyword`; `KeywordConstant` → `constant.language`; `KeywordDeclaration` → `storage.type`/`keyword.declaration`; `KeywordNamespace` → `keyword.control`/`storage.modifier`; `KeywordType` → `storage.type`; `NameBuiltin` → `support.function`/`support.type`; `NameFunction` → `entity.name.function`; `NameClass` → `entity.name.class`/`entity.name.type`; `NameDecorator` → `entity.name.function.decorator`/`meta.decorator`; `LiteralString` → `string`; `LiteralStringEscape` → `constant.character.escape`; `LiteralNumber` → `constant.numeric`; `Comment` → `comment`; `CommentPreproc` → `comment.preprocessor`; `Operator` → `keyword.operator`; `Punctuation` → `punctuation`; `Text` → fallback.
  - Base monokai colours remain available in syntect only as a `.tmTheme` you author/convert; since agente overrides the 19 tokens it actually consumes, the port can synthesize the base from chroma's monokai XML (chroma v2 styles are embedded XML; registry at `styles.Registry`) into a generated `.tmTheme`/in-memory `Theme`, then apply the overrides.
- Lexer selection: agente's map is ~50 extensions + 10 filename rules (`highlight.go:266-338`). Syntect default syntaxes lack TOML/TypeScript/TSX/Dockerfile/PowerShell/Fish/Nix/Protobuf/Svelte/Vue/Terraform/GraphQL — `two-face` supplies exactly those and more, with a full inclusion list (docs.rs/two-face "Syntaxes"). `SyntaxSet::find_syntax_by_extension`/`find_syntax_by_token` replace `chooseLexer`, and `SyntaxSet` is `Send+Sync` for a shared cache (`syntect` README "Parallelizing").
- Result shape: syntect yields styled ranges per line (`HighlightLines::highlight_line`), which maps cleanly onto agente's `[]string` of pre-styled rows and its LRU keyed by (style, lang, src) (`highlight.go:66-105`).
- Background stripping: keep agente's SGR `48`/`49`/`40-47`/`100-107` filter regardless (`highlight.go:124-177`); it is orthogonal to syntect's `Theme` background and is what makes the dark-grey tool boxes safe.

Verdict: syntect + two-face is the only credible path; **Onig is the default recommendation if the C build is acceptable, fancy-regex if purity beats PowerShell/Sass/build-time performance**. The 18-override mapping is heuristic by nature — token-for-token identity with chroma is not achievable because Sublime grammars tokenize differently; parity target should be "same palette roles, same visual hierarchy per language", validated per language with golden snippets.

## 3. Unicode width divergence risks

Go side (pinned versions): `mattn/go-runewidth v0.0.16` (+ `rivo/uniseg v0.2.0` indirect) — `go.mod`.

- v0.0.16 `StringWidth` iterates **grapheme clusters** (uniseg) and takes "the width of the first non-zero-width rune" per cluster (`runewidth.go` v0.0.16 source). `RuneWidth` is a wcwidth-style per-rune table; `EastAsianWidth` is auto-detected from locale at init (`handleEnv`, `RUNEWIDTH_EASTASIAN` override).
- Rust `unicode-width` 0.2.2 computes string width from whole-string rules with explicit exceptions: well-formed emoji ZWJ sequences = 2, emoji modifier sequences = 2, emoji presentation sequences = 2, text presentation sequences = 1 outside CJK, ZWJ/combining insertion transparent for named ligatures, plus Arabic/Buginese/Hebrew/Khmer/… ligature widths (crate docs "Rules for determining width"). `width()` vs `width_cjk()` is explicit; there is **no locale auto-detection**.
- Concrete divergence classes:
  1. **VS16 / emoji presentation**: `❤️` (U+2764 U+FE0F) is an emoji presentation sequence → 2 cells in Rust. In Go, the cluster's first rune U+2764 decides v0.0.16 `StringWidth`; if U+2764 is not in the doublewidth table it measures 1. (Exact Go value not verified against the generated table here; direction of the rule difference is verified in source.) Result: table padding/wrapping thresholds can shift by 1 cell for such content.
  2. **Rune-level vs cluster-level use inside Go itself**: `render.go` truncates with `runewidth.RuneWidth` per rune (`render.go:13,292`), while `editor.go` wraps/measures with cluster-aware `StringWidth` (`editor.go:610,657,851,917`). A ZWJ family therefore over-counts in truncation but under-counts (first-rune rule) in wrapping. A Rust port that uses a single `str::width()` everywhere will visibly change truncation of emoji strings; a port that uses `char::width()` preserves the Go truncation quirk. Decide per call site explicitly.
  3. **CJK ambiguous width**: Go flips a process-global from the locale (`LC_*`/`RUNEWIDTH_EASTASIAN`); Rust requires choosing `width_cjk()`. To keep parity, replicate the env heuristic in one place and never call `width_cjk()` implicitly.
  4. **ratatui's own handling**: `ratatui-crossterm` skips redundant cursor moves except for "a VS16 cluster's terminal cursor advance [which] is not reliably predictable" (`lib.rs` draw, with a commented constant) — evidence that even the Rust ecosystem treats VS16 width as terminal-dependent. Do not assume `unicode-width` == what the terminal paints; keep geometric calculations on one width function consistently.
- Recommendation: use `unicode-width` for `str` width, `unicode-segmentation::UnicodeSegmentation::graphemes` for boundary-safe truncation/wrapping, and carry an explicit policy enum mirroring the Go call sites (cluster width vs per-rune width) so parity tests can pin each. Port est. 250–400 lines for strip-ANSI + width + wrap + truncate utilities.

## 4. Custom-code gap list (rough Rust sizes)

Estimates are for a faithful first port (logic + tests), assuming idiomatic Rust and the recommended crates; they are not commitments.

| # | Gap | Recommended base | Est. Rust LOC |
|---|---|---|---:|
| 1 | Flow renderer: logical buffer, diff window, scroll/hardware-row tracking, reflow rebase, full-repaint triggers (image/selection/theme), synchronized fencing, VS Code policy, alt-screen history renderer, bottom margin | crossterm primitives | 900–1,300 |
| 2 | View/timeline builders: chat rows, tool boxes, diffs, compaction rows, selection, input layouts — port of `view.go` (2,676 lines) and `layout.go` | plain string building | 2,500–3,500 |
| 3 | Markdown pipeline: GFM tables with width fit/wrap, lists/quotes/headings, inline styling, partial fenced code, code-block highlighting hook | pulldown-cmark (parse) | 400–700 |
| 4 | Highlight mapping: chroma monokai base → syntect `Theme`, 18 overrides, style-entry parser (`#hex [bold|italic]`), terminal256 nearest-palette (redmean over xterm table), background-SGR stripper, lexer maps, memo | syntect + two-face | 300–500 |
| 5 | Clipboard + image platform layer: text read (3 OS), image read + normalization limits (32 MiB/25 MP/64 MiB), temp-file lifecycle, kitty/iTerm2 encoders, PNG transcode, footprint/cell-aspect math | arboard (text), image | 600–950 |
| 6 | Editor: multi-line buffer, visual-row navigation, word ops, mask, paste collapse + placeholders, drag-drop path chips, ANSI-aware wrap | unicode-width/segmentation | 1,100–1,600 |
| 7 | Input: crossterm integration + modifyOtherKeys extension + Esc 50 ms discipline + Ctrl+V mapping + paste payload normalization | crossterm | 120–300 |
| 8 | Partial JSON extractor (`ExtractPartialStringField`, `ExtractLastNewText`) | none | ~200 |
| 9 | Theme files: JSON schema (shared/dark/light, color value union int/hex/object), apply overrides, discovery/picker, OSC 11 detection + `AGENTE_THEME` | serde + custom OSC 11 (or termbg) | 450–700 |
| 10 | Stream pacer + redraw throttle + spinner tick + resize invalidation glue | std/tokio timers | ~150 |
| 11 | Terminal bootstrap: startup/teardown sequence set, OSC 7 report, cursor color/shape, image deletion, `\x1b[r` reset | crossterm + raw writes | 150–250 |
| | **Total** | | **≈ 7.0k–10.2k** (plus tests) |

Top five by risk: **(1) flow renderer**, **(2) view/timeline port**, **(3) highlight mapping + terminal256 parity**, **(4) clipboard/image platform layer**, **(5) markdown/partial-JSON streaming pipeline**. Items 6–11 are mechanical ports; item 7 is small but has one hard blocker (modifyOtherKeys).

## 5. Blockers and decision inputs for ticket 02

1. **Onig vs fancy-regex is a product decision, not a technical detail.** Onig = C toolchain + Windows cross-build friction; fancy = ~2× slower and **excludes PowerShell/Sass/etc. syntaxes** via two-face. A middle path: onig in release CI, fancy only for WASM/dev.
2. **No 256-color formatter in syntect.** Either port chroma's redmean mapper (~60 lines + table) or accept 24-bit, which changes output bytes and terminal support matrix.
3. **crossterm cannot parse modifyOtherKeys** (`CSI 27;...~`), which agente enables as fallback (`terminal.go:116-117`). Options: (a) accept loss (Shift+Enter etc. only on kitty-protocol terminals), (b) patch/fork crossterm, (c) keep agente's own input parser over raw `rustix`/stdin bytes for the fallback path. This is the single hardest input gap.
4. **termbg is pinned to crossterm 0.28** while ratatui 0.30 defaults to crossterm 0.29 (`crossterm_0_29` feature). Adopting termbg adds a second crossterm version; the Go OSC 11 path is ~150 lines.
5. **ratatui's inline viewport is not a drop-in for DrawLog.** Its Windows scroll-region support is `Unsupported`; its buffer model conflicts with pre-styled line strings and embedded image escapes. If ratatui is adopted at all, scope it to the alt-screen history browser.
6. **iTerm2 `doNotMoveCursor` is undocumented in the current official protocol page** (fetched 2026-09-24). Agente already disables iTerm2 by default; keep that policy unless a user forces it.
7. **ansi_colours is LGPL-3.0-or-later.** Avoid for a permissively licensed portable binary; implement the tiny mapper.
8. **Mouse wheel is dormant in Go** (parser exists, `SeqMouseOn` never emitted). Porting the parser is cheap; enabling it is a behavior change that should be a separate decision.

## 6. Primary sources

Crates (docs.rs pages fetched 2026-09-24):
- ratatui 0.30.2 crate/features/`Terminal`/`Viewport`: https://docs.rs/ratatui/latest/ratatui/index.html , https://docs.rs/ratatui/latest/ratatui/struct.Terminal.html
- ratatui-crossterm backend source (SavePosition = ESC7/8; scrolling-region commands; Windows `Unsupported`; VS16 cursor-move caveat): https://github.com/ratatui/ratatui/blob/main/ratatui-crossterm/src/lib.rs
- crossterm 0.29.0 terminal/event/features: https://docs.rs/crossterm/latest/crossterm/terminal/index.html , https://docs.rs/crossterm/latest/crossterm/event/index.html , https://docs.rs/crate/crossterm/latest/features , https://docs.rs/crossterm/latest/crossterm/terminal/enum.ClearType.html
- crossterm cursor source (`ESC7`/`ESC8`): https://github.com/crossterm-rs/crossterm/blob/master/src/cursor.rs
- crossterm Unix input parser (CSI-u, SGR mouse, bracketed paste, modifyOtherKeys absent): https://github.com/crossterm-rs/crossterm/blob/master/src/event/sys/unix/parse.rs
- termion README (Unix-only support list): https://github.com/redox-os/termion#readme
- rustix 1.1.5 termios: https://docs.rs/rustix/latest/rustix/termios/index.html
- unicode-width 0.2.2 (string rules, VS16/ZWJ/emoji, `cjk` feature): https://docs.rs/unicode-width/latest/unicode_width/
- unicode-segmentation 1.13.3: https://docs.rs/unicode-segmentation/latest/unicode_segmentation/
- syntect 5.3.0: https://docs.rs/syntect/latest/syntect/ , util (24-bit only): https://docs.rs/syntect/latest/syntect/util/ , ThemeSet: https://docs.rs/syntect/latest/syntect/highlighting/struct.ThemeSet.html , features/Cargo.toml: https://github.com/trishume/syntect/blob/master/Cargo.toml , README (onig difficulty, fancy-regex ~half speed/debug slowness, 138 ms/23 ms loading, cache/parallel notes): https://github.com/trishume/syntect#readme
- two-face 0.5.2 (extra syntaxes incl. TypeScript/TOML/Dockerfile/PowerShell/…; fancy-excluded list; asset sizes): https://docs.rs/two-face/latest/two_face/
- image 0.25.10 feature list: https://docs.rs/crate/image/latest/features
- arboard 3.6.1 (platform backends, ImageData): https://docs.rs/arboard/latest/arboard/index.html , https://docs.rs/arboard/latest/arboard/struct.ImageData.html
- ratatui-image 11.1.0 (picker/font-size query, chafa default, thread protocol): https://docs.rs/ratatui-image/latest/ratatui_image/index.html
- viuer (README; docs.rs build for 0.11.0 failed): https://docs.rs/crate/viuer/latest , https://docs.rs/viuer/0.9.2/viuer/
- termbg 0.6.2 (OSC 11 helper; crossterm 0.28 dep): https://docs.rs/termbg/latest/termbg/
- ansi_colours 1.2.3 (RGB↔ANSI256; LGPL-3.0-or-later): https://docs.rs/ansi_colours/latest/ansi_colours/
- pulldown-cmark 0.13.4 Options (tables/Gfm/strikethrough/tasklists): https://docs.rs/pulldown-cmark/latest/pulldown_cmark/struct.Options.html

Protocols:
- Kitty graphics protocol (`f=100` PNG, 4096-byte chunk rule, `C=1` no-cursor-move, delete actions): https://sw.kovidgoyal.net/kitty/graphics-protocol/
- iTerm2 inline images (documented keys; `doNotMoveCursor` absent as of fetch): https://iterm2.com/documentation-images.html

Go dependencies surveyed for parity:
- go-runewidth v0.0.16 source (uniseg-based `StringWidth`, first-non-zero-rune rule, locale auto-detect): https://github.com/mattn/go-runewidth/blob/v0.0.16/runewidth.go
- rivo/uniseg README (grapheme clusters, `StringWidth("🇩🇪🏳️🌈!") == 5`): https://github.com/rivo/uniseg#readme
- chroma v2.23.1 `formatters/tty_indexed.go` (256-table, nearest match, background clear): https://github.com/alecthomas/chroma/blob/v2.23.1/formatters/tty_indexed.go
- chroma v2.23.1 `colour.go` (`Distance` = compuphase redmean): https://github.com/alecthomas/chroma/blob/v2.23.1/colour.go
- chroma `StyleEntry` grammar: https://pkg.go.dev/github.com/alecthomas/chroma/v2#StyleEntry ; styles registry (XML-embedded styles, `Get`/`Fallback`): https://pkg.go.dev/github.com/alecthomas/chroma/v2/styles

Local (Agente, this repo): `packages/tui/render.go`, `terminal.go`, `input.go`, `editor.go`, `highlight.go`, `markdown.go`, `partialjson.go`, `image.go`, `theme.go`, `theme_loader.go`, `layout.go`, `detect_bg.go`, `clipboard_*.go`, `resize_*.go`; `packages/agent/modes/interactive.go`; `go.mod`.

## 7. Context7 spot-check (2026-09-24)

- ratatui 0.30 `Terminal::insert_before` + `Viewport::Inline`: docs confirm inserted content renders above the viewport, pushing it down until it reaches the bottom of the screen; further insertions scroll the area above upward and excess lines spill into terminal scrollback (if the viewport occupies the whole screen, all inserted lines go straight to scrollback). This strengthens ratatui as a flow-mode candidate for the scrollback path, but the cell-buffer vs pre-styled-line-string mismatch and the Windows scroll-region limitation still stand — row 1 keeps its "custom (glue at best)" verdict.
- All other decision-relevant crates (crossterm, syntect/two-face, unicode-width, image, arboard) were already primary-source cited and needed no correction.
