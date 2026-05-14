# Invoice 发票产品需求文档 (PRD)

**创建时间**: 2026-05-08
**状态**: Implemented
**优先级**: P0

---

## 1. 相关用户故事

> 详细故事与验收标准请查看 `docs/user-stories/13-invoice-user-stories.md`。

### 1.1 相关故事

- `[US-IV-001]` 创建发票，优先级 P0，来源 `docs/user-stories/13-invoice-user-stories.md`
  - 角色：Realm Admin
  - 摘要：创建发票草稿，添加行项目，设置费用和双方信息

- `[US-IV-002]` 编辑发票草稿，优先级 P0，来源 `docs/user-stories/13-invoice-user-stories.md`
  - 角色：Realm Admin
  - 摘要：修改草稿发票的行项目、费用和双方信息

- `[US-IV-003]` 查看发票列表，优先级 P0，来源 `docs/user-stories/13-invoice-user-stories.md`
  - 角色：Realm Admin
  - 摘要：查看本 Realm 所有发票，支持筛选和分页

- `[US-IV-004]` 查看发票详情，优先级 P0，来源 `docs/user-stories/13-invoice-user-stories.md`
  - 角色：Realm Admin
  - 摘要：查看发票完整详情，包括行项目和状态历史

- `[US-IV-005]` 开具发票，优先级 P0，来源 `docs/user-stories/13-invoice-user-stories.md`
  - 角色：Realm Admin
  - 摘要：将草稿发票正式开具（draft → issued）

- `[US-IV-006]` 作废发票，优先级 P1，来源 `docs/user-stories/13-invoice-user-stories.md`
  - 角色：Realm Admin
  - 摘要：作废草稿或已开具的发票

- `[US-IV-007]` 标记发票已付，优先级 P0，来源 `docs/user-stories/13-invoice-user-stories.md`
  - 角色：Realm Admin
  - 摘要：手动将发票标记为已付款

- `[US-IV-008]` 查看我的发票，优先级 P1，来源 `docs/user-stories/13-invoice-user-stories.md`
  - 角色：Regular User
  - 摘要：查看自己的发票列表和详情

- `[US-IV-009]` 系统标记逾期发票，优先级 P1，来源 `docs/user-stories/13-invoice-user-stories.md`
  - 角色：Herald 系统
  - 摘要：自动将超过到期日未支付的发票标记为逾期

- `[US-IV-010]` 配置销售方信息，优先级 P0，来源 `docs/user-stories/13-invoice-user-stories.md`
  - 角色：Realm Admin
  - 摘要：在 Billing 设置中配置本 Realm 的销售方信息，用户申请发票时自动填充

- `[US-IV-011]` 申请发票，优先级 P0，来源 `docs/user-stories/13-invoice-user-stories.md`
  - 角色：Regular User
  - 摘要：为已付款订单或订阅申请发票

- `[US-IV-012]` 审核并开具用户申请的发票，优先级 P0，来源 `docs/user-stories/13-invoice-user-stories.md`
  - 角色：Realm Admin
  - 摘要：审核用户申请的发票，确认后开具或作废

### 1.2 优先级汇总

| 优先级 | 数量 | 关键故事 |
|--------|------|----------|
| P0 | 9 | 创建发票、编辑草稿、查看列表、查看详情、开具发票、标记已付、配置销售方、申请发票、审核开具 |
| P1 | 3 | 作废发票、查看我的发票、系统标记逾期 |
| P2 | 0 | - |

---

## 2. 范围界定

### 2.1 包含功能

- 销售方信息配置（Realm Admin 在 Billing 设置中一次性配置，后续发票自动填充）
- 用户申请发票（Regular User 为已付款订单申请，系统自动创建草稿）
- 管理员审核开具（Realm Admin 审核用户申请，确认后开具或作废）
- 管理员手动创建发票（辅助路径）
- 发票 CRUD（查看、编辑、作废）
- 行项目管理（添加、编辑、删除、排序）
- 费用计算（折扣、税费、运费，支持固定金额和百分比模式）
- 发票状态机（draft → issued → paid / void / overdue）
- 发票编号自动生成（租户内按年递增，格式 INV-{YEAR}-{SEQ}）
- 买方信息管理（用户申请时填写开票抬头）
- Regular User 查看自己的发票及申请状态
- 系统自动标记逾期发票（定时任务）
- 发票审计追踪（状态变更历史）
- 发票可关联 Subscription 和 Payment Attempt（可选，不触发自动行为）

