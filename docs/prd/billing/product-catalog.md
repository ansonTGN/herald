# Product 编目管理产品需求文档 (PRD)

**创建时间**: 2026-03-26
**状态**: Implemented
**优先级**: P1

---
## 1. 相关用户故事

> **说明**: 详细的用户故事和验收标准请查看 `../../user-stories/` 目录中的对应文件。

### 1.1 Product 管理用户故事

- 📄 [docs/user-stories/billing/product-management.md](/docs/user-stories/billing/product-management.md)
  - **[US-PR-001] 创建 Product** (P0): 作为 Realm Admin，我想要创建 Product，以便将相关的套餐组织在一起
  - **[US-PR-002] 编辑 Product** (P0): 作为 Realm Admin，我想要编辑 Product 信息，以便更新产品描述
  - **[US-PR-003] 查看 Product 列表** (P0): 作为 Realm Admin，我想要查看所有 Product 列表，以便了解产品目录结构
  - **[US-PR-004] 启用/禁用 Product** (P1): 作为 Realm Admin，我想要启用或禁用 Product，以便控制产品的可见性
  - **[US-PR-005] 删除 Product** (P1): 作为 Realm Admin，我想要删除不再需要的 Product，以便保持产品目录整洁
  - **[US-PR-006] 在 Product 下管理 Plan** (P0): 作为 Realm Admin，我想要在 Product 视图下管理 Plan，以便按产品线组织套餐

### 1.2 用户故事优先级汇总

| 优先级 | 用户故事数量 | 关键故事 |
|--------|------------|---------|
| P0 | 3 | 创建 Product、编辑 Product、查看 Product 列表、在 Product 下管理 Plan |
| P1 | 2 | 启用/禁用 Product、删除 Product |
| P2 | 0 | - |

---

## 2. 范围界定

### 2.1 包含功能

- ✅ Product 实体管理（创建、编辑、删除、查看）
- ✅ Product-Plan 层级关系管理
- ✅ Product 启用/禁用控制
- ✅ Product 排序（按创建时间）
- ✅ Plan 到 Product 的归属管理
- ✅ Product 删除约束（有 Plan 时不可删除）
- ✅ 前端 Product 管理页面
- ✅ 为后续 Points 扩展保留 Product 语义边界

### 2.2 不包含功能 (Out of Scope)

- ❌ **Product 的功能（features）管理** (原因: Product 是编目对象，功能由 Plan 或第三方应用管理)
- ❌ **Product 级别的订阅统计** (原因: 统计功能在后续阶段实现)
- ❌ **Product 级别的配额（quotas）管理** (原因: 采用简化模型，由第三方应用自行管理)
- ❌ **Product 之间的关联关系** (原因: Product 是独立实体，不支持父子或关联关系)
- ❌ **Product 模板** (原因: 首版不实现模板功能)

**注意**：
- ✅ **Points 边界**：本 PRD 只要求 Product 成为 Billing 编目层的正式对象；Points 是否升级为 Product 分层规则，由 `docs/prd/billing/points.md` 另行定义
- ✅ **Plan 归属**：所有 Plan 必须属于某个 Product，这是强制约束

### 2.3 依赖项

- ✅ **Realm 系统** (状态: 已实现) - Product 功能属于 Realm 级别
- ✅ **Billing Plan 系统** (状态: 部分实现) - Plan 需要归属到 Product
- ✅ **权限管理系统** (状态: 已实现) - Realm Admin 权限检查
- ⚠️ **Points 积分系统** (状态: 草稿) - 当前阶段仅需与新的 Product 编目语义保持兼容

---

## 3. 需求概述

### 3.1 功能描述

Product 编目管理是 Herald 系统为 Realm 提供的产品线组织能力。通过引入 Product 作为 Realm 下的正式业务对象，实现从 `Realm -> Plan` 的平面模型升级为 `Realm -> Product -> Plan` 的层次化模型。

**关键特性**：
- **层次化编目**：Product 作为 Plan 的上层组织对象
- **多产品线支持**：一个 Realm 可以运营多个产品线（如云存储、AI 服务、CRM）
- **灵活管理**：支持 Product 的创建、编辑、启用/禁用、删除
- **积分边界清晰**：Product 先作为 Billing 编目层正式对象落地，为后续 Points 扩展提供稳定语义基础
- **向后兼容**：现有 Plan 数据可以通过迁移归属到默认 Product

