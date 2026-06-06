# Stripe 支付集成产品需求文档 (PRD)

**创建时间**: 2026-03-20
**优先级**: P1

---

## 1. 相关用户故事

> 详细故事与验收标准请查看 `docs/user-stories/` 中对应文档。

### 1.1 故事引用

**支付平台配置用户故事**
- `[US-PP-001]` 配置支付平台，优先级 P0，来源 `docs/user-stories/billing/payment-provider.md`
- `[US-PP-002]` 查看支付平台配置，优先级 P0，来源 `docs/user-stories/billing/payment-provider.md`
- 角色：Realm Admin
- 摘要：管理员配置和查看支付平台（包括 Stripe）

**Stripe 支付用户故事**
- 配置 Stripe Webhook 端点（在 US-PP-001 场景 2 中涵盖）
- 使用 Stripe 支付订阅（通过 Stripe Checkout 实现，不在 Herald 前端）
- 管理支付方式（通过 Stripe Customer Portal 实现，不在 Herald 前端）

**重要说明**：Stripe 支付的用户体验在第三方应用中完成，Herald 系统只负责配置管理和 Webhook 处理。最终用户通过 Stripe Checkout 和 Stripe Customer Portal 与 Stripe 交互。

### 1.2 优先级汇总表

| 优先级 | 数量 | 关键故事 |
|--------|------|----------|
| P0 | 2 | 配置支付平台、查看支付平台配置 |
| P1 | 0 | - |
| P2 | 0 | - |

---

## 2. 范围界定

### 2.1 包含功能

- Stripe 作为支付平台选项之一（与 Creem 并列）
- Stripe 配置管理——通过通用 `realm_config` API（`/api/realms/{realmId}/config`，ConfigType::Stripe）统一管理，支持 api_key、webhook_secret、publishable_key、timeout、webhook_endpoint_id 配置项
- 订阅支付处理（周期性计费）
- 一次性支付处理（Payment Intents）
- Webhook 事件处理（支付状态同步）
- 退款处理——`charge.refunded` 事件处理，支持 topup（按比例回收积分）和 subscription（回收未使用积分）两种退款类型的积分回收
- Demo/Mock 模式——支持 `mock://stripe` 作为 base URL，用于测试环境模拟 Stripe 响应
- 支付历史记录查询

### 2.2 不包含功能

- 批量导入配置
- 平台健康检查（仅支持 Webhook 连接测试）
- 其他支付网关的详细实现（Creem 是模拟平台，其他平台需单独 PRD）
- Disputes 处理
- 多币种转换（使用 Stripe 原生币种支持）
- 税务计算（使用 Stripe Tax 或后续集成）
- `payment_intent.payment_failed` 和 `invoice.payment_failed` 事件处理（✅ RESOLVED — 已在 `stripe_webhook_handlers.rs` 中实现）

### 2.3 依赖项

- 通用支付平台配置系统（见 Billing PRD）
- Billing 订阅计费系统（`docs/prd/billing/subscription.md`）
- Realm 管理系统
- 用户管理系统
- Stripe 账户和 API 密钥（需配置）

---

## 3. 需求概述

### 3.1 功能描述

Stripe 支付集成是 Herald 系统支付平台选项之一，与 Creem（模拟平台）并列。Realm Admin 可以选择使用 Stripe 作为订阅和一次性支付的处理平台。

### 3.2 关键特性

- **订阅支付**：处理周期性订阅计费，与现有 Billing 系统集成
- **一次性支付**：处理一次性购买和充值场景
- **Webhook 同步**：实时接收 Stripe 事件，保持支付状态同步
- **多租户支持**：每个 Realm 可以配置独立的 Stripe 账户

---

## 4. 业务规则与状态

### 4.1 业务规则

- **配置管理规则**：每个 Realm 可配置独立的 Stripe 账户；通过通用 `realm_config` API（`/api/realms/{realmId}/config`，ConfigType::Stripe）管理，配置项包括 api_key（Secret Key）、webhook_secret（Webhook Signing Secret）、publishable_key（Publishable Key）、timeout（HTTP 请求超时秒数）、webhook_endpoint_id（Webhook 端点 ID，用于验证）
  - **配置项差异说明**：Account ID 未作为独立 config_key 实现；Environment（test/live）由 API Key 前缀自动决定（`sk_test_*` / `sk_live_*`）；Webhook Endpoint URL 由 `public_base_url` 动态拼接，不作为独立配置项
- **密钥加密存储**：API Key 必须加密存储在数据库中
- **密钥脱敏**：Secret Key 查看时显示脱敏信息
- **编辑时密钥保留**：更新配置时，敏感字段（Secret Key、Webhook Secret）为可选，留空则保留现有值；非敏感字段正常更新（✅ RESOLVED — `is_empty_stripe_secret` in `realm_config/mod.rs` L32-36 detects empty secrets and preserves existing values）
- **权限控制**：只有 Realm Admin 可以查看和更新 Stripe 配置
- **删除保护**：无活跃订阅时才可删除配置（✅ RESOLVED — `ensure_stripe_config_deletable` in `realm_config/mod.rs` L38-61 checks active subscriptions before allowing deletion）
- **数据隔离**：不同 Realm 的支付数据完全隔离；用户只能查看自己的支付历史；Realm Admin 只能查看所属 Realm 的支付数据

### 4.2 关键状态与异常