### 2.2 不包含功能 (Out of Scope)

- ❌ PDF 生成和下载（后续迭代）
- ❌ 多格式导出（CSV / XLSX / XML，后续迭代）
- ❌ 邮件发送发票（后续迭代）
- ❌ Subscription 续费自动创建 Invoice（后续迭代）
- ❌ Payment Attempt 支付成功自动创建 Invoice（可关联但不自动生成）
- ❌ 在线支付集成（发票仅手动标记已付）
- ❌ 发票模板自定义
- ❌ 多币种自动转换

### 2.3 依赖项

- ✅ **Realm 系统**（已实现）— 发票属于 Realm 级别
- ✅ **Account 系统**（已实现）— 开票对象关联 Account
- ✅ **现有角色与登录身份判断**（已实现）— 复用 Realm Admin / Regular User 角色边界，不新增 Invoice 细粒度权限
- ⚠️ **Subscription 系统**（部分实现）— Invoice 可选关联 Subscription
- ⚠️ **Payment Attempt 系统**（已实现）— Invoice 可选关联 Payment Attempt

---

## 3. 需求概述

### 3.1 功能描述

为 Herald 多租户系统增补发票功能。主流程为：Realm Admin 配置销售方信息 → Regular User 申请发票 → Realm Admin 审核开具。同时保留 Admin 手动创建发票的辅助路径。发票与现有 billing 模块集成，可关联 Subscription 和 Payment Attempt，但不自动生成。

### 3.2 关键特性

- **以用户申请为主**：Regular User 主动申请发票，Admin 审核
- **销售方信息预配置**：Realm Admin 一次性配置，后续自动填充
- 发票状态机管理（draft / issued / paid / void / overdue）
- 行项目驱动的金额计算，以最小货币单位（分）存储
- 折扣 / 税费 / 运费支持固定金额和百分比两种模式
- 发票编号在租户内按年自动递增
- 严格的租户数据隔离

---

## 4. 当前实现状态

| 功能模块 | 状态 | 备注 |
|---------|------|------|
| 销售方信息配置 | ❌ 未实现 | Realm Billing 设置 |
| 用户申请发票 | ❌ 未实现 | - |
| 管理员审核开具 | ❌ 未实现 | - |
| 管理员手动创建发票 | ❌ 未实现 | 辅助路径 |
| 行项目管理 | ❌ 未实现 | - |
| 状态机 | ❌ 未实现 | - |
| 金额计算 | ❌ 未实现 | - |
| 发票编号生成 | ❌ 未实现 | - |
| 逾期标记定时任务 | ❌ 未实现 | - |
| 前端发票管理页面 | ❌ 未实现 | - |
| 前端发票申请页面 | ❌ 未实现 | - |

---

## 5. 功能需求

### 5.1 核心需求

- **销售方信息配置**：Realm Admin 在 Billing 设置中配置本 Realm 的销售方信息（公司名称、地址、邮箱、电话等），用户申请发票时自动填充；未配置时用户无法提交申请
- **用户申请发票（主流程）**：Regular User 为已付款的订单或订阅申请发票，填写开票抬头信息，系统创建草稿发票（来源标记为 user_application），销售方信息自动从 Realm 配置填充
- **管理员审核开具**：Realm Admin 在发票列表中筛选待审核发票，审核通过后开具（draft → issued），审核不通过可作废并注明原因；审核时允许编辑草稿内容
- **管理员手动创建（辅助路径）**：Realm Admin 可直接创建草稿发票，手动填写双方信息和行项目
- **发票编辑**：仅草稿状态可编辑行项目、费用和双方信息；编辑后自动重算金额
- **发票开具**：将草稿发票正式开具（draft → issued），记录开票日期；空发票不可开具
- **发票作废**：将草稿或已开具的发票作废（draft / issued → void）；已付款发票不可作废
- **标记已付**：手动将已开具或逾期发票标记为已付款（issued / overdue → paid）
- **逾期标记**：系统定时检查到期日已过的 issued 发票，自动标记为 overdue

