# Shopify Pay 支付集成产品需求文档 (PRD)

**创建时间**: 2026-04-01
**最后更新**: 2026-04-02
**状态**: Partially Implemented - v2（购买后补归属方案）
**优先级**: P1

---
## 1. 相关用户故事

> **说明**: 详细的用户故事和验收标准请查看 `docs/user-stories/` 目录中的对应文件。

### 1.1 Shopify Pay 配置用户故事

- 📄 [docs/user-stories/08-shopify-pay-user-stories.md](/docs/user-stories/08-shopify-pay-user-stories.md)
  - **[US-PP-007] 配置 Shopify Payment Provider** (P0): 作为 Realm Admin，我想要配置 Shopify 作为支付平台，以便用户可以使用 Shopify 进行订阅支付
  - **[US-PP-008] 查看 Shopify Payment Provider 配置** (P0): 作为 Realm Admin，我想要查看 Shopify 配置和状态，以便管理支付集成
  - **[US-PP-009] 编辑 Shopify Payment Provider 配置** (P1): 作为 Realm Admin，我想要更新 Shopify 配置，以便进行密钥轮换和配置变更
  - **[US-PP-010] 删除 Shopify Payment Provider 配置** (P1): 作为 Realm Admin，我想要删除 Shopify 配置，以便移除不再使用的平台
  - **[US-PP-011] Shopify Subscription Contract 创建和同步** (P0): 作为 Herald 系统，我想要接收并处理 Shopify webhook 事件，以便创建和同步订阅记录
  - **[US-PP-012] Shopify Subscription 续费和状态同步** (P0): 作为 Herald 系统，我想要处理续费、升级、降级、取消和退款事件，以便保持订阅状态同步
  - **[US-PP-013] 用户认领 Shopify 订阅** (P0): 作为 Herald 用户，我想要认领我在 Shopify 购买的订阅，从而获得订阅积分
  - **[US-PP-014] Webhook 处理未归属订阅** (P0): 作为 Herald 系统，我想要正确处理未归属订阅的 webhook 事件，从而避免积分发放错误
  - **[US-PP-015] 通过 Customer Binding 自动归属** (P1): 作为 Herald 系统，我想要通过 Customer 绑定自动归属订阅，从而减少用户手动认领

### 1.2 用户故事优先级汇总

| 优先级 | 用户故事数量 | 关键故事 |
|--------|------------|---------|
| P0 | 6 | 配置 Shopify、查看配置、订阅创建和同步、续费和状态同步、用户认领订阅、Webhook 处理未归属订阅 |
| P1 | 3 | 编辑配置、删除配置、通过 Customer Binding 自动归属 |
| P2 | 0 | - |

---

## 2. 范围界定

### 2.1 包含功能

- ✅ Shopify 作为支付平台选项之一（与 Creem、Stripe 并列）
- ✅ Shopify 配置管理（Shop Domain、API Tokens、Webhook Secret）
- ✅ Webhook 事件处理（订阅合同生命周期）
- ✅ 订阅合同同步（create、update、billing attempts）
- ✅ 续费和积分发放处理
- ✅ 升级/降级/取消处理
- ✅ 退款事件处理
- ✅ 订阅状态映射和维护
- ✅ 与现有 Billing/Subscription/Points 系统集成

### 2.2 不包含功能 (Out of Scope)

- ❌ **Checkout 预创建流程**（原因：本方案采用 webhook 驱动，不在用户进入支付前创建本地订阅记录）
- ❌ **Shopify App Billing 集成**（原因：本方案将 Shopify 作为 Gateway + Recurring 提供商，不走 App Billing 路径）
- ❌ **多 Shopify Shop 绑定**（原因：一个 realm 只绑定一个 Shopify shop）
- ❌ **第二套订阅聚合系统**（原因：复用现有 Subscription 领域模型）
- ❌ **前端配置 UI 详细设计**（原因：v1 版本不要求，后续迭代添加）
- ❌ **第三方订阅应用兼容模式**（原因：不支持"其他应用已创建合同，Herald 只旁路监听"的场景）
- ❌ **基于积分账本反查用户的归属方式**（原因：points_credit_ledger 是派生数据，不是身份真源）
- ❌ **邮箱自动匹配归属**（原因：v1 不支持通过邮箱推断用户，避免误绑定风险）
- ❌ **历史事件全量重放**（原因：认领后只补发当前有效周期权益，不重放所有历史 webhook）

### 2.3 依赖项

- ✅ 通用支付平台配置系统（状态: 待实现，见 Billing PRD）
- ✅ Billing 订阅计费系统（状态: 部分实现，docs/prd/billing/billing.md）
- ✅ Points 积分系统（状态: 已实现，docs/prd/billing/points.md）
- ✅ Subscription History 订阅历史（状态: 部分实现，docs/prd/billing/subscription-history.md）
- ✅ Realm 管理系统（状态: 已实现）
- ✅ 用户管理系统（状态: 已实现）
- ❌ Shopify Shop 和 API 凭据（状态: 待配置）
- ❌ Shopify Subscription Contract 能力（状态: 待启用）

### 2.4 核心约束

- ✅ **真源原则**：subscription.user_id 是订阅归属的唯一真源，禁止从 points_credit_ledger 反查
- ✅ **两阶段归属**：支持"已归属订阅"（user_id NOT NULL）和"未归属订阅"（user_id NULL）
- ✅ **延迟执行**：未归属订阅不发放积分，等待用户认领后补偿

---

## 3. 需求概述

### 3.1 功能描述

Shopify Pay 集成是 Herald 系统支付平台选项之一，与 Creem（模拟平台）、Stripe 并列。Realm Admin 可以选择使用 Shopify 作为订阅支付的处理平台。

**关键特性**：
1. **订阅合同同步**: 通过 webhook 接收 Shopify Subscription Contract 事件，保持本地订阅状态同步
2. **两阶段归属**: 支持用户直接在 Shopify 购买（不带 user_id），后续登录 Herald 时认领订阅
3. **延迟积分发放**: 未归属订阅暂不发放积分，认领后补偿当前有效周期权益
4. **状态管理**: 支持升级、降级、取消、退款等订阅变更操作
5. **多租户支持**: 每个 Realm 绑定一个独立的 Shopify Shop
6. **事件驱动**: 所有订阅创建和更新由 Shopify webhook 驱动，无本地 checkout 预创建

### 3.2 与其他支付平台的对比

| 特性 | Creem | Stripe | Shopify |
|------|-------|--------|---------|
| 类型 | 模拟平台 | 真实支付 | 真实支付 |
| 环境 | Sandbox | Test + Live | Production |
| 适用场景 | 开发测试 | 生产环境 | 生产环境 |
| 订阅模型 | Herald Checkout | Stripe Checkout | Shopify Subscription Contract |
| Webhook | 模拟 | 真实 | 真实 |
| 配置模型 | API Keys | API Keys + Webhook Secret | Shop Domain + Multiple Tokens |
| 状态 | 已实现 | 待实现 | 待实现 |

### 3.3 业务价值

- **用户价值**: 利用 Shopify 成熟的电商订阅基础设施，提供稳定的周期性计费体验
- **业务价值**: 降低支付集成复杂度，复用现有 Subscription + Points 领域模型
- **技术价值**: 事件驱动架构，通过 webhook 实现最终一致性，减少状态同步复杂度

### 3.4 核心场景：先购买、后认领

**业务背景**：
用户可能直接进入 Shopify 购买入口，不携带 Herald user_id。此时 webhook 无法立即创建已归属订阅。

**解决方案**：
1. **创建阶段**：webhook 创建未归属订阅（subscription.user_id = NULL）
2. **认领阶段**：用户登录 Herald 后，通过 Shopify Customer ID 或 Contract ID 认领订阅
3. **补偿阶段**：认领成功后，补发当前有效周期的一次订阅积分

**关键约束**：
- 所有后续 webhook（续费、退款）必须先查 subscription.user_id
- 未归属订阅只同步状态，不执行积分动作
- 禁止从 points_credit_ledger 反查用户身份

---

## 4. 当前实现状态

| 功能模块 | 状态 | 备注 |
|---------|------|------|
| 通用支付平台配置 API | ❌ | 待实现（见 Billing PRD） |
| Shopify 配置管理 | ❌ | 待实现 |
| Shopify Webhook 端点 | ❌ | 待实现 |
| Webhook 事件处理 | ❌ | 待实现 |
| Subscription Contract 同步 | ❌ | 待实现 |
| 续费积分发放 | ❌ | 待实现（需集成现有 Points 系统） |
| 升级/降级处理 | ❌ | 待实现 |
| 退款处理 | ❌ | 待实现 |
| 与 Billing 系统集成 | ❌ | 待实现 |
| shopify_subscription_binding 表 | ❌ | 待实现 |

---

## 5. 功能需求

### 5.1 Shopify 配置管理

- 每个 Realm 可以配置一个 Shopify Shop（支持多租户）
- 配置项包括：
  - Shop Domain（形如 `demo-store.myshopify.com`）
  - Admin Access Token（Admin API 调用）
  - Storefront Access Token（Storefront API 调用）
  - App Client Secret（Webhook HMAC 验证）
  - API Version（固定版本号）
  - Webhook Subscription Mode（默认 `admin_api`）
  - Timeout（HTTP 超时秒数）
- 所有敏感信息（API Tokens）必须加密存储在数据库中
- 只有 Realm Admin 可以查看和更新 Shopify 配置
- 敏感信息查看时显示脱敏信息
- **编辑时密钥保留**：更新配置时，敏感字段（Admin Access Token、Storefront Access Token、App Client Secret）为可选；留空则保留现有值，不会覆盖为空。非敏感字段（Shop Domain、API Version 等）正常更新

### 5.2 Webhook 驱动的订阅创建流程

**关键约束**: 本方案不采用 checkout 预创建策略，只有在 Shopify webhook 确认支付成功后，Herald 才创建本地订阅记录。

**订阅创建流程**:
1. 用户在 Shopify 侧完成订阅购买
2. Shopify 发送 `subscription_contracts/create` webhook 到 Herald
3. Herald 验证 webhook HMAC 签名
4. Herald 检查幂等性（基于 `X-Shopify-Event-Id`）
5. Herald 从 webhook payload 中提取 Herald 标识符（realm_id、user_id、client_app_id、plan_id）
6. Herald 执行归属判断：
   - **情况 A**：payload 包含 `casUserId` → 创建已归属订阅，发放初始积分
   - **情况 B**：payload 无 `casUserId`，但存在 `shopify_user_binding` → 自动归属，发放初始积分
   - **情况 C**：payload 无 `casUserId` 且无 binding → 创建未归属订阅，不发放积分
7. Herald 创建本地 Subscription 记录和 Shopify 绑定记录

**关联前提**: Webhook payload 优选包含 Herald 标识符，但允许缺失（创建未归属订阅）。

### 5.3 续费和积分发放流程

**续费流程**:
1. Shopify 发送 `subscription_billing_attempts/success` webhook
2. Herald 验证 webhook 并检查幂等性
3. Herald 找到对应的本地 Subscription 记录
4. Herald 调用 `subscription_service.handle_subscription_paid(..., is_renewal = true, ...)`
5. Points 系统发放续费积分
6. Herald 更新 Subscription 当前周期边界

### 5.4 升级/降级处理流程

**升级/降级流程**:
1. Shopify 发送 `subscription_contracts/update` webhook
2. Herald 验证 webhook 并检查幂等性
3. Herald 比较 contract 的 `revision_id`，只有更高版本才能覆盖本地状态
4. 若 plan 发生变更：
   - 升级（新计划积分更多）：调用 `handle_subscription_upgrade`，发放差额积分
   - 降级（新计划积分更少）：仅更新 Subscription 和 history，不回收既有积分
5. Herald 更新 Subscription 和 Shopify 绑定记录

### 5.5 取消和退款处理

**取消流程**:
1. Shopify 发送 `subscription_contracts/update` webhook（标记周期末取消或已终止）
2. Herald 根据 contract 状态映射到 Herald SubscriptionStatus：
   - 周期末取消 → `ScheduledCancel`
   - 合同已终止 → `Canceled`
3. Herald 更新本地 Subscription 状态

**退款流程**:
1. Shopify 发送 `refunds/create` webhook
2. Herald 记录退款事件
3. Herald 关联 order / billing attempt / contract
4. Herald 进入现有退款积分回收路径

### 5.6 安全要求

- Webhook 端点必须验证 HMAC-SHA256 签名（使用 `X-Shopify-Hmac-SHA256` header）
- 所有 webhook 处理必须使用 raw body，不能先 JSON parse 再验签
- API Tokens 不得暴露给前端
- 所有支付操作必须通过 HTTPS
- 支付敏感信息不得存储在本地数据库
- 所有 webhook 事件必须记录审计日志

### 5.7 错误处理

- Webhook 验证失败时返回 401
- Webhook 处理成功后尽快返回 202 Accepted，异步处理业务事件
- Webhook 失败时 Shopify 会重试（最多 8 次），持续失败可能导致订阅被移除
- 记录所有 webhook 事件用于分析和调试
- 支持 Admin API 补偿查询，处理 payload 缺失 Herald 标识符的情况

### 5.8 数据隔离

- 不同 Realm 的支付数据完全隔离（通过 realm 路径）
- Webhook 端点按 Realm 隔离：`/api/third/pay/{realmId}/shopify/webhooks`
- 一个 Realm 绑定一个 Shopify Shop

### 5.9 订阅认领流程

**认领触发时机**：
- 用户购买后首次登录 Herald
- 用户在"我的订阅"页面点击"同步 Shopify 订阅"
- OAuth 回调已获得 Shopify Customer 信息

**认领输入**：
- 当前登录 Herald 用户 user_id
- realm_id
- shopify_customer_id（推荐）或 contract_id / order_id

**认领流程**：
1. 校验当前用户身份
2. 查询或创建 shopify_user_binding
3. 查找该 customer 对应的所有未归属订阅
4. 将这些订阅的 subscription.user_id 更新为当前用户
5. 对符合条件的订阅补发当前有效周期权益
6. 记录认领审计日志

**补偿规则**：
- 仅补发当前有效周期的一次订阅积分
- 不重放历史 billing_attempts/success 事件
- 使用幂等键防止重复发放：shopify_claim_grant:{subscription_id}:{period_end}

**安全约束**：
- 同一 Shopify Customer 不能被两个 Herald 用户认领（唯一约束）
- 认领时发现 Customer 已绑定其他用户 → 返回冲突错误

### 5.10 订阅归属关系设计

**正式关系链路**：
所有 Shopify webhook 的用户查找必须统一走以下路径：

**路径 A：合同/订单先查订阅，再查用户**
```
contract_id / order_id
-> shopify_subscription_binding.subscription_id
-> subscription.user_id
```

**路径 B：Shopify Customer 先查绑定，再查用户**
```
shopify_customer_id
-> shopify_user_binding.user_id
```

**禁止的旁路**：
```
subscription_id -> points_credit_ledger -> user_id  ❌
```
原因：ledger 是业务派生数据，不是身份关系真源。

**Webhook 处理分支规则**：
| Webhook 事件 | 已归属订阅 | 未归属订阅 |
|------------|-----------|-----------|
| subscription_contracts/create | 创建订阅 + 发放积分 | 创建订阅 + 不发放积分 |
| subscription_billing_attempts/success | 发放续费积分 | 更新周期边界 + 不发积分 |
| refunds/create | 回收积分 | 仅记录事件 |
| subscription_contracts/update | 执行升级/降级/取消逻辑 | 仅同步订阅状态 |

---

## 6. API 相关约束

**状态**: 必填

- 仅说明 Shopify webhook 处理的能力边界，不在 PRD 中列出端点、schema 或状态码细节。
- 必须遵守 realm 隔离、webhook 验证、幂等处理和事件顺序约束。
- Webhook 事件处理必须支持以下主题：
  - `subscription_contracts/create`: 创建本地订阅
  - `subscription_contracts/update`: 更新订阅状态、处理升级/降级/取消
  - `subscription_billing_attempts/success`: 续费成功，发放积分
  - `subscription_billing_attempts/failure`: 续费失败，标记 PastDue
  - `orders/paid`: 初次支付补偿（非主真相）
  - `refunds/create`: 退款处理
  - `app/uninstalled`: 停用 Shopify 配置
- 与 Shopify Admin API、Storefront API 的详细契约应下沉到技术设计、接口说明或实现代码。

---

## 7. 前端/交互约束

**状态**: 必填

- v1 版本不要求详细的前端配置 UI 设计，仅需支持基本的配置管理功能。
- 仅保留管理入口、关键操作路径、状态反馈，不写组件实现、数据层封装或代码结构。
- Shopify 配置页面应包含：
  - 配置列表展示（Shop Domain、Environment、Last Updated、Actions）
  - 配置创建表单（Shop Domain、Admin Access Token、Storefront Access Token、App Client Secret、API Version）
  - 配置编辑功能（支持密钥轮换；敏感字段留空保留现有值）
  - 配置删除功能（无活跃订阅时可删除）
  - 连接测试功能（测试 Admin API 和 Storefront API）
- 必须突出敏感信息的脱敏显示和加密存储说明。
- 删除配置前必须提示活跃订阅数量和影响范围。

---

## 8. 技术设计承接

**状态**: 必填

- 接口细节、数据库结构、迁移策略、类型定义、SDK 设计和实现步骤，应在 `.ai/future/shopify_pay_codex.md`、技术设计文档或代码中承接。

### 8.1 数据模型扩展

**subscription 表调整**：
- 新增字段：user_id UUID（可为 NULL，用于支持"先购买、后认领"）
- 新增索引：idx_subscription_user_id, idx_subscription_realm_user_id

**shopify_user_binding 表（新增）**：
- 职责：保存 Shopify Customer 与 Herald User 的正式绑定关系
- 关键字段：
  - realm_id, shop_domain, shopify_customer_id（唯一约束）
  - user_id（绑定到的 Herald 用户）
  - status（active/inactive）
- 用途：支持 webhook 缺失 casUserId 时通过 customer_id 查到用户

**shopify_subscription_binding 表（保持单一职责）**：
- 职责：仅负责 contract_id/order_id/billing_attempt_id → subscription_id 的映射
- 不新增用户字段，不承担用户归属职责

**points_credit_ledger 表（职责明确）**：
- 仅用于记录积分授予、消耗、回收
- 不承担身份归属查询
- 删除 subscription_id 字段（如果存在）

**通用模型扩展**：
- 新增 `ConfigType::Shopify` 到 `realm_config` 表
- 复用现有 `plan`、`subscription`、`payment_event`、`subscription_history` 表
- Webhook 处理逻辑详见技术方案第 10 节
- 状态映射规则详见技术方案第 11 节
- 补偿与一致性处理详见技术方案第 12 节

---

## 9. 相关文件索引

### 9.1 后端文件

**领域层**:
- `backend/core/src/domain/billing/entities.rs` - 支付实体定义（复用现有 Plan、Subscription、PaymentEvent）
- `backend/core/src/domain/realm_config/entities.rs` - 新增 `ConfigType::Shopify`
- `backend/core/src/domain/points/subscription_service.rs` - 积分发放服务（复用现有）

