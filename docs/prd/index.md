# PRD 文档索引

本文档索引列出所有 Herald 系统的产品需求文档（PRD）。

## 文档组织结构

```
docs/
├── prd/                    # 产品需求文档（Product Requirements）
│   ├── index.md           # 本文件 - PRD 全局索引
│   ├── core/              # 核心功能（Realm、用户、审计等）
│   ├── auth/              # 认证与授权（OAuth、TOTP、权限等）
│   ├── billing/           # 计费与订阅（订阅、积分、支付等）
│   └── integration/       # 集成与扩展（Client App、SDK 等）
└── user-stories/           # 用户故事（User Stories）
    ├── index.md           # 用户故事索引
    ├── core/              # 核心功能用户故事
    ├── auth/              # 认证授权用户故事
    ├── billing/           # 计费相关用户故事
    └── integration/       # 集成相关用户故事
```

## 文档类型说明

| 文档类型 | 目的 | 受众 | 更新频率 |
|---------|------|------|---------|
| **PRD** | 描述产品功能、用户需求、业务规则 | 产品经理、开发、测试 | 功能开发前/中更新 |
| **用户故事** | 从用户视角描述功能需求 | 开发、测试 | 功能开发中持续更新 |
| **技术规范** | 描述技术实现细节、架构设计 | 开发、架构师 | 技术设计决策时更新 |

## PRD 文档列表

### Core 核心功能

| PRD 文档 | 标题 | 状态 | 相关角色 |
|---------|------|------|---------|
| [realm.md](core/realm.md) | Realm 管理 | ✅ Implemented | Admin Realm, Realm Admin |
| [users.md](core/users.md) | 用户管理 | ✅ Implemented | Realm Admin, Regular User |
| [realm-settings.md](core/realm-settings.md) | Realm 设置 | 🚧 Partially Implemented | Realm Admin |
| [audit.md](core/audit.md) | Audit 审计日志 | 📝 Draft | Realm Admin, Admin Realm |
| [dashboard.md](core/dashboard.md) | Dashboard | 🚧 Partially Implemented | Realm Admin |

### Auth 认证与授权

| PRD 文档 | 标题 | 状态 | 相关角色 |
|---------|------|------|---------|
| [oauth.md](auth/oauth.md) | OAuth 与第三方集成 | 🚧 Partially Implemented | Realm Admin, Regular User, Third-Party App |
| [wechat-oauth.md](auth/wechat-oauth.md) | 微信 OAuth 集成 | ✅ Implemented | Realm Admin, Regular User |
| [totp.md](auth/totp.md) | TOTP 二次认证 | ✅ Implemented | TOTP User, Realm Admin |
| [permissions.md](auth/permissions.md) | 权限管理 | ✅ Implemented | Realm Admin |
| [device-code.md](auth/device-code.md) | Device Code 登录 | 🚧 Partially Implemented | Third-Party App, Regular User, Realm Admin |

### Billing 计费与订阅

| PRD 文档 | 标题 | 状态 | 相关角色 |
|---------|------|------|---------|
| [subscription.md](billing/subscription.md) | 订阅计费与变更历史 | 🚧 Partially Implemented | Realm Admin, Regular User |
| [product-catalog.md](billing/product-catalog.md) | Product 编目管理 | ✅ Implemented | Realm Admin |
| [points.md](billing/points.md) | 积分系统（含免费用户积分） | ✅ Implemented | Realm Admin, Regular User |
| [unified-purchase.md](billing/unified-purchase.md) | 统一购买架构 | 🚧 Partially Implemented | Realm Admin, Regular User |
| [stripe-payment.md](billing/stripe-payment.md) | Stripe 支付集成 | 🚧 Partially Implemented | Realm Admin |
| [shopify-pay.md](billing/shopify-pay.md) | Shopify Pay 支付集成 | 🚧 Partially Implemented | Realm Admin |
| [wechat-pay.md](billing/wechat-pay.md) | 微信支付集成 | 🚧 Partially Implemented | Realm Admin, Regular User |
| [invoice.md](billing/invoice.md) | Invoice 发票管理 | ✅ Implemented | Realm Admin, Regular User |

### Integration 集成与扩展

| PRD 文档 | 标题 | 状态 | 相关角色 |
|---------|------|------|---------|
| [client-app.md](integration/client-app.md) | Client App 管理 | ✅ Implemented | Realm Admin, Third-Party App |
| [sdk.md](integration/sdk.md) | SDK 资源管理 | 🚧 Partially Implemented | Third-Party App |

## 状态说明

| 状态 | 说明 | 示例 |
|------|------|------|
| ✅ Implemented | 已完全实现并通过测试 | permissions.md, users.md |
| 🚧 Partially Implemented | 部分实现（核心功能完成，边缘功能待实现） | subscription.md, device-code.md, sdk.md |
| 📝 Draft | 草稿阶段（待评审或实现） | — |

## 相关文档

- **用户故事索引**: [docs/user-stories/index.md](/docs/user-stories/index.md)

## PRD 分层约束

- PRD 只承载业务范围、规则、约束、验收目标与必要的交互边界。
- PRD 不承载接口端点清单、请求响应 schema、状态码矩阵、数据库建表/迁移细节或代码类型定义。
- 详细接口契约、数据库结构和实现方案应下沉到技术设计、接口说明和代码。
