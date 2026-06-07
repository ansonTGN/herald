# Points Package One-Time Payment 产品需求文档 (PRD)

**创建时间**: 2026-06-07
**优先级**: P0

---

## 1. 相关用户故事

> 详细故事与验收标准请查看 `docs/user-stories/` 中对应文档。

### 1.1 保留并更新的故事

**积分包购买**: `docs/user-stories/billing/points-package-purchase.md`
- **[US-PU-006]** 购买积分包，优先级 P0
  - 角色：Regular User
  - 摘要：用户购买积分包获得充值积分；产品来源从本地 points_packages 改为 one-time entitlement mapping
- **[US-PU-007]** 查看积分包购买记录，优先级 P1
  - 角色：Regular User
  - 摘要：查看积分包购买历史；数据源从本地 purchase 表改为支付尝试和积分账本投影
- **[US-PU-008]** 理解积分包与订阅购买的区别，优先级 P1
  - 角色：Regular User
  - 摘要：不变，一次性购买 vs 订阅购买的对比说明

**Entitlement 映射（直接引用）**: `docs/user-stories/billing/entitlement-mapping.md`
- **[US-EM-001]** 查看 Provider Entitlement 映射 (P0)：管理员查看映射列表（含 one-time 映射）
- **[US-EM-004]** 基于 Entitlement 应用积分策略 (P0)：系统基于 entitlement_key 应用积分策略（含 one-time 发放）

**Payment Attempt（直接引用）**: `docs/user-stories/billing/payment-attempt.md`
- **[US-PA-001]** 创建支付尝试 (P0)：支持订阅和 one-time 购买
- **[US-PA-002]** 查询支付尝试状态 (P0)
- **[US-PA-003]** 处理支付成功后的履约 (P0)：含 one-time 积分发放
- **[US-PA-004]** 关闭过期的支付尝试 (P1)

### 1.2 被取代的故事

以下用户故事因本地积分包目录移除而不再适用，对应文档已删除：

- US-PP-001~005: 积分包 CRUD（创建、编辑、配置映射、查看列表、删除）
- US-PP-006, US-PP-016~018: 促销积分包（创建、编辑、用户查看、自动过期）

这些能力由支付平台产品管理 + Entitlement 映射取代，与 product_reduce PRD 中移除 Product/Plan 的方向一致。

### 1.3 优先级汇总

| 优先级 | 数量 | 关键故事 |
|--------|------|----------|
| P0 | 4 | US-PU-006: 购买积分包, US-PA-001~003: 支付尝试与履约 |
| P1 | 3 | US-PU-007: 购买记录, US-PU-008: 购买方式对比, US-PA-004: 过期关闭 |

---

## 2. 范围界定

### 2.1 包含功能

- 移除积分包本地产品目录及相关管理接口和管理页面
- 移除积分包支付平台映射能力
- 移除积分包购买记录查询能力
- 购买目标收敛为 entitlement_mapping，通过 mapping 的 billing_type 决定履约类型
- Stripe Checkout 支持一次性支付模式，与订阅支付模式正确分流
- Creem Checkout 支持 one-time 产品支付
- Webhook 处理区分 one-time 和 recurring：one-time 直接发放 topup_credit，不创建 subscription
- 用户购买页面改为从 one-time entitlement mapping 列表选择产品
- 购买历史查询基于支付尝试记录和积分交易记录
- Provider 产品同步支持 Stripe one-time price 和 Creem one-time 产品

### 2.2 不包含功能 (Out of Scope)

- 本地促销逻辑（原价/折扣价/促销时段）→ 由支付平台优惠券和折扣码管理
- 划线价本地展示能力 → 委托给支付平台
- 折扣码/优惠码本地系统
- 订阅购买流程的变更（recurring 流程不变）
- Shopify 支付流程变更（Shopify 保持 webhook-driven 模式）
- 本地 Product/Plan 目录相关变更（已在 product_reduce PRD 完成）

### 2.3 依赖项

- **Entitlement 映射系统**：one-time 购买依赖 provider_entitlement_mappings 和 billing_type 字段
- **Provider 产品同步**：依赖 Stripe/Creem 产品同步能力（US-EM-002）
- **PaymentAttempt 模型**：支付尝试需支持 entitlement_mapping 目标类型
- **积分系统**：one-time 购买发放 topup_credit，复用现有积分账本和交易记录
- **产品目录移除**：本次变更与 product_reduce PRD 的架构方向一致，属于同一迁移的延续

