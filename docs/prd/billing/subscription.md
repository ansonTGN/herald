# 订阅计费产品需求文档 (PRD)

**创建时间**: 2025-01-30
**状态**: Partially Implemented

---
## 1. 相关用户故事

> **说明**: 详细的用户故事和验收标准请查看 `../../user-stories/` 目录中的对应文件。

### 1.1 Realm Admin 用户故事

- 📄 [docs/user-stories/core/realm-admin.md](/docs/user-stories/core/realm-admin.md)
  - **订阅套餐管理** (P0): 作为 Realm Admin，我想要管理订阅套餐，以便为用户提供不同的订阅选项

### 1.2 Billing 用户故事

- 📄 [docs/user-stories/billing/subscription.md](/docs/user-stories/billing/subscription.md)
  - **[US-BI-001] 创建订阅套餐** (P0): 作为 Realm Admin，我想要在 Product 上下文中创建订阅套餐，以便定义价格和计费信息
  - **[US-BI-002] 编辑订阅套餐** (P0): 作为 Realm Admin，我想要在 Product 上下文中编辑订阅套餐，以便更新价格和描述
  - **[US-BI-003] 配置 Plan 的支付平台映射** (P0): 作为 Realm Admin，我想要为 Plan 配置一个或多个支付平台映射，以便该套餐可以在不同支付平台上售卖
  - **[US-BI-004] 删除订阅套餐** (P0): 作为 Realm Admin，我想要在 Product 上下文中删除订阅套餐，以便移除不再需要的套餐
  - **[US-BI-005] 分配套餐到 Client App** (P0): 作为 Realm Admin，我想要将套餐分配到 Client App，以便控制哪些应用可以提供哪些订阅
  - **[US-BI-006] 查看订阅列表** (P0): 作为 Realm Admin，我想要查看订阅列表，以便了解订阅情况
  - **[US-BI-007] 第三方应用查询套餐状态** (P0): 作为 Third-party App，我想要通过 SDK 查询用户的订阅和套餐状态，以及可用的支付平台选项

### 1.3 订阅变更历史用户故事

- 📄 [docs/user-stories/billing/subscription.md](/docs/user-stories/billing/subscription.md)
  - **[US-BI-008] 查看订阅变更历史** (P1): 作为 Realm Admin，我想要查看所有用户的订阅变更历史，以便监控和管理订阅情况
  - **[US-BI-009] 查看自己的订阅变更历史** (P1): 作为 Regular User，我想要查看我的订阅变更历史，以便了解订阅的变更轨迹

### 1.4 用户故事优先级汇总

| 优先级 | 用户故事数量 | 关键故事 |
|--------|------------|---------|
| P0 | 7 | 创建/编辑订阅套餐、配置支付平台映射、删除套餐、分配套餐到 Client App、查看订阅列表、第三方应用查询套餐状态（SDK） |
| P1 | 2 | 查看订阅变更历史（Realm Admin）、查看自己的订阅变更历史（Regular User） |
| P2 | 0 | - |

---

## 2. 范围界定

### 2.1 包含功能

- ✅ 订阅套餐管理（创建、编辑、删除、查看）
- ✅ 套餐分配到 Client App
- ✅ 支持多种支付平台（Creem - 模拟支付平台）
- ✅ 灵活的订阅套餐管理（月付/年付）
- ✅ 套餐基本信息（name、title、description、type、price、currency、checkout_url）
- ✅ 前端套餐管理页面
- ✅ 前端套餐分配对话框
- ✅ 查询单个订阅的变更时间线
- ✅ 按用户、套餐、时间等维度查询订阅历史（Realm Admin）
- ✅ 显示变更类型（创建、升级、降级、取消、续费等）
- ✅ 显示变更前后的状态对比
- ✅ 前端历史记录页面（全局查询和单订阅详情）
- ✅ 权限控制（Realm Admin 可查看所有历史，Regular User 只能查看自己的）

### 2.2 不包含功能 (Out of Scope)

- ❌ **订阅自动续费** (原因: 由支付平台处理，Herald 不负责)
- ❌ **套餐的功能（features）和配额（quotas）管理** (原因: 采用简化模型，由第三方应用自行管理)
- ❌ **支付方式管理** (原因: 前后端均未实现)
- ❌ **计费统计和报表** (原因: 前后端均未实现)
- ❌ **通知系统** (原因: 邮件/短信通知未实现)
- ❌ **支付事件历史** (原因: 支付事件由支付平台处理，Herald 不负责记录)
- ❌ **导出历史记录** (原因: 属于 P2 功能，暂不实现)
- ❌ **历史记录审计日志** (原因: 可选扩展功能，暂不实现)
- ❌ **历史记录统计分析** (原因: 属于计费统计报表功能，单独规划)

