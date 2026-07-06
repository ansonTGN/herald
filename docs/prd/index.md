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

## PRD 文档列表

### Core 核心功能

| PRD 文档 | 标题 | 相关角色 |
|---------|------|---------|
| [realm.md](core/realm.md) | Realm 管理 | Admin Realm, Realm Admin |
| [users.md](core/users.md) | 用户管理 | Realm Admin, Regular User |
| [realm-settings.md](core/realm-settings.md) | Realm 设置 | Realm Admin |
| [audit.md](core/audit.md) | Audit 审计日志 | Realm Admin, Admin Realm |
| [dashboard.md](core/dashboard.md) | Dashboard | Realm Admin |
| [i18n.md](core/i18n.md) | 国际化（i18n）支持 | All Users |
| [legal-consent-account-deletion.md](core/legal-consent-account-deletion.md) | 合规适配（用户协议 / 隐私政策 / 账户注销） | Regular User, Realm Admin |

### Auth 认证与授权

| PRD 文档 | 标题 | 相关角色 |
|---------|------|---------|
| [oauth.md](auth/oauth.md) | OAuth 与第三方集成 | Realm Admin, Regular User, Third-Party App |
| [wechat-oauth.md](auth/wechat-oauth.md) | 微信 OAuth 集成 | Realm Admin, Regular User |
| [totp.md](auth/totp.md) | TOTP 二次认证 | TOTP User, Realm Admin |
| [permissions.md](auth/permissions.md) | 权限管理 | Realm Admin |
| [device-code.md](auth/device-code.md) | Device Code 登录 | Third-Party App, Regular User, Realm Admin |

### Billing 计费与订阅

| PRD 文档 | 标题 | 相关角色 |
|---------|------|---------|
| [subscription.md](billing/subscription.md) | 订阅计费、Entitlement 映射、Webhook 处理（含 One-time 购买） | Realm Admin, Regular User, Third-Party App, System |
| [support-multiple-price.md](billing/support-multiple-price.md) | 多价格 Entitlement 映射（Product→Price 对齐、按价格配置/解析/购买） | Realm Admin, Regular User, System |
| [points.md](billing/points.md) | 积分系统（含发放、免费用户积分、发放时序与可用性） | Realm Admin, Regular User, Third-Party App |
| [stripe-payment.md](billing/stripe-payment.md) | Stripe 支付集成 | Realm Admin |
| [invoice.md](billing/invoice.md) | Invoice 发票管理（含 Provider 发票同步和自研 Fallback） | Realm Admin, Regular User |
| [payment-invoice-mapping.md](billing/payment-invoice-mapping.md) | 支付-发票强制映射（订阅续费支付记录、外部发票本地归属、归属异常可观测） | Realm Admin, System |
| [sync-payment.md](billing/sync-payment.md) | 支付产品同步增强（产品名主标签 / 价格单位修正 / Stripe metadata 同步 / 计费周期只读） | Admin Realm |

### Integration 集成与扩展

| PRD 文档 | 标题 | 相关角色 |
|---------|------|---------|
| [client-app.md](integration/client-app.md) | Client App 管理 | Realm Admin, Third-Party App |
| [sdk.md](integration/sdk.md) | SDK 资源管理 | Third-Party App |
| [api-key-roles.md](integration/api-key-roles.md) | API Key 角色绑定 | Realm Admin |

## 相关文档

- **用户故事索引**: [docs/user-stories/index.md](/docs/user-stories/index.md)

## PRD 分层约束

- PRD 只承载业务范围、规则、约束、验收目标与必要的交互边界。
- PRD 不承载接口端点清单、请求响应 schema、状态码矩阵、数据库建表/迁移细节或代码类型定义。
- 详细接口契约、数据库结构和实现方案应下沉到技术设计、接口说明和代码。
