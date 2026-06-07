# Invoice 发票产品需求文档 (PRD)

**创建时间**: 2026-05-08
**优先级**: P0

---

## 1. 相关用户故事

> 详细故事与验收标准请查看 `docs/user-stories/billing/invoice.md`。

### 1.1 故事引用

- `[US-IV-001]` 创建发票，优先级 P0，来源 `docs/user-stories/billing/invoice.md`
  - 角色：Realm Admin
  - 摘要：创建发票草稿，添加行项目，设置费用和双方信息

- `[US-IV-002]` 编辑发票草稿，优先级 P0，来源 `docs/user-stories/billing/invoice.md`
  - 角色：Realm Admin
  - 摘要：修改草稿发票的行项目、费用和双方信息

- `[US-IV-003]` 查看发票列表，优先级 P0，来源 `docs/user-stories/billing/invoice.md`
  - 角色：Realm Admin
  - 摘要：查看本 Realm 所有发票，支持筛选和分页

- `[US-IV-004]` 查看发票详情，优先级 P0，来源 `docs/user-stories/billing/invoice.md`
  - 角色：Realm Admin
  - 摘要：查看发票完整详情，包括行项目和状态历史

- `[US-IV-005]` 开具发票，优先级 P0，来源 `docs/user-stories/billing/invoice.md`
  - 角色：Realm Admin
  - 摘要：将草稿发票正式开具（draft → issued）

- `[US-IV-006]` 作废发票，优先级 P1，来源 `docs/user-stories/billing/invoice.md`
  - 角色：Realm Admin
  - 摘要：作废草稿或已开具的发票

- `[US-IV-007]` 标记发票已付，优先级 P0，来源 `docs/user-stories/billing/invoice.md`
  - 角色：Realm Admin
  - 摘要：手动将发票标记为已付款

- `[US-IV-008]` 查看我的发票，优先级 P1，来源 `docs/user-stories/billing/invoice.md`
  - 角色：Regular User
  - 摘要：查看自己的发票列表和详情

- `[US-IV-009]` 系统标记逾期发票，优先级 P1，来源 `docs/user-stories/billing/invoice.md`
  - 角色：Herald 系统
  - 摘要：自动将超过到期日未支付的发票标记为逾期

- `[US-IV-010]` 配置销售方信息，优先级 P0，来源 `docs/user-stories/billing/invoice.md`
  - 角色：Realm Admin
  - 摘要：在 Billing 设置中配置本 Realm 的销售方信息，用户申请发票时自动填充

- `[US-IV-011]` 申请发票，优先级 P0，来源 `docs/user-stories/billing/invoice.md`
  - 角色：Regular User
  - 摘要：为已付款订单或订阅申请发票

- `[US-IV-012]` 审核并开具用户申请的发票，优先级 P0，来源 `docs/user-stories/billing/invoice.md`
  - 角色：Realm Admin
  - 摘要：审核用户申请的发票，确认后开具或作废

### 1.2 优先级汇总表

| 优先级 | 数量 | 关键故事 |
|--------|------|----------|
| P0 | 9 | 创建发票、编辑草稿、查看列表、查看详情、开具发票、标记已付、配置销售方、申请发票、审核开具 |
| P1 | 3 | 作废发票、查看我的发票、系统标记逾期 |
| P2 | 0 | - |

---

## 2. 范围界定

### 2.1 包含功能

- 销售方信息配置（Realm Admin 在 Billing 设置中一次性配置，后续发票自动填充；包含默认付款条款 default_payment_terms 字段）
- 用户申请发票（Regular User 可从购买历史或订阅历史上下文入口申请，系统预填并隐藏内部引用 ID；独立申请页保留手动填写引用的兼容路径）
- 管理员审核开具（Realm Admin 审核用户申请，确认后开具或作废）
- 管理员手动创建发票（辅助路径）
- 发票 CRUD（查看、编辑、作废；列表支持按 invoice_number 和 billing_name 模糊搜索）
- 行项目管理（添加、编辑、删除、排序）
- 费用计算（折扣、税费支持固定金额和百分比模式；运费仅支持固定金额模式）
- 发票状态机（draft → issued → paid / void / overdue）
- 发票编号自动生成（租户内按年递增，格式 INV-{YEAR}-{SEQ}）
- 买方信息管理（用户申请时填写开票抬头，含税号）
- 销售方税号（必填，用于发票合规）
- Regular User 查看自己的发票及申请状态
- 系统自动标记逾期发票（定时任务）
- 发票审计追踪（状态变更历史）
- 发票可关联 Subscription 和 Payment Attempt（上下文入口自动传递关联 ID；独立表单仍可手动填写）
- PDF 发票生成和下载

### 2.2 不包含功能

