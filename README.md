# Herald

[中文](README-zh.md) | English

Multi-tenant authentication and authorization system. Rust backend + React frontend, single-process deployment, Docker in production.

This project practices AI-assisted programming using Claude Code + GLM model and Codex hybrid development.

## Tech Stack

- **Backend**: Rust 2024 edition / Axum 0.8 / SeaORM 1.1 / PostgreSQL 16+ / Redis
- **Frontend**: React 19 / TypeScript / TanStack Router & Query / Tailwind CSS v4 / Vite
- **Deployment**: Docker multi-stage build + Caddy TLS reverse proxy

## Quick Start

Requires Python 3.12+ ([uv](https://github.com/astral-sh/uv)), Docker, Cargo, npm.

```bash
uv run scripts/demo-start.py
```

Once running: frontend at http://localhost:3000, backend API at http://localhost:8080.

## Demo

- **URL**: https://auth.fornetcode.com
- **Admin**: admin@fornetcode.com / Herald@2026Admin

## Documentation

Full tutorials at [docs/tutorials/](docs/tutorials/), covering local development, architecture, configuration, and deployment.

- [Getting Started](docs/tutorials/getting-started-en.md)
- [Architecture](docs/tutorials/architecture-en.md)
- [Configuration](docs/tutorials/configuration-en.md)
- [Deployment](docs/tutorials/deployment-en.md)

## License

[Apache-2.0](LICENSE)
