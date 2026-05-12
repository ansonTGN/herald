# Herald

Multi-tenant authentication and authorization system built with Rust and React, developed using Claude Code + GLM model 以及 Codex 混合开发。

本项目用于实践 AI编程。

给人类阅读的在 human/*.md

## Prerequisites

- **Python 3.12+** (use [uv](https://github.com/astral-sh/uv) for version management)
  ```bash
  curl -LsSf https://astral.sh/uv/install.sh | sh
  uv python install 3.12
  uv python pin 3.12
  ```
- Docker (for development/test environments)
- Cargo (for backend development)
- npm (for frontend development)

## Quick Start

```bash
# Install Python 3.12 (see Prerequisites)
uv run scripts/dev-start.py
```

## License

Apache-2.0
