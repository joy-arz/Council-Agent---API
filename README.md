# council agent api (rust)

a high-performance, multi-agent ai engineering council built with rust. this system orchestrates local cli agents (like `gemini-cli` or `qwen-cli`) to collaborate as active engineering partners, directly modifying the project's codebase until tasks are complete.

> **note:** this project is inspired by pewdiepie's local council ai, and even though this may not be as good as his, it aims to provide a powerful local multi-agent experience.

## features

- **modern oled dashboard**: a minimalist, deep-black interface designed for high-density engineering workflows.
- **active engineering partners**: agents don't just talk; they **act**. in autonomous mode, they use tools to edit files, refactor code, and run shell commands.
- **propose & review**: in non-autonomous mode, agents suggest file changes that can be reviewed and applied with a single click.
- **workspace folder selection**: browse and select your project directory directly from the web ui using a native folder picker (supports macos and windows).
- **workflow-driven deliberation**: the council follows a structured engineering journey (architect -> reviewer -> refactorer -> maintainer -> lead engineer).
- **council configuration**: dynamically map different local cli models to specific workflow roles directly from the sidebar.
- **project state persistence**: maintains a `.council_state.md` file in the workspace to track progress across sessions.
- **autonomous mode toggle**: a safety switch that grants agents permission to use their internal tools (like `write_file`) to modify the local workspace.
- **real-time streaming**: watch the council's deliberation and file edits in real-time via sse.

---

## 🚀 getting started

### 1. prerequisites

- **rust**: install via [rustup](https://rustup.rs/).
- **cli agents**: ensure you have agents like `gemini-cli` or `qwen-cli` installed and accessible in your path.

### 2. configuration

set up your environment variables by copying the template:

```bash
cp .env.example .env
```

open `.env` and map your agent roles to your local binaries:

```env
# cli binary mapping
STRATEGIST_BINARY=gemini-cli
CRITIC_BINARY=qwen-cli
OPTIMIZER_BINARY=gemini-cli
CONTRARIAN_BINARY=qwen-cli
JUDGE_BINARY=gemini-cli

# optional defaults
WORKSPACE_DIR=/absolute/path/to/default/project
AUTONOMOUS_MODE=false
MAX_ROUNDS=2
MAX_TOKENS_PER_AGENT=1000
```

### 3. installation

```bash
cargo build --release
```

---

## 🛠 usage

### server mode (web ui)
start the web interface on `localhost:8000`:

```bash
cargo run -- --server
```
*use the **workspace path** input in the sidebar to select the directory where agents will perform their work. toggle **autonomous** to allow agents to modify code.*

---

## 🧠 the council (workflow-specific)

1. **architect**: designs the technical roadmap and implements the foundational changes.
2. **reviewer**: rigorously checks implementation for bugs, security risks, and edge cases.
3. **refactorer**: optimizes code for performance, readability, and style adherence.
4. **maintainer**: ensures long-term sustainability and documents architectural decisions.
5. **lead engineer**: evaluates the entire workflow and issues a project status: `FINISHED`, `CONTINUE`, or `PAUSED`.

---

## 🔒 security & safety

- **read-only mode**: by default, agents are instructed not to modify files.
- **autonomous mode**: when enabled, agents are given explicit permission to use their internal tools (like `write_file`, `replace`, `run_shell_command`) to edit the local workspace.
- **stateful continuity**: the council reads `.council_state.md` at the start of every session to ensure they pick up exactly where they left off.
- **xss protection**: all agent output and tool logs are rendered safely in the ui.