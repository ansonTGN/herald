# 订阅计费产品需求文档 (PRD)

**创建时间**: 2025-01-30
**优先级**: P0

---

## 1. 相关用户故事

> 详细故事与验收标准请查看 `docs/user-stories/` 中对应文档。

### 1.1 故事引用

- `[US-BI-001]` 创建订阅套餐，优先级 P0，来源 `docs/user-stories/billing/subscription.md`
  - 角色：Realm Admin
  - 摘要：在 Product 上下文中创建订阅套餐，定义价格和计费信息

- `[US-BI-002]` 编辑订阅套餐，优先级 P0，来源 `docs/user-stories/billing/subscription.md`
  - 角色：Realm Admin
  - 摘要：在 Product 上下文中编辑订阅套餐，更新价格和描述

- `[US-BI-003]` 配置 Plan 的支付平台映射，优先级 P0，来源 `docs/user-stories/billing/subscription.md`
  - 角色：Realm Admin
  - 摘要：为 Plan 配置一个或多个支付平台映射，使该套餐可在不同支付平台上售卖

- `[US-BI-004]` 删除订阅套餐，优先级 P0，来源 `docs/user-stories/billing/subscription.md`
  - 角色：Realm Admin
  - 摘要：在 Product 上下文中删除订阅套餐，移除不再需要的套餐

- `[US-BI-005]` 分配套餐到 Client App，优先级 P0，来源 `docs/user-stories/billing/subscription.md`
  - 角色：Realm Admin
  - 摘要：将套餐分配到 Client App，控制哪些应用可以提供哪些订阅

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

- 订阅套餐管理（创建、编辑、删除、查看）
- 套餐分配到 Client App
- 支持多种支付平台（当前已支持 Creem 模拟支付平台，未来扩展 Stripe、支付宝/微信支付）
- 灵活的订阅套餐管理（月付/年付）
- 套餐基本信息管理（name、title、description、type、price、currency、checkout_url）
- Plan 多支付平台映射配置（一个 Plan 可关联多个支付平台）
- 前端套餐管理页面
- 前端套餐分配对话框
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
- **支付平台集成** — 当前已集成 Creem 模拟支付平台；Stripe/支付宝/微信支付待实现

---

## 3. 需求概述

### 3.1 功能描述

Billing（订阅计费）是 Herald 系统为 Realm 提供的灵活订阅管理和计费方案功能。涵盖支付平台配置、订阅套餐管理、套餐分配、订阅升级/降级、订阅变更历史等功能。

系统采用简化模型：Herald 不管理套餐的功能（features）和配额（quotas），由第三方应用自行管理。套餐只包含基本信息（name, title, description, type, price, currency, checkout_url）。Realm Admin 可以创建套餐并分配到 Client App，最终用户通过第三方应用进行订阅和支付。

**编目边界**：Billing 编目正在从 `Realm -> Plan` 演进为 `Realm -> Product -> Plan`；Product 的主定义以 `docs/prd/billing/product-catalog.md` 为准，本文档继续聚焦订阅、支付与 Plan 计费语义。

### 3.2 关键特性

- 支持多种支付平台（Creem、Stripe/支付宝/微信支付）
- 灵活的订阅套餐管理（月付/年付）
- Plan 多支付平台映射：一个 Plan 可关联多个支付平台的商品/价格配置，无需为每个支付平台复制 Plan
- 套餐分配到 Client App，控制哪些应用可以提供哪些订阅
- 订阅升级/降级（按比例计费）
- Webhook 集成和事件处理
- 完整的订阅变更历史记录
- 订阅事件与积分系统联动

---

## 4. 业务规则与状态

### 4.1 业务规则

**套餐管理规则**：

- Plan 表示业务套餐本身，不包含支付平台映射信息；一个 Plan 可以关联多个支付平台配置
- Plan Payment Provider 是 Plan 的下属配置对象，每个映射分别保存外部商品、价格、checkout 等接入信息
- Plan 是订阅和计费的直接承载对象，Product 是 Plan 的上层编目对象
- 套餐的 `name` 字段是唯一标识符，用于 API 调用和前端路由，创建后不可修改
- 套餐的 `title` 字段是用户友好的显示名称
- `features` 和 `quotas` 由第三方应用自行管理，Herald 不存储这些信息
- 更新价格影响新订阅用户，已订阅用户保持原价格直到续费
- 更新 checkout_url 立即生效，所有新订阅用户使用新 URL

**删除规则**：
- 无法删除有活跃订阅的套餐
- 可以删除无订阅的套餐（包括已取消订阅的套餐）
- 删除套餐时级联删除所有支付平台映射

**支付平台映射规则**：
- 同一 Plan 不能重复配置同一个支付平台
- 删除支付平台映射前需检查是否有活跃订阅
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
| 查看套餐列表 | `billing.view` | 所有已认证用户 |
| 创建套餐 | `billing.manage` | Realm Admin |
| 编辑套餐 | `billing.manage` | Realm Admin |
| 删除套餐 | `billing.manage` | Realm Admin |
| 分配套餐 | `billing.manage` | Realm Admin |
| 查看订阅统计 | `billing.view` | Realm Admin |
| 管理订阅 | `billing.manage` | Realm Admin |
| 查看订阅变更历史（Realm Admin） | `billing.view` | Realm Admin |
| 查看自己的订阅变更历史 | 认证用户 | Regular User |

