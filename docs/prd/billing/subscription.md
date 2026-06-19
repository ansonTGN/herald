# 订阅计费产品需求文档 (PRD)

**创建时间**: 2025-01-30
**优先级**: P0

---

## 1. 相关用户故事

> 详细故事与验收标准请查看 `docs/user-stories/` 中对应文档。

### 1.1 故事引用

- `[US-BI-001]` 管理 Entitlement 映射，优先级 P0，来源 `docs/user-stories/billing/subscription.md`
  - 角色：Realm Admin
  - 摘要：维护支付方商品到 Herald `entitlement_key` 的映射和积分策略

- `[US-BI-002]` 编辑 Entitlement 映射，优先级 P0，来源 `docs/user-stories/billing/subscription.md`
  - 角色：Realm Admin
  - 摘要：更新 `entitlement_key`、启用状态、积分发放策略和 provider 商品缓存

- `[US-BI-003]` 同步支付方商品，优先级 P0，来源 `docs/user-stories/billing/subscription.md`
  - 角色：Realm Admin
  - 摘要：从 Stripe/Creem 等支付方同步商品并生成本地 entitlement 映射

- `[US-BI-004]` 禁用 Entitlement 映射，优先级 P0，来源 `docs/user-stories/billing/subscription.md`
  - 角色：Realm Admin
  - 摘要：禁用映射后不再允许新购买，也不触发积分发放/回收

- `[US-BI-005]` 发起订阅 Checkout，优先级 P0，来源 `docs/user-stories/billing/subscription.md`
  - 角色：Realm Admin
  - 摘要：通过 `entitlement_key + payment_provider` 创建支付方 checkout

- `[US-BI-006]` 查看订阅列表，优先级 P0，来源 `docs/user-stories/billing/subscription.md`
  - 角色：Realm Admin
  - 摘要：查看订阅列表，了解订阅情况

- `[US-BI-007]` 第三方应用查询套餐状态，优先级 P0，来源 `docs/user-stories/billing/subscription.md`
  - 角色：Third-party App
  - 摘要：通过 SDK 查询用户的订阅和套餐状态，以及可用的支付平台选项

- `[US-BI-008]` 查看订阅变更历史，优先级 P1，来源 `docs/user-stories/billing/subscription.md`
  - 角色：Realm Admin
  - 摘要：查看所有用户的订阅变更历史，监控和管理订阅情况

- `[US-BI-009]` 查看自己的订阅变更历史，优先级 P1，来源 `docs/user-stories/billing/subscription.md`
  - 角色：Regular User
  - 摘要：查看我的订阅变更历史，了解订阅的变更轨迹

- **Realm Admin 订阅套餐管理**，优先级 P0，来源 `docs/user-stories/core/realm-admin.md`
  - 角色：Realm Admin
  - 摘要：管理订阅套餐，为用户提供不同的订阅选项

### 1.2 优先级汇总

| 优先级 | 数量 | 关键故事 |
|--------|------|----------|
| P0 | 8 | 创建/编辑/删除订阅套餐、配置支付平台映射、分配套餐到 Client App、查看订阅列表、第三方应用查询套餐状态（SDK） |
| P1 | 2 | 查看订阅变更历史（Realm Admin）、查看自己的订阅变更历史（Regular User） |
| P2 | 0 | - |

---

## 2. 范围界定

### 2.1 包含功能

- 支持多种支付平台：Creem（模拟支付平台，已实现）、Stripe（已实现，详见 `docs/prd/billing/stripe-payment.md`）、微信支付（已实现，详见 `docs/prd/billing/wechat-pay.md`）；支付宝待实现
- Provider Entitlement 映射管理（查看、配置积分策略、启用/禁用）
- 支付方商品同步与 provider-sourced cache
- 前端 Entitlement 映射管理页面
- 查询单个订阅的变更时间线
- 按用户、套餐、时间等维度查询订阅历史（Realm Admin）
- 显示变更类型（创建、升级、降级、取消、续费等）
- 显示变更前后的状态对比
- 前端历史记录页面（全局查询和单订阅详情）
- 权限控制（Realm Admin 可查看所有历史，Regular User 只能查看自己的）
- 支付平台 Webhook 集成与事件处理
- 订阅升级/降级（按比例计费）
- 积分充值与订阅事件联动

### 2.2 不包含功能 (Out of Scope)