- 多格式导出（CSV / XLSX / XML，后续迭代）
- 邮件发送发票（后续迭代）
- Subscription 续费自动创建 Invoice（后续迭代）
- Payment Attempt 支付成功自动创建 Invoice（可关联但不自动生成）
- 在线支付集成（发票仅手动标记已付）
- 发票模板自定义
- 多币种自动转换

### 2.3 依赖项

- **Realm 系统** — 发票属于 Realm 级别
- **Account 系统** — 开票对象关联 Account
- **现有角色与权限系统** — 管理端使用 `billing.view` / `billing.manage` 权限控制，用户端复用登录用户身份判断
- **Subscription 系统**（部分实现）— Invoice 可选关联 Subscription
- **Payment Attempt 系统** — Invoice 可选关联 Payment Attempt

---

## 3. 需求概述

### 3.1 功能描述

为 Herald 多租户系统增补发票功能。主流程为：Realm Admin 配置销售方信息 → Regular User 申请发票 → Realm Admin 审核开具。同时保留 Admin 手动创建发票的辅助路径。发票与现有 billing 模块集成，可关联 Subscription 和 Payment Attempt，但不自动生成。

### 3.2 关键特性

- **以用户申请为主**：Regular User 主动申请发票，Admin 审核
- **销售方信息预配置**：Realm Admin 一次性配置，后续自动填充
- 发票状态机管理（draft / issued / paid / void / overdue）
- 行项目驱动的金额计算，以最小货币单位（分）存储
- 折扣 / 税费支持固定金额和百分比两种模式；运费仅支持固定金额模式
- 发票编号在租户内按年自动递增
- 严格的租户数据隔离

---

## 4. 业务规则与状态

### 4.1 业务规则

- **销售方信息前置条件**：Realm Admin 必须先配置销售方信息（公司名称、地址、邮箱、电话、税号），否则用户无法提交发票申请
- **用户申请验证**：用户申请发票需验证拥有对应的支付记录；申请时填写开票抬头信息（含税号），系统创建草稿发票（来源标记为 user_application），销售方信息自动从 Realm 配置填充
- **用户申请发票时必须填写开票抬头税号**：用户申请发票时，`billing_tax_id` 为必填字段，不可为空字符串
- **发票编辑时双方税号为必填字段**：编辑发票时，`billing_tax_id` 和 `seller_tax_id` 均为必填字段，不可为空字符串
- **列表搜索**：发票列表支持通过 `search` 查询参数对 `invoice_number` 和 `billing_name` 进行模糊搜索（ILIKE），不区分大小写
- **销售方默认付款条款**：销售方配置（`SellerConfigRequest`）包含 `default_payment_terms` 可选字段，用户申请发票时自动填充为发票的 `payment_terms`；管理员手动创建时也可单独指定
- **发票编号唯一性**：发票编号（invoice_number）在 realm + 年范围内唯一，格式 INV-{YEAR}-{SEQ}
- **编辑约束**：仅 draft 状态可编辑行项目、费用和双方信息；编辑后自动重算金额
- **开具约束**：空发票不可开具；开具时记录开票日期；支持通过 `issue_date` 可选参数覆盖开票日期（默认为当天）；若存在 `due_date`，则 `due_date` 必须大于等于 `issue_date`；开具时 `billing_email` 和 `billing_phone` 至少需填写一个非空值，用于联系开票对象
- **标记已付约束**：仅 issued / overdue 状态可标记已付；支持通过 `paid_at` 可选时间戳参数覆盖实际付款时间（默认为当前时间）
- **作废约束**：已付款发票不可作废；可作废 draft 和 issued 状态
- **来源标记**：发票来源（admin_manual / user_application）需持久化，用于筛选和审计
- **关联可选**：发票可关联 subscription_id 和 payment_attempt_id，关联为可选，不触发自动行为
- **金额计算规则**：line_item.subtotal = quantity x unit_price；invoice.subtotal = SUM(line_items.subtotal)；invoice.total = subtotal - discount_amount + tax_amount + shipping_amount；所有金额以最小货币单位（分）存储。折扣、税费、运费均以 subtotal 为基准计算，税费未考虑折扣影响（即税费不基于折后金额）
- **运费模式限制**：运费（shipping_mode）仅支持固定金额（fixed）模式，不支持百分比模式（数据库 CHECK 约束限制）

### 4.2 关键状态与异常

- **发票状态机**：draft → issued → paid / void / overdue
- **逾期标记**：系统定时检查到期日已过的 issued 发票，自动标记为 overdue
- **审计追踪**：所有状态变更操作需记录审计事件（actor、timestamp、changes）
- **权限边界**：管理端接口通过 `billing.view` / `billing.manage` 权限检查控制访问（非直接检查 Realm Admin 角色），用户端复用登录用户身份判断；Regular User 只能查询和申请自己的发票

---