### 4.2 关键状态与异常

**订阅状态定义**：

- **Active** — 订阅正常生效中，用户享有完整访问权限
- **Past Due（逾期）** — 收到 `subscription.past_due` 事件触发；立即撤销访问权限；用户更新支付方式后可恢复 Active 状态；多次支付失败后转为 Expired
- **Disputed（争议中）** — 收到 `dispute.created` 事件触发；争议调查期间保持访问权限；记录争议详情（ID、金额、原因）；争议解决后根据结果转为 Active 或 Canceled
- **Scheduled Cancel（预定取消）** — 收到 `subscription.scheduled_cancel` 事件触发；用户在计费周期结束前仍可正常访问；可在周期结束前取消预定取消操作
- **Refund（退款）** — 收到 `refund.created` 事件触发；退款不影响访问权限；仅记录日志用于审计
- **Canceled** — 订阅已取消
- **Expired** — 订阅已过期

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
- 当前支持 Creem（模拟支付平台），未来扩展 Stripe、支付宝/微信支付
- 每个 realm 使用独立的 webhook URL，实现多租户隔离

**订阅套餐管理**：
- 创建订阅套餐：填写套餐基本信息（name, title, description, type, price, currency, checkout_url, trial_days, sort_order），不需要同时配置支付平台映射
- 编辑订阅套餐：除 name 外所有字段可修改
- 删除订阅套餐：仅无活跃订阅时可删除
- 查看订阅套餐列表：支持按启用状态、计费周期筛选
- 配置 Plan 的支付平台映射：为套餐添加、编辑、删除支付平台映射

**套餐分配管理**：
- 将套餐分配到 Client App
- 查看套餐分配情况
- 移除套餐分配

**订阅生命周期管理**：
- 创建订阅：用户在第三方应用选择套餐 -> 重定向到支付页面 -> 完成支付 -> Webhook 通知 -> 创建订阅记录
- 升级订阅：按比例计费
- 降级订阅：下个计费周期生效
- 取消订阅：当前计费周期结束生效

**Webhook 事件处理**：
- 支持 Creem 的 checkout.completed、subscription.active/trialing/paid/paused/canceled/expired/update 事件
- 签名验证、事件幂等性处理、状态转换验证
- 与积分系统联动：首次订阅充值、定期续费充值

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
- 套餐 CRUD 操作：Plan 本身的增删改查
- 支付平台映射管理：独立的 CRUD 接口，与套餐主体解耦
- 套餐分配管理：将套餐分配到 Client App 或移除分配
- 订阅查询：套餐列表、订阅状态、订阅变更历史
- SDK 查询：第三方应用查询用户订阅和套餐状态，返回套餐及支持的支付平台列表
- Checkout 发起：显式传递 plan_id + payment_provider 参数
- Webhook 接收：根据外部商品/价格标识定位到正确的 Plan 和支付平台映射

**访问控制与数据边界**：
- 所有接口遵守 realm 隔离原则
- 写入类操作（创建、编辑、删除套餐/映射/分配）需要 `billing.manage` 权限
- 读取类操作需要 `billing.view` 权限或认证用户身份
- 删除套餐时级联删除所有支付平台映射配置
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

**状态反馈**：
- 创建套餐成功后提示："Please configure payment providers for this plan"
- 删除套餐时提示："This will delete all payment provider mappings for this plan"
- 删除支付平台映射时提示活跃订阅数量："Cannot delete mapping with X active subscriptions"
- 禁用支付平台映射时提示："Existing subscriptions will continue to work, new users cannot select this provider"

---

## 8. 已确认决策

### 8.1 已确认决策

- **简化模型**：Herald 不管理套餐的功能（features）和配额（quotas），由第三方应用自行管理
- **多支付平台映射**：采用 Plan + Plan Payment Provider 映射模型，一个 Plan 可关联多个支付平台，无需为每个支付平台复制 Plan
- **Webhook 隔离**：每个 realm 使用独立的 webhook URL，realm_id 从 URL 路径提取实现多租户隔离
- **编目演进**：Billing 编目从 `Realm -> Plan` 演进为 `Realm -> Product -> Plan`，Product 主定义以 product-catalog PRD 为准
- **退款边界**：支付平台处理金额退款，Herald 处理积分回收

---

## 9. 参考资料

- 用户故事：`docs/user-stories/billing/subscription.md`
- 相关 PRD：`docs/prd/billing/product-catalog.md`
- 相关 PRD：`docs/prd/billing/points.md`
- 相关 PRD：`docs/prd/core/realm-settings.md`
- Realm Admin 用户故事：`docs/user-stories/core/realm-admin.md`