- **订阅自动续费**：由支付平台处理，Herald 不负责
- **套餐的功能（features）和配额（quotas）管理**：采用简化模型，由第三方应用自行管理
- **支付方式管理**：不在首版范围
- **计费统计和报表**：不在首版范围
- **通知系统**：邮件/短信通知不在首版范围
- **支付事件历史**：支付事件由支付平台处理，Herald 不负责记录
- **导出历史记录**：属于 P2 功能，暂不实现
- **历史记录审计日志**：可选扩展功能，暂不实现
- **历史记录统计分析**：属于计费统计报表功能，单独规划

**相关边界说明**：
- 退款功能：支付平台处理金额退款，Herald 处理积分回收（详见 `docs/prd/billing/points.md`）
- 订阅管理：即使前后端订阅管理功能未完整实现，积分系统仍需定义订阅变更的积分处理规则（详见 `docs/prd/billing/points.md`）

### 2.3 依赖项

- **Realm 系统** — Billing 功能属于 Realm 级别
- **Client App 系统** — 套餐分配到 Client App
- **权限管理系统** — Realm Admin 权限检查
- **支付平台集成** — 当前已集成 Creem（模拟支付平台）、Stripe（`docs/prd/billing/stripe-payment.md`）、微信支付（`docs/prd/billing/wechat-pay.md`）；支付宝待实现

---

## 3. 需求概述

### 3.1 功能描述

Billing（订阅计费）是 Herald 系统为 Realm 提供的灵活订阅管理和计费方案功能。涵盖支付平台配置、订阅套餐管理、套餐分配、订阅升级/降级、订阅变更历史等功能。

系统采用 provider-sourced entitlement 模型：Herald 不维护本地 Product/Plan 目录，不创建、编辑或删除本地套餐。支付平台商品和价格是商业目录来源，Herald 只维护 `provider_entitlement_mappings`，用 `entitlement_key` 表示第三方应用可识别的订阅权益。

**编目边界**：Herald 不维护本地 Product/Plan 目录。支付方是商业目录的 source of truth，Herald 只维护 `provider_entitlement_mappings` 用 `entitlement_key` 表示第三方应用可识别的订阅权益。

### 3.2 关键特性

- 支持多种支付平台（Creem、Stripe、微信支付；支付宝待实现）
- Provider entitlement 映射管理
- 通过 `entitlement_key` 统一表示订阅权益
- 支付方商品同步与 provider-sourced cache
- 订阅升级/降级（按比例计费）
- Webhook 集成和事件处理
- 完整的订阅变更历史记录
- 订阅事件与积分系统联动

---

## 4. 业务规则与状态

### 4.1 业务规则

**Entitlement 映射规则**：

- `entitlement_key` 是 Herald 内部和第三方应用识别订阅权益的稳定业务标识
- `provider_entitlement_mappings` 记录 provider、external_product_id、external_price_id、entitlement_key、billing_type、billing_period 和 provider_product_info
- provider 商品/价格展示信息来自支付方同步缓存，不由 Herald 本地手工维护
- `features` 和 `quotas` 由第三方应用自行管理，Herald 不存储这些信息
- 更新价格以支付方为准；Herald 通过同步刷新 provider_product_info
- 更新 checkout_url 立即生效，所有新订阅用户使用新 URL
- Provider-to-Entitlement 映射是 Herald 本地的 allowlist 和只读缓存，不是本地商业目录
- 映射数据以 Herald 本地配置为准；Stripe Product/Price metadata 可作为导入入口，Creem 需要在 Herald 中配置 entitlement 和积分策略
- 映射承载的信息包括：provider、external_product_id、external_price_id（Creem 不适用）、entitlement_key、积分策略字段、provider_product_info、synced_at
- 禁用映射后，匹配该映射的 webhook 订阅事件仍更新订阅投影，但不触发积分策略的发放或回收；管理员重新启用后恢复积分策略执行
- 映射同步失败不应静默降级为默认策略，应 fail loud 并记录诊断

**删除规则**：
- 无法删除有活跃订阅的套餐
- 可以删除无订阅的套餐（包括已取消订阅的套餐）
- 删除套餐时级联删除所有支付平台映射（数据库层面通过外键级联或应用层逐条删除实现）

**支付平台映射规则**：
- 同一 Plan 不能重复配置同一个支付平台
- 删除支付平台映射前需检查是否有活跃订阅
  > 存在活跃订阅时拒绝删除支付平台映射。
