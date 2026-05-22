# Stripe 支付集成产品需求文档 (PRD)

**创建时间**: 2026-03-20
**状态**: Partially Implemented
**优先级**: P1

---
## 1. 相关用户故事

> **说明**: 详细的用户故事和验收标准请查看 `docs/user-stories/` 目录中的对应文件。

### 1.1 支付平台配置用户故事

- 📄 [docs/user-stories/billing/payment-provider.md](/docs/user-stories/billing/payment-provider.md)
  - **[US-PP-001] 配置支付平台** (P0): 作为 Realm Admin，我想要配置支付平台（Creem/Stripe），以便用户可以使用该平台进行支付
  - **[US-PP-002] 查看支付平台配置** (P0): 作为 Realm Admin，我想要查看支付平台配置和状态，以便管理支付集成

### 1.2 Stripe 支付用户故事

**说明**：Stripe 支付的用户故事已整合到支付平台配置用户故事中（US-PP-001 ~ US-PP-003），涵盖以下功能：
- ✅ **配置 Stripe Webhook 端点**：作为 Realm Admin，我想要配置 Stripe Webhook 端点，以便接收支付事件通知（已在 US-PP-001 场景 2 中涵盖）
- ✅ **使用 Stripe 支付订阅**：作为第三方应用用户，我想要使用 Stripe 支付订阅费用（通过 Stripe Checkout 实现，不在 Herald 前端）
- ✅ **管理支付方式**：作为第三方应用用户，我想要管理我的 Stripe 支付方式（通过 Stripe Customer Portal 实现，不在 Herald 前端）

**重要说明**：
- Stripe 支付的用户体验在第三方应用中完成，Herald 系统只负责配置管理和 Webhook 处理
- 最终用户通过 Stripe Checkout 和 Stripe Customer Portal 与 Stripe 交互，不通过 Herald 前端

### 1.3 用户故事优先级汇总

| 优先级 | 用户故事数量 | 关键故事 |
|--------|------------|---------|
| P0 | 2 | 配置支付平台、查看支付平台配置 |
| P1 | 0 | - |
| P2 | 0 | - |

**注意**：所有 P0/P1 用户故事已在 `docs/user-stories/billing/payment-provider.md` 中定义，无需额外创建 Stripe 专用用户故事。

---

## 2. 范围界定

### 2.1 包含功能

- ✅ Stripe 作为支付平台选项之一（与 Creem 并列）
- ✅ Stripe 配置管理（API Key、Webhook Secret 等）
- ✅ 订阅支付处理（周期性计费）
- ✅ 一次性支付处理（Payment Intents）
- ✅ Webhook 事件处理（支付状态同步）
- ✅ 支付历史记录查询

### 2.2 不包含功能 (Out of Scope)

- ❌ **批量导入配置**（原因：手动配置单个支付平台，初期不支持批量操作）
- ❌ **平台健康检查**（原因：初期版本不支持平台状态监控，仅支持 Webhook 连接测试）
- ❌ **其他支付网关的详细实现**（原因：Creem 是模拟平台，其他支付平台需要单独的 PRD）
- ❌ **退款处理**（原因：初期版本不支持，可后续添加）
- ❌ **Disputes 处理**（原因：初期版本不支持，可后续添加）
- ❌ **多币种转换**（原因：使用 Stripe 原生币种支持）
- ❌ **税务计算**（原因：使用 Stripe Tax 或后续集成）

### 2.3 依赖项

- ✅ 通用支付平台配置系统（状态: 待实现，见 Billing PRD 第 3.1 节）
- ✅ Billing 订阅计费系统（状态: 部分实现，docs/prd/billing/subscription.md）
- ✅ Realm 管理系统（状态: 已实现）
- ✅ 用户管理系统（状态: 已实现）
- ❌ Stripe 账户和 API 密钥（状态: 待配置）

---

## 3. 需求概述

### 3.1 功能描述

Stripe 支付集成是 Herald 系统支付平台选项之一，与 Creem（模拟平台）并列。Realm Admin 可以选择使用 Stripe 作为订阅和一次性支付的处理平台。

**关键特性**：
1. **订阅支付**: 处理周期性订阅计费，与现有 Billing 系统集成
2. **一次性支付**: 处理一次性购买和充值场景
3. **Webhook 同步**: 实时接收 Stripe 事件，保持支付状态同步
4. **多租户支持**: 每个 Realm 可以配置独立的 Stripe 账户

### 3.2 与其他支付平台的对比

| 特性 | Creem | Stripe |
|------|-------|--------|
| 类型 | 模拟平台 | 真实支付 |
| 环境 | Sandbox | Test + Live |
| 适用场景 | 开发测试 | 生产环境 |
| Webhook | 模拟 | 真实 |
| 支持币种 | 任意 | 135+ |
| 状态 | 已实现 | 待实现 |

### 3.3 业务价值