**注意**：
- ✅ **退款功能**：支付平台处理金额退款，Herald 处理积分回收（详见 `docs/prd/billing/points.md` 第 5.8 节）
- ✅ **订阅管理**：即使前后端订阅管理功能未完整实现，积分系统仍需定义订阅变更的积分处理规则（详见 `docs/prd/billing/points.md` 第 5.9 节）

### 2.3 新增状态说明

#### Past Due (逾期)
- **触发条件**: 收到 `subscription.past_due` 事件
- **访问权限**: 否（立即撤销访问）
- **后续处理**:
  - 用户更新支付方式后恢复 `Active` 状态
  - 多次支付失败后转为 `Expired` 状态
  - 显示警告信息提示用户更新支付方式

#### Disputed (争议中)
- **触发条件**: 收到 `dispute.created` 事件
- **访问权限**: 是（争议调查期间保持访问）
- **后续处理**:
  - 记录争议详情（ID、金额、原因）
  - 发送告警给运营团队
  - 争议解决后根据结果转为 `Active` 或 `Canceled`
  - 收集证据准备应对争议

#### Scheduled Cancel (预定取消)
- **触发条件**: 收到 `subscription.scheduled_cancel` 事件
- **访问权限**: 是（直到计费周期结束）
- **后续处理**:
  - 用户在周期结束前仍可正常访问
  - 显示"将在周期结束时取消"提示
  - 可在周期结束前取消预定取消操作

#### Refund (退款)
- **触发条件**: 收到 `refund.created` 事件
- **访问权限**: 不受影响（退款不影响访问权限）
- **处理方式**: 仅记录日志用于审计，不撤销访问权限

### 2.4 依赖项

- ✅ **Realm 系统** (状态: 已实现) - Billing 功能属于 Realm 级别
- ✅ **Client App 系统** (状态: 已实现) - 套餐分配到 Client App
- ✅ **权限管理系统** (状态: 已实现) - Realm Admin 权限检查
- ⚠️ **支付平台集成** (状态: 部分实现) - Creem 模拟支付平台已实现，Stripe/支付宝/微信支付待实现

---

## 3. 需求概述

Billing（订阅计费）是 Herald 系统为 Realm 提供的灵活订阅管理和计费方案功能。本文档描述支付平台配置、订阅套餐管理、套餐分配、订阅升级/降级、订阅变更历史等功能需求。

**关键特性**：
- 支持多种支付平台（Creem）
- 灵活的订阅套餐管理（月付/年付）
- 套餐分配到 Client App
- 订阅升级/降级（按比例计费）
- Webhook 集成和通知系统
- 完整的计费统计和报表
- 完整的订阅变更历史记录

**架构说明**：
- ⭐ **简化模型**：Herald 不管理套餐的功能（features）和配额（quotas），由第三方应用自行管理
- 套餐只包含基本信息：name, title, description, type, price, currency, checkout_url
- Realm Admin 可以创建套餐并分配到 Client App
- 最终用户通过第三方应用进行订阅和支付

**注意**：
- ❌ 当前项目**不提供**订阅自动续费功能（由支付平台处理）
- ✅ **退款功能**：支付平台处理金额退款，Herald 处理积分回收（详见积分系统 PRD）
- ✅ **订阅生命周期积分处理**：即使订阅管理功能未完整实现，积分系统仍需定义订阅变更的积分处理规则（详见积分系统 PRD）
- ✅ **编目边界**：Billing 编目正在从 `Realm -> Plan` 演进为 `Realm -> Product -> Plan`；Product 的主定义以 `docs/prd/billing/product-catalog.md` 为准，本文档继续聚焦订阅、支付与 Plan 计费语义

---

## 4. 当前实现状态

### 4.1 已实现功能

- ✅ **后端实体层**：Billing Plan、Plan Assignment 实体定义
- ✅ **后端 Repository 层**：Plan 数据库操作接口和实现
- ✅ **后端 Service 层**：Plan 业务逻辑和权限检查
- ✅ **后端 HTTP API**：Plan CRUD、Plan Assignment RESTful API
- ✅ **前端数据层**：Plan API 调用函数
- ✅ **前端类型定义**：TypeScript 类型定义
- ✅ **前端套餐管理页面**：套餐列表、创建、编辑、删除
- ✅ **前端套餐分配对话框**：套餐分配到 Client App
- ✅ **演示测试**：billing-plan-crud.e2e.ts、billing-plan-assignments.e2e.ts

### 4.2 部分实现功能

- ⚠️ **支付提供商配置**：前端 UI 已实现，后端 API 待完成
- ⚠️ **订阅管理**：前端 UI 部分实现，后端 API 待完成
- ⚠️ **多支付平台支持**：模型设计已完成，待实施重构

### 4.3 未实现功能

- ❌ **Plan Payment Provider 映射管理**：前后端均未实现（需重构现有单平台模型）
- ❌ **订阅升级/降级**：前后端均未实现
- ❌ **支付方式管理**：前后端均未实现
- ❌ **计费统计和报表**：前后端均未实现
- ❌ **通知系统**：邮件/短信通知未实现
- ❌ **订阅变更历史后端 API**：需要实现历史查询接口
- ❌ **订阅变更历史前端页面**：需要实现历史记录展示
- ❌ **订阅变更历史数据模型**：需要新建 subscription_history 表
- ❌ **订阅变更历史记录创建**：需要在订阅变更时创建历史记录

