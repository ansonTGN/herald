# Subscription History 订阅变更历史产品需求文档 (PRD)

**创建时间**: 2026-03-13
**状态**: 📝 Draft

---
## 1. 相关用户故事

> **说明**: 详细的用户故事和验收标准请查看 `../../user-stories/` 目录中的对应文件。

### 1.1 Realm Admin 用户故事

- 📄 [docs/user-stories/06-billing-user-stories.md](/docs/user-stories/06-billing-user-stories.md#故事-7查看订阅变更历史-us-bi-007)
  - **[US-BI-007] 查看订阅变更历史** (P1): 作为 Realm Admin，我想要查看所有用户的订阅变更历史，以便监控和管理订阅情况

### 1.2 Regular User 用户故事

- 📄 [docs/user-stories/06-billing-user-stories.md](/docs/user-stories/06-billing-user-stories.md#故事-8查看自己的订阅变更历史-us-bi-008)
  - **[US-BI-008] 查看自己的订阅变更历史** (P1): 作为 Regular User，我想要查看我的订阅变更历史，以便了解订阅的变更轨迹

### 1.3 用户故事优先级汇总

| 优先级 | 用户故事数量 | 关键故事 |
|--------|------------|---------|
| P0 | 0 | - |
| P1 | 2 | 查看订阅变更历史（Realm Admin）、查看自己的订阅变更历史（Regular User） |
| P2 | 0 | - |

---

## 2. 范围界定

### 2.1 包含功能

- ✅ 查询单个订阅的变更时间线
- ✅ 按用户、套餐、时间等维度查询订阅历史（Realm Admin）
- ✅ 显示变更类型（创建、升级、降级、取消、续费等）
- ✅ 显示变更前后的状态对比
- ✅ 前端历史记录页面（全局查询和单订阅详情）
- ✅ 权限控制（Realm Admin 可查看所有历史，Regular User 只能查看自己的）

### 2.2 不包含功能 (Out of Scope)

- ❌ **支付事件历史** (原因: 支付事件由支付平台处理，Herald 不负责记录)
- ❌ **导出历史记录** (原因: 属于 P2 功能，暂不实现)
- ❌ **历史记录审计日志** (原因: 可选扩展功能，暂不实现)
- ❌ **历史记录统计分析** (原因: 属于计费统计报表功能，单独规划)

---

## 3. 需求概述

### 3.1 功能描述

Subscription History 功能提供了订阅变更历史记录的查询和展示能力。通过记录每次订阅变更的详细信息（包括变更类型、操作者、变更前后状态等），帮助 Realm Admin 监控和管理订阅情况，同时帮助 Regular User 了解自己的订阅变更轨迹。

### 3.2 目标用户

- **Realm Admin**：查看和管理 Realm 内所有用户的订阅变更历史
- **Regular User**：查看自己的订阅变更历史

### 3.3 关键特性

- **完整的变更时间线**：记录从订阅创建到当前的所有变更事件
- **多维度筛选**：支持按用户、套餐、变更类型、时间等维度筛选
- **变更前后对比**：清晰展示每次变更的前后状态
- **权限控制**：Realm Admin 可查看所有历史，Regular User 只能查看自己的
- **变更类型丰富**：支持创建、升级、降级、取消、过期、续费、激活、计费周期变更等多种变更类型

### 3.4 业务价值

- **透明度**：用户可以清晰了解订阅的变更历史和原因
- **可追溯性**：Realm Admin 可以追踪任何订阅变更的来源和时间
- **问题排查**：当出现订阅异常时，可以通过历史记录快速定位问题
- **运营洞察**：通过历史数据分析用户的订阅行为模式

---

## 4. 当前实现状态

| 功能模块 | 状态 | 备注 |
|---------|------|------|
| 后端 API | ❌ 未实现 | 需要实现历史查询接口 |
| 前端页面 | ❌ 未实现 | 需要实现历史记录展示 |
| 数据模型 | ❌ 未实现 | 需要新建 subscription_history 表 |
| 历史记录创建 | ❌ 未实现 | 需要在订阅变更时创建历史记录 |

---

## 5. 功能需求

### 3.1 单订阅历史

#### 功能描述
展示单个订阅从创建到当前的所有变更事件，按时间倒序排列。

#### 需求细节
| Event ID | 事件唯一标识符 | evt_1234567890 |
| Event Type | 变更类型 | created, upgraded, downgraded, canceled, renewed, reactivated |
| Timestamp | 变更时间 | 2025-01-15 10:30:00 UTC |
| Actor | 操作者 | user@example.com, system |
| Changes | 变更详情 | Plan: basic → pro |
| Previous State | 变更前状态 | { "status": "active", "plan": "basic" } |
| New State | 变更后状态 | { "status": "active", "plan": "pro" } |

#### 变更类型定义

| 类型 | 说明 | 触发场景 |
|------|------|---------|
| `created` | 创建订阅 | 用户首次订阅套餐 |
| `upgraded` | 升级套餐 | 用户从低级套餐升级到高级套餐 |
| `downgraded` | 降级套餐 | 用户从高级套餐降级到低级套餐 |
| `canceled` | 取消订阅 | 用户主动取消订阅 |
| `expired` | 订阅过期 | 订阅因未续费而过期 |
| `renewed` | 续费订阅 | 订阅成功续费 |
| `reactivated` | 激活订阅 | 已取消的订阅重新激活 |
| `billing_period_changed` | 计费周期变更 | 从月付改为年付或反之 |

### 3.2 全局历史查询（Realm Admin）

#### 功能描述
Realm Admin 可以查询 Realm 内所有订阅的历史记录，支持多维度筛选和分页。

#### 筛选条件

| 筛选维度 | 字段名 | 类型 | 示例 |
|---------|--------|------|------|
| 用户 | `user_id` | UUID | 123e4567-e89b-12d3-a456-426614174000 |
| 套餐 | `plan_id` | UUID | 987e6543-e21b-43d3-b456-426614174999 |
| 变更类型 | `event_type` | enum | created, upgraded, canceled |
| 时间范围 | `from_date`, `to_date` | datetime | 2025-01-01, 2025-01-31 |
| 订阅状态 | `subscription_status` | enum | active, canceled, past_due |

#### 分页参数
| `page` | 页码 | 1 |
| `page_size` | 每页数量 | 20 |
| `sort_by` | 排序字段 | timestamp |
| `sort_order` | 排序方向 | desc |

### 3.3 权限控制

| 角色 | 可访问范围 | 说明 |
|------|----------|------|
| Realm Admin | Realm 内所有订阅历史 | 可查看和筛选所有用户的订阅变更 |
| Regular User | 仅限自己的订阅历史 | 只能查看自己订阅的变更记录 |

### 8.1 性能要求

| 指标 | 要求 |
|------|------|
| 单订阅历史查询响应时间 | < 500ms |
| 全局历史查询响应时间（分页） | < 1000ms |
| 支持的历史记录数量 | 无限制（基于分页） |

### 8.2 数据一致性

- 订阅变更时必须同步创建历史记录
- 历史记录一旦创建不可修改
- 确保变更前后的状态准确性

### 8.3 安全性

- Realm Admin 可查看所有历史记录
- Regular User 只能查看自己的历史记录
- 敏感信息（如支付详情）不记录在历史中

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

### 13.1 后端文件（待创建）

| 文件路径 | 说明 | 状态 |
|---------|------|------|
| `backend/core/src/entity/subscription_history.rs` | Entity 模型 | ❌ 未创建 |

### 13.2 前端文件（待创建）

| 文件路径 | 说明 | 状态 |
|---------|------|------|
| `frontend/src/routes/$realmId/manage/subscription-history.tsx` | 全局历史页面 | ❌ 未创建 |
| `frontend/src/routes/$realmId/account/subscription/$subscriptionId/history.tsx` | 单订阅历史页面 | ❌ 未创建 |
| `frontend/src/components/billing/subscription-history-list.tsx` | 历史列表组件 | ❌ 未创建 |
| `frontend/src/components/billing/subscription-history-filter.tsx` | 筛选组件 | ❌ 未创建 |
| `frontend/src/components/billing/subscription-timeline.tsx` | 时间线组件 | ❌ 未创建 |
| `frontend/src/components/billing/history-event-badge.tsx` | 事件标签组件 | ❌ 未创建 |

### 13.3 测试文件（待创建）

| 文件路径 | 说明 | 状态 |
|---------|------|------|
| `backend/test-support/tests/subscription-history-scenarios.md` | 后端场景测试 | ❌ 未创建 |
| `demo/e2e/subscription-history.spec.ts` | Demo/E2E 测试 | ❌ 未创建 |

---

## 10. 参考资料

- **相关 PRD**: `docs/prd/billing.md`
- **相关用户故事**: `docs/user-stories/06-billing-user-stories.md`
- **现有订阅模型**: `backend/core/src/entity/subscription.rs`
- **现有套餐模型**: `backend/core/src/entity/plan.rs`