---

## 3. 需求概述

### 3.1 功能描述

Product/SubscriptionPlan 本地目录已通过 product_reduce PRD 移除，订阅产品完全由 Stripe/Creem 支付平台定义，Herald 通过 EntitlementMapping 映射外部产品到 entitlement_key。

当前 points_packages 表仍是 Herald 本地定义的产品目录残留（名称、积分数量、价格、促销信息），与已移除的 Product/SubscriptionPlan 属于同一类"本地商业目录"。本次变更将 one-time 积分产品目录也交给支付平台，统一用 EntitlementMapping(billing_type=one_time) 替代本地 points_packages，与 product_reduce 保持一致的架构方向。

迁移后的职责划分：
- **支付平台拥有**：one-time 产品定义、价格、促销策略
- **Herald 保留**：entitlement 映射、积分策略配置（发放数量、有效期）、支付尝试跟踪、积分账本

### 3.2 关键特性

- **无本地积分包目录**：Herald 不再维护 points_packages 作为可编辑的本地产品目录
- **Provider 产品驱动**：one-time 产品由 Stripe/Creem 定义，Herald 通过 entitlement mapping 映射
- **统一购买对象**：购买目标收敛为 entitlement_mapping，通过 billing_type 决定履约类型
- **购买体验不变**：用户选择产品 → 选择支付方式 → 完成支付 → 获得积分的核心流程不变
- **积分策略本地配置**：每次购买发放的积分数量由 Herald mapping 的 points_per_period 配置

---

## 4. 业务规则与状态

### 4.1 业务规则

**产品目录归属**：
- one-time 产品定义、价格、促销策略由支付平台管理
- Herald 通过 EntitlementMapping 缓存产品展示信息（provider_product_info）
- 积分发放数量由 Herald 本地配置（points_per_period）
- 积分有效期由 Herald 本地配置（validity_days，若为空则永久有效）

**购买对象统一**：
- 购买目标统一为 entitlement_mapping，通过 mapping 的 billing_type 决定履约
- billing_type=one_time → 发放 topup_credit，不创建 subscription
- billing_type=recurring → 创建/更新 subscription，积分由后续 webhook 事件触发

**One-time 购买规则**：
- 购买成功后发放 topup_credit，不创建 subscription 记录
- 发放积分数量从 mapping 的 points_per_period 读取
- 积分有效期从 mapping 的 validity_days 读取
- grant_on_subscribe 字段对 one-time mapping 不适用，购买成功默认发放

**Webhook 分发规则**：
- Stripe：checkout.session.completed 按 mode 分发
  - mode=payment（one-time）：完成支付尝试，发放 topup_credit
  - mode=subscription（recurring）：走现有 subscription 创建/同步逻辑
- Creem：checkout.completed 按 metadata 或 mapping 的 billing type 分发
  - one-time：完成支付尝试，发放 topup_credit
  - recurring：等待 subscription.paid 事件

**购买历史**：
- 购买记录基于支付尝试记录和积分交易记录查询
- 展示产品信息（名称、积分数量、价格）、支付平台、支付时间
- 支持按时间范围、支付平台筛选

**用户购买页**：
- 列出 enabled 且 billing_type=one_time 的 entitlement mappings
- 产品信息（名称、价格、描述）从 mapping 的 provider_product_info 读取
- 支付平台选择基于 mapping 关联的 provider
- 没有启用的 one-time mapping 时不显示购买入口

**促销定价**：
- 促销策略由支付平台管理（Stripe Coupons/Promotion Codes、Creem Discount Codes）
- Herald 不在本地实现促销逻辑
- 前端可通过 provider_product_info 展示折扣信息（若 provider 返回）

### 4.2 关键状态与异常

- **Mapping 未启用**：disabled 的 one-time mapping 不出现在用户购买页
- **Provider 产品信息缺失**：mapping 未同步或 provider_product_info 为空时，用户购买页不展示该产品
- **Webhook 重复事件**：同一支付成功事件可能重复到达，需幂等处理防止重复发放积分
- **购买时 mapping 已被禁用**：支付尝试已创建但 mapping 被禁用 → 仍完成履约，记录警告
- **积分发放失败**：支付成功但积分发放失败 → 系统记录失败并告警，保持支付尝试状态正确
- **Provider billing type 标准化**：Creem 可能使用 `onetime`，Herald domain 使用 `one_time`，需在同步时标准化

---

## 5. 功能需求

