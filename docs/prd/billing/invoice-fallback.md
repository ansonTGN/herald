# Invoice Fallback 产品需求文档 (PRD)

**创建时间**: 2026-06-05
**优先级**: P0

---

## 1. 相关用户故事

> 详细故事与验收标准请查看 `docs/user-stories/billing/invoice-fallback.md`。

### 1.1 故事引用

- `[US-IF-001]` 配置发票策略，优先级 P0，来源 `docs/user-stories/billing/invoice-fallback.md`
  - 角色：Realm Admin
  - 摘要：配置 Realm 发票策略（provider_first / manual_only / none）和各支付平台的外部发票能力开关

- `[US-IF-002]` 系统同步 Stripe 发票，优先级 P0，来源 `docs/user-stories/billing/invoice-fallback.md`
  - 角色：Herald 系统
  - 摘要：通过 Stripe webhook 自动同步 Stripe 发票数据到 Herald

- `[US-IF-003]` 系统同步 Creem 交易税务数据，优先级 P0，来源 `docs/user-stories/billing/invoice-fallback.md`
  - 角色：Herald 系统
  - 摘要：同步 Creem MoR 交易的税务数据到 Herald

- `[US-IF-004]` 查看外部 Provider 发票（管理员），优先级 P0，来源 `docs/user-stories/billing/invoice-fallback.md`
  - 角色：Realm Admin
  - 摘要：在发票列表中查看外部 provider 同步的发票（只读）

- `[US-IF-005]` 查看外部 Provider 发票（普通用户），优先级 P1，来源 `docs/user-stories/billing/invoice-fallback.md`
  - 角色：Regular User
  - 摘要：在"我的发票"中查看外部 provider 同步的发票（只读）

- `[US-IF-006]` 下载外部发票 PDF 或查看 Provider 页面，优先级 P1，来源 `docs/user-stories/billing/invoice-fallback.md`
  - 角色：Realm Admin / Regular User
  - 摘要：通过外部 URL 下载或查看 provider 管理的发票 PDF

### 1.2 优先级汇总表

| 优先级 | 数量 | 关键故事 |
|--------|------|----------|
| P0 | 4 | 配置发票策略、同步 Stripe 发票、同步 Creem 税务、管理员查看外部发票 |
| P1 | 2 | 用户查看外部发票、下载外部 PDF |
| P2 | 0 | - |

---

## 2. 范围界定

### 2.1 包含功能

- **发票策略配置**：Realm Admin 配置 `invoice_policy`（provider_first / manual_only / none）和每个支付平台的外部发票能力开关
- **Stripe 发票同步**：通过 webhook 自动同步 Stripe Invoicing 产生的发票数据到 Herald（只读镜像）
- **Creem 交易税务数据同步**：Creem MoR 交易支付成功后同步税务数据到 Herald
- **只读展示外部发票**：provider-owned 发票在 Herald 中只读展示，禁止创建、编辑、开具、作废、标记已付
- **自研发票 Fallback**：provider 不支持或未启用外部发票时，走现有 Herald 自研发票系统（功能不变）
- **Provider 来源标识**：发票列表和详情页显示发票来源 provider（Manual / Stripe / Creem）
- **外部 PDF / 托管页面跳转**：有外部 PDF URL 时直接重定向下载；有托管页面 URL 时显示 "View in Provider" 链接
- **Creem MoR 保护**：Creem MoR 交易不允许创建 Herald manual 发票
- **发票列表 provider 筛选**：支持按 provider 类型筛选发票
- **数据模型预留**：发票记录新增来源标识、支付平台关联和外部链接等属性，预留 WeChat / Shopify 来源类型

### 2.2 不包含功能 (Out of Scope)

- 微信电子发票 API 接入（后续独立迭代）
- Shopify 订单/发票文档同步（后续独立迭代）
- Herald 主动调用 Stripe Invoice API 创建发票（本阶段仅 webhook 被动同步）
- Herald 主动调用 Creem API 查询交易税务（本阶段仅通过支付回调同步）
- 发票邮件发送
- 发票多格式导出（CSV / XLSX / XML）
- 外部发票的金额换算或多币种自动转换
- Subscription 续费自动创建 Invoice

### 2.3 依赖项

- **Invoice 自研系统**（`docs/prd/billing/invoice.md`）— fallback 路径复用现有全部功能
- **Payment Attempt 系统** — 发票路由依赖 payment_attempt 的 payment_provider 字段
- **Subscription 系统** — 发票路由依赖 subscription 的 payment_provider 字段
- **Stripe 支付集成**（`docs/prd/billing/stripe-payment.md`）— Stripe 发票同步依赖 Stripe webhook 基础设施
- **Creem 支付集成** — Creem 税务同步依赖 Creem 回调处理
- **现有角色与权限系统** — 管理端使用 `billing.view` / `billing.manage` 权限控制
- **Realm 配置系统** — 发票策略存储在 realm_config

---

## 3. 需求概述