## 5. 功能需求

### 5.1 核心需求

- **销售方信息配置**：Realm Admin 在 Billing 设置中配置本 Realm 的销售方信息，用户申请发票时自动填充
- **用户申请发票（主流程）**：Regular User 为已付款的订单或订阅申请发票，填写开票抬头信息，系统创建草稿发票
- **管理员审核开具**：Realm Admin 在发票列表中筛选待审核发票，审核通过后开具，审核不通过可作废并注明原因；审核时允许编辑草稿内容
- **管理员手动创建（辅助路径）**：Realm Admin 可直接创建草稿发票，手动填写双方信息和行项目
- **发票编辑**：仅草稿状态可编辑，编辑后自动重算金额
- **发票开具**：将草稿发票正式开具，记录开票日期；支持通过 `issue_date` 可选参数覆盖开票日期
- **发票作废**：将草稿或已开具的发票作废；已付款发票不可作废
- **标记已付**：手动将已开具或逾期发票标记为已付款；支持通过 `paid_at` 可选参数指定实际付款时间
- **逾期标记**：系统定时检查到期日已过的 issued 发票，自动标记为 overdue
- **PDF 生成和下载**：支持发票 PDF 生成和下载

### 5.2 验收目标

- Realm Admin 能配置销售方信息，后续发票自动填充
- Regular User 能为已付款订单申请发票，看到申请状态变化
- Realm Admin 能审核用户申请的发票，开具或作废
- Realm Admin 也能手动创建发票（辅助路径）
- 金额计算准确无误，包括百分比税费和折扣
- 发票编号在租户内唯一且按年递增
- 状态变更全部记录到审计历史
- Regular User 只能查看和申请自己的发票，无法访问他人发票

---

## 6. API 相关约束

**适用性**: 适用

- **接口能力范围**：发票 CRUD、销售方信息配置、发票开具/作废/标记已付、用户申请发票、PDF 生成下载的能力边界；在 api-billing crate 中新增
- **访问控制原则**：管理端接口通过 `billing.view` / `billing.manage` 权限检查控制（`require_billing_permission` 辅助函数实现）；用户端接口复用登录用户身份判断；Realm Admin 可管理本 Realm 所有发票；Regular User 只能查询和申请自己的发票；销售方信息配置 API 归属 Realm Billing 设置，需 `billing.manage` 权限
- **租户/Realm 数据边界**：发票按 Realm 隔离；发票编号在 realm + 年范围内唯一
- **状态操作约束**：仅 draft 可编辑；issued / overdue 可标记已付或作废；paid 不可修改

---

## 7. 前端/交互约束

**适用性**: 适用

- **管理后台**（Realm Admin）：
  - 入口：Realm 管理后台的 Billing 区域新增 "Invoices" 菜单；只按管理权限控制，不因当前 Realm 尚未配置销售方信息或尚无发票记录而隐藏
  - 销售方配置：Billing 设置页面新增销售方信息配置区域（公司名称、地址、邮箱、电话、税号）
  - 发票列表页：表格展示编号、开票对象、金额、状态、来源、到期日，支持状态、来源和日期筛选
  - 待审核视图：筛选来源为 "user_application" 且状态为 "draft" 的发票，快速审核
  - 发票创建/编辑表单：行项目动态添加删除，实时计算金额汇总
  - 发票详情页：展示完整发票信息、行项目和状态历史时间线
  - 状态操作：Issue、Void、Mark as Paid 按钮根据当前状态启用/禁用

- **个人页面**（Regular User）：
  - 入口：当 Realm 已配置销售方信息时，用户个人中心显示 "My Invoices" 菜单；未配置时隐藏
  - 申请发票入口：当 Realm 已配置销售方信息时，在支付记录或订阅详情旁提供 "Apply for Invoice" 按钮；未配置时不展示
  - 申请表单：选择支付记录、填写开票抬头（名称、地址、邮箱、税号）
  - 列表页：展示属于自己的发票，包含编号、金额、状态、到期日、申请状态
  - 详情页：查看发票完整信息

- **状态反馈**：操作成功后显示对应成功消息；状态不合法时禁用按钮并提示原因；金额变动后实时更新汇总区域

---

## 8. 已确认决策

### 8.1 已确认决策

- 主流程为用户申请 + 管理员审核开具，保留管理员手动创建辅助路径
- 不新增 Invoice 细粒度权限，管理端使用 `billing.view` / `billing.manage` 权限控制，用户端复用登录用户身份判断
- 发票可关联 Subscription 和 Payment Attempt 但不自动生成
- 发票编号格式为 INV-{YEAR}-{SEQ}，租户内按年递增

---

## 9. 参考资料

- 用户故事：`docs/user-stories/billing/invoice.md`
- 相关 PRD：`docs/prd/billing/subscription.md`
