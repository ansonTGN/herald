# 产品多价格与 Stripe 抽象对齐 产品需求文档 (PRD)

**创建时间**: 2026-06-26
**优先级**: P1
**域**: billing

---

## 1. 相关用户故事

> 详细故事与验收标准请查看 `docs/user-stories/` 中对应文档。本特性主要补充 `docs/user-stories/billing/entitlement-mapping.md`。

### 1.1 相关故事

**本特性新增（多价格场景）**
- `[US-EM-007]` 同步并配置一个产品的多个价格，优先级 P0，来源 `docs/user-stories/billing/entitlement-mapping.md`
  - 角色：Realm Admin
  - 摘要：当一个产品存在多个价格时，为每个价格分别配置计费类型/周期与积分策略，与 Stripe Product→Price 对齐
- `[US-EM-008]` Webhook 在一产品多价格时正确解析订阅归属，优先级 P0，来源 `docs/user-stories/billing/entitlement-mapping.md`
  - 角色：System
  - 摘要：产品多价格时识别订阅实际归属哪个价格/entitlement，按正确价格策略发放或回收积分
- `[US-EM-009]` 用户购买多价格产品的指定价格，优先级 P0，来源 `docs/user-stories/billing/entitlement-mapping.md`
  - 角色：Regular User
  - 摘要：购买多价格产品时选择具体价格（月付/年付），checkout 指向真实价格并按该价格授权发放

**复用既有（单价格与映射基线）**
- `[US-EM-001]` 查看 Provider Entitlement 映射，P0，来源 `docs/user-stories/billing/entitlement-mapping.md`
- `[US-EM-002]` 触发 Provider 产品同步，P1，来源 `docs/user-stories/billing/entitlement-mapping.md`
- `[US-EM-003]` Webhook 通过 Metadata 映射订阅，P0，来源 `docs/user-stories/billing/entitlement-mapping.md`
- `[US-EM-004]` 基于 Entitlement 应用积分策略，P0，来源 `docs/user-stories/billing/entitlement-mapping.md`
- `[US-EM-005]` SDK 通过 Entitlement 查询订阅状态，P0，来源 `docs/user-stories/billing/entitlement-mapping.md`
- `[US-EM-006]` 查看订阅投影列表，P0，来源 `docs/user-stories/billing/entitlement-mapping.md`
- `[US-BI-005]` 发起订阅 Checkout，P0，来源 `docs/user-stories/billing/subscription.md`

### 1.2 优先级汇总

| 优先级 | 数量 | 关键故事 |
|--------|------|----------|
| P0 | 9 | 多价格同步配置(007)、多价格 webhook 解析(008)、指定价格购买(009)、查看映射(001)、Metadata 映射(003)、应用积分(004)、SDK 查询(005)、订阅投影(006)、Checkout(BI-005) |
| P1 | 1 | Provider 产品同步（含多价格） |
| P2 | 0 | - |

---

## 2. 范围界定

### 2.1 包含功能

- 将 **Price（价格）提升为 provider 模型的一等概念**，对齐 Stripe 的 Product→Price 抽象：一个产品可拥有多个价格。
- **按价格粒度的 entitlement 映射**：每个价格承载各自的计费类型（recurring / one_time）、计费周期与积分策略；同一产品的多个价格可共享一个 `entitlement_key`（如月付/年付同属 `pro-plan`），也可映射到不同 entitlement。
- **Price-aware 产品同步**：同步按价格粒度建立/更新映射，而非仅取首个价格。
- **Price-aware webhook 解析**：产品多价格时，能识别订阅实际归属的价格/entitlement。
- **Price-aware 购买**：checkout 引用支付方真实价格（对具备 price 概念的支付方），不再为每次购买重建临时价格。
- 管理后台与购买页对多价格的展示与选择交互。

### 2.2 不包含功能 (Out of Scope)

- **本地可编辑产品/价格目录**：Herald 仍不维护商业目录，产品和价格始终以支付方为 source of truth（沿用 `subscription.md` 的编目边界）。
- **按量计费 / usage-based pricing**：本特性聚焦"一个产品多个固定价格选项"（如月付/年付、recurring/one-time），不引入 metered usage 计量与结算。
- **价格级促销与折扣逻辑**：仍由支付方管理（Stripe Coupons / Promotion Codes、Creem Discount Codes），Herald 不在本地实现促销。
- **多币种转换与税务计算**：沿用支付方原生币种与税务能力，不在 Herald 内做换算。
- **价格的归档/停售生命周期管理**：仅沿用支付方给出的 active/inactive 状态，不在 Herald 内单独设计价格下架流程。
- **新支付方接入**：不新增支付方；Creem/WeChat/Shopify 的多价格能力以其自身模型为准（见 4.1）。

