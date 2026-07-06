# Herald

中文 | [English](README.md)

开箱即用的 SaaS 底座：多租户账户体系、对接 Stripe / Creem 支付、自带积分系统。Rust 后端 + React 前端，单体部署，Docker 上线。

本项目用于实践 AI 编程，使用 Claude Code + GLM 模型以及 Codex 混合开发。

## 核心特性

- **SaaS 账户体系** — 多租户（Realm）架构，开箱即用的认证、授权与后台管理
- **认证与社交登录** — 邮箱密码登录，支持 Google / GitHub / Apple / Facebook / 微信等 OAuth 提供商，可选 TOTP 两步验证
- **支付集成** — 对接 Stripe、Creem，支持订阅、发票与 Webhook 驱动的权益发放
- **积分系统** — 自带积分钱包，支持交易流水、定时发放、过期与幂等
- **开发者友好** — OpenAPI / Swagger 自动生成，OpenTelemetry 链路追踪，提供 Rust SDK 便于第三方后端接入

## 快速开始

需要 Python 3.12+（[uv](https://github.com/astral-sh/uv)）、Docker、Cargo、npm。

```bash
uv run scripts/demo-start.py
```

启动完成后前端在 http://localhost:3000 ，后端 API 在 http://localhost:8080 。

## 演示

- **地址**：https://auth.fornetcode.com
- **管理员**：admin@fornetcode.com / Herald@2026Admin

## 许可证

[Apache-2.0](LICENSE)