### 4.4 待重构内容

**多支付平台重构**（详见 `.ai/future/plan_pay_problem.md`）：

**当前问题**：
- Plan 模型将支付平台映射直接内嵌在主表中（payment_provider、external_product_id、external_price_id）
- 一条 Plan 记录只能表达一个支付平台下的售卖配置
- 要支持多支付平台需要复制多条 Plan（如 pro-monthly-stripe、pro-monthly-creem）

**重构目标**：
- Plan 只表示业务套餐本身
- 一个 Plan 可以关联多个支付平台配置
- 新增 Plan Payment Provider 映射对象承载支付平台接入信息

**重构范围**：
- 数据库：创建 plan_payment_provider 映射表
- 后端：调整 Plan 实体、Repository、Service 和 API
- 前端：调整套餐创建/编辑/查看界面，新增支付平台配置界面
- 迁移：将现有 Plan 的支付平台数据回填到新表

**说明**：
- Billing 功能采用**简化模型**，Herald 只负责套餐基本信息管理，不管理 features 和 quotas
- 功能开关和配额限制由第三方应用自行管理
- 支付流程由第三方支付平台（Creem）处理，Herald 只负责套餐配置和分发
- 多支付平台重构是模型升级，不是简单的实现细节重构

### 4.5 安全警告

**P1 - 权限检查缺失**：

当前实现存在**严重安全问题**：
- ❌ **所有计费相关 API 缺少权限检查**
- ❌ 任何认证用户都可以创建、编辑、删除计费计划
- ❌ 任何认证用户都可以分配计划到客户端应用

**影响**：
- 非管理员用户可以修改计费配置
- 可能导致计费混乱和安全漏洞

**修复计划**：
- 🔄 **待实施**：在 Service 层添加权限检查
- 🔄 **待实施**：验证用户是否为 Realm Admin
- 🔄 **待实施**：限制只有 Realm Admin 才能管理计费计划

**临时缓解措施**：
- 在生产环境中**谨慎部署**此功能
- 确保只有可信用户能访问系统
- 监控计费配置变更

---

## 5. 功能需求

### 5.1 支付平台配置

#### 5.1.1 支持的支付平台

**当前支持**：
- ✅ **Creem**：模拟支付平台（用于开发和测试）

**未来支持**（待实现）：
- ❌ **Stripe**：真实支付平台
- ❌ **支付宝/微信支付**：中国支付平台

#### 5.1.2 配置支付平台

**路由**：`/$realmId/billing/payment-providers`

**功能**：
- 添加支付平台配置（API Key、Secret Key、Webhook Secret）
- 编辑支付平台配置
- 启用/禁用支付平台
- 删除支付平台配置（无订阅时可删除）

**安全说明**：
- API Secret Key 和 Webhook Secret 必须加密存储
- 只在创建和更新时显示 Secret，后续只显示部分掩码
- 删除支付平台前需检查是否有活跃订阅

#### 5.1.3 Webhook 端点配置

**架构说明**：
系统采用 realm 隔离的 webhook 端点架构，每个 realm 使用独立的 webhook URL。这种设计提供了更好的安全性、可扩展性和多租户隔离。

**Webhook 端点格式**：

**Creem Webhook**:

**Stripe Webhook**:

- `realmId`: Realm ID（从 URL 路径提取，用于多租户隔离）

**关键特性**：
1. **Realm 隔离**：每个 realm 使用独立的 webhook URL
3. **自动配置**：Checkout session 创建时自动设置正确的 webhook URL
4. **签名验证**：Webhook secret 存储在 realm_config 表中，用于签名验证

**配置示例**：

在 Creem Dashboard 中配置 webhook URL：

在 Stripe Dashboard 中配置 webhook URL：

**Realm 配置存储**：

**Checkout Session 创建时的自动配置**：
当创建 checkout session 时，系统会自动设置 realm 特定的 webhook URL：

**Stripe 示例**：

**Creem 示例**：

**安全性说明**：
- Webhook URL 使用 HTTPS 协议
- Webhook secret 使用 AES-256-GCM 加密存储
- 每个 realm 的 webhook secret 独立生成
- Webhook 签名验证失败时拒绝处理请求

**迁移说明**（从旧架构迁移）：
- ✅ 已完成：webhook URL 从共享端点改为 realm 特定端点
- ✅ 已完成：realm_id 从 metadata 提取改为从 URL 路径提取
- ✅ 已完成：checkout session 创建时自动设置正确的 webhook URL
- ⚠️ 注意：需要在支付平台（Creem/Stripe）Dashboard 中重新配置 webhook URL

### 5.2 订阅套餐管理