### 3.2 业务价值

**对 Realm Admin 的价值**：
- **清晰的产品组织**：按产品线分组管理套餐，提升管理效率
- **灵活的营销策略**：可以针对不同产品线组织套餐和定价策略
- **更好的用户体验**：用户可以看到按产品组织的套餐，选择更清晰

**对用户的价值**：
- **产品线清晰**：用户可以按产品线浏览和订阅套餐
- **发现相关服务**：同一产品下的套餐更容易被发现和比较

**对系统的价值**：
- **可扩展性**：为未来的产品级功能（如产品级统计、产品级权限）奠定基础
- **数据一致性**：强制 Plan 归属 Product，避免数据孤岛

---

## 4. 当前实现状态

| 功能模块 | 状态 | 备注 |
|---------|------|------|
| Product 数据模型 | ❌ | 待实施 |
| Product-Plan 关联 | ❌ | 待实施 |
| Product CRUD API | ❌ | 待实施 |
| Product 列表查询 API | ❌ | 待实施 |
| Plan 的 product_id 字段 | ❌ | 待实施（需迁移现有数据） |
| 前端 Product 管理页面 | ❌ | 待实施 |
| Points 与 Product 的语义对齐 | ⚠️ | 当前阶段仅要求保持兼容，分层规则不在本 PRD 落地 |

---

## 5. 功能需求

### 5.1 Product 实体定义

Product 是 Realm 下的正式业务对象，具有以下特征：

**基本属性**：
- `id`: UUID v7（主键）
- `realm_id`: 所属 Realm（外键到 realm 表）
- `code`: 产品代码（Realm 内唯一，如 "ai-services"）
- `title`: 产品标题（显示名称，如 "AI 服务"）
- `description`: 产品描述
- `enabled`: 是否启用（默认 true）
- `created_at`: 创建时间
- `updated_at`: 更新时间

**命名规范**：
- `code` 只能包含字母、数字、横线（-）和下划线（_）
- `code` 在同一 Realm 内必须唯一
- `code` 长度限制：3-50 字符
- `title` 长度限制：1-100 字符
- `description` 长度限制：0-500 字符（可选）

### 5.2 Product 与 Plan 的关系

**关系类型**：一对多（1:N）
- 一个 Product 可以包含多个 Plan
- 一个 Plan 必须属于且仅属于一个 Product

**业务规则**：
1. **强制归属**：所有 Plan 必须有 `product_id` 字段
2. **非空约束**：Plan 的 `product_id` 不能为 NULL
3. **级联约束**：删除 Product 前必须先删除或移动其下所有 Plan
4. **迁移支持**：现有 Plan 可以迁移到其他 Product

### 5.3 Product 生命周期管理

#### 5.3.1 创建 Product

**功能点**：
- Realm Admin 可以创建新的 Product
- 必须提供 `code`、`title`、`description`
- `enabled` 有默认值
- 创建时检查 `code` 唯一性

**验证规则**：
- `code` 唯一性检查（同一 Realm 内）
- `code` 格式验证（字母、数字、横线、下划线）
- `code` 长度验证（3-50 字符）
- `title` 长度验证（1-100 字符）
- `description` 长度验证（0-500 字符）

#### 5.3.2 编辑 Product

**功能点**：
- Realm Admin 可以编辑 Product 的 `title`、`description`、`enabled`
- `code` 字段不可修改（作为唯一标识符）
- 更新 `enabled` 状态会影响其下所有 Plan 的可见性

**可见性规则**：
- Product 禁用（`enabled = false`）时，其下所有 Plan 对新用户不可见
- 已订阅用户不受影响，可以继续使用
- Product 启用（`enabled = true`）时，其下 Plan 恢复可见（前提是 Plan 本身也是 enabled）

#### 5.3.3 删除 Product

**前置条件**：
- Product 下没有 Plan
- 或者所有 Plan 已被移动到其他 Product

**删除限制**：
- 如果 Product 下有 Plan，不允许删除
- 系统应提示用户先处理 Plan（删除或移动）

**删除流程**：
1. 检查 Product 下是否有 Plan
2. 如果有 Plan，返回错误并提示 Plan 数量
3. 如果没有 Plan，执行删除
4. 记录删除审计日志

#### 5.3.4 查看 Product 列表