### 3.1 功能描述

Herald 发票系统从纯自研模式升级为"外部平台发票优先 + 自研发票 Fallback"的双模式架构。核心原则：Herald 不与支付平台的发票/税务能力竞争，外部平台已开发票时 Herald 只做只读展示。发票来源由实际收款 payment_provider 的发票能力决定，而非按产品或 Realm 全局决定。

### 3.2 关键特性

- **发票跟随实际收款 provider**：同一产品支持多支付平台时，发票归属由实际 payment_provider 的发票能力决定
- **三种发票策略**：provider_first（优先外部 provider）、manual_only（仅自研）、none（不提供自研发票入口）
- **只读展示 provider-owned 发票**：数据由 webhook/API 同步，不可通过 Herald API 修改
- **Creem MoR 不可覆盖**：Creem 作为 Merchant of Record 的交易，Herald 不得创建 manual 发票
- **自研发票完全保留**：provider 不支持外部发票时，现有 Herald 自研发票系统功能不变
- **WeChat / Shopify 仅预留数据模型**：当前阶段 provider 枚举值和字段已预留，不实现具体同步逻辑

---

## 4. 业务规则与状态

### 4.1 业务规则

- **发票来源路由**：根据 payment_attempt / subscription 上的实际 payment_provider 和该 provider 的外部发票能力配置决定发票来源；不按产品或 Realm 全局决定
- **invoice_policy 行为矩阵**：

  | 操作 | provider_first | manual_only | none |
  |------|---------------|-------------|------|
  | 发票列表 | 展示自研 + 外部 provider 同步数据 | 展示自研数据 | 仅展示外部 provider 同步数据 |
  | 发票详情 | 外部发票只读，自研发票按现有逻辑 | 全部按现有逻辑 | 外部发票只读 |
  | 创建发票 | 仅 manual fallback 场景 | 允许 | 不允许 |
  | 编辑/开具/作废/标记已付 | 仅 manual 发票 | 全部允许 | 不允许 |
  | 用户申请发票 | 仅 manual fallback 场景 | 允许 | 不允许 |
  | PDF 下载 | 外部发票用外部 URL，自研发票用 IronPress | IronPress 生成 | 外部 URL |
  | "View in Provider" 链接 | 有 external_hosted_url 时显示 | 不显示 | 有时显示 |

- **Creem MoR 约束**：Creem 交易的发票必须由 Creem 管理；无论 invoice_policy 设置如何，Herald 不得为 Creem 交易创建 manual 发票
- **Stripe 发票同步触发**：通过 Stripe webhook 被动同步（invoice.created / invoice.finalized / invoice.voided / invoice.paid），Herald 不主动调用 Stripe Invoice API 创建发票
- **Stripe 发票状态映射**：Stripe `draft` → Herald `draft`，Stripe `open` → Herald `issued`，Stripe `paid` → Herald `paid`，Stripe `void` → Herald `void`
- **Creem 税务数据同步**：Creem 交易支付成功后同步交易金额、税额、税区等税务信息作为发票记录
- **Provider 切换兼容**：Realm 从 manual_only 切到 provider_first 时，已有 manual 发票保持 provider='manual' 不变，策略切换只影响新发票的路由决策
- **发票编号规则不变**：外部 provider 发票使用 provider 分配的编号（如 Stripe 的发票编号），自研发票继续使用 INV-{YEAR}-{SEQ} 格式
- **Webhook 幂等性**：复用现有 payment_event 表的 external_event_id 唯一约束，重复 webhook 更新而非创建
- **外部发票不可操作**：provider != 'manual' 的发票禁止通过 Herald API 执行创建、编辑、开具、作废、标记已付操作
- **权限复用**：管理端继续使用 `billing.view` / `billing.manage` 权限控制，不新增发票细粒度权限

### 4.2 关键状态与异常

- **外部发票状态**：由 provider 驱动更新，Herald 只做状态映射和只读展示；自研发票状态机（draft → issued → paid / void / overdue）保持不变
- **Provider 未启用外部发票能力**：当 provider_first 策略下某 provider 未启用外部发票时，该 provider 的交易降级到 manual fallback
- **Stripe 未启用 Invoicing**：Stripe Dashboard 未启用 Stripe Invoicing 时，不会发送 invoice.* webhook 事件，Stripe 支付的交易走 manual fallback
- **Creem 无 PDF URL**：Creem API 当前不返回发票 PDF URL，用户需通过 Creem 平台查看完整发票
- **多 provider 同一交易**：一笔交易只对应一个 payment_provider，不存在多 provider 同一交易的冲突

---

## 5. 功能需求

### 5.1 核心需求

