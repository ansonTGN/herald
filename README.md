# Herald

[中文](README-zh.md) | English

An out-of-the-box SaaS foundation: multi-tenant account system, Stripe/Creem payment integration, and a built-in credits system. Rust backend + React frontend, single-process deployment, Docker in production.

This project practices AI-assisted programming using Claude Code + GLM model and Codex hybrid development.

## Features

- **SaaS account system** — multi-tenant (realm) architecture with authentication, authorization, and an admin console out of the box
- **Auth & social login** — email/password login, OAuth providers (Google / GitHub / Apple / Facebook / WeChat), and optional TOTP two-factor authentication
- **Payments** — Stripe and Creem integration for subscriptions, invoices, and webhook-driven entitlement provisioning
- **Credits system** — built-in wallet with transactions, scheduled grants, expiration, and idempotency
- **Developer-friendly** — auto-generated OpenAPI / Swagger, OpenTelemetry tracing, and a Rust SDK for integrating third-party backends

## Quick Start

Requires Python 3.12+ ([uv](https://github.com/astral-sh/uv)), Docker, Cargo, npm.

```bash
uv run scripts/demo-start.py
```

Once running: frontend at http://localhost:3000 , backend API at http://localhost:8080 .

## Demo

- **URL**: https://auth.fornetcode.com
- **Admin**: admin@fornetcode.com / Herald@2026Admin

## License

[Apache-2.0](LICENSE)
