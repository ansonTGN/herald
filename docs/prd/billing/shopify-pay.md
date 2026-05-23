# Shopify Pay 支付集成产品需求文档 (PRD)

**创建时间**: 2026-04-01
**优先级**: P1

---

## 1. 相关用户故事

> 详细故事与验收标准请查看 `docs/user-stories/` 中对应文档。

### 1.1 故事引用

- `[US-PP-007]` 配置 Shopify Payment Provider，优先级 P0，来源 `docs/user-stories/billing/shopify-pay.md`
- `[US-PP-008]` 查看 Shopify Payment Provider 配置，优先级 P0，来源 `docs/user-stories/billing/shopify-pay.md`
- `[US-PP-009]` 编辑 Shopify Payment Provider 配置，优先级 P1，来源 `docs/user-stories/billing/shopify-pay.md`
- `[US-PP-010]` 删除 Shopify Payment Provider 配置，优先级 P1，来源 `docs/user-stories/billing/shopify-pay.md`
- `[US-PP-011]` Shopify Subscription Contract 创建和同步，优先级 P0，来源 `docs/user-stories/billing/shopify-pay.md`
- `[US-PP-012]` Shopify Subscription 续费和状态同步，优先级 P0，来源 `docs/user-stories/billing/shopify-pay.md`
- `[US-PP-013]` 用户认领 Shopify 订阅，优先级 P0，来源 `docs/user-stories/billing/shopify-pay.md`
- `[US-PP-014]` Webhook 处理未归属订阅，优先级 P0，来源 `docs/user-stories/billing/shopify-pay.md`
- `[US-PP-015]` 通过 Customer Binding 自动归属，优先级 P1，来源 `docs/user-stories/billing/shopify-pay.md`

### 1.2 优先级汇总表

| 优先级 | 数量 | 关键故事 |
|--------|------|----------|
| P0 | 6 | 配置 Shopify、查看配置、订阅创建和同步、续费和状态同步、用户认领订阅、Webhook 处理未归属订阅 |
| P1 | 3 | 编辑配置、删除配置、通过 Customer Binding 自动归属 |
| P2 | 0 | - |

---

## 2. 范围界定

### 2.1 包含功能

- Shopify 作为支付平台选项之一（与 Creem、Stripe 并列）
- Shopify 配置管理（Shop Domain、API Tokens、Webhook Secret）
- Webhook 事件处理（订阅合同生命周期）
- 订阅合同同步（create、update、billing attempts）
- 续费和积分发放处理
- 升级/降级/取消处理
- 退款事件处理
- 订阅状态映射和维护
- 与现有 Billing/Subscription/Points 系统集成

### 2.2 不包含功能

- Checkout 预创建流程（本方案采用 webhook 驱动，不在用户进入支付前创建本地订阅记录）
- Shopify App Billing 集成（本方案将 Shopify 作为 Gateway + Recurring 提供商）
- 多 Shopify Shop 绑定（一个 Realm 只绑定一个 Shopify Shop）
- 第二套订阅聚合系统（复用现有 Subscription 领域模型）
- 第三方订阅应用兼容模式
- 基于积分账本反查用户的归属方式（points_credit_ledger 是派生数据，不是身份真源）
- 邮箱自动匹配归属（v1 不支持通过邮箱推断用户）
- 历史事件全量重放（认领后只补发当前有效周期权益）

### 2.3 依赖项

- 通用支付平台配置系统（见 Billing PRD）
- Billing 订阅计费系统（`docs/prd/billing/subscription.md`）
- Points 积分系统（`docs/prd/billing/points.md`）
- Subscription History 订阅历史（`docs/prd/billing/subscription.md`）
- Realm 管理系统
- 用户管理系统
- Shopify Shop 和 API 凭据（需配置）
- Shopify Subscription Contract 能力（需启用）

---

## 3. 需求概述

### 3.1 功能描述

Shopify Pay 集成是 Herald 系统支付平台选项之一，与 Creem（模拟平台）、Stripe 并列。Realm Admin 可以选择使用 Shopify 作为订阅支付的处理平台。核心采用 webhook-driven 模式，用户在 Shopify 侧完成购买后通过 webhook 同步订阅状态到 Herald。

### 3.2 关键特性

- **订阅合同同步**：通过 webhook 接收 Shopify Subscription Contract 事件，保持本地订阅状态同步
- **两阶段归属**：支持用户直接在 Shopify 购买（不带 user_id），后续登录 Herald 时认领订阅
- **延迟积分发放**：未归属订阅暂不发放积分，认领后补偿当前有效周期权益
- **状态管理**：支持升级、降级、取消、退款等订阅变更操作
- **多租户支持**：每个 Realm 绑定一个独立的 Shopify Shop
- **事件驱动**：所有订阅创建和更新由 Shopify webhook 驱动，无本地 checkout 预创建

---

## 4. 业务规则与状态

### 4.1 业务规则

- **配置管理规则**：每个 Realm 可配置一个 Shopify Shop；配置项包括 Shop Domain、Admin Access Token、Storefront Access Token、App Client Secret、API Version、Webhook Subscription Mode、Timeout；敏感信息加密存储，查看时脱敏；编辑时敏感字段留空则保留现有值
- **权限控制**：只有 Realm Admin 可以查看和更新 Shopify 配置；删除配置前需确认无活跃订阅
- **真源原则**：subscription.user_id 是订阅归属的唯一真源，禁止从 points_credit_ledger 反查
- **两阶段归属**：支持"已归属订阅"（user_id NOT NULL）和"未归属订阅"（user_id NULL）
- **延迟执行**：未归属订阅不发放积分，等待用户认领后补偿
- **归属关系链路**：
  - 路径 A：contract_id / order_id → shopify_subscription_binding.subscription_id → subscription.user_id
  - 路径 B：shopify_customer_id → shopify_user_binding.user_id
  - 禁止路径：subscription_id → points_credit_ledger → user_id（ledger 是派生数据）
