# Herald

[中文](README-zh.md) | English

An out-of-the-box SaaS foundation: multi-tenant account system, Stripe/Creem payment integration, and a built-in credits system. Single-process deployment, Docker in production.

This project practices AI-assisted programming using Claude Code + GLM model and Codex hybrid development.

## Features

- **Day-one SaaS foundation** — multi-tenant accounts, auth, billing, and admin, ready out of the box
- **Flexible authentication** — email/password, social login (Google / GitHub / Apple / Facebook / WeChat), passkeys, 2FA, and bot protection
- **Subscriptions & one-time payments** — Stripe and Creem; entitlements auto-provision on payment
- **Payment-driven paywalls** — purchases grant access, refunds and churn revoke it
- **Built-in credits wallet** — prepaid credits with top-ups, expiry, refunds, and a per-user ledger; ideal for AI and metered pricing
- **Single sign-on across your apps** — one Herald login unlocks every product, including device and WeChat Mini Program flows
- **Custom domain & white-label** — your domain, brand name, logo, and transactional emails
- **Compliance built in** — versioned agreements, consent capture, and a full audit trail
- **All-in-one admin console** — users, roles, billing, credits, apps, and per-tenant settings
- **Open & integration-ready** — auto-generated API docs and SDKs to connect your backend fast

## Quick Start

Requires Python 3.12+ ([uv](https://github.com/astral-sh/uv)), Docker, Cargo, npm.

```bash
uv run scripts/demo-start.py
```

Once running: frontend at http://localhost:3000 , backend API at http://localhost:8080 .

## Links

- **Website**: https://www.fornetcode.com
- **Live demo**: https://auth.fornetcode.com (admin@fornetcode.com / Herald@2026Admin)

## License

[Apache-2.0](LICENSE)