- 禁用支付平台映射不影响已订阅用户，但新用户无法使用该支付平台
- 启用支付平台映射时，如果该平台未在 Realm 层配置，应提示用户先配置支付平台

**套餐分配规则**：
- Realm Admin 为特定 Client App 分配可用套餐，最终用户只能看到已分配的套餐
- 移除分配不影响已订阅用户，但新用户无法看到该套餐

**订阅变更规则**：
- 升级订阅采用按比例计费（Proration）
- 降级在下个计费周期生效，当前周期保持原套餐
- 取消在当前计费周期结束生效，用户可继续使用直到周期结束
- 取消后不自动删除数据，数据保留期由第三方应用决定

**Webhook 处理规则**：
- 系统采用 realm 隔离的 webhook 端点架构，每个 realm 使用独立的 webhook URL
- Webhook 签名验证失败时拒绝处理请求
- 支持事件幂等性处理，防止重复处理
- 订阅状态转换需通过合法性验证

**Provider Metadata 契约**：
- 所有 Herald 使用的 metadata key 使用 `herald_` 前缀，统一命名避免混用
- 必填 metadata：`herald_realm_id`、`herald_client_app_id`、`herald_user_id`、`herald_entitlement_key`
- 计费类型标识：`herald_billing_kind`，值为 `subscription` 或 `points_package`
- Stripe 分层策略：稳定映射放 Product/Price metadata，请求特定信息（user、client app）放 Checkout Session/Subscription metadata
- Creem：metadata 写入 checkout 请求，后续 webhook 返回该 metadata
- Checkout 创建时验证必填 metadata，缺失时拒绝创建

**Webhook Entitlement 解析链**：
- Webhook 通过 metadata 提取 herald_entitlement_key 等映射信息
- 解析 fallback 链：webhook metadata 中的 herald_entitlement_key → 本地 mapping（按 provider + external_product_id 查询）→ fail loud
- 用户绑定优先使用 Subscription metadata，fallback 到本地 mapping
- Metadata 缺失 entitlement_key 时 fail loud，记录诊断，不静默跳过

**Provider 同步规则**：
- 全量同步：管理员手动触发，调用支付方 API 读取所有 Product/Price 信息并更新 provider-sourced cache；Stripe 可同时导入 metadata
- 增量同步：webhook 事件触发，从 webhook payload 中提取 metadata、外部 ID 和可用产品信息更新订阅投影或缓存
- 同步失败时本地缓存继续服务，但记录失败诊断
- 管理员可查看同步状态（最后同步时间、同步来源、同步结果）

**购买对象统一**：
- 购买目标统一为 entitlement_mapping，通过 mapping 的 billing_type 决定履约
- billing_type=one_time → 发放 topup_credit，不创建 subscription
- billing_type=recurring → 创建/更新 subscription，积分由后续 webhook 事件触发

**One-time vs Recurring Webhook 分发规则**：
- Stripe：checkout.session.completed 按 mode 分发
  - mode=payment（one-time）：完成支付尝试，发放 topup_credit
  - mode=subscription（recurring）：走现有 subscription 创建/同步逻辑
- Creem：checkout.completed 按 metadata 或 mapping 的 billing type 分发
  - one-time：完成支付尝试，发放 topup_credit
  - recurring：等待 subscription.paid 事件

**One-time 购买规则**：
- 购买成功后发放 topup_credit，不创建 subscription 记录
- 发放积分数量从 mapping 的 points_per_period 读取
- 积分有效期从 mapping 的 validity_days 读取
- grant_on_subscribe 字段对 one-time mapping 不适用，购买成功默认发放
- 用户购买页列出 enabled 且 billing_type=one_time 的 entitlement mappings
- 没有启用的 one-time mapping 时不显示购买入口
- 促销策略由支付平台管理（Stripe Coupons/Promotion Codes、Creem Discount Codes），Herald 不在本地实现促销逻辑
- 购买记录基于支付尝试记录和积分交易记录查询

**积分充值联动规则**：
- 首次订阅时，根据积分套餐配置中的 `points_on_subscribe` 进行充值
- 定期续费时，根据 `renewal_enabled` 和 `points_on_renewal` 配置决定是否充值
- 充值操作与数据库事务绑定，使用乐观锁防止并发问题
- 充值失败不影响订阅状态
- 支持最大累计积分限制（`max_accumulation`）

