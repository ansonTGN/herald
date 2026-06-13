# Herald

[中文](README-zh.md) | English

An out-of-the-box SaaS foundation: multi-tenant account system, Stripe/Creem payment integration, and a built-in credits system. Rust backend + React frontend, single-process deployment, Docker in production.

This project practices AI-assisted programming using Claude Code + GLM model and Codex hybrid development.

## Features

- **SaaS account system** — multi-tenant (realm) architecture with authentication, authorization, and an admin console out of the box
- **Payments** — Stripe and Creem integration for subscriptions, invoices, and webhook-driven entitlement provisioning
- **Credits system** — built-in wallet with transactions, scheduled grants, expiration, and idempotency

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

Full tutorials at [docs/tutorials/](docs/tutorials/), covering local development, architecture, configuration, deployment, and billing.

- [Getting Started](docs/tutorials/getting-started-en.md) — local development setup
- [Architecture](docs/tutorials/architecture-en.md) — project structure and tech choices
- [Configuration](docs/tutorials/configuration-en.md) — configuration reference
- [Deployment](docs/tutorials/deployment-en.md) — Docker production deployment
- [Billing Architecture](docs/tutorials/billing-overview.md) — entitlement mapping, subscription projection, credits policy
- [Stripe Integration](docs/tutorials/billing-stripe-payment.md) — provider setup and webhook handling
- [Creem Integration](docs/tutorials/billing-creem-payment.md) — provider setup and webhook handling
- [Invoice Management](docs/tutorials/billing-invoice.md) — invoice creation, issuance, and PDF
- [Third-Party Integration](docs/tutorials/third-party-integration-en.md) — integrate via the SDK

## License

[Apache-2.0](LICENSE)
