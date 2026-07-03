# 支付产品同步增强（产品名 / 价格单位 / metadata） 产品需求文档 (PRD)

**创建时间**: 2026-07-02
**优先级**: P0（产品名 / 价格单位 / 计费周期）/ P1（metadata、credit 周期解耦）
**域**: billing

---

## 1. 相关用户故事

> 详细故事与验收标准见 `docs/user-stories/billing/entitlement-mapping.md`（US-BL-SYNC-001~004）。

### 1.1 相关故事
- `[US-BL-SYNC-002]` 列表展示产品名便于识别，优先级 P0，来源 `docs/user-stories/billing/entitlement-mapping.md`
- `[US-BL-SYNC-003]` Stripe / Creem 价格单位正确展示，优先级 P0，来源 `docs/user-stories/billing/entitlement-mapping.md`
- `[US-BL-SYNC-004]` 计费周期以 Stripe 为准、只读且不被人工覆盖，优先级 P0，来源 `docs/user-stories/billing/entitlement-mapping.md`
- `[US-BL-SYNC-001]` 同步携带商户自定义 metadata 并可查看，优先级 P1，来源 `docs/user-stories/billing/entitlement-mapping.md`

### 1.2 优先级汇总
| 优先级 | 数量 | 关键故事 |
|--------|------|----------|
| P0 | 3 | 产品名展示、价格单位修正、计费周期以 Stripe 为准 |
| P1 | 2 | provider metadata 同步、credit/额度周期与计费周期解耦 |
| P2 | 0 | — |

---

## 2. 范围界定

