# Agente

Agente is a compact, provider-neutral Go coding-agent harness built as a single portable binary.

## Features

- **Single Portable Binary**: Zero heavy external dependencies, compiling to a standalone binary for macOS, Linux, and Windows.
- **Provider-Neutral Agent Engine**: Built-in support for 30+ providers including Anthropic, OpenAI/Codex, Google Gemini/Vertex, Kimi, DeepSeek, GitHub Copilot, Amazon Bedrock, Azure OpenAI, OpenRouter, Groq, Cerebras, Mistral, Ollama, and local models.
- **Terminal UI**: Fast, responsive terminal interface without heavy framework overhead.
- **Multiple Execution Modes**:
  - **Interactive TUI**: Full-screen conversation with syntax highlighting, diff viewing, and command history.
  - **Print (`-p`)**: Run once and print the final output to stdout.
  - **Streaming (`--stream`)**: Stream tokens in real time.
  - **JSON (`--json`)**: Stream structured JSON lines for automated pipelines.
  - **JSON-RPC (`rpc`)**: Standardized loop for editor and tooling integrations.
  - **Telegram Bot (`telegram-bot`)**: Optional two-way Telegram gateway.
- **Extensibility**:
  - Subprocess JSON-RPC extensions in any language (Go, Python, TypeScript, Rust, shell).
  - Custom color themes via simple JSON definitions.
  - Project and global standing instructions via `AGENTS.md`.
  - Reusable skills discovery via `SKILL.md`.
  - Portable packaged agents.

---

## Getting Started

### Prerequisites

- Go 1.25+ (for building from source)
- Git

### Building

Clone or navigate to the repository and build the binary:

```bash
# Build the binary into bin/agente
make build

# Or using standard Go tooling
go build -o bin/agente ./cmd/agente
```

### Running

```bash
# Launch interactive TUI
./bin/agente

# Run a single prompt in print mode
./bin/agente -p "explain what this repository does"

# Stream response directly to terminal
./bin/agente --stream "write a hello world in Go"

# Pass a prompt via stdin
echo "summarize README.md" | ./bin/agente
```

---

## Configuration & Authentication

All state (configuration, credentials, active sessions, and logs) is stored in `$AGENTE_HOME` (defaults to `~/.agente` or the OS application data directory).

To authenticate with your preferred provider:

1. Launch interactive mode: `./bin/agente`
2. Run `/login` to configure credentials interactively, or provide API keys via standard environment variables (e.g. `ANTHROPIC_API_KEY`, `OPENAI_API_KEY`, `GEMINI_API_KEY`).

---

## Repository Structure

```
.
├── cmd/
│   └── agente/              # Main CLI entrypoint
├── packages/
│   ├── core/                # Core agent loop, session transcripts, compaction, tools
│   ├── provider/            # Provider implementations, wire formats, and authentication
│   ├── agent/               # Runtime assembly, CLI parsing, modes, extensions, skills
│   │   ├── modes/           # Interactive, print, stream, json, rpc, telegram
│   │   ├── extensions/      # Subprocess extension host
│   │   └── skills/          # Skill discovery and built-in guides
│   ├── tui/                 # Terminal rendering, ANSI handling, markdown parser, themes
│   └── ignore/              # File ignore matching (.gitignore semantics)
├── docs/                    # Architectural and extension guides
├── AGENTS.md                # Development agreement and repository rules
├── Makefile                 # Build and test orchestration
├── go.mod                   # Module definition (agente)
└── LICENSE                  # MIT License
```

---

## Development & Verification

Run tests:

```bash
make test
```

Format code:

```bash
make fmt
```

Run linter:

```bash
make lint
```

---

## License

This project is licensed under the [MIT License](LICENSE).