#### 5.2.1 套餐模型（多支付平台支持）

**核心模型变更**：
- Plan 表示业务套餐本身，不包含支付平台映射信息
- 一个 Plan 可以关联多个支付平台配置
- 每个支付平台配置分别保存外部商品、价格、checkout 等接入信息

**套餐实体结构**：

**Plan 实体**（业务套餐）：
- `realm_id`: Realm ID（多租户隔离）
- `product_id`: 所属 Product ID（编目上层）
- `name`: 套餐唯一标识符（如 "basic-monthly"）
- `title`: 用户友好显示名称（如 "基础版"）
- `description`: 套餐描述
- `type`: 计费周期（monthly/yearly）
- `price`: 价格（整数，如 1000 = $10.00）
- `currency`: 货币（USD/EUR/CNY）
- `checkout_url`: 默认 checkout URL
- `trial_days`: 试用期天数（0-365）
- `active`: 是否启用
- `sort_order`: 排序顺序

**Plan Payment Provider 映射实体**：
- `plan_id`: 所属 Plan ID
- `payment_provider`: 支付平台名称（stripe/creem）
- `external_product_id`: 外部平台的产品 ID
- `external_price_id`: 外部平台的价格 ID
- `enabled`: 该映射是否启用
- `created_at`: 创建时间
- `updated_at`: 更新时间

**编目语义补充**：
- Plan 是订阅和计费的直接承载对象，表示业务套餐本身
- Product 是 Plan 的上层编目对象
- Plan Payment Provider 是 Plan 的下属配置对象，表示支付平台映射
- Product 生命周期与管理能力不在本文档主定义，详见 `docs/prd/billing/product-catalog.md`

**已移除字段**（简化模型 + 多支付平台重构）：
- ❌ `features: Record<string, boolean>` - 功能开关（由第三方应用管理）
- ❌ `quotas: Record<string, number>` - 配额限制（由第三方应用管理）
- ❌ `payment_provider` - 已从 Plan 主表移除，迁移到 Plan Payment Provider 映射表
- ❌ `external_product_id` - 已从 Plan 主表移除，迁移到 Plan Payment Provider 映射表
- ❌ `external_price_id` - 已从 Plan 主表移除，迁移到 Plan Payment Provider 映射表

**重要说明**：
- 套餐的 `name` 字段是唯一标识符，用于 API 调用和前端路由
- 套餐的 `title` 字段是用户友好的显示名称
- `features` 和 `quotas` 由第三方应用自行管理，Herald 不存储这些信息
- 支付平台映射独立管理，一个 Plan 可以配置多个支付平台
- 不需要为每个支付平台复制 Plan

#### 5.2.2 创建订阅套餐

**路由**：`/$realmId/billing`

**表单字段**：
| 字段名 | 类型 | 必填 | 说明 |
|--------|------|------|------|
| Name | string | 是 | 字母数字、横线、下划线，3-50字符，全局唯一 |
| Title | string | 是 | 1-100字符 |
| Description | string | 是 | 1-500字符 |
| Type | enum | 是 | monthly 或 yearly |
| Price | number | 是 | 大于0，整数（美分） |
| Currency | enum | 是 | USD, EUR, CNY |
| Product ID | string | 是 | 所属 Product 的 ID |
| Checkout URL | string | 是 | 有效的 HTTP/HTTPS URL。业务套餐的默认 checkout URL，用于前端展示和参考 |
| Trial Days | number | 否 | 0-365，默认0 |
| Sort Order | number | 否 | 整数，默认0 |

**注意事项**：
- 本表单只包含业务套餐字段，不包含支付平台映射字段（payment_provider、external_product_id、external_price_id 等）
- 创建套餐时不需要配置支付平台映射
- 创建成功后引导用户配置支付平台映射
- 支付平台映射通过独立的配置界面管理
- 实际支付流程使用的 checkout URL 由各支付平台映射配置决定

#### 5.2.3 配置 Plan 的支付平台映射

**目的**：为 Plan 配置一个或多个支付平台映射，使该套餐可以在不同支付平台上售卖。

**配置界面入口**：
- 在套餐详情页面点击 "Manage Payment Providers" 按钮
- 在套餐列表中点击套餐的 "Payment Providers" 列
- 在创建套餐成功后点击引导提示中的 "Add Payment Provider" 按钮

**配置界面交互流程**：
1. 进入支付平台配置页面，显示当前套餐信息（名称、标题、价格等）
2. 显示已配置的支付平台映射列表
3. 提供 "Add Payment Provider" 按钮
4. 点击按钮后弹出配置表单对话框

**配置表单字段**：
| 字段名 | 类型 | 必填 | 说明 |
|--------|------|------|------|
| Payment Provider | enum | 是 | 从 Realm 已配置的支付平台中选择（stripe/creem） |
| External Product ID | string | 是 | 外部平台的产品 ID |
| External Price ID | string | 否 | 外部平台的价格 ID（可选） |
| Enabled | boolean | 是 | 该支付平台映射是否启用（默认启用） |