- **支付失败处理**：返回用户友好的错误信息，支持支付重试（针对临时性错误），记录所有支付失败事件
- **Webhook 重试**：代码未实现自动重试机制，依赖 Stripe 自身重试发送策略（已知设计决策）
- **安全约束**：API Key 不得暴露给前端（仅 Publishable Key 可暴露）；Webhook 端点必须验证 Stripe Signature；所有支付操作必须通过 HTTPS；支付敏感信息不得存储在本地数据库；所有支付操作必须记录审计日志
- **Webhook 签名验证**：使用 HMAC-SHA256 验证，签名格式为 `stripe-signature` 头中的 `t=...,v1=...`；包含时间戳重放攻击防护（15 分钟窗口，即 900 秒），拒绝过旧或未来时间戳的请求

---

## 5. 功能需求

### 5.1 核心需求

- **Stripe 配置管理**：每个 Realm 通过通用 `realm_config` API（`/api/realms/{realmId}/config`，ConfigType::Stripe）配置独立 Stripe 账户，支持创建、查看（脱敏）、更新、删除配置
- **一次性支付处理**：创建 Payment Intent → 获取 Client Secret → 确认支付 → 处理支付结果
- **订阅支付处理**：创建 Stripe Subscription → 处理首次支付 → 处理续费事件 → 取消订阅
- **Webhook 事件处理**：验证 Stripe Signature（HMAC-SHA256 + 时间戳重放防护）→ 解析事件类型 → 执行业务逻辑 → 更新本地状态 → 记录事件日志；已实现事件：checkout.session.completed、customer.subscription.created/updated/deleted、charge.refunded、payment_intent.payment_failed、invoice.payment_failed（✅ RESOLVED — `handle_payment_failed` in `stripe_webhook_handlers.rs` L735-763）
- **支付历史查询**：用户查看自己的支付历史，Realm Admin 查看 Realm 所有支付记录，支持按状态/时间/金额筛选和分页

### 5.2 验收目标

- Realm Admin 可以创建、查看、更新、删除 Stripe 配置
- 一次性支付和订阅支付流程正常工作
- Webhook 事件正确处理并更新本地状态
- 支付历史可以正确查询和筛选
- 不同 Realm 的数据完全隔离
- 所有支付操作记录审计日志

---

## 6. API 相关约束

**适用性**: 适用

- **接口能力范围**：计费、套餐、积分、支付配置、订阅变更、webhook 处理的能力边界；不在 PRD 中列出端点、schema 或状态码细节
- **访问控制原则**：必须遵守 realm 隔离、管理员权限、金额与积分变更可追溯、回调幂等和失败补偿要求
- **兼容性要求**：与支付平台、积分账本、订阅系统的详细契约应下沉到技术设计或接口说明

---

## 7. 前端/交互约束

**适用性**: 适用

- **管理入口**：支付平台配置管理页面，Realm Admin 可管理 Stripe 配置
- **关键操作路径**：配置创建表单、配置编辑（密钥轮换）、配置删除、支付历史查看
- **状态反馈**：敏感信息脱敏显示、配置状态展示、操作成功/失败反馈
- **权限可见性**：仅 Realm Admin 可访问配置管理页面
- **金额/积分变化**：支付场景必须突出金额变化、变更影响范围、不可逆风险提示和回调同步中的状态说明

---

## 8. 已确认决策

### 8.1 已确认决策

- Stripe 支付用户体验在第三方应用中完成，Herald 只负责配置管理和 Webhook 处理
- Stripe 与 Creem 作为支付平台选项并列存在
- 复用通用支付平台配置系统
- Webhook 处理失败不自动重试，依赖 Stripe 自身重试发送策略
- Demo/Mock 模式使用 `mock://stripe` 作为 base URL，客户端检测到后跳过真实 HTTP 调用并返回模拟响应

### 8.2 与实现已知差异

| 项 | 状态 | 说明 |
|----|------|------|
| 密钥保留逻辑 | ✅ RESOLVED | `is_empty_stripe_secret` in `realm_config/mod.rs` L32-36 detects empty secrets and preserves existing values |
| 删除保护 | ✅ RESOLVED | `ensure_stripe_config_deletable` in `realm_config/mod.rs` L38-61 checks active subscriptions before allowing deletion |
| payment_intent.payment_failed 事件 | ✅ RESOLVED | `handle_payment_failed` in `stripe_webhook_handlers.rs` L735-763 |
| invoice.payment_failed 事件 | ✅ RESOLVED | Handled by same `handle_payment_failed` in `stripe_webhook_handlers.rs` L735-763 |
| Account ID 配置项 | 未实现 | Account ID 未作为独立 config_key 实现，如需要可通过 metadata 扩展 |
| Environment 配置项 | 不需要 | test/live 环境由 API Key 前缀（`sk_test_*` / `sk_live_*`）自动决定，无需独立配置 |
| Webhook URL 配置项 | 不需要 | 由 `public_base_url` 动态拼接，不作为独立配置项 |

---

## 9. 参考资料

- 用户故事：`docs/user-stories/billing/payment-provider.md`
- 相关 PRD：`docs/prd/billing/subscription.md`
- Stripe 官方文档：[Stripe API](https://stripe.com/docs/api)、[Webhooks 指南](https://stripe.com/docs/webhooks)、[Stripe.js](https://stripe.com/docs/js)
