# Product & Subscription Model Reduction 产品需求文档 (PRD)

**创建时间**: 2026-06-05
**优先级**: P0

---

## 1. 相关用户故事

> 详细故事与验收标准请查看 `docs/user-stories/` 中对应文档。

### 1.1 新增故事

- `docs/user-stories/billing/entitlement-mapping.md`
  - [US-EM-001] 查看 Provider Entitlement 映射 (P0): 作为 Realm Admin，查看支付方产品到 Herald Entitlement 的映射列表
  - [US-EM-002] 触发 Provider 产品同步 (P1): 作为 Realm Admin，手动触发支付方产品的全量同步
  - [US-EM-003] Webhook 通过 Metadata 映射订阅 (P0): 作为 System，通过支付方 webhook metadata 将外部订阅映射到 Herald 订阅投影
  - [US-EM-004] 基于 Entitlement 应用积分策略 (P0): 作为 System，在订阅事件时基于 entitlement_key 查询和应用积分策略
  - [US-EM-005] SDK 通过 Entitlement 查询订阅状态 (P0): 作为 Third-Party App，通过 entitlement_key 查询用户订阅状态
  - [US-EM-006] 查看订阅投影列表 (P0): 作为 Realm Admin，查看 Realm 内所有订阅投影列表

### 1.2 需更新的现有故事

- `docs/user-stories/billing/subscription.md`
  - [US-BI-001 ~ US-BI-009]: 订阅套餐管理故事需从 plan_id 语义迁移到 entitlement_key 语义
- `docs/user-stories/integration/sdk.md`
  - [US-TP-013, US-TP-017]: SDK 用户管理和积分发放故事需适配 entitlement_key
- `docs/user-stories/billing/points-admin.md`
  - [US-PO-001 ~ US-PO-008]: 积分管理故事需从 plan-based 迁移到 entitlement-based

### 1.3 已删除的现有故事

- Product 编目管理故事 [US-PR-001 ~ US-PR-006] 已删除；本地 Product/Plan CRUD 不再提供

### 1.4 优先级汇总

| 优先级 | 数量 | 关键故事 |
|--------|------|----------|
| P0 | 5 | Entitlement 映射查看、Webhook 映射、积分策略、SDK 查询、订阅投影 |
| P1 | 1 | Provider 产品同步 |
| P2 | 0 | - |

---

## 2. 范围界定

### 2.1 包含功能

- 移除本地 Product CRUD（管理页面、接口、数据表）
- 移除本地 SubscriptionPlan CRUD（管理页面、接口、数据表）
- 移除关联表（subscription_plan_payment_provider、client_app_subscription_plan、points_plan_configs）
- 引入 Provider-to-Entitlement 映射（allowlist/read model 和积分策略同步缓存）
- 引入基于 entitlement_key 的订阅投影模型
- 引入基于 entitlement_key 的积分策略查询和应用
- 统一 Provider Metadata 契约（herald_* 前缀 key）
- Provider 产品/价格同步机制（全量手动同步 + webhook 增量同步）
- SDK 订阅查询返回 entitlement_key
- Admin UI：移除 Product/Plan 管理页面，新增 Provider 同步状态只读视图
- Subscription 投影字段替换：plan_id/tier/billing_period → entitlement_key/external_price_id/provider_metadata

### 2.2 不包含功能 (Out of Scope)

- **支付方 API 实现细节**：Stripe/Creem/WeChat/Shopify 的具体 API 调用逻辑属于技术设计
- **数据库 schema 设计**：具体表结构和迁移方案属于技术设计
- **数据迁移脚本**：项目未上线，无生产数据，不需要数据迁移
- **各支付方集成细节**：Stripe、Creem、WeChat、Shopify 各自的 metadata 承载方式和 webhook 契约由各自 PRD 和技术设计覆盖
- **Checkout/Purchase 流程重设计**：统一购买路径的调整由 `docs/prd/billing/points-package-one-time-payment.md` 覆盖
- **Shopify 独立集成适配**：Shopify webhook 对 plan_id 的深度依赖需要单独的 provider contract 设计

### 2.3 依赖项

- **支付方已配置**：Provider 产品同步需要 Realm 已配置至少一个支付方
- **Provider Metadata 支持**：Stripe Product/Price/Checkout/Subscription 支持 metadata；Creem checkout metadata 在 webhook 中返回
- **Provider API 支持**：Stripe Product/Price/Checkout/Webhook API 与 Creem Product/Checkout/Webhook API 能提供 Herald 去掉本地可编辑 Product/Plan 目录所需的外部 ID、产品展示信息、价格信息、订阅状态和 checkout metadata
- **项目未上线**：无生产数据和历史用户，可直接替换所有旧表和旧代码

---

## 3. 需求概述

### 3.1 功能描述

Herald 当前维护本地 Product、SubscriptionPlan、Subscription 三层目录模型，与 Stripe/Creem/WeChat/Shopify 等支付方的 Product/Price/Subscription 概念重复，导致数据冗余和同步负担。

本次改造将商业目录和订阅生命周期交给支付方，Herald 仅保留授权、webhook 幂等、租户隔离、SDK 查询和积分发放所需的最小数据：

- **支付方拥有**：Product、Price、Checkout、Customer billing、Subscription lifecycle、Invoice/payment
- **Herald 保留**：Realm/Client App 边界、User binding、Access entitlement projection、Webhook 幂等、积分策略和账本、本地读模型

### 3.2 关键特性

- **无本地商业目录**：Herald 不再维护 Product/Plan 作为可编辑的本地目录
- **Entitlement-based 访问控制**：用 entitlement_key 替代 plan_id 作为订阅访问的核心标识
- **Provider Metadata + 本地映射驱动**：通过统一的 herald_* metadata 契约和 Herald 本地 provider-to-entitlement allowlist 实现支付方到 Herald 的映射
- **积分策略解耦**：积分策略从 plan_id 解耦到 entitlement_key，source of truth 在 Herald 本地映射/策略配置；Stripe metadata 只作为可选导入来源
- **本地投影 + 同步缓存**：Herald 维护订阅投影和 provider 映射缓存，SDK 查询不依赖实时 provider API

### 3.3 支付平台 API 能力依据

技术预研已确认 Stripe 和 Creem 均提供足够的 API 能力支撑 Herald 去掉本地可编辑 Product/Plan 目录。两家平台在以下六个维度均满足 Herald 需求：列出 provider 侧产品、列出或读取价格与计费周期、创建托管 checkout 页、将 checkout 绑定到 Herald 租户/用户/entitlement、订阅生命周期投影、产品详情缺失时补偿查询。

关键设计约束：
- Stripe webhook 不作为完整产品展示数据源；它主要提供外部 ID、订阅状态和 metadata，产品名称/价格/计费周期通过 Product/Price API 或 provider-sourced cache 补齐
- Creem webhook 示例包含 `object.product`，实现可用于增量发现，但仍必须允许字段缺失并 fallback 到 Product API 或本地映射
- 各 API 端点详情和字段清单见技术预研报告 §3.1 Provider API 能力矩阵

---

## 4. 业务规则与状态

### 4.1 业务规则

**Provider Ownership 边界**：
- 支付方是商业目录的 source of truth：Product、Price、Checkout、Subscription lifecycle
- Herald 只维护订阅投影（provider 订阅状态的本地只读副本）
- Herald 不提供本地 Product/Plan 的创建、编辑或删除入口

**Metadata 契约**：
- 所有 Herald 使用的 metadata key 使用 `herald_` 前缀，统一命名避免混用
- 必填 metadata：`herald_realm_id`、`herald_client_app_id`、`herald_user_id`、`herald_entitlement_key`
- 计费类型标识：`herald_billing_kind`，值为 `subscription` 或 `points_package`
- Stripe 分层策略：稳定映射放 Product/Price metadata，请求特定信息（user、client app）放 Checkout Session/Subscription metadata
- Creem：metadata 写入 checkout 请求，后续 webhook 返回该 metadata
- Checkout 创建时验证必填 metadata，缺失时拒绝创建

**Entitlement 映射规则**：
- Provider-to-Entitlement 映射是 Herald 本地的 allowlist 和只读缓存，不是本地商业目录
- 映射数据以 Herald 本地配置为准；Stripe Product/Price metadata 可作为导入入口，Creem 需要在 Herald 中配置 entitlement 和积分策略
- 映射承载的信息包括：provider、external_product_id、external_price_id（Creem 不适用）、entitlement_key、积分策略字段、provider_product_info、synced_at
- 管理员可以查看映射列表、触发同步、配置 entitlement/积分策略、启用/禁用单个映射
- 禁用映射后，匹配该映射的 webhook 订阅事件仍更新订阅投影，但不触发积分策略的发放或回收；管理员重新启用后恢复积分策略执行
- 映射同步失败不应静默降级为默认策略，应 fail loud 并记录诊断