### 5.2 计算规则

- `line_item.subtotal` = quantity × unit_price
- `invoice.subtotal` = SUM(line_items.subtotal)
- `invoice.total` = subtotal - discount_amount + tax_amount + shipping_amount
- 折扣 / 税费 / 运费：请求中传入 mode（fixed / percent）和 value，服务端计算最终金额
- 所有金额以最小货币单位（分）存储，避免浮点误差

### 5.3 验收目标

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

**状态**: 必填

- 发票 API 遵循 realm 隔离原则，在 `api-billing` crate 中新增
- 不新增 Invoice 专属细粒度权限；管理端接口复用 Realm Admin 角色判断，用户端接口复用登录用户身份判断
- Realm Admin 可管理本 Realm 所有发票；Regular User 只能查询和申请自己的发票
- 销售方信息配置 API 归属 Realm Billing 设置，仅 Realm Admin 可访问
- 用户申请发票 API 需验证用户拥有对应的支付记录
- 发票编号（invoice_number）在 realm + 年范围内唯一
- 仅 draft 状态可编辑；issued / overdue 可标记已付或作废；paid 不可修改
- 发票可关联 subscription_id 和 payment_attempt_id，关联为可选，不触发自动行为
- 发票来源标记（admin_manual / user_application）需持久化，用于筛选和审计
- 状态变更操作需记录审计事件（actor、timestamp、changes）

---

## 7. 前端/交互约束

**状态**: 必填

- **管理后台**（Realm Admin）：
  - 入口：Realm 管理后台的 Billing 区域新增 "Invoices" 菜单
  - 销售方配置：Billing 设置页面新增销售方信息配置区域（公司名称、地址、邮箱、电话）
  - 发票列表页：表格展示编号、开票对象、金额、状态、来源、到期日，支持状态、来源和日期筛选
  - 待审核视图：筛选来源为 "user_application" 且状态为 "draft" 的发票，快速审核
  - 发票创建 / 编辑表单：行项目动态添加删除，实时计算金额汇总
  - 发票详情页：展示完整发票信息、行项目和状态历史时间线
  - 状态操作：Issue、Void、Mark as Paid 按钮根据当前状态启用 / 禁用

- **个人页面**（Regular User）：
  - 入口：用户个人中心的 "My Invoices" 菜单
  - 申请发票入口：在支付记录或订阅详情旁提供 "Apply for Invoice" 按钮
  - 申请表单：选择支付记录、填写开票抬头（名称、地址、邮箱）
  - 列表页：展示属于自己的发票，包含编号、金额、状态、到期日、申请状态
  - 详情页：查看发票完整信息

- **状态反馈**：
  - 操作成功后显示对应成功消息
  - 状态不合法时禁用按钮并提示原因
  - 金额变动后实时更新汇总区域

---

## 8. 技术设计承接

**状态**: 必填

- 数据库表结构、迁移方案、状态机实现、金额计算逻辑、编号生成策略、定时任务调度等详细设计应在 `.ai/design/invoice.md` 中承接
- 参考设计来源：`.ai/future/invoice.md`
- 相关模块：billing domain、api-billing crate、worker 定时任务

---

## 9. 相关文件索引

### 9.1 后端文件

- `backend/` 相关实现文件待技术设计文档确定后补充

### 9.2 前端文件

- `frontend/` 相关实现文件待技术设计文档确定后补充

---

## 10. 参考资料

- 用户故事：`docs/user-stories/13-invoice-user-stories.md`
- 设计参考：`.ai/future/invoice.md`
- 相关 PRD：`docs/prd/billing/billing.md`
- 角色：`docs/user-stories/_roles.md`