### 2.3 依赖项

- **支付平台配置系统** — Realm 级 Stripe/Creem 配置（`docs/prd/billing/stripe-payment.md`）。
- **Entitlement 映射与 provider-sourced cache** — 多价格映射建立在现有 `provider_entitlement_mappings` 模型之上（`docs/prd/billing/subscription.md`）。
- **Webhook 处理链** — `herald_*` metadata 契约、幂等与 fail-loud 解析链。
- **Checkout / 购买流程** — 购买目标仍为 entitlement mapping，价格作为购买单元的细化。
- **积分系统** — 按价格配置的积分策略的发放/回收（`docs/prd/billing/points.md`）。

---

## 3. 需求概述

### 3.1 功能描述

当前 Herald 的 provider entitlement 模型是**产品粒度**的：一个外部产品只能映射到一个 entitlement，`external_price_id` 仅作展示、不参与唯一性、不参与 webhook 解析，checkout 还会重建临时价格而非引用真实 Stripe 价格。因此当一个 Stripe 产品拥有多个价格（如月付与年付、recurring 与 one-time）时，Herald 无法表达、无法区分，也无法按价格授权与发放。

本特性把 **Price 提升为一等概念**，使 Herald 的 provider 抽象与 Stripe 的 Product→Price 对齐：同一个产品的多个价格各自成为独立可购、可配置的单元，分别承载计费类型/周期与积分策略；同步、webhook 解析与 checkout 全部变为 price-aware。项目尚未上线，因此直接以 price 粒度建模，不保留单价格兼容路径。

### 3.2 关键特性

- Price 一等概念，Product→Price 对齐 Stripe。
- 按价格粒度配置 entitlement / 计费类型 / 周期 / 积分策略，entitlement_key 可跨价格共享或独立。
- Price-aware 同步（不再仅取首个价格）。
- Price-aware webhook 解析（metadata 优先，价格回退，fail loud）。
- Price-aware 购买（checkout 引用真实 provider 价格）。

---

## 4. 业务规则与状态

### 4.1 业务规则

**Price 建模规则**

- Herald 的 provider 模型引入 **Price 维度**，对齐 Stripe 的 Product→Price：一个产品可拥有多个价格，每个价格是独立可购与可配置单元。
- 价格的属性（金额、币种、计费类型 recurring/one_time、计费周期等）来自支付方，Herald 不本地编辑；Herald 只在价格上配置 entitlement、计费语义标签与积分策略。
- **支付方差异**：
  - **Stripe**：完整支持 Product→多 Price（recurring 不同 interval、one_time 等），多价格能力在本特性中完全生效。
  - **Creem**：无独立 Price 概念（Product 即价格），按"每个产品一行映射"处理，不强制引入多价格。
  - **WeChat / Shopify**：以其自身模型为准；本特性不要求改造，但解析与同步规则应允许其退化为单价格。

**映射规则**

- entitlement 映射以**(支付方, 产品, 价格)**为可分辨单元；同一产品的不同价格是各自独立的映射行。
- `entitlement_key` **按价格设置，可共享**：同一产品多个价格可共享一个 `entitlement_key`（典型：月付与年付同属 `pro-plan`），也可映射到不同 `entitlement_key`。
- 计费类型（recurring / one_time）、计费周期与积分策略（points_per_period、validity_days、grant_on_subscribe 等）**按价格配置**；不同价格可有不同策略（如年付发放更多积分）。
- 沿用既有边界：禁用某价格映射后，匹配该映射的 webhook 订阅事件仍更新订阅投影，但不触发该价格的积分发放/回收；重新启用后恢复。删除/禁用有活跃订阅的价格映射时，沿用现有"拒绝并提示活跃订阅数量"的保护。

**同步规则（Price-aware）**