**操作约束**：
- 同一 Plan 不能重复配置同一个支付平台
- 删除支付平台映射前需检查是否有活跃订阅
- 禁用支付平台映射不影响已订阅用户，但新用户无法使用该支付平台
- 启用支付平台映射时，如果该平台未在 Realm 层配置，应提示用户先配置支付平台

**配置列表展示**：
在套餐详情页面显示所有已配置的支付平台映射：
| Provider | External Product ID | External Price ID | Status | Actions |
|----------|---------------------|-------------------|--------|---------|
| Stripe   | prod_basic_monthly  | price_basic_monthly| enabled| Edit/Delete |
| Creem    | prod_basic_creem    | -                 | disabled| Edit/Delete |

**状态反馈**：
- 创建成功：显示成功消息 "Payment provider 'Stripe' added to plan 'basic'"
- 编辑成功：显示成功消息 "Payment provider mapping updated successfully"
- 删除成功：显示成功消息 "Payment provider mapping deleted successfully"
- 删除失败（有活跃订阅）：显示错误消息 "Cannot delete payment provider mapping with X active subscriptions"
- 禁用成功：显示提示消息 "Existing subscriptions will continue to work, new users cannot select this provider"

#### 5.2.4 查看订阅套餐列表

**路由**：`/$realmId/billing`

**Query Parameters**:
- `enabled`: boolean (optional) - 筛选启用状态的套餐
- `type`: 'monthly' | 'yearly' (optional) - 筛选计费周期

**前端表格列**（多支付平台支持）：
| 列名 | 说明 | 数据源 |
|------|------|--------|
| Plan | 套餐标题、名称、描述 | plan.title, plan.name, plan.description |
| Billing | 计费周期（月付/年付） | plan.type |
| Price | 价格 | plan.price, plan.currency |
| Payment Providers | 支持的支付平台列表 | plan_payment_providers 映射表 |
| Trial Days | 试用期 | plan.trial_days |
| Status | 状态（启用/禁用） | plan.active |
| Order | 排序顺序 | plan.sort_order |
| Actions | 操作（编辑、删除、分配、配置支付平台） | - |

**Payment Providers 列展示**：
- 如果配置了多个支付平台，显示为 "Stripe, Creem"
- 如果只配置了一个支付平台，显示该平台名称
- 如果没有配置支付平台，显示 "Not configured" 并高亮提示
- 点击支付平台列可以跳转到支付平台配置页面

**已移除列**（简化模型 + 多支付平台重构）：
- ❌ Features - 功能开关（由第三方应用管理）
- ❌ Quotas - 配额限制（由第三方应用管理）
- ❌ Provider - 单一支付平台列（已替换为 Payment Providers 多平台列）

#### 5.2.5 编辑订阅套餐

**说明**：
- `name` 字段不可修改（套餐的唯一标识符）
- 更新价格会影响新订阅用户，已订阅用户保持原价格直到续费
- 更新 `checkout_url` 会立即生效，所有新订阅用户使用新 URL

#### 5.2.6 删除订阅套餐

**删除限制**：
- ❌ 无法删除有活跃订阅的套餐
- ✅ 可以删除无订阅的套餐（包括已取消订阅的套餐）
- ✅ 删除套餐时会级联删除所有支付平台映射

**错误响应**：

### 5.3 套餐分配管理

#### 5.3.1 分配套餐到 Client App

**目的**：允许 Realm Admin 为特定的 Client App 分配可用套餐，最终用户只能看到已分配的套餐。

#### 5.3.2 查看套餐分配

#### 5.3.3 移除套餐分配

**说明**：
- 移除分配不会影响已订阅用户
- 移除分配后，新用户无法看到该套餐

### 5.4 订阅管理（待实现）

#### 5.4.1 创建订阅

**流程**：
1. 用户在第三方应用中选择套餐
2. 第三方应用重定向到 Herald 的 `checkout_url`
3. Herald 显示支付平台（Creem）的付费页面
4. 用户完成支付
5. Creem 发送 Webhook 通知 Herald
6. Herald 创建订阅记录

#### 5.4.2 升级订阅

**按比例计费（Proration）**：

#### 5.4.3 降级订阅

**降级规则**：
- 降级在**下个计费周期**生效（当前周期保持原套餐）
- 如果当前用户数超过目标套餐限制，不允许降级

#### 5.4.4 取消订阅

**取消规则**：
- 取消在**当前计费周期结束**生效
- 取消后用户可以继续使用直到周期结束
- 取消后不自动删除数据，数据保留期由第三方应用决定

### 5.5 Webhook 处理（已实现）

#### 5.5.1 支持的 Webhook 事件

