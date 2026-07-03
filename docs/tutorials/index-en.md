# Herald

An out-of-the-box SaaS foundation: multi-tenant account system, Stripe/Creem payment integration, and a built-in credits system. Rust backend + React frontend, single-process deployment, Docker in production.

## Who this is for

Developers with Rust or React experience, in their first two weeks on the project.

## Prerequisites

- Rust basics (comfortable reading Axum handlers and SeaORM queries)
- React + TypeScript basics
- Docker fundamentals

## Chapters

- [Getting Started](getting-started-en.md) — Local development environment setup
- [Architecture](architecture-en.md) — Project structure and technology choices
- [Configuration](configuration-en.md) — Configuration reference
- [Deployment](deployment-en.md) — Docker production deployment
- [Billing Architecture](billing-overview.md) — Credit Account, Entitlement Mapping, subscription projection, metadata contract, points policy
- [Stripe Integration](billing-stripe-payment.md) — Stripe payment provider setup and webhook handling
- [Creem Integration](billing-creem-payment.md) — Creem payment provider setup and webhook handling
- [Invoice Management](billing-invoice.md) — Invoice creation, issuance, and PDF generation
- [Third-Party Integration](third-party-integration-en.md) — Integrate with Herald using the SDK for auth, permissions, points, and subscriptions