**数据安全规则**：
- API Secret Key 和 Webhook Secret 必须加密存储
- 只在创建和更新时显示完整 Secret，查询时只显示部分掩码
- 删除支付平台前需检查是否有活跃订阅

**数据一致性规则**：
- 订阅变更时必须同步创建历史记录
- 历史记录一旦创建不可修改
- 确保变更前后的状态准确性
- 敏感信息（如支付详情）不记录在历史中

**权限规则**：

| 操作 | 需要权限 | 说明 |
|------|---------|------|
| 查看 Entitlement 映射 | `billing.view` | Realm Admin |
| 触发 Provider 同步 | `billing.manage` | Realm Admin |
| 查看/禁用映射 | `billing.manage` | Realm Admin |
| 查看订阅投影 | `billing.view` | Realm Admin |
| 管理订阅 | `billing.manage` | Realm Admin |
| 查看订阅变更历史（Realm Admin） | `billing.view` | Realm Admin |
| 查看自己的订阅变更历史 | 认证用户 | Regular User |

### 4.2 关键状态与异常

**订阅状态定义**：

- **Active** — 订阅正常生效中，用户享有完整访问权限
- **Past Due（逾期）** — 收到 `subscription.past_due` 事件触发；立即撤销访问权限；用户更新支付方式后可恢复 Active 状态；多次支付失败后转为 Expired
- **Disputed（争议中）** — 收到 `dispute.created` 事件触发；争议调查期间保持访问权限；记录争议详情（ID、金额、原因）；争议解决后根据结果转为 Active 或 Canceled
- **Scheduled Cancel（预定取消）** — 收到 `subscription.scheduled_cancel` 事件触发；用户在计费周期结束前仍可正常访问；可在周期结束前取消预定取消操作
- **Canceled** — 订阅已取消
- **Expired** — 订阅已过期；过期后自动降级为 Free tier
- **Incomplete（未完成）** — 支付需在 23 小时内完成，期间无访问权限；超时未完成则转为 Expired
- **Trialing（试用中）** — 试用期间享有完整访问权限
- **Paused（暂停）** — 订阅暂停中，无访问权限
- **Pending（待处理）** — 本地扩展状态，用于支付流程中间态

> **退款说明**：`refund.created`（Creem）/ `charge.refunded`（Stripe）事件不作为独立订阅状态。退款事件仅记录审计日志并触发积分回收（按退款类型比例撤销），不改变订阅状态。详见 `docs/prd/billing/points.md`。
> **[待完善]** 当前退款事件仅通过日志记录，未创建 `SubscriptionHistoryEvent`。应补充退款事件的历史记录创建逻辑。

**订阅变更事件类型**：

| 类型 | 说明 | 触发场景 |
|------|------|---------|
| `created` | 创建订阅 | 用户首次订阅套餐 |
| `upgraded` | 升级套餐 | 从低级套餐升级到高级套餐 |
| `downgraded` | 降级套餐 | 从高级套餐降级到低级套餐 |
| `canceled` | 取消订阅 | 用户主动取消订阅 |
| `expired` | 订阅过期 | 因未续费而过期 |
| `renewed` | 续费订阅 | 成功续费 |
| `reactivated` | 激活订阅 | 已取消的订阅重新激活 |
| `billing_period_changed` | 计费周期变更 | 从月付改为年付或反之 |
| `past_due` | 支付逾期 | 续费支付失败 |
| `disputed` | 争议中 | 客户发起拒付（chargeback） |
| `paused` | 暂停订阅 | 订阅被暂停 |

**异常场景**：
- 删除有活跃订阅的套餐：拒绝操作并提示活跃订阅数量
- 删除有活跃订阅的支付平台映射：拒绝操作并提示活跃订阅数量
- 订阅降级时当前用户数超过目标套餐限制：不允许降级
- Webhook 签名验证失败：拒绝处理请求

---

## 5. 功能需求

### 5.1 核心需求

**支付平台配置**：
- 支持添加、编辑、启用/禁用、删除支付平台配置（API Key、Secret Key、Webhook Secret）
- 当前支持 Creem（模拟支付平台）、Stripe、微信支付；支付宝待实现
- 每个 realm 使用独立的 webhook URL，实现多租户隔离