### 2.1 包含功能
- 在 entitlement mappings 列表/分组展示中，把已同步的 provider 产品名（`provider_product_info.name`）作为主可识别标签，缺失时回退到外部产品 id。
- 修正 Stripe / Creem 价格单位的展示逻辑：Stripe 同步写入的价格为整数最小货币单位（分），Creem 价格为字符串/数字显示值，展示路径必须按 provider 区分，不再共用单一换算。
- 同步过程中携带 **Stripe** 的 `Product.metadata`、`Price.metadata`（商户自定义键值对），存入与产品同级的展示信息中，供管理员只读查看。
  - 注：Creem 的 Product 对象在 [Creem List all products](https://docs.creem.io/api-reference/endpoint/search-products) 响应中没有 `metadata` 字段（Creem 的 `metadata` 是 checkout 会话级，不属于 Product），因此 Creem 侧不适用 metadata 同步，详见 §4.1。

### 2.2 不包含功能 (Out of Scope)
- **不**对 Creem 产品同步 metadata：Creem Product 对象无原生 metadata 字段（其 `custom_fields` 是结账时收集客户信息的表单定义，语义 ≠ Stripe metadata），不强行伪造。
- 不在 Herald 端编辑 provider metadata：metadata 的权威源仍是 Stripe 后台，Herald 仅做同步与只读展示。
- 不引入新独立表/字段来重构 `provider_product_info` 的存储结构（沿用既有 JSON 展示信息字段，按需扩充其内容）。
- 不改变同步触发入口、权限模型与同步状态机（仍由现有 `ProviderSyncButton` + `sync_provider_products` 承担）。
- 不在本期引入 provider metadata 驱动的自动 entitlement 映射规则（仅展示，不自动派生）。
- 不调整 Stripe webhook / checkout 路径里 `provider_product_info` 的写入语义。

### 2.3 依赖项
- 现有 `ProviderProductSyncService` + `ConfiguredProviderProductApi` 同步链路（Stripe/Creem）。
- 现有 `provider_entitlement_mappings.provider_product_info` JSON 字段作为展示信息载体。
- Stripe `/v1/products`、`/v1/prices`、Creem `/v1/products/search` 的响应字段（`metadata`、`name`、`unit_amount`/`price`）。

---

## 3. 需求概述

### 3.1 功能描述
现有的 Stripe/Creem 产品同步已写入产品名、描述、价格、币种、计费类型等基础信息，但在两端存在三处可识别性缺口：列表里没有用产品名作为主标签、Stripe 与 Creem 的价格单位被同一展示路径误判、provider 上的商户自定义 metadata 完全没有被同步。本 PRD 围绕"管理员能仅凭 Herald 端就识别并理解一条 provider mapping"这一目标，补齐产品名展示、修正价格单位、并新增 metadata 同步。

### 3.2 关键特性
- **产品名为第一识别标签**：列表分组/行的主标签优先取产品名，外部产品 id 退化为兜底标签。
- **按 provider 区分价格单位**：Stripe 整数最小货币单位 vs Creem 显示值，展示路径不再混淆。
- **provider metadata 跟随同步**：同步时把 provider 的自定义键值对一并带过来，并在详情中只读呈现。

---

## 4. 业务规则与状态

### 4.1 业务规则
- **适用 provider（metadata）**：metadata 同步**仅适用于 Stripe**。Stripe `Product` 与 `Price` 对象都有稳定的 `metadata` 键值对字段（[Stripe metadata 文档](https://docs.stripe.com/metadata)）。Creem 的 Product 对象在 [List all products](https://docs.creem.io/api-reference/endpoint/search-products) 响应中**没有** `metadata` 字段（Creem 的 `metadata` 是 checkout 会话级、非 Product 级，`custom_fields` 是向客户收集信息的表单定义、语义不同），因此 Creem 产品**不适用** metadata 同步。
- **权威源**：Stripe metadata 的权威源是 Stripe 后台；Herald 端只读，不做回写或编辑。每次同步以 Stripe 当前值为准覆盖本地展示值。
- **同步覆盖语义**：重新同步同一 (provider, external_product_id, external_price_id) 时，name / description / 价格 / metadata / 计费周期（billing_period）等展示字段以最新一次同步为准；由 Herald 管理员配置的 entitlement_key、points、grant 策略、quota 等业务字段不被同步覆盖（与现行行为一致）。
- **计费周期以 Stripe 为准**：计费周期（订阅周期 / `billing_period`，如 month / year / week / day）的唯一权威来源是 Stripe `Price.recurring.interval`。Herald 端**不做独立配置或人工覆盖**：前端展示保持只读，保存/更新链路不得以人工填写值覆盖同步值。重新同步时以 Stripe 当前 interval 为准覆盖本地。计费周期是 Stripe-only 概念，Creem 产品响应不含该字段，按空（"—"）展示，不伪造、不推断。
- **credit/额度周期与计费周期解耦（不冲突）**：`billing_period`（计费周期，来自 Stripe）与 credit/额度的滚动窗口（`quota_windows`，如 5 小时 / 周）是**两个独立概念，刻意解耦，互不绑定**：
  - **计费周期**决定"Stripe 何时扣款、何时发续费 webhook"；**额度窗口**决定"用户在滚动窗口内可用多少额度"。两者**不要求相等、不要求整除、不做强制对齐**。
  - **额度授予由事件驱动**：每次 Stripe/Creem 续费 webhook 按 provider 返回的 `(period_start, period_end)` 锚定授予一期额度权益，**不预发整个计费周期的总额余额、不按日历固定日清零**（区别于 GitHub Copilot 式固定日历重置）。
  - **支持"纯窗口、无周期总额"模型（OpenAI 式）**：mapping 可不设 `points_per_period`（周期总额为空），仅靠 `quota_windows`（小时/周窗）即构成完整订阅额度。
  - **设计意图对齐**：长计费周期（如年付）内仍可配滚动限额（月窗/周窗），这是有意为之，不视为冲突。
  - **不阻塞同步**：额度窗口与计费周期的组合不参与同步链路的成功/失败判定；同步只负责拉取展示信息（含 `billing_period`），不读取/校验 `quota_windows`。
- **缺失即空**：provider 未提供 name、description、metadata、计费周期（含 Creem 本无 interval 的情形）时按空处理，不报错、不阻塞同步。
- **metadata 限定**：仅同步 Stripe 原生 `metadata` 字段；不收集产品/价格对象上其它杂项字段。
- **展示位**：所有同步来的展示信息（含 metadata）继续存放于既有 `provider_product_info` 结构内，作为面向管理员的展示数据，不作为计费/扣点依据。

### 4.2 关键状态与异常
- **同步失败保留既有信息**：当某产品的 metadata 或价格子请求失败时，已成功同步的产品仍计入 `Completed`/`Partial`，失败项进入既有 `partial_errors` 通道，不因单个 metadata 拉取失败而回滚整次同步。
- **权限可见性**：产品名、价格、metadata 展示仅在 Admin Realm（具备 `billing.manage` 或对应权限）的 entitlement 管理页面可见；普通用户侧不展示 provider 内部信息。
- **价格单位歧义消除**：展示价格时必须可由 provider 来源（stripe/creem）可靠推断出单位语义；当 provider 字段缺失导致无法判断时，回退为"原始值 + 单位未知"标识，不做隐式换算。

---

## 5. 功能需求

### 5.1 核心需求
- **FR-1 产品名作为列表识别标签**：在 entitlement mappings 列表/分组的主标签位置展示产品名；name 缺失时回退到外部产品 id；二者皆不可用时给出可识别的占位（不显示空标签）。
- **FR-2 产品名过滤**：现有产品过滤器支持按产品名匹配（兼顾外部 id 命中），命中规则与现有过滤体验一致。
- **FR-3 价格单位修正**：展示价格按 provider 区分单位——Stripe 取最小货币单位整数换算为主货币单位展示；Creem 按其原值展示；单位换算必须由 provider 来源驱动，不允许跨 provider 共享同一条换算分支。
- **FR-4 Stripe metadata 同步**：同步流程在拉取 Stripe 产品/价格时一并读取 `Product.metadata`、`Price.metadata`（商户自定义键值对），按字段实际形状落入本地展示信息；缺失时记为空。Creem 产品不适用（无原生 metadata 字段）。
- **FR-5 metadata 只读展示**：在 mapping 详情中只读展示 metadata 键值对；空时显示"无 metadata"或省略；不提供新增/编辑入口。
- **FR-6 既有字段不回归**：name、description、价格、币种、计费类型、计费周期等已同步字段在本期变更后仍正确写入与展示，不出现回归。
- **FR-7 计费周期以 Stripe 为准且只读**：计费周期（`billing_period`）取自 Stripe `Price.recurring.interval`，前端只读、不接受人工输入；保存/更新不得写入与同步值不一致的周期。重新同步以 Stripe 当前值为准。Creem 产品无该字段时按空展示，不推断、不伪造。
- **FR-8 credit/额度周期与计费周期解耦**：`billing_period` 与 `quota_windows`（额度滚动窗口）保持独立，不强制对齐、不整除、不互相覆盖；同步链路不读取/校验 `quota_windows`，额度授予由续费 webhook 事件驱动（按 provider 的 `period_start/end` 锚定），非日历清零。支持"纯窗口、无周期总额"（OpenAI 式）配置。

### 5.2 验收目标
- 管理员在 entitlement mappings 列表里看到每条产品的可读名称（产品名或外部 id），无需打开详情即可识别。
- 同一产品在 Stripe 与 Creem 上同价（如 9.99）时，两端展示金额一致且正确，不再出现 Stripe 被 Creem 路径二次缩放或反之的情况。
- 同步后，在 Stripe 产品的 mapping 详情可看到与 Stripe 后台一致的 metadata 键值对；重复同步后以最新值为准。
- 不带 metadata、不带 name 的产品（含 Creem 产品本无 metadata）仍能完成同步并正常展示（无报错、无空标签）。
- 计费周期（订阅周期）在 Stripe 产品上展示为 Stripe `Price.recurring.interval`（如 month/year）；Creem 产品展示为"—"，不出现人工配置值或与同步值不一致的周期。

---

## 6. API 相关约束

**适用性**: 适用（仅描述接口能力边界，不含具体 schema）

- 同步入口沿用既有 `billing.entitlement-mappings` 同步能力，不新增独立同步端点；metadata 与 name 的获取在现有同步请求链路内完成，避免对 provider 发起额外高频调用。
- 列表/详情读取接口需把产品名与 metadata 通过既有展示信息字段返回给前端，前端据此渲染；不要求新增独立的 metadata 读取端点。
- 访问控制沿用现有权限（具备 entitlement mapping 管理权限的 Admin Realm 角色方可触发同步与查看详情），不做权限模型变更。
- Realm/租户边界：同步与展示严格限定在当前 realm；不跨 realm 暴露 provider metadata。
- 兼容性：对历史已同步但无 metadata 的 mapping，展示按"空 metadata"处理，不做历史回填强制要求；如需回填由管理员再次触发同步。

> 具体端点、请求/响应字段与 schema 不在 PRD 范围；如需细化请走 `/t-design`。

---

## 7. 前端/交互约束

**适用性**: 适用

- 页面入口：Admin Realm 的 entitlement mappings 管理页（现有页面），不新增路由。
- 主交互：
  - 列表/分组的主标签展示产品名（缺失回退外部 id）。
  - 现有产品过滤器支持按产品名匹配。
  - mapping 详情中以只读方式展示 metadata 键值对，不提供编辑入口。
  - 价格展示按 provider 区分单位，遵循 FR-3。
- 状态反馈：
  - 同步成功/部分失败/失败沿用现有同步结果反馈（`SyncProviderResponse` 通道）。
  - provider metadata/name 缺失时不显示错误态，按空处理。
- 权限可见性：仅 Admin Realm 有相应权限的管理员可见；普通用户页面不展示 provider 内部信息。

---

## 8. 已确认决策
- **范围**：同时做「修复并补齐现有字段展示（产品名 + 价格单位）」与「新增同步 provider 原生 metadata 字段」。
- **Creem metadata 适用性（经网络调研确认）**：Creem Product 对象在 [List all products](https://docs.creem.io/api-reference/endpoint/search-products) 响应中**无** `metadata` 字段，Creem 的 `metadata` 仅是 checkout 会话级；因此 metadata 同步**仅适用于 Stripe**，不对 Creem 强行同步/伪造。Stripe `Product`/`Price` 的 `metadata` 稳定可用（[Stripe metadata 文档](https://docs.stripe.com/metadata)）。
- **metadata 权威源**：Stripe metadata 以 Stripe 后台为权威，Herald 只读展示，不编辑、不回写。
- **展示信息载体**：继续使用既有 `provider_product_info` 展示信息字段承载新增内容，不在本期引入新独立存储结构。
- **同步入口与权限**：沿用现有 `ProviderSyncButton` + `sync_provider_products` 链路与 entitlement mapping 管理权限。
- **前端 UI**：entitlement mappings 列表 UI 已存在，本期在其既有结构上增量补充产品名标签与价格单位修正，不重做页面。

---

## 9. 参考资料
- 用户故事：`docs/user-stories/billing/entitlement-mapping.md`（US-BL-SYNC-001~004）
- 相关 PRD：`docs/prd/billing/payment-invoice-mapping.md`、`docs/prd/billing/support-multiple-price.md`、`docs/prd/billing/subscription.md`
- Stripe metadata 文档：[https://docs.stripe.com/metadata](https://docs.stripe.com/metadata)
- Creem List all products：[https://docs.creem.io/api-reference/endpoint/search-products](https://docs.creem.io/api-reference/endpoint/search-products)
