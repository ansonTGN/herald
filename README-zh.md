# Herald

中文 | [English](README.md)

开箱即用的 SaaS 底座：多租户账户体系、对接 Stripe / Creem 支付、自带积分系统。Rust 后端 + React 前端，单体部署，Docker 上线。

本项目用于实践 AI 编程，使用 Claude Code + GLM 模型以及 Codex 混合开发。

## 核心特性

- **SaaS 账户体系** — 多租户（Realm）架构，开箱即用的认证、授权与后台管理
- **支付集成** — 对接 Stripe、Creem，支持订阅、发票与 Webhook 驱动的权益发放
- **积分系统** — 自带积分钱包，支持交易流水、定时发放、过期与幂等

## 技术栈

- **后端**：Rust 2024 edition / Axum 0.8 / SeaORM 1.1 / PostgreSQL 16+ / Redis
- **前端**：React 19 / TypeScript / TanStack Router & Query / Tailwind CSS v4 / Vite
- **部署**：Docker 多阶段构建 + Caddy TLS 反代

## 快速开始

需要 Python 3.12+（[uv](https://github.com/astral-sh/uv)）、Docker、Cargo、npm。

```bash
uv run scripts/demo-start.py
```

启动完成后前端在 http://localhost:3000，后端 API 在 http://localhost:8080。

## 演示

- **地址**：https://auth.fornetcode.com
- **管理员**：admin@fornetcode.com / Herald@2026Admin

## 文档

完整教程见 [docs/tutorials/](docs/tutorials/)，涵盖本地开发、架构、配置、部署和计费。

- [快速上手](docs/tutorials/getting-started.md) — 本地开发环境搭建
- [架构](docs/tutorials/architecture.md) — 项目结构和技术选型
- [配置](docs/tutorials/configuration.md) — 配置项说明
- [部署](docs/tutorials/deployment.md) — Docker 生产环境部署
- [计费架构](docs/tutorials/billing-overview.md) — Entitlement Mapping、订阅投影、积分策略
- [Stripe 支付对接](docs/tutorials/billing-stripe-payment.md) — 支付方配置和 Webhook 处理
- [Creem 支付对接](docs/tutorials/billing-creem-payment.md) — 支付方配置和 Webhook 处理
- [发票管理](docs/tutorials/billing-invoice.md) — 发票创建、开票、PDF 生成
- [第三方后端对接](docs/tutorials/third-party-integration.md) — 用 SDK 接入 Herald

## 许可证

[Apache-2.0](LICENSE)