- 产品同步按**价格粒度**建立/更新映射：对具备 price 概念的支付方，同一产品的每个可售价格生成一条映射，不再仅取首个价格。
- 同步保留既有 provider-sourced cache 语义：价格展示信息（名称、金额、币种、计费类型/周期）以支付方为准，刷新覆盖本地缓存；映射的 entitlement/积分策略以 Herald 本地配置为准，不被同步覆盖。
- 同步失败时本地缓存继续服务，并记录诊断；不静默降级为默认策略（fail loud，沿用既有规则）。

**Webhook 解析规则（Price-aware）**

- 解析链：**webhook metadata 中的 `herald_entitlement_key` → 本地映射（按 支付方 + 产品 + 价格 查询）→ fail loud**。
- 当 metadata 携带 `herald_entitlement_key` 时，按 entitlement 解析订阅投影；积分策略进一步按 webhook 标识的实际价格匹配（用于区分共享 entitlement 下的不同计费/积分策略）。
- 当 metadata 缺失 `herald_entitlement_key` 时，按 **(支付方, 产品, 价格)** 命中唯一映射。
- 当一产品多价格且 webhook 既无 entitlement_key 又无法唯一确定价格时，**不静默使用默认策略**：记录诊断并让错误对管理员可见（fail loud，沿用既有规则）。
- 其余 webhook 规则（签名验证、幂等、metadata 必填校验、fallback 不静默跳过）沿用 `subscription.md` / `entitlement-mapping.md`。

**购买规则（Price-aware）**

- 购买目标仍为 entitlement mapping，**价格作为购买单元的细化**：用户在多价格产品上选择具体价格发起购买。
- checkout 引用**支付方真实价格**（对具备 price 概念的支付方），不再为每次购买重建临时价格；`herald_*` 必填 metadata（realm / client app / user / entitlement_key / billing_kind）仍随 checkout 传递。
- 只有价格对应映射已启用、且其支付方已在 Realm 启用时，才允许购买；否则该价格选项不可购买或被禁用并给出提示（沿用既有可用性判断）。
- one_time 价格走 one-time 履约（发放 topup_credit，不创建订阅）；recurring 价格走订阅创建/同步；判定依据价格自身的计费类型。

**数据与权限规则**

- 沿用 realm 隔离：所有读写遵守 realm 边界。
- 沿用权限：查看映射 `billing.view`；触发同步、配置/启用/禁用映射 `billing.manage`（或现有 provider 映射管理权限）；积分策略相关写入沿用 `points.manage`。
- 金额与积分变更必须可追溯；敏感信息沿用既有脱敏与加密存储规则。

### 4.2 关键状态与异常

- **订阅状态**沿用 `subscription.md` 的状态定义（Active / Past Due / Canceled / Expired 等）；本特性不新增订阅状态。订阅投影记录其归属的价格/entitlement，用于积分策略匹配。
- **异常场景**：
  - 一产品多价格、webhook 无法唯一确定价格且无 entitlement_key：fail loud，记录诊断，错误对管理员可见。
  - 同步失败：现有映射不受影响，继续服务，记录诊断。
  - 购买未启用或未配置支付方的价格：选项不可购买/禁用并提示。
  - 删除/禁用有活跃订阅的价格映射：拒绝并提示活跃订阅数量（沿用既有保护）。

---

## 5. 功能需求

### 5.1 核心需求

- **多价格映射配置**：Realm Admin 可查看与配置同一产品下多个价格的映射，每个价格独立设置 entitlement_key、计费类型、计费周期与积分策略；entitlement_key 可跨价格共享或独立。
- **Price-aware 同步**：触发产品同步时，对具备 price 概念的支付方按价格粒度建立/更新映射；同步结果可展示同步到的产品数与价格数。
- **Price-aware webhook 解析**：webhook 处理在产品多价格时正确识别归属价格/entitlement，并按对应价格策略发放/回收积分；无法确定时 fail loud。
- **Price-aware 购买**：购买流程支持在多价格产品上选择具体价格，checkout 引用真实 provider 价格并按该价格授权发放。

### 5.2 验收目标

- 同一个 Stripe 产品的多个价格（如月付/年付）能各自成为独立可购选项，并可分别配置计费类型/周期与积分策略。
- 多个价格可共享同一 `entitlement_key`（如月付/年付同属 pro-plan），也可映射到不同 entitlement，二者均能正确授权与发放。
- 当一个产品有多个价格时，webhook 能正确识别订阅归属的价格/entitlement，并按该价格的策略发放或回收积分；无法唯一确定时不静默降级，错误对管理员可见。
- 用户购买多价格产品时，checkout 指向所选价格对应的真实支付方价格，购买完成后按该价格授权与发放。
- SDK 通过 `entitlement_key` 查询订阅状态的行为不变（共享 entitlement 的多价格产品对第三方应用仍表现为同一 entitlement）。