**Entitlement 映射管理**：
- 查看 Provider Entitlement 映射列表：显示 provider、external IDs、entitlement_key、积分策略、同步状态
- 触发 Provider 产品同步：手动触发全量同步，更新 provider-sourced cache
- 启用/禁用映射：禁用后不触发积分发放/回收，重新启用后恢复

**Provider 同步与缓存**：
- 全量同步：调用支付方 API 读取 Product/Price 信息并更新本地缓存
- 增量同步：webhook 事件触发，更新订阅投影或缓存
- 查看同步状态（最后同步时间、来源、结果）

**订阅生命周期管理**：
- 创建订阅：用户在第三方应用选择套餐 -> 重定向到支付页面 -> 完成支付 -> Webhook 通知 -> 创建订阅记录
- 升级订阅：按比例计费
- 降级订阅：下个计费周期生效
- 取消订阅：当前计费周期结束生效

**Webhook 事件处理**：
- 支持 Creem 的 checkout.completed、subscription.active/trialing/paid/paused/canceled/expired/update/scheduled_cancel、subscription.past_due、dispute.created、refund.created 事件
- 支持 Stripe 的 checkout.session.completed/expired/async_payment_succeeded/async_payment_failed、customer.subscription.created/updated/deleted/paused/resumed、charge.refunded、charge.dispute.created/closed、invoice.payment_succeeded/payment_failed/payment_action_required/created/finalized/paid/voided、payment_intent.succeeded/payment_failed 事件（详见 `docs/prd/billing/stripe-payment.md`）
- 签名验证、事件幂等性处理、状态转换验证
- 与积分系统联动：首次订阅充值、定期续费充值、退款积分回收

**订阅变更历史**：
- 单订阅历史：展示单个订阅从创建到当前的所有变更事件，按时间倒序排列
- 全局历史查询（Realm Admin）：支持按用户、套餐、变更类型、时间范围、订阅状态等维度筛选，支持分页和排序
- 变更记录包含：变更类型、操作者、变更详情、变更前后状态对比

### 5.2 验收目标

- Realm Admin 可完成套餐的完整生命周期管理（创建、编辑、删除、查看、分配）
- 一个 Plan 可配置多个支付平台映射，且各映射可独立启用/禁用
- Realm Admin 可查看 Realm 内所有订阅变更历史，支持多维度筛选
- Regular User 可查看自己的订阅变更历史
- 删除有活跃订阅的套餐或支付平台映射时，系统拒绝并给出明确错误提示
- Webhook 事件处理后订阅状态正确转换
- 订阅事件（首次订阅、续费）正确触发积分充值

---

## 6. API 相关约束

**适用性**: 适用

**能力边界**：
- 不提供本地 Product/Plan CRUD
- Entitlement mapping 查询、更新、禁用和 provider 产品同步由 Billing Admin API 提供
- 订阅查询：订阅状态、`entitlement_key`、支付平台、周期和订阅变更历史
- SDK 查询：第三方应用查询 client app 当前订阅状态，返回 `entitlement_key`
- Checkout 发起：显式传递 `entitlement_key + payment_provider`
- Webhook 接收：优先使用 `herald_entitlement_key`，fallback 到本地 provider mapping

**访问控制与数据边界**：
- 所有接口遵守 realm 隔离原则
- 写入类操作（创建、编辑、删除套餐/映射/分配）需要 `billing.manage` 权限
- 读取类操作需要 `billing.view` 权限或认证用户身份
- 删除套餐时级联删除所有支付平台映射配置（数据库层面通过外键级联或应用层逐条删除实现）
- 金额与积分变更必须可追溯

**兼容性要求**：
- Webhook 处理需支持回调幂等和失败补偿
- 与支付平台、积分账本、订阅系统的详细契约应下沉到技术设计或接口文档

---

## 7. 前端/交互约束

**适用性**: 适用

**导航与可见性约束**：
- 管理后台入口按权限控制：拥有 `billing.view` 的用户可看到 Billing 相关管理入口（Products、Payment Providers、Subscription Plans、Invoices、Subscription History），不因当前 Realm 尚未配置产品或套餐而隐藏入口
- 个人中心的 Subscription 入口按 Realm 能力开通状态显示：当 Realm 下存在已启用的订阅 Plan 时显示；仅存在历史订阅记录但没有已启用 Plan 时不单独作为显示入口依据
- 订阅购买按钮或 checkout 流程的可用性独立于 Subscription 入口：只有当 Plan 已配置启用的支付平台映射，且对应支付平台已在 Realm 中启用时，才允许用户发起购买；否则显示不可购买状态或禁用购买操作

