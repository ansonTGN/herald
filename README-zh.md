# Herald

中文 | [English](README.md)

开箱即用的 SaaS 底座：多租户账户体系、对接 Stripe / Creem 支付、自带积分系统。单体部署，Docker 上线。

本项目用于实践 AI 编程，使用 Claude Code + GLM 模型以及 Codex 混合开发。AI 开发套件均基于 [web-dev-skills](https://github.com/timzaak/web-dev-skills) 构建。

## 核心特性

- **开箱即用的 SaaS 底座** — 多租户账户、认证、计费与后台，全部就绪
- **灵活的认证** — 邮箱密码、社交登录（Google / GitHub / Apple / Facebook / 微信）、Passkey、二步验证、人机验证
- **订阅与一次性付费** — 对接 Stripe、Creem，付款到账自动发放权益
- **支付驱动的付费墙** — 购买即开通，退款或流失即收回
- **自带积分钱包** — 预付积分，支持充值、过期、退款、用户账本，适配 AI 与计量计价
- **跨应用单点登录** — 一次 Herald 登录打通所有产品，含设备授权与微信小程序
- **自定义域名与白标** — 你的域名、品牌名、Logo 与交易邮件
- **内置合规** — 版本化协议、同意记录、完整审计轨迹
- **一体化后台** — 用户、角色、计费、积分、应用、各租户设置一站管理
- **开放易集成** — 自动生成 API 文档与 SDK，快速接入现有后端

## 快速开始

需要 Python 3.12+（[uv](https://github.com/astral-sh/uv)）、Docker、Cargo、npm。

```bash
uv run scripts/demo-start.py
```

启动完成后前端在 http://localhost:3000 ，后端 API 在 http://localhost:8080 。

## 链接

- **官网**：https://www.fornetcode.com
- **在线演示**：https://auth.fornetcode.com （admin@fornetcode.com / Herald@2026Admin）

## 许可证

[Apache-2.0](LICENSE)