**功能点**：
- Realm Admin 可以查看 Realm 内所有 Product
- 列表默认按创建时间排序
- 支持按 `enabled` 状态筛选
- 每个 Product 显示其下的 Plan 数量

**显示信息**：
- Product 基本信息（code, title, description）
- Plan 数量
- 启用状态
- 创建和更新时间

### 5.4 Product 与积分系统边界

本 PRD 对 Points 的约束仅限于业务边界，不在这里定义 Product 分层积分规则。

当前要求如下：

1. Product 在 Billing 领域中成为正式编目对象。
2. Plan 的业务归属切换为 Product，不再直接平铺在 Realm 下。
3. Points 领域在当前阶段仍以 Plan 级配置为主。
4. 如果未来需要 Product 级默认规则、Client App 差异化规则或多层优先级匹配，应由 `docs/prd/billing/points.md` 在后续版本单独升级定义。

换言之，Product 编目管理的当前目标是先解决“产品线编目”和“Plan 归属语义”问题，而不是同步改写 Points 的整套分层规则系统。

### 11.1 权限要求

| 操作 | 需要权限 | 说明 |
|------|---------|------|
| 查看 Product 列表 | `billing.view` | Realm Admin |
| 创建 Product | `billing.manage` | Realm Admin |
| 编辑 Product | `billing.manage` | Realm Admin |
| 删除 Product | `billing.manage` | Realm Admin |
| 管理 Product 下的 Plan | `billing.manage` | Realm Admin |

### 11.2 数据安全

**Realm 隔离**：
- Product 属于 Realm，跨 Realm 不可见
- API 自动检查 Realm 权限

**删除保护**：
- 有 Plan 的 Product 不能删除
- 删除操作需要二次确认

**审计日志**：
- 所有 Product 创建、更新、删除操作记录审计日志
- 日志包含：操作人、操作时间、操作内容

---

## 6. API 相关约束

**状态**: 必填

- 仅说明计费、套餐、积分、支付配置、订阅变更或 webhook 处理的能力边界，不在 PRD 中列出端点、schema 或状态码细节。
- 必须遵守 realm 隔离、管理员权限、金额与积分变更可追溯、回调幂等和失败补偿要求。
- 与支付平台、积分账本、订阅系统的详细契约应下沉到技术设计、接口说明或实现代码。

---

## 7. 前端/交互约束

**状态**: 必填

- 仅保留管理入口、关键操作路径、筛选/查看/变更的交互约束和状态反馈，不写组件实现、数据层封装或代码结构。
- 计费与积分场景必须突出金额/积分变化、变更影响范围、不可逆风险提示和回调同步中的状态说明。

---

## 8. 技术设计承接

**状态**: 必填

- 接口细节、数据库结构、迁移策略、类型定义、调度方案、SDK 设计和实现步骤，应在 `docs/design/`、`.ai/design/`、接口说明或代码中承接。
- 如历史实现已经存在，应以现有设计文档、OpenAPI、迁移文件和代码为依据补充，不回写到 PRD 正文。

---

## 9. 相关文件索引

- **用户故事**: [docs/user-stories/billing/product-management.md](/docs/user-stories/billing/product-management.md)
- **Billing PRD**: [docs/prd/billing/subscription.md](billing.md) - Billing 订阅计费产品需求文档
- **Points PRD**: [docs/prd/billing/points.md](points.md) - Points 积分系统产品需求文档（当前仍以 Plan 级配置为主）
- **用户故事**: [docs/user-stories/billing/subscription.md](/docs/user-stories/billing/subscription.md) - Billing 原有用户故事
- **后端开发指南**: [../../spec/backend/development.md](/spec/backend/development.md)
- **前端开发指南**: [../../spec/frontend/development.md](/spec/frontend/development.md)
- **权限管理**: [docs/prd/auth/permissions.md](/docs/prd/auth/permissions.md)

---

## 10. 参考资料

- 用户故事：请参考对应的 `docs/user-stories/` 文档。
- 相关 PRD：请参考 `docs/prd/index.md` 与相邻业务域文档。
- 技术设计：如需实现细节，请补充或引用 `docs/design/`、`.ai/design/` 或接口说明。
- 本文档已按最新 `/t-prd` 分层要求收敛：PRD 仅保留业务语义、范围、规则、约束与验收目标。