- **修订版本控制**：升级/降级时比较 contract 的 revision_id，只有更高版本才能覆盖本地状态
- **升级降级规则**：升级（新计划积分更多）发放差额积分；降级（新计划积分更少）仅更新 Subscription 和 history，不回收既有积分
- **数据隔离**：不同 Realm 的支付数据完全隔离；一个 Realm 绑定一个 Shopify Shop

### 4.2 关键状态与异常

- **Webhook 处理安全**：Webhook 端点必须验证 HMAC-SHA256 签名（使用 X-Shopify-Hmac-SHA256 header）；使用 raw body 验签，不能先 JSON parse 再验签；API Tokens 不得暴露给前端；所有操作必须通过 HTTPS；所有 webhook 事件必须记录审计日志
- **幂等处理**：基于 X-Shopify-Event-Id 做幂等判断
- **错误处理**：Webhook 验证失败返回 401；处理成功后尽快返回 202 Accepted，异步处理业务事件；Shopify 会重试失败的 webhook（最多 8 次），持续失败可能导致订阅被移除
- **认领冲突**：同一 Shopify Customer 不能被两个 Herald 用户认领（唯一约束）
- **补偿规则**：认领后仅补发当前有效周期的一次订阅积分，不重放历史 billing_attempts/success 事件；使用幂等键防止重复发放

---

## 5. 功能需求

### 5.1 核心需求

- **Shopify 配置管理**：支持创建、查看（脱敏）、更新、删除配置；支持连接测试（Admin API 和 Storefront API）
- **Webhook 驱动订阅创建**：接收 subscription_contracts/create webhook → 验证 HMAC 签名 → 幂等检查 → 提取 Herald 标识符 → 归属判断（含 casUserId / 存在 binding / 无标识符三种情况）→ 创建本地 Subscription 和绑定记录
- **续费和积分发放**：接收 subscription_billing_attempts/success webhook → 验证签名 → 查找 Subscription → 调用积分发放 → 更新周期边界
- **升级/降级处理**：接收 subscription_contracts/update webhook → revision_id 比对 → 执行升级差额积分发放或降级状态更新
- **取消处理**：接收 subscription_contracts/update webhook → 映射 Shopify 状态到 Herald 状态（ScheduledCancel / Canceled）
- **退款处理**：接收 refunds/create webhook → 记录退款事件 → 进入现有退款积分回收路径
- **订阅认领**：用户登录后通过 Shopify Customer ID 或 Contract ID 认领未归属订阅 → 更新 user_id → 补发当前有效周期权益 → 记录审计日志
- **Webhook 事件处理分支**：已归属订阅正常发放积分；未归属订阅只同步状态不发放积分

### 5.2 验收目标

- Realm Admin 可以配置和管理 Shopify 支付平台
- Webhook 事件正确处理订阅创建、续费、升级/降级、取消、退款
- 未归属订阅正确创建，用户可以认领并获取权益补偿
- 认领冲突正确检测和提示
- 不同 Realm 的数据完全隔离
- 所有 webhook 事件记录审计日志

---

## 6. API 相关约束

**适用性**: 适用

- **接口能力范围**：Shopify webhook 处理的能力边界；不在 PRD 中列出端点、schema 或状态码细节
- **访问控制原则**：必须遵守 realm 隔离、webhook 验证、幂等处理和事件顺序约束
- **Webhook 事件主题**：subscription_contracts/create、subscription_contracts/update、subscription_billing_attempts/success、subscription_billing_attempts/failure、orders/paid、refunds/create、app/uninstalled
- **兼容性要求**：与 Shopify Admin API、Storefront API 的详细契约应下沉到技术设计或接口说明

---

## 7. 前端/交互约束

**适用性**: 适用

- **管理入口**：支付平台配置管理页面，包含 Shopify 配置列表、创建表单、编辑（密钥轮换）、删除、连接测试
- **关键操作路径**：Shopify 配置创建（Shop Domain、Admin Access Token、Storefront Access Token、App Client Secret、API Version）；编辑时敏感字段留空保留现有值
- **状态反馈**：敏感信息脱敏显示和加密存储说明；配置状态展示；删除前提示活跃订阅数量和影响范围
- **权限可见性**：仅 Realm Admin 可访问配置管理页面

---

## 8. 已确认决策

### 8.1 已确认决策

- 采用 webhook-driven 模式，不使用 checkout 预创建策略
- 两阶段归属模型：支持未归属订阅和用户认领
- 复用现有 Subscription + Points 领域模型
- 认领后仅补发当前有效周期权益，不重放历史
- 一个 Realm 绑定一个 Shopify Shop

---

## 9. 参考资料

- 用户故事：`docs/user-stories/billing/shopify-pay.md`、`docs/user-stories/billing/payment-provider.md`
- 相关 PRD：`docs/prd/billing/subscription.md`、`docs/prd/billing/points.md`、`docs/prd/billing/stripe-payment.md`
- Shopify 官方文档：[Webhooks Guide](https://shopify.dev/docs/apps/build/webhooks)、[Admin API](https://shopify.dev/docs/api/admin-graphql)、[Storefront API](https://shopify.dev/docs/api/storefront)
