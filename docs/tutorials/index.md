# Herald

开箱即用的 SaaS 底座：多租户账户体系、对接 Stripe / Creem 支付、自带积分系统。Rust 后端 + React 前端，单体部署，Docker 上线。

## 给谁看

有 Rust 或 React 经验的开发者，刚加入项目的前两周。

## 前置知识

- Rust 基础（能读懂 Axum handler 和 SeaORM 查询）
- React + TypeScript 基础
- Docker 基本操作

## 章节

- [快速上手](getting-started.md) — 本地开发环境搭建
- [架构](architecture.md) — 项目结构和技术选型
- [配置](configuration.md) — 配置项说明
- [部署](deployment.md) — Docker 生产环境部署
- [计费架构](billing-overview.md) — Credit Bucket、Entitlement Mapping、订阅投影、Metadata 契约、积分策略
- [Stripe 支付对接](billing-stripe-payment.md) — Stripe 支付方配置和 Webhook 处理
- [Creem 支付对接](billing-creem-payment.md) — Creem 支付方配置和 Webhook 处理
- [发票管理](billing-invoice.md) — 发票创建、开票、PDF 生成
- [第三方后端对接](third-party-integration.md) — 用 SDK 接入 Herald 的认证、权限、积分、订阅