**基础设施层**:
- `backend/core/src/infrastructure/shopify/mod.rs` - Shopify 模块导出（新增）
- `backend/core/src/infrastructure/shopify/client.rs` - Shopify Admin/Storefront API 客户端（新增）
- `backend/core/src/infrastructure/shopify/models.rs` - Shopify 数据模型（新增）

**应用层**:
- `backend/api/src/application/http/billing/shopify_webhook_handlers.rs` - Shopify Webhook 处理器（新增）
- `backend/api/src/application/http/billing/routes.rs` - Webhook 路由定义（修改）
- `backend/api/src/application/http/billing/types.rs` - 支付平台类型（新增 `shopify`）

**数据库迁移**:
- `backend/app/migrations/20260401_add_shopify_support.sql` - Shopify 基础支持
- `backend/app/migrations/20260402_add_subscription_id_to_credit_ledger.sql` - 积分账本调整
- `backend/app/migrations/20260403_add_shopify_user_binding.sql` - 用户绑定表（新增）

### 9.2 前端文件

**页面组件**:
- `frontend/src/routes/$realmId/billing/payment-providers.tsx` - 支付平台配置管理页面（修改）

**业务组件**:
- `frontend/src/components/billing/PaymentProviderForm.tsx` - 支付平台配置表单（扩展 Shopify 支持）
- `frontend/src/lib/billing-constants.ts` - 支付平台常量（新增 Shopify）
- `frontend/src/lib/schemas/billing-forms.ts` - 支付表单 schema（新增 Shopify）

### 9.3 测试文件

**后端场景测试**:
- `backend/tests/scenarios/billing/shopify_provider.rs` - Shopify provider 场景测试（新增）
- `backend/tests/scenarios/billing/shopify_webhooks.rs` - Shopify webhook 场景测试（新增）

**E2E Demo 测试**:
- `demo/e2e/billing/shopify-payment-flow.spec.ts` - Shopify 支付流程 E2E 测试（新增，可选）

### 9.4 技术方案文档

- `.ai/future/shopify_pay_codex.md` - Shopify 支付接入技术方案（参考）

---

## 10. 参考资料

### 10.1 Shopify 官方文档
- [Shopify Webhooks Guide](https://shopify.dev/docs/apps/build/webhooks)
- [Shopify Webhook Subscription Management](https://shopify.dev/docs/apps/build/webhooks/subscribe)
- [Shopify HTTPS Webhook Verification](https://shopify.dev/docs/apps/build/webhooks/subscribe/https)
- [Shopify Webhook Topics Reference](https://shopify.dev/docs/api/webhooks/latest)
- [Shopify Admin API Reference](https://shopify.dev/docs/api/admin-graphql)
- [Shopify Storefront API Reference](https://shopify.dev/docs/api/storefront)

### 10.2 相关用户故事
- 📄 [docs/user-stories/08-shopify-pay-user-stories.md](/docs/user-stories/08-shopify-pay-user-stories.md) - Shopify Pay 用户故事
- 📄 [docs/user-stories/07-payment-provider-user-stories.md](/docs/user-stories/07-payment-provider-user-stories.md) - 通用支付平台配置用户故事

### 10.3 相关 PRD
- [Billing 订阅计费 PRD](/docs/prd/billing/billing.md) - 现有订阅计费系统
- [Points 积分系统 PRD](/docs/prd/billing/points.md) - 积分发放和回收逻辑
- [Subscription History PRD](/docs/prd/billing/subscription-history.md) - 订阅变更历史
- [Stripe Payment PRD](/docs/prd/billing/stripe-payment.md) - Stripe 支付集成参考

### 10.4 技术资源
- [Shopify Rust SDK](https://github.com/sinha-sahil/shopify-rust-client)