- **用户价值**: 提供业界领先的支付体验，支持多种支付方式（信用卡、借记卡、Apple Pay、Google Pay）
- **业务价值**: 降低支付集成复杂度，提高支付成功率（Stripe 平均成功率 92%+）
- **技术价值**: 利用 Stripe 的成熟基础设施，减少维护成本

---

## 4. 当前实现状态

| 功能模块 | 状态 | 备注 |
|---------|------|------|
| 通用支付平台配置 API | ❌ | 待实现（见 Billing PRD） |
| Stripe 配置管理 | ❌ | 待实现 |
| Payment Intent 创建 | ❌ | 待实现 |
| 支付确认处理 | ❌ | 待实现 |
| Webhook 端点 | ❌ | 待实现 |
| Webhook 事件处理 | ❌ | 待实现 |
| 支付历史记录 | ❌ | 待实现 |
| 支付方式管理 | ❌ | 待实现 |
| 与 Billing 系统集成 | ❌ | 待实现 |
| 前端支付页面 | ❌ | 待实现 |

---

## 5. 功能需求

### 5.1 Stripe 配置管理

- 每个 Realm 可以配置独立的 Stripe 账户（支持多租户）
- 配置项包括：Account ID、Publishable Key、Secret Key、Webhook Signing Secret
- API Key 必须加密存储在数据库中
- 只有 Realm Admin 可以查看和更新 Stripe 配置
- Secret Key 查看时显示脱敏信息（如 `sk_test_*******************`）
- **编辑时密钥保留**：更新配置时，敏感字段（Secret Key、Webhook Secret）为可选；留空则保留现有值。非敏感字段（Publishable Key、Timeout 等）正常更新

### 5.2 支付处理流程

**一次性支付流程**:
1. 用户选择商品或服务
2. 前端请求创建 Payment Intent
3. 后端调用 Stripe API 创建 Payment Intent
4. 后端返回 Client Secret 给前端
5. 前端使用 Stripe.js 确认支付
6. Webhook 异步通知支付结果
7. 后端更新本地支付状态

**订阅支付流程**:
1. 用户选择订阅套餐
2. 前端请求创建订阅
3. 后端调用 Stripe API 创建 Subscription
4. 后端返回 Client Secret 给前端（如需支付确认）
5. 前端使用 Stripe.js 确认支付
6. Webhook 异步通知订阅状态
7. 后端更新本地订阅状态

### 5.3 安全要求

- API Key 不得暴露给前端（只有 Publishable Key 可以暴露）
- Webhook 端点必须验证 Stripe Signature
- 所有支付操作必须通过 HTTPS
- 支付敏感信息不得存储在本地数据库
- 所有支付操作必须记录审计日志

### 5.4 错误处理

- 支付失败时返回用户友好的错误信息
- 支持支付重试机制（针对临时性错误）
- 记录所有支付失败事件用于分析和调试
- Webhook 处理失败时支持重试（最多 3 次）

### 5.5 数据隔离

- 不同 Realm 的支付数据完全隔离
- 用户只能查看自己的支付历史
- Realm Admin 只能查看所属 Realm 的支付数据

### 6.1 Realm Admin 配置管理

**配置项**:
- Stripe Account ID（可选，用于多账户管理）
- Publishable Key（pk_*）
- Secret Key（sk_*）
- Webhook Signing Secret（whsec_*）
- Webhook Endpoint URL
- Environment（test 或 live）

**操作**:
- 创建 Stripe 配置
- 查看 Stripe 配置（Secret Key 脱敏显示）
- 更新 Stripe 配置
- 删除 Stripe 配置（无活跃订阅时可删除）

### 6.2 支付处理

**一次性支付**:
- 创建 Payment Intent
- 获取 Client Secret
- 确认支付
- 处理支付结果

**订阅支付**:
- 创建 Stripe Subscription
- 处理首次支付
- 处理续费事件
- 取消订阅

### 6.3 Webhook 事件处理

**支持的事件类型**:
- `payment_intent.succeeded`: 支付成功
- `payment_intent.payment_failed`: 支付失败
- `invoice.paid`: 发票已支付
- `invoice.payment_failed`: 发票支付失败
- `customer.subscription.created`: 订阅创建
- `customer.subscription.updated`: 订阅更新
- `customer.subscription.deleted`: 订阅取消
- `invoice.payment_succeeded`: 订阅续费成功

**处理逻辑**:
1. 接收 Webhook 请求
2. 验证 Stripe Signature
3. 解析事件类型
4. 根据事件类型执行相应的业务逻辑
5. 更新本地数据库状态
6. 记录事件日志

### 6.4 支付历史记录

**查询功能**:
- 用户查看自己的支付历史
- Realm Admin 查看 Realm 的所有支付记录
- 支持按状态、时间范围、金额筛选
- 支持分页查询

**显示信息**:
- 支付 ID
- 支付金额和币种
- 支付状态
- 支付时间
- 支付方式（脱敏显示）
- 关联的订阅或订单 ID

---

## 6. API 相关约束

**状态**: 必填