**Creem Webhook 事件**（已实现）：
- `checkout.completed` - 一次性支付成功
- `subscription.active` - 订阅激活
- `subscription.trialing` - 开始试用期
- `subscription.paid` - 订阅续费成功
- `subscription.paused` - 订阅暂停
- `subscription.canceled` - 订阅取消
- `subscription.expired` - 订阅过期
- `subscription.update` - 订阅更新（升级/降级）

#### 5.5.2 Webhook 端点

**架构说明**：

**Creem Webhook**:

**Stripe Webhook**:

- `realmId`: Realm ID（从 URL 路径提取，用于多租户隔离）

**Request Headers**:

**实现功能**：
- ✅ Creem 签名验证（HMAC-SHA256）
- ✅ Stripe 签名验证（HMAC-SHA256）
- ✅ 事件幂等性处理（防止重复处理）
- ✅ 订阅状态解析和转换
- ✅ 状态转换验证（`can_transition_to()`）

#### 5.5.3 积分充值集成

当订阅创建或续费时，billing webhook handler 会自动调用积分系统进行充值：

1. **首次订阅充值** (`subscription.paid` 事件):
   - 查询积分套餐配置 (`points_plan_configs`)
   - 根据 `points_on_subscribe` 配置进行充值
   - 调用 `PointsService::recharge_points_internal()` 直接充值
   - 不经过 HTTP 接口，提升性能

2. **定期续费充值** (`subscription.renewed` 事件):
   - 查询积分套餐配置
   - 根据 `renewal_enabled` 和 `points_on_renewal` 配置决定是否充值
   - 如果禁用自动充值（`renewal_enabled=false`），则跳过充值
   - 调用 `PointsService::recharge_points_internal()` 直接充值

**充值实现方式**:
- 使用内部 service 方法 `recharge_points_internal()`
- 不对外暴露 HTTP 接口
- 支持最大累计积分限制（`max_accumulation`）
- 自动创建积分账户（如果不存在）

**事务保证**:
- 充值操作与数据库事务绑定
- 使用乐观锁防止并发问题
- 充值失败不影响订阅状态

### 5.6 计费统计和报表（待实现）

#### 5.6.1 订阅总览

#### 5.6.2 套餐分布统计

#### 5.6.3 收入趋势

**Query Parameters**:
- `period`: 1m, 3m, 6m, 12m（统计周期）

---

## 6. 订阅变更历史

### 6.1 功能描述

Subscription History 功能提供了订阅变更历史记录的查询和展示能力。通过记录每次订阅变更的详细信息（包括变更类型、操作者、变更前后状态等），帮助 Realm Admin 监控和管理订阅情况，同时帮助 Regular User 了解自己的订阅变更轨迹。

### 6.2 目标用户

- **Realm Admin**：查看和管理 Realm 内所有用户的订阅变更历史
- **Regular User**：查看自己的订阅变更历史

### 6.3 关键特性

- **完整的变更时间线**：记录从订阅创建到当前的所有变更事件
- **多维度筛选**：支持按用户、套餐、变更类型、时间等维度筛选
- **变更前后对比**：清晰展示每次变更的前后状态
- **权限控制**：Realm Admin 可查看所有历史，Regular User 只能查看自己的
- **变更类型丰富**：支持创建、升级、降级、取消、过期、续费、激活、计费周期变更等多种变更类型

### 6.4 业务价值

- **透明度**：用户可以清晰了解订阅的变更历史和原因
- **可追溯性**：Realm Admin 可以追踪任何订阅变更的来源和时间
- **问题排查**：当出现订阅异常时，可以通过历史记录快速定位问题
- **运营洞察**：通过历史数据分析用户的订阅行为模式

### 6.5 单订阅历史

#### 功能描述
展示单个订阅从创建到当前的所有变更事件，按时间倒序排列。

#### 需求细节
| 字段 | 说明 | 示例 |
|------|------|------|
| Event ID | 事件唯一标识符 | evt_1234567890 |
| Event Type | 变更类型 | created, upgraded, downgraded, canceled, renewed, reactivated |
| Timestamp | 变更时间 | 2025-01-15 10:30:00 UTC |
| Actor | 操作者 | user@example.com, system |
| Changes | 变更详情 | Plan: basic → pro |
| Previous State | 变更前状态 | { "status": "active", "plan": "basic" } |
| New State | 变更后状态 | { "status": "active", "plan": "pro" } |

#### 变更类型定义

| 类型 | 说明 | 触发场景 |
|------|------|---------|
| `created` | 创建订阅 | 用户首次订阅套餐 |
| `upgraded` | 升级套餐 | 用户从低级套餐升级到高级套餐 |
| `downgraded` | 降级套餐 | 用户从高级套餐降级到低级套餐 |
| `canceled` | 取消订阅 | 用户主动取消订阅 |
| `expired` | 订阅过期 | 订阅因未续费而过期 |
| `renewed` | 续费订阅 | 订阅成功续费 |
| `reactivated` | 激活订阅 | 已取消的订阅重新激活 |
| `billing_period_changed` | 计费周期变更 | 从月付改为年付或反之 |

