# Billing 计费模块 PRD 索引

本目录包含 Herald 系统计费与订阅相关的全部 PRD。各文档之间存在功能依赖，阅读和开发时应注意先后顺序。

## 文档依赖关系

```
billing.md (订阅计费基础)
├── product-catalog.md (Product 编目)
├── points.md (积分系统)
│   └── points-free-user.md (免费用户积分)
├── subscription-history.md (订阅变更历史)
├── unified-purchase.md (统一购买架构)
│   ├── payment_attempt (内部概念，无独立 PRD)
│   ├── stripe-payment.md
│   ├── shopify-pay.md
│   └── wechat-pay.md
└── invoice.md (发票管理)
```

## 文档列表

| PRD | 标题 | 状态 | 核心功能 |
|-----|------|------|---------|
| [billing.md](billing.md) | 订阅计费基础 | 🚧 部分实现 | Plan CRUD、套餐分配、Webhook、订阅状态机 |
| [product-catalog.md](product-catalog.md) | Product 编目 | ✅ 已实现 | Product CRUD、Product-Plan 层级、排序 |
| [points.md](points.md) | 积分系统 | ✅ 已实现 | 积分账户、充值、消费、套餐配置 |
| [points-free-user.md](points-free-user.md) | 免费用户积分 | ✅ 已实现 | 注册初始积分、定期免费积分、升级保留 |
| [subscription-history.md](subscription-history.md) | 订阅变更历史 | ✅ 已实现 | 变更时间线查询、状态对比 |
| [unified-purchase.md](unified-purchase.md) | 统一购买架构 | 🚧 部分实现 | PaymentAttempt、积分包购买、跨平台支付流程 |
| [stripe-payment.md](stripe-payment.md) | Stripe 支付 | 🚧 部分实现 | Stripe 配置、Webhook 接收 |
| [shopify-pay.md](shopify-pay.md) | Shopify 支付 | 🚧 部分实现 | Shopify 配置、订阅合同同步、认领流程 |
| [wechat-pay.md](wechat-pay.md) | 微信支付 | 🚧 部分实现 | 微信配置、Native 下单、Webhook 回调 |
| [invoice.md](invoice.md) | 发票管理 | ✅ 已实现 | 发票 CRUD、状态机、PDF 生成、用户申请 |

## 阅读顺序建议

1. **billing.md** — 理解订阅计费基础模型和 Plan 管理
2. **product-catalog.md** — 理解 Product-Plan 编目层级
3. **points.md** → **points-free-user.md** — 理解积分系统
4. **unified-purchase.md** — 理解统一购买和支付尝试流程
5. 支付平台集成（stripe / shopify / wechat）— 按需阅读
6. **invoice.md** — 发票管理
