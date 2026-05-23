# Product 编目管理 产品需求文档 (PRD)

**创建时间**: 2026-03-26
**优先级**: P1

---

## 1. 相关用户故事

> 详细故事与验收标准请查看 `docs/user-stories/` 中对应文档。

### 1.1 相关故事

- `docs/user-stories/billing/product-management.md`
  - [US-PR-001] 创建 Product (P0): 作为 Realm Admin，创建 Product 以将相关套餐组织在一起
  - [US-PR-002] 编辑 Product (P0): 作为 Realm Admin，编辑 Product 信息以更新产品描述
  - [US-PR-003] 查看 Product 列表 (P0): 作为 Realm Admin，查看所有 Product 列表以了解产品目录结构
  - [US-PR-004] 启用/禁用 Product (P1): 作为 Realm Admin，启用或禁用 Product 以控制产品可见性
  - [US-PR-005] 删除 Product (P1): 作为 Realm Admin，删除不再需要的 Product 以保持产品目录整洁
  - [US-PR-006] 在 Product 下管理 Plan (P0): 作为 Realm Admin，在 Product 视图下管理 Plan 以按产品线组织套餐

### 1.2 优先级汇总

| 优先级 | 数量 | 关键故事 |
|--------|------|----------|
| P0 | 4 | 创建 Product、编辑 Product、查看 Product 列表、在 Product 下管理 Plan |
| P1 | 2 | 启用/禁用 Product、删除 Product |
| P2 | 0 | - |

---

## 2. 范围界定

### 2.1 包含功能

- Product 实体管理（创建、编辑、删除、查看）
- Product-Plan 层级关系管理
- Product 启用/禁用控制
- Product 排序（按创建时间）
- Plan 到 Product 的归属管理
- Product 删除约束（有 Plan 时不可删除）
- 前端 Product 管理页面
- 为后续 Points 扩展保留 Product 语义边界

### 2.2 不包含功能 (Out of Scope)

- Product 的功能（features）管理（Product 是编目对象，功能由 Plan 或第三方应用管理）
- Product 级别的订阅统计（统计功能在后续阶段实现）
- Product 级别的配额（quotas）管理（采用简化模型，由第三方应用自行管理）
- Product 之间的关联关系（Product 是独立实体，不支持父子或关联关系）
- Product 模板（首版不实现模板功能）

**Points 边界说明**：本 PRD 只要求 Product 成为 Billing 编目层的正式对象；Points 是否升级为 Product 分层规则，由 `docs/prd/billing/points.md` 另行定义。所有 Plan 必须属于某个 Product，这是强制约束。

### 2.3 依赖项

- **Realm 系统**：Product 功能属于 Realm 级别
- **Billing Plan 系统**：Plan 需要归属到 Product
- **权限管理系统**：Realm Admin 权限检查
- **Points 积分系统**：当前阶段仅需与新的 Product 编目语义保持兼容

---

## 3. 需求概述

### 3.1 功能描述

Product 编目管理是 Herald 系统为 Realm 提供的产品线组织能力。通过引入 Product 作为 Realm 下的正式业务对象，实现从 Realm -> Plan 的平面模型升级为 Realm -> Product -> Plan 的层次化模型。

### 3.2 关键特性

- **层次化编目**：Product 作为 Plan 的上层组织对象
- **多产品线支持**：一个 Realm 可以运营多个产品线（如云存储、AI 服务、CRM）
- **灵活管理**：支持 Product 的创建、编辑、启用/禁用、删除
- **积分边界清晰**：Product 先作为 Billing 编目层正式对象落地，为后续 Points 扩展提供稳定语义基础
- **向后兼容**：现有 Plan 数据可以通过迁移归属到默认 Product

---

## 4. 业务规则与状态

### 4.1 业务规则

**Product 命名规范**：
- code 只能包含字母、数字、横线（-）和下划线（_）
- code 在同一 Realm 内必须唯一
- code 长度限制：3-50 字符
- title 长度限制：1-100 字符
- description 长度限制：0-500 字符（可选）

**Product-Plan 关系**：
- 关系类型：一对多（1:N），一个 Product 可包含多个 Plan，一个 Plan 必须属于且仅属于一个 Product
- 强制归属：所有 Plan 必须有 product_id，不可为 NULL
- 级联约束：删除 Product 前必须先删除或移动其下所有 Plan
- 迁移支持：现有 Plan 可以迁移到其他 Product

**创建 Product**：
- Realm Admin 可创建新 Product，必须提供 code、title、description
- 创建时检查 code 在同一 Realm 内的唯一性和格式合规性