**订阅投影规则**：
- Subscription 是支付方订阅状态的本地投影，不是 Herald 拥有的订阅
- 订阅投影字段：realm_id、client_app_id、user_id、payment_provider、external IDs、entitlement_key、status、period 信息、provider_metadata、synced_at
- 不再维护：plan_id、本地 tier、本地 billing_period（如仍需可从 entitlement_key 或 provider_metadata 派生）
- SDK 和授权查询读取本地投影，不依赖实时 provider API

**积分策略规则**：
- 积分策略的 source of truth 是 Herald 本地 provider-to-entitlement mapping 或 entitlement policy
- Herald 可从 Stripe Product/Price metadata 导入积分策略初值；Creem Product 无 metadata，必须在 Herald 中配置
- 积分策略按 entitlement_key 查询，覆盖：首次订阅发放、续费发放、取消回收、退款回收、升级/降级处理
- 管理员可在 Herald 中查看和修改积分策略；修改的是 Herald 业务规则，不回写 provider 商业目录

**Provider 同步规则**：
- 全量同步：管理员手动触发，调用支付方 API 读取所有 Product/Price 信息并更新 provider-sourced cache；Stripe 可同时导入 metadata
- 增量同步：webhook 事件触发，从 webhook payload 中提取 metadata、外部 ID 和可用产品信息更新订阅投影或缓存
- 同步失败时本地缓存继续服务，但记录失败诊断
- 管理员可查看同步状态（最后同步时间、同步来源、同步结果）

**Webhook 处理规则**：
- Webhook 通过 metadata 提取 herald_entitlement_key 等映射信息
- Entitlement 解析 fallback 链：webhook metadata 中的 herald_entitlement_key → 本地 mapping（按 provider + external_product_id 查询）→ fail loud
- 用户绑定优先使用 Subscription metadata，fallback 到本地 mapping；需要展示详情时再读取 Price/Product API 或 provider-sourced cache
- Metadata 缺失 entitlement_key 时 fail loud，记录诊断，不静默跳过
- 保持现有幂等性机制，event_id 去重不依赖 plan_id

**访问控制**：

| 操作 | 需要权限 | 说明 |
|------|---------|------|
| 查看 Entitlement 映射 | `billing.view` | Realm Admin |
| 触发 Provider 同步 | `billing.manage` | Realm Admin |
| 禁用/启用 Entitlement 映射 | `billing.manage` | Realm Admin |
| 查看订阅投影 | `billing.view` | Realm Admin |
| SDK 查询订阅状态 | 认证 + SDK 凭证 | Third-Party App |

**数据安全**：
- Provider metadata 中的 herald_realm_id 等信息需与请求上下文校验一致
- 跨 Realm 的 metadata 映射必须被拒绝
- Webhook 签名验证保持现有机制

### 4.2 关键状态与异常

- **Metadata 缺失**：webhook 中缺少 herald_entitlement_key → 记录错误诊断，订阅投影更新失败，对管理员可见
- **同步失败**：provider API 不可用 → 本地缓存继续服务，记录失败诊断，管理员可重试
- **映射不存在**：webhook 中的 entitlement_key 在本地无映射 → 视为未配置，记录诊断
- **Checkout metadata 验证失败**：创建 checkout 时缺少必填 metadata → 拒绝创建并提示

---

## 5. 功能需求

### 5.1 核心需求

- 系统不再暴露本地 Product 和 SubscriptionPlan 的创建、编辑、删除功能
- 系统通过 provider metadata 和本地 provider-to-entitlement mapping 将支付方订阅映射到 Herald 订阅投影，使用 entitlement_key 替代 plan_id
- 系统在 Herald 本地维护 entitlement-based 积分策略；Stripe Product/Price metadata 可作为导入来源，按 entitlement_key 查询和应用
- 系统通过 entitlement_key 处理积分的发放、续费、取消、退款、升级/降级
- SDK 订阅查询返回 entitlement_key 替代 plan_id，读取本地投影保持快速响应
- 管理员可查看 provider entitlement 映射列表、触发产品同步、查看同步状态
- 管理员可查看订阅投影列表（显示 entitlement_key、provider、status、synced_at）

### 5.2 验收目标

- 本地 Product 和 SubscriptionPlan 的 CRUD 接口和管理页面已移除
- Webhook 通过 herald_* metadata 正确映射外部订阅到 Herald 订阅投影，无需本地 Product/Plan
- SDK 订阅查询返回 entitlement_key，查询性能不依赖实时 provider API
- 积分发放、续费、取消、退款、过期按 entitlement_key 正确执行，保持幂等和可审计
- Metadata 缺失或映射不存在时 fail loud，错误对管理员可见
- 本地 Product/Plan 相关的所有数据表（含关联表和积分套餐配置表）已移除

---

## 6. API 相关约束

**适用性**: 适用

- 移除能力：Product CRUD、SubscriptionPlan CRUD、Plan Payment Provider 映射管理、Plan 分配到 Client App
- 新增能力：Provider 产品同步触发、Entitlement 映射查看、映射启用/禁用
- 修改能力：SDK 订阅查询返回 entitlement_key、Webhook 解析 herald_* metadata
- 访问控制：所有接口遵守 realm 隔离和 billing 权限
- 兼容性：项目未上线，无向后兼容要求，plan_id 可一步替换为 entitlement_key
- 接口设计：具体端点、请求响应结构和状态码由技术设计文档定义

---

## 7. 前端/交互约束

**适用性**: 适用

**移除的页面/组件**：
- Product 管理页面、Product 创建/编辑表单、Product 删除确认
- Plan 创建/编辑表单、Plan 分配对话框、Plan 支付平台映射管理
- Billing 页面中的 Products 和 Plans 导航入口

**新增的视图**：
- Provider Entitlement Mappings 视图：只读列表展示从支付方同步的产品/价格映射到 entitlement 的情况，包括积分策略同步状态
- Provider Sync 操作：手动触发全量同步的按钮和同步结果反馈
- Subscription 投影列表：显示 entitlement_key、provider、status、synced_at

**修改的视图**：
- Subscription 管理视图：显示 entitlement_key 替代 plan 名称和 tier
- Subscription 变更历史：显示 entitlement 变更替代 plan 变更
- 用户订阅详情：显示 entitlement_key 替代 plan 信息

**状态反馈**：
- 同步完成："X products and Y prices synced successfully"
- 同步失败："Failed to sync provider products: [reason]. Local cache still serving."
- Metadata 缺失："Webhook processing failed: missing herald_entitlement_key"
- 映射不存在："No mapping found for entitlement 'xxx'. Please sync provider products."

---

## 8. 已确认决策

- **Provider 拥有商业目录**：支付方是 Product、Price、Subscription lifecycle 的 source of truth，Herald 不维护独立的本地商业目录
- **Herald 保留最小数据**：订阅投影、webhook 事件记录、provider-to-entitlement 映射 allowlist、entitlement-based 积分策略配置、provider-sourced 产品信息缓存
- **积分策略归属**：Herald 本地 mapping/entitlement policy 是积分策略 source of truth；provider metadata 只作为可选导入来源
- **无生产数据迁移**：项目未上线，直接替换所有旧表和旧代码，无需兼容旧路径
- **Metadata 统一契约**：使用 `herald_*` 前缀统一 metadata key，不再混用 camelCase 和 snake_case
- **Entitlement 映射表保留**：采用 safer path，保留 provider-to-entitlement 映射作为 allowlist 和积分策略同步缓存，不纯粹依赖 provider metadata 运行时解析

---

## 9. 参考资料

- 用户故事：`docs/user-stories/billing/entitlement-mapping.md`
- 需求来源：Product and Subscription Local Model Reduction 原始需求
- 技术研究：Product & Subscription Model Reduction 技术预研报告
- 待更新 PRD：`docs/prd/billing/subscription.md`
- 待更新 PRD：`docs/prd/billing/points.md`
- 待更新 PRD：`docs/prd/billing/stripe-payment.md`
- 已删除本地 Product/Plan 编目 PRD
- 相关 PRD：`docs/prd/billing/shopify-pay.md`
- 相关 PRD：`docs/prd/billing/wechat-pay.md`