- **发票策略配置**：Realm Admin 在 Billing 设置中配置 invoice_policy 和每个支付平台的外部发票能力开关；WeChat 和 Shopify 在未接入前不展示开关
- **Stripe 发票 webhook 同步**：Herald 自动接收 Stripe 的 invoice.created / invoice.finalized / invoice.voided / invoice.paid 事件，同步发票数据到本地，状态按映射规则转换
- **Creem 交易税务同步**：Creem 支付成功后，系统创建 provider='creem' 的发票记录，同步交易税务数据
- **外部发票只读展示**：发票列表和详情页显示 provider 来源标识；provider != manual 的发票隐藏所有编辑操作按钮，显示 "View in Provider" 链接
- **自研发票 Fallback**：invoice_policy=provider_first 时，不支持外部发票的 provider 交易仍可使用 Herald 自研发票
- **外部 PDF / 页面跳转**：有 external_pdf_url 时重定向下载；有 external_hosted_url 时显示跳转链接；无 URL 时提示由 provider 管理
- **发票列表 provider 筛选**：支持按 provider 类型（Manual / Stripe / Creem）筛选发票列表

### 5.2 验收目标

- Realm Admin 能配置 invoice_policy，启用/禁用各 provider 的外部发票能力
- Stripe Invoicing 产生的发票能通过 webhook 自动同步到 Herald 并正确映射状态
- Creem MoR 交易的税务数据能同步到 Herald 并只读展示
- 外部 provider 发票在管理端和用户端均为只读，无法通过 Herald API 修改
- 自研发票功能在 manual_only 和 fallback 场景下完全保持不变
- Creem MoR 交易无法创建 Herald manual 发票
- 发票列表能按 provider 筛选，能区分显示不同来源的发票
- PDF 下载正确区分自研（IronPress）和外部（URL 重定向）

---

## 6. API 相关约束

**适用性**: 适用

- **接口能力边界**：现有发票 CRUD API 对 provider != manual 的发票禁止写操作（创建、编辑、开具、作废、标记已付）；新增发票策略配置 API；新增 provider 筛选查询参数
- **访问控制原则**：复用现有 `billing.view` / `billing.manage` 权限控制；不新增发票细粒度权限；外部发票只读展示权限与自研发票查看权限一致
- **租户/Realm 数据边界**：发票按 Realm 隔离；发票策略配置按 Realm 独立；provider 能力开关按 Realm + Provider 独立
- **兼容性要求**：现有 invoice API 响应向后兼容（新增字段可选，默认 provider='manual'）；自研发票的全部 API 行为不变

---

## 7. 前端/交互约束

**适用性**: 适用

- **管理后台**（Realm Admin）：
  - 发票策略配置入口：Billing 设置页面新增发票策略区域，包含 invoice_policy 选择和每个已启用支付平台的外部发票能力开关；未启用的 provider（WeChat 电子发票、Shopify）不展示开关
  - 发票列表页：表格新增 provider 来源列（Manual / Stripe / Creem），支持按 provider 筛选；外部 provider 发票行不显示编辑/开具/作废/标记已付操作
  - 发票详情页：provider != manual 时切换为只读模式，隐藏所有操作按钮，显示 provider 标识和 "View in Provider" 链接（有 external_hosted_url 时）
  - PDF 下载：外部发票使用外部 URL 重定向；自研发票使用现有 IronPress 生成

- **个人页面**（Regular User）：
  - 发票列表页：显示 provider 来源标识
  - 发票详情页：外部发票只读，显示 "View in Provider" 链接
  - 申请发票：Creem 交易的申请入口不可用；其他 provider 交易根据 invoice_policy 决定是否可申请

- **状态反馈**：操作外部 provider 发票时提示 "This invoice is managed by {Provider}"；Creem 交易申请发票时提示由 Creem 管理

---

## 8. 已确认决策

- 发票来源跟随实际收款 payment_provider，而非跟随产品或 Realm 全局选择
- 三种发票策略：provider_first / manual_only / none
- Stripe 发票同步通过 webhook 被动驱动，Herald 不主动调用 Stripe Invoice API 创建发票
- Creem MoR 交易的发票不可被 Herald manual 覆盖，无论 invoice_policy 设置
- WeChat 电子发票和 Shopify 发票文档同步为后续独立迭代，本次仅预留数据模型
- 已有 manual 发票在策略切换后保持 provider='manual' 不变
- 不新增发票细粒度权限，复用现有 billing.view / billing.manage
- 外部发票 PDF 有 URL 时直接重定向，无 URL 时提示由 provider 管理
- 外部发票编号使用 provider 分配的编号，自研发票继续 INV-{YEAR}-{SEQ}

---

## 9. 参考资料

- 用户故事：`docs/user-stories/billing/invoice-fallback.md`
- 自研发票 PRD：`docs/prd/billing/invoice.md`
- 技术预研：`.ai/tech-research/invoice_fallback.md`
- 需求文档：`.ai/future/invoice_fallback.md`
- Stripe 支付 PRD：`docs/prd/billing/stripe-payment.md`
- Creem 支付 PRD：`docs/prd/billing/shopify-pay.md`（Creem 原名）
- 微信支付 PRD：`docs/prd/billing/wechat-pay.md`