**套餐创建流程**：
- 创建套餐表单不包含支付平台相关字段
- 创建成功后显示引导信息，提示用户配置支付平台映射，提供 "Add Payment Provider" 按钮跳转到配置页面

**套餐管理界面**：
- 套餐列表展示 "Payment Providers" 列，显示该套餐支持的所有支付平台；未配置支付平台的套餐显示高亮提示
- 套餐详情页面显示 "Payment Providers" 配置区域，包含所有已配置的映射及其状态
- 支持为套餐添加、编辑、删除支付平台映射；添加时从 Realm 已配置的支付平台中选择
- 禁用或删除支付平台映射时提示对已订阅用户的影响

**用户订阅流程**：
- 用户选择套餐后，展示该套餐支持的支付平台选项
- 用户选择具体的支付平台后发起 checkout 请求
- 如果套餐没有可用的支付平台，禁用订阅按钮并显示提示

**订阅历史界面**：
- Realm Admin 可查看全局订阅变更历史，支持按用户、套餐、变更类型、时间范围、订阅状态筛选
- 用户可在订阅详情中查看该订阅的完整变更时间线
- 历史记录按时间倒序排列，显示变更类型、操作者、变更详情和前后状态对比

**One-time 购买页面**：
- 用户购买页列出 enabled 且 billing_type=one_time 的 entitlement mappings
- 产品信息（名称、价格、描述）从 mapping 的 provider_product_info 读取
- 支付平台选择基于 mapping 关联的 provider
- 没有启用的 one-time mapping 时不显示购买入口
- 购买历史基于支付尝试记录和积分交易记录查询

**状态反馈**：
- 创建套餐成功后提示："Please configure payment providers for this plan"
- 删除套餐时提示："This will delete all payment provider mappings for this plan"
- 删除支付平台映射时提示活跃订阅数量："Cannot delete mapping with X active subscriptions"
- 禁用支付平台映射时提示："Existing subscriptions will continue to work, new users cannot select this provider"

---

## 8. 已确认决策

### 8.1 已确认决策

- **简化模型**：Herald 不管理权益的功能（features）和配额（quotas），由第三方应用自行管理
- **Entitlement 映射**：采用 provider 商品到 `entitlement_key` 的映射模型，不维护本地 Plan
- **Webhook 隔离**：每个 realm 使用独立的 webhook URL，realm_id 从 URL 路径提取实现多租户隔离
- **编目决策**：本地 Product/Plan 编目已废弃，当前模型以 `entitlement_key` 为准
- **积分策略归属**：Herald 本地 mapping/entitlement policy 是积分策略 source of truth；provider metadata 只作为可选导入来源
- **Entitlement 映射表保留**：保留 provider-to-entitlement 映射作为 allowlist 和积分策略同步缓存，不纯粹依赖 provider metadata 运行时解析
- **Metadata 统一契约**：使用 `herald_*` 前缀统一 metadata key
- **退款边界**：支付平台处理金额退款，Herald 处理积分回收。退款不作为独立订阅状态，`refund.created`/`charge.refunded` 事件仅记录审计日志并触发积分回收，不改变订阅状态
- **订阅过期降级**：订阅过期后状态变为 expired/canceled；具体权限降级由第三方应用根据 `entitlement_key` 和订阅状态处理
- **One-time 购买不创建 subscription**：one-time 购买发放 topup_credit，不创建 subscription 记录
- **billing_type 决定履约路径**：entitlement_mapping 的 billing_type 区分 one-time 和 recurring 购买
- **促销策略委托支付平台**：Herald 不在本地实现促销逻辑，由支付平台优惠券/折扣码管理
- **购买历史数据源**：基于支付尝试记录和积分交易记录查询，不依赖本地产品目录

---

## 9. 参考资料

- 用户故事：`docs/user-stories/billing/subscription.md`
- 相关 PRD：`docs/prd/billing/points.md`
- 相关 PRD：`docs/prd/billing/stripe-payment.md`
- 相关 PRD：`docs/prd/billing/wechat-pay.md`
- 相关 PRD：`docs/prd/core/realm-settings.md`
- 用户故事：`docs/user-stories/billing/entitlement-mapping.md`
- Realm Admin 用户故事：`docs/user-stories/core/realm-admin.md`