- 仅说明计费、套餐、积分、支付配置、订阅变更或 webhook 处理的能力边界，不在 PRD 中列出端点、schema 或状态码细节。
- 必须遵守 realm 隔离、管理员权限、金额与积分变更可追溯、回调幂等和失败补偿要求。
- 与支付平台、积分账本、订阅系统的详细契约应下沉到技术设计、接口说明或实现代码。

---

## 7. 前端/交互约束

**状态**: 必填

- 仅保留管理入口、关键操作路径、筛选/查看/变更的交互约束和状态反馈，不写组件实现、数据层封装或代码结构。
- 计费与积分场景必须突出金额/积分变化、变更影响范围、不可逆风险提示和回调同步中的状态说明。

---

## 8. 技术设计承接

**状态**: 必填

- 接口细节、数据库结构、迁移策略、类型定义、调度方案、SDK 设计和实现步骤，应在 `docs/design/`、`.ai/design/`、接口说明或代码中承接。
- 如历史实现已经存在，应以现有设计文档、OpenAPI、迁移文件和代码为依据补充，不回写到 PRD 正文。

---

## 9. 相关文件索引

### 10.1 后端文件

**领域层**:
- `backend/core/src/domain/billing/mod.rs` - 支付平台模块导出
- `backend/core/src/domain/billing/providers.rs` - 支付平台抽象接口（PaymentProvider trait）
- `backend/core/src/domain/billing/stripe.rs` - Stripe 支付平台实现
- `backend/core/src/domain/billing/creem.rs` - Creem 支付平台实现（模拟）
- `backend/core/src/domain/billing/entities.rs` - 支付实体定义（PaymentRecord、SubscriptionRecord）
- `backend/core/src/domain/billing/webhook_handler.rs` - Webhook 事件处理器

**应用层**:

**数据库**:

### 10.2 前端文件

**页面组件**:
- `frontend/src/routes/$realmId/billing/payment-providers.tsx` - 支付平台配置管理页面
- `frontend/src/routes/$realmId/billing/checkout.tsx` - 一次性支付页面
- `frontend/src/routes/$realmId/billing/subscribe.tsx` - 订阅支付页面
- `frontend/src/routes/$realmId/billing/payment-history.tsx` - 支付历史页面

**业务组件**:
- `frontend/src/components/billing/PaymentProviderForm.tsx` - 支付平台配置表单
- `frontend/src/components/billing/PaymentProviderList.tsx` - 支付平台列表
- `frontend/src/components/billing/StripeCardInput.tsx` - Stripe 卡片输入组件
- `frontend/src/components/billing/PaymentStatus.tsx` - 支付状态显示组件
- `frontend/src/components/billing/PaymentMethodSelector.tsx` - 支付方式选择器

**服务层**:
- `frontend/src/lib/payment-service.ts` - 支付 API 服务封装
- `frontend/src/lib/stripe-service.ts` - Stripe 特定服务封装

**状态管理**:
- `frontend/src/stores/payment-provider-store.ts` - 支付平台配置状态管理
- `frontend/src/stores/payment-store.ts` - 支付流程状态管理

### 10.3 测试文件

**后端场景测试**:

**前端组件测试**:
- `frontend/tests/billing/payment-provider-form.test.tsx` - 支付平台配置表单测试
- `frontend/tests/billing/payment-provider-list.test.tsx` - 支付平台列表测试
- `frontend/tests/billing/stripe-card-input.test.tsx` - Stripe 卡片输入测试
- `frontend/tests/billing/payment-method-selector.test.tsx` - 支付方式选择器测试

**E2E Demo 测试**:
- `demo/e2e/billing/payment-provider-config.spec.ts` - 支付平台配置 E2E 测试
- `demo/e2e/billing/stripe-payment-flow.spec.ts` - Stripe 支付流程 E2E 测试

### 10.4 SDK 集成

- `backend/sdk/src/lib.rs` - 主 SDK 入口
- `backend/sdk/src/payments.rs` - 支付相关 SDK 方法
- `backend/sdk/src/subscriptions.rs` - 订阅相关 SDK 方法

---

## 10. 参考资料

### 11.1 Stripe 官方文档
- [Stripe API 文档](https://stripe.com/docs/api)
- [Webhooks 指南](https://stripe.com/docs/webhooks)
- [Stripe.js 文档](https://stripe.com/docs/js)

### 11.2 相关用户故事
- 📄 [docs/user-stories/billing/payment-provider.md](/docs/user-stories/billing/payment-provider.md) - 支付平台配置用户故事

### 11.3 相关 PRD
- [Billing 订阅计费 PRD](/docs/prd/billing/subscription.md) - 现有订阅计费系统
- [Subscription History PRD](/docs/prd/billing/subscription.md) - 订阅变更历史

### 11.4 技术资源
- [Stripe Rust SDK](https://docs.rs/stripe-rust/)
- [Stripe React SDK](https://stripe.com/docs/stripe-js/react)