**编辑 Product**：
- Realm Admin 可编辑 title、description、enabled
- code 字段不可修改（作为唯一标识符）
- 更新 enabled 状态会影响其下所有 Plan 的可见性

**可见性规则**：
- Product 禁用时，其下所有 Plan 对新用户不可见
- 已订阅用户不受影响，可以继续使用
- Product 启用时，其下 Plan 恢复可见（前提是 Plan 本身也是 enabled）

**删除 Product**：
- 前置条件：Product 下没有 Plan，或所有 Plan 已被移动到其他 Product
- 如果 Product 下有 Plan，不允许删除，系统应提示用户先处理 Plan（删除或移动）
- 删除操作需二次确认，记录审计日志

**Product 与积分系统边界**：
- Product 在 Billing 领域中成为正式编目对象
- Plan 的业务归属切换为 Product，不再直接平铺在 Realm 下
- Points 领域在当前阶段仍以 Plan 级配置为主
- Product 级默认规则、Client App 差异化规则或多层优先级匹配，应由 points.md 在后续版本单独升级定义

**访问控制**：
- 查看 Product 列表：需要 billing.view 权限（Realm Admin）
- 创建 Product：需要 billing.manage 权限（Realm Admin）
- 编辑 Product：需要 billing.manage 权限（Realm Admin）
- 删除 Product：需要 billing.manage 权限（Realm Admin）
- 管理 Product 下的 Plan：需要 billing.manage 权限（Realm Admin）

**数据安全**：
- Product 属于 Realm，跨 Realm 不可见
- 所有 Product 创建、更新、删除操作记录审计日志（操作人、操作时间、操作内容）

### 4.2 关键状态与异常

- **Plan 归属冲突**：如果 Plan 已属于其他 Product，迁移时需处理归属变更
- **删除保护**：有 Plan 的 Product 不能删除，需先处理 Plan
- **禁用影响**：Product 禁用后其下 Plan 对新用户不可见，但已订阅用户不受影响

---

## 5. 功能需求

### 5.1 核心需求

- Realm Admin 可以创建 Product，必须提供 code、title、description，系统校验 code 格式和唯一性
- Realm Admin 可以编辑 Product 的 title、description、enabled，code 不可修改
- Realm Admin 可以删除 Product（仅当其下无 Plan 时）
- Realm Admin 可以查看 Realm 内所有 Product，列表按创建时间排序，支持按 enabled 状态筛选
- 每个 Product 显示其下的 Plan 数量
- 所有 Plan 必须归属到某个 Product
- Product 的 enabled 状态控制其下 Plan 的可见性

### 5.2 验收目标

- Realm Admin 能完成 Product 的创建、编辑、启用/禁用、删除全生命周期操作
- Plan 正确归属到 Product，支持按产品线组织套餐
- Product 禁用后其下 Plan 对新用户不可见，已订阅用户不受影响
- 有 Plan 的 Product 无法删除，系统给出明确提示
- 审计日志完整记录所有 Product 操作
- 现有 Plan 数据可迁移到默认 Product，向后兼容

---

## 6. API 相关约束

**适用性**: 适用

- 接口能力范围包括：Product 的创建、编辑、删除、列表查询、启用/禁用管理类接口
- 访问控制：所有接口需 Realm Admin 权限（billing.view / billing.manage），严格遵守 realm 隔离
- 删除保护：服务端必须校验 Product 下是否存在 Plan，存在时拒绝删除
- 数据变更可追溯：所有 Product 变更操作记录审计日志

---

## 7. 前端/交互约束

**适用性**: 适用

- 管理入口：Realm Admin 可在管理后台访问 Product 管理页面
- 列表视图：展示所有 Product 及其下 Plan 数量，支持按启用状态筛选，按创建时间排序
- 创建/编辑表单：code（创建后不可修改）、title、description、enabled
- 删除操作：需二次确认，如果 Product 下有 Plan 则显示错误提示和 Plan 数量
- 状态反馈：Product 启用/禁用变更时需提示对 Plan 可见性的影响范围
- Plan 管理视图：在 Product 详情下管理归属的 Plan

---

## 8. 已确认决策

### 8.1 已确认决策

- Product code 创建后不可修改，作为唯一标识符
- Product 禁用是软操作（不影响已订阅用户），不影响数据完整性
- 所有 Plan 必须归属到某个 Product，强制约束
- Points 是否升级为 Product 分层规则由 points.md 另行定义

---

## 9. 参考资料

- 用户故事：`docs/user-stories/billing/product-management.md`
- 相关 PRD：`docs/prd/billing/subscription.md`
- 相关 PRD：`docs/prd/billing/points.md`
- 权限管理：`docs/prd/auth/permissions.md`