### 6.6 全局历史查询（Realm Admin）

#### 功能描述
Realm Admin 可以查询 Realm 内所有订阅的历史记录，支持多维度筛选和分页。

#### 筛选条件

| 筛选维度 | 字段名 | 类型 | 示例 |
|---------|--------|------|------|
| 用户 | `user_id` | UUID | 123e4567-e89b-12d3-a456-426614174000 |
| 套餐 | `plan_id` | UUID | 987e6543-e21b-43d3-b456-426614174999 |
| 变更类型 | `event_type` | enum | created, upgraded, canceled |
| 时间范围 | `from_date`, `to_date` | datetime | 2025-01-01, 2025-01-31 |
| 订阅状态 | `subscription_status` | enum | active, canceled, past_due |

#### 分页参数
| 参数 | 说明 | 示例 |
|------|------|------|
| `page` | 页码 | 1 |
| `page_size` | 每页数量 | 20 |
| `sort_by` | 排序字段 | timestamp |
| `sort_order` | 排序方向 | desc |

### 6.7 订阅历史权限控制

| 角色 | 可访问范围 | 说明 |
|------|----------|------|
| Realm Admin | Realm 内所有订阅历史 | 可查看和筛选所有用户的订阅变更 |
| Regular User | 仅限自己的订阅历史 | 只能查看自己订阅的变更记录 |

---

## 7. 非功能需求

### 7.1 权限要求

| 操作 | 需要权限 | 说明 |
|------|---------|------|
| 查看套餐列表 | `billing.view` | 所有已认证用户 |
| 创建套餐 | `billing.manage` | Realm Admin |
| 编辑套餐 | `billing.manage` | Realm Admin |
| 删除套餐 | `billing.manage` | Realm Admin |
| 分配套餐 | `billing.manage` | Realm Admin |
| 查看订阅统计 | `billing.view` | Realm Admin |
| 管理订阅 | `billing.manage` | Realm Admin |
| 查看订阅变更历史（Realm Admin） | `billing.view` | Realm Admin |
| 查看自己的订阅变更历史 | 认证用户 | Regular User |

### 7.2 数据加密

**敏感数据加密存储**：
- ✅ API Secret Key：AES-256-GCM 加密
- ✅ Webhook Secret：AES-256-GCM 加密
- ✅ 只在创建和更新时显示完整 Secret
- ✅ 查询时只显示部分掩码（如 `sk_test_...123`）

### 7.3 性能要求

| 指标 | 要求 |
|------|------|
| 单订阅历史查询响应时间 | < 500ms |
| 全局历史查询响应时间（分页） | < 1000ms |
| 支持的历史记录数量 | 无限制（基于分页） |

### 7.4 数据一致性

- 订阅变更时必须同步创建历史记录
- 历史记录一旦创建不可修改
- 确保变更前后的状态准确性

### 7.5 安全性

- Realm Admin 可查看所有历史记录
- Regular User 只能查看自己的历史记录
- 敏感信息（如支付详情）不记录在历史中

---

## 8. API 相关约束

**状态**: 必填

- 仅说明计费、套餐、积分、支付配置、订阅变更或 webhook 处理的能力边界，不在 PRD 中列出端点、schema 或状态码细节。
- 必须遵守 realm 隔离、管理员权限、金额与积分变更可追溯、回调幂等和失败补偿要求。
- 与支付平台、积分账本、订阅系统的详细契约应下沉到技术设计、接口说明或实现代码。

**多支付平台相关约束**：
- Plan API 应提供套餐本身的 CRUD 操作，不包含支付平台映射信息
- 支付平台映射应通过独立的 API 进行管理（创建、编辑、删除、查看）
- Checkout 请求应显式传递 plan_id + payment_provider 参数
- SDK 查询套餐列表时，应返回套餐及其支持的支付平台列表
- Webhook 处理应能根据外部商品/价格标识定位到正确的 Plan 和支付平台映射
- 删除套餐时需级联删除其所有支付平台映射配置

---

## 9. 前端/交互约束

**状态**: 必填

- 仅保留管理入口、关键操作路径、筛选/查看/变更的交互约束和状态反馈，不写组件实现、数据层封装或代码结构。
- 计费与积分场景必须突出金额/积分变化、变更影响范围、不可逆风险提示和回调同步中的状态说明。

**导航与可见性约束**：

- 管理后台入口只按权限控制：拥有 `billing.view` 的用户可看到 Billing 相关管理入口，包括 Products、Payment Providers、Subscription Plans、Invoices 和 Subscription History；不再因为当前 Realm 尚未配置产品、套餐、发票或订阅历史而隐藏管理入口。
- 个人中心的 Subscription 入口按 Realm 能力开通状态显示：当 Realm 下存在已启用的订阅 Plan 时显示；仅存在历史订阅记录但没有已启用 Plan 时，不单独作为显示入口的依据。
- 订阅购买按钮或 checkout 流程的可用性独立于 Subscription 入口：只有当 Plan 已配置启用的支付平台映射，并且对应支付平台已在 Realm 中启用时，才允许用户发起购买；否则在订阅页面内显示不可购买状态或禁用购买操作。