---

## 6. API 相关约束

**适用性**: 适用

**能力边界**

- 不提供本地 Product/Price CRUD；产品与价格始终以支付方为 source of truth。
- 提供：按价格粒度查看/配置 entitlement 映射、按价格触发 provider 产品同步、按价格发起 checkout、价格感知的订阅投影查询。
- checkout 显式传递所选价格；webhook 接收优先使用 `herald_entitlement_key`，回退到按 (支付方, 产品, 价格) 的本地映射。

**访问控制与数据边界**

- 所有接口遵守 realm 隔离。
- 写入类操作（配置/启用/禁用映射、触发同步）需要 `billing.manage`（或现有 provider 映射管理权限）；积分策略相关写入沿用 `points.manage`。
- 读取类操作需要 `billing.view` 或认证用户身份。
- 金额与积分变更必须可追溯。

**兼容性要求**

- webhook 处理需支持回调幂等与失败补偿（fail loud，不静默降级）。
- 单价格映射与无 price 支持支付方保持向后兼容；与支付方、积分账本、订阅系统的详细契约下沉到技术设计与接口文档（建议执行 `/t-design`）。

---

## 7. 前端/交互约束

**适用性**: 适用

**管理后台（Entitlement Mappings）**

- 映射列表在产品维度下展示其各价格：同一产品的多个价格可分组展示，每条价格显示外部价格、计费类型/周期、entitlement_key、积分策略与启用状态。
- 支持为同一产品的不同价格分别配置 entitlement_key、计费类型、计费周期与积分策略；支持 entitlement_key 在多价格间共享或独立。
- 支持按支付方、产品、entitlement_key 筛选。
- 同步结果展示同步到的产品数与价格数；空状态与引导沿用既有提示。

**购买页**

- 多价格产品向用户展示各可选价格（如月付/年付）及其金额/周期；用户选择具体价格后发起购买。
- 未启用或未配置可用支付方的价格选项不可购买或禁用，并给出明确提示。

**状态反馈**

- 配置/同步成功与失败反馈沿用既有提示风格；金额/积分相关变化需突出影响范围。
- webhook 无法唯一确定价格时，错误对管理员可见（诊断信息）。

---

## 8. 已确认决策

- **Price 一等概念（按价格配置）**：Herald 引入 Price 维度对齐 Stripe 的 Product→Price；一个产品可拥有多个价格，每个价格是独立可购与可配置单元，承载各自的计费类型/周期与积分策略。entitlement_key 按价格设置、可跨价格共享或独立。（用户在 PRD grill 中确认，选择"按价格配置"方案）
- **Price-aware 同步**：产品同步按价格粒度建立/更新映射，不再仅取首个价格。
- **Price-aware webhook 解析**：解析链为 `herald_entitlement_key` → (支付方, 产品, 价格) 本地映射 → fail loud；不静默降级。
- **Price-aware 购买**：checkout 引用支付方真实价格（对具备 price 概念的支付方），不再重建临时价格。
- **不考虑向后兼容**：项目尚未上线，直接以 price 粒度建模与落地，不保留单价格兼容路径或既有数据负担。
- **支付方范围（推断，待二次确认）**：多价格能力对 Stripe 完全生效；Creem（无 price 概念）以产品为价格单元；WeChat/Shopify 暂以单价格处理。
- **编目边界不变**：Herald 仍不维护本地可编辑产品/价格目录，支付方始终是 source of truth。

---

## 9. 参考资料

- 用户故事（含多价格新增）：`docs/user-stories/billing/entitlement-mapping.md`（US-EM-007/008/009）
- 用户故事：`docs/user-stories/billing/subscription.md`
- 正式 PRD：`docs/prd/billing/subscription.md`（订阅计费、Entitlement 映射、Metadata 契约）
- 正式 PRD：`docs/prd/billing/stripe-payment.md`（Stripe 集成）
- 正式 PRD：`docs/prd/billing/points.md`（积分系统）
- Stripe 文档：[Products and Prices](https://docs.stripe.com/products-prices/how-products-and-prices-work)、[Prices API](https://docs.stripe.com/api/prices)