### 5.1 核心需求

- 移除 points_packages 相关的管理接口和管理页面
- 购买目标收敛为 entitlement_mapping，支持通过 billing_type 区分履约类型
- Stripe Checkout 支持一次性支付模式，与订阅模式正确分流
- Creem Checkout 支持 one-time 产品
- Webhook 正确区分 one-time 和 recurring 履约路径
- 用户购买页列出 enabled 且 billing_type=one_time 的 entitlement mappings
- 购买历史基于支付尝试记录和积分交易记录查询
- Provider 产品同步支持 one-time 产品的识别和标准化
- 前端删除积分包管理页面，更新购买页面数据源

### 5.2 验收目标

- 用户能通过 Stripe one-time 产品购买积分并正确获得 topup_credit
- 用户能通过 Creem one-time 产品购买积分并正确获得 topup_credit
- 用户能通过 WeChat 购买 one-time 积分产品
- one-time 购买不创建 subscription 记录
- 订阅购买流程（recurring）不受 one-time 分支影响
- Webhook 正确区分 one-time 和 recurring，one-time 发放积分，recurring 创建订阅
- 重复 webhook 事件不重复发放积分
- 管理员不再能通过 Herald 管理 points_packages（产品管理在支付平台）
- 管理员能查看和配置 one-time entitlement mapping 的积分策略
- 购买历史可查询，展示产品名称、积分数、价格、支付平台、时间
- 积分包管理页面已从前端移除
- 用户购买页正确展示 one-time entitlement mapping 产品

---

## 6. API 相关约束

**适用性**: 适用

- 移除能力：points_packages CRUD、points_package_payment_providers 映射管理、points_package_purchases 历史查询
- 新增/修改能力：支付尝试创建支持 entitlement_mapping 目标类型，购买历史查询支持 entitlement mapping 维度
- 访问控制：购买接口已登录用户可访问；管理类接口需对应 RBAC 权限（billing.view/billing.manage）
- Realm 隔离：所有接口遵守 realm 数据边界
- 兼容性：项目未上线，无向后兼容要求，旧接口直接移除
- 外部 API：旧的 points-packages 外部接口移除，替换为返回 one-time entitlement mappings 的新接口

---

## 7. 前端/交互约束

**适用性**: 适用

- **移除的页面**：积分包管理页面（CRUD、促销包管理、支付平台映射配置），侧边栏管理入口
- **更新的页面**：
  - 用户购买页面：数据源从 points_packages 改为 one-time entitlement mappings
  - 购买历史页面：数据源改为基于支付尝试和积分交易的投影
- **交互不变**：用户选择产品 → 选择支付方式 → 创建支付 → 轮询状态 → 查看结果的核心流程不变
- **Points 入口可见性**：个人中心 Points 菜单按是否存在 enabled 的 one-time mapping 显示
- **状态反馈**：沿用现有 Pending/Succeeded/Failed/Expired/Cancelled 状态反馈

---

## 8. 已确认决策

- 移除 points_packages 本地产品目录，与 product_reduce PRD 架构方向保持一致
- 购买目标统一为 entitlement_mapping，不再保留 points_package 目标类型
- 支付平台是 one-time 产品的 source of truth（产品定义、价格、促销）
- Herald 保留积分策略配置（发放数量、有效期）
- 促销策略委托给支付平台（Stripe Coupons/Promotion Codes、Creem Discount Codes）
- 项目未上线，无向后兼容要求，直接移除旧表、旧接口、旧页面
- grant_on_subscribe 对 one-time mapping 不适用，购买成功默认发放
- one-time 购买发放 topup_credit，不创建 subscription
- WeChat one-time 购买继续支持（微信支付天然为一次性支付）
- US-PP-001~018 用户故事被支付平台产品管理 + Entitlement 映射取代

---

## 9. 参考资料

- 技术研究：`.ai/tech-research/points-package-one-time-payment.md`
- 用户故事：`docs/user-stories/billing/points-package-purchase.md`（US-PU-006~008）
- 用户故事：US-PP-001~018 已删除（被支付平台产品管理 + Entitlement 映射取代）
- 用户故事：`docs/user-stories/billing/payment-attempt.md`（US-PA-001~004）
- 用户故事：`docs/user-stories/billing/entitlement-mapping.md`（US-EM-001~006）
- 依赖 PRD：`docs/prd/billing/product_reduce.md`
- 依赖 PRD：`docs/prd/billing/points.md`