**多支付平台相关交互约束**：

**套餐创建流程**：
- 创建套餐表单不包含支付平台相关字段
- 创建成功后显示引导信息，提示用户配置支付平台映射
- 提供 "Add Payment Provider" 按钮跳转到支付平台配置页面

**套餐管理界面**：
- 套餐列表的 "Payment Providers" 列显示该套餐支持的所有支付平台
- 未配置支付平台的套餐显示高亮提示
- 套餐详情页面显示 "Payment Providers" 配置区域
- 支付平台配置区域显示所有已配置的映射及其状态

**支付平台配置界面**：
- 支持为套餐添加、编辑、删除支付平台映射
- 添加支付平台时，从 Realm 已配置的支付平台中选择
- 显示每个支付平台映射的启用状态
- 禁用或删除支付平台映射时提示对已订阅用户的影响

**用户订阅流程**：
- 用户选择套餐后，展示该套餐支持的支付平台选项
- 用户选择具体的支付平台后发起 checkout 请求
- 如果套餐没有可用的支付平台，禁用订阅按钮并显示提示

**状态反馈**：
- 创建套餐成功后提示："Please configure payment providers for this plan"
- 删除套餐时提示："This will delete all payment provider mappings for this plan"
- 删除支付平台映射时提示活跃订阅数量："Cannot delete mapping with X active subscriptions"
- 禁用支付平台映射时提示："Existing subscriptions will continue to work, new users cannot select this provider"

---

## 10. 技术设计承接

**状态**: 必填

- 接口细节、数据库结构、迁移策略、类型定义、调度方案、SDK 设计和实现步骤，应在 `docs/design/`、`.ai/design/`、接口说明或代码中承接。
- 如历史实现已经存在，应以现有设计文档、OpenAPI、迁移文件和代码为依据补充，不回写到 PRD 正文。

**多支付平台重构技术设计**：
本次模型升级需要专门的技术设计文档，应包含：
- Plan Payment Provider 映射表的数据库结构和索引设计
- Plan API 与支付平台映射 API 的边界划分
- Checkout 请求与响应契约（plan_id + payment_provider）
- Webhook 到 Plan 和支付平台映射的解析路径
- 前端套餐管理与支付平台配置的交互设计
- 数据迁移策略（现有 Plan 支付平台数据回填到新表）
- 迁移过程中的数据校验和回滚方案
- 测试覆盖范围与关键验收场景

相关参考文档：
- `.ai/future/plan_pay_problem.md` - 多支付平台支持问题分析
- `.ai/design/fix-plan-pay.md` - 多支付平台重构技术设计（待创建）

---

## 11. 相关文件索引

- 相关实现文件请以本功能对应的 `backend/`、`frontend/`、`demo/` 目录和现有设计文档为准。
- 若需补充精确文件清单，应在技术设计文档中维护，避免在 PRD 中混入实现级细节。

### 11.1 后端文件（待创建）

| 文件路径 | 说明 | 状态 |
|---------|------|------|
| `backend/core/src/entity/subscription_history.rs` | Entity 模型 | ❌ 未创建 |

### 11.2 前端文件（待创建）

| 文件路径 | 说明 | 状态 |
|---------|------|------|
| `frontend/src/routes/$realmId/manage/subscription-history.tsx` | 全局历史页面 | ❌ 未创建 |
| `frontend/src/routes/$realmId/account/subscription/$subscriptionId/history.tsx` | 单订阅历史页面 | ❌ 未创建 |
| `frontend/src/components/billing/subscription-history-list.tsx` | 历史列表组件 | ❌ 未创建 |
| `frontend/src/components/billing/subscription-history-filter.tsx` | 筛选组件 | ❌ 未创建 |
| `frontend/src/components/billing/subscription-timeline.tsx` | 时间线组件 | ❌ 未创建 |
| `frontend/src/components/billing/history-event-badge.tsx` | 事件标签组件 | ❌ 未创建 |

### 11.3 测试文件（待创建）

| 文件路径 | 说明 | 状态 |
|---------|------|------|
| `backend/test-support/tests/subscription-history-scenarios.md` | 后端场景测试 | ❌ 未创建 |
| `demo/e2e/subscription-history.spec.ts` | Demo/E2E 测试 | ❌ 未创建 |

---

## 12. 参考资料

- 用户故事: `docs/user-stories/billing/subscription.md`
- 后端开发指南: `../../spec/backend/development.md`
- 前端开发指南: `../../spec/frontend/development.md`
- 权限管理: `docs/prd/permissions.md`
- Client App 管理: `docs/prd/client-app.md`
- 现有订阅模型: `backend/core/src/entity/subscription.rs`
- 现有套餐模型: `backend/core/src/entity/plan.rs`
