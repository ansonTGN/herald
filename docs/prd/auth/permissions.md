# 权限与角色管理产品需求文档 (PRD)

**创建时间**: 2025-01-10
**最后更新**: 2026-05-22
**状态**: 🚧 Partially Implemented

---
## 1. 相关用户故事

> **说明**: 详细的用户故事和验收标准请查看 `../../user-stories/` 目录中的对应文件。

### 1.1 Realm Admin 用户故事

- 📄 [docs/user-stories/core/realm-admin.md](/docs/user-stories/core/realm-admin.md)
  - **[US-RA-001] Realm 隔离访问** (P0): 作为 Realm Admin，我只能访问自己 Realm 的资源
  - **[US-RA-002] 角色定义管理** (P0): 作为 Realm Admin，我想要管理角色定义
  - **[US-RA-003] 权限定义管理** (P0): 作为 Realm Admin，我想要管理权限定义
  - **[US-RA-004] 为角色分配权限** (P0): 作为 Realm Admin，我想要为角色分配权限
  - **[US-RA-005] 查看角色权限** (P0): 作为 Realm Admin，我想要查看角色的权限
  - **[US-RA-006] 用户角色分配** (P0): 作为 Realm Admin，我想要为用户分配角色
  - **[US-RA-007] 权限策略管理** (P0): 作为 Realm Admin，我想要管理权限策略
  - **[US-RA-009] 权限层级验证** (P0): 作为 Realm Admin，系统应自动应用权限层级规则
  - **[US-RA-010] 查看 Dashboard 用户活跃概览** (P1)
  - **[US-RA-011] 查看 Dashboard 认证趋势图** (P1)
  - **[US-RA-012] 通过 Dashboard 快捷导航跳转** (P1)

### 1.2 内置保护用户故事

- 📄 [docs/user-stories/core/builtin-protection.md](/docs/user-stories/core/builtin-protection.md)
  - **[US-BP-001] 默认角色和权限保护** (P0): 默认的角色和权限不能被删除或修改

### 1.3 审计日志用户故事

- 📄 [docs/user-stories/core/audit.md](/docs/user-stories/core/audit.md)
  - **[US-AU-001] 查看 Realm 审计日志** (P0)
  - **[US-AU-002] 按条件筛选审计日志** (P0)
  - **[US-AU-003] 查看审计日志详情** (P1)
  - **[US-AU-004] 查看 Admin Realm 审计日志** (P0)
  - **[US-AU-005] 系统自动记录核心操作** (P0)

### 1.4 用户故事优先级汇总

| 优先级 | 用户故事数量 | 关键故事 |
|--------|------------|---------|
| P0 | 13 | Realm 隔离访问、角色定义管理、权限定义管理、为角色分配权限、查看角色权限、用户角色分配、权限策略管理、权限层级验证、默认角色和权限保护、审计日志查看/筛选/Admin Realm/自动记录 |
| P1 | 4 | Dashboard 活跃概览、认证趋势图、快捷导航、审计详情 |
| P2 | 0 | - |

---

## 2. 范围界定

### 2.1 包含功能

- ✅ RBAC 元数据层管理（角色定义、权限定义、角色权限关联）
- ✅ 自研权限运行时层（用户角色分配、资源访问策略）
- ✅ 两层架构（PostgreSQL + Redis 缓存）
- ✅ 前端角色管理页面
- ✅ 前端权限管理页面
- ✅ 权限检查集成（Service 层集成）
- ✅ 默认角色（`realm-admin`、`user`）
- ✅ 默认权限定义（见下方权限清单）
- 🚧 菜单级和按钮级前端权限控制（对齐后端权限）

### 2.2 不包含功能 (Out of Scope)

- ❌ **权限策略可视化** (原因: 前端没有权限策略可视化工具)
- ❌ **权限冲突检测** (原因: 没有自动检测权限冲突的功能)
- ❌ **通配符或全局隐式权限** (原因: 所有权限必须精确匹配，不引入 `*` 或 `admin` 动作)
- ❌ **历史数据迁移** (原因: 项目尚未上线)

### 2.3 依赖项

- ✅ **用户认证系统** (状态: 已实现) - 提供登录和会话管理
- ✅ **Realm 系统** (状态: 已实现) - 权限属于 Realm 级别
- ✅ **Client App 系统** (状态: 已实现) - 权限与 Client App 关联
- ✅ **Redis 缓存** (状态: 已实现) - 提升权限检查性能（P95 < 50ms）

---

## 3. 需求概述

Herald 系统实现了完整的 RBAC (基于角色的访问控制) 权限管理体系，采用**两层权限控制**：

1. **RBAC 元数据层** - 定义角色、权限及其关联关系
2. **自研权限运行时层** - 实际权限检查和用户角色分配（Redis 缓存 + PostgreSQL）

**架构说明**: RBAC 元数据层用于管理（如创建角色定义），自研运行时层用于运行时权限检查（如用户是否有权限访问某个资源）。

### 3.1 权限模型

权限格式为 `resource.action`，遵循以下规则：

- `resource` 必须精确匹配（不支持通配符）。
- `manage` 是唯一具有向下隐含能力的 action，覆盖同一 resource 下的 `view`、`create` 和 `manage`。
- `create` 仅匹配自身，不隐含 `view`。
- `view` 仅匹配自身。
- 所有层级规则仅在**同一 resource 内**生效。
- 不使用 `admin` action，不引入特殊 `resource:action` 组合。
- 不引入隐式全局权限。

### 3.2 Principal Types

| Principal Type | 标识 | 说明 |
|---------------|------|------|
| User | `user` | 已登录用户 |
| API Key | `api_key` | API Key 凭证 |
| Client | `client` | OAuth 客户端应用 |

---

## 4. 当前实现状态

| 功能模块 | 状态 | 备注 |
|---------|------|------|
| 后端实体层 | ✅ | Role、Permission、RolePermission 实体定义 |
| 后端 Repository 层 | ✅ | 角色、权限数据库操作接口和实现 |
| 后端 Service 层 | ✅ | 角色、权限业务逻辑和权限检查 |
| 后端 HTTP API | ✅ | 角色、权限 CRUD RESTful API |
| 权限运行时 | ✅ | Redis 缓存 + PostgreSQL 存储的两层架构 |
| 前端数据层 | ✅ | 角色、权限 API 调用函数 |
| 前端类型定义 | ✅ | TypeScript 类型定义 |
| 前端角色管理页面 | ✅ | 角色列表、创建、编辑、删除 |
| 前端权限管理页面 | ✅ | 权限列表、创建、编辑、删除 |
| 权限检查集成 | ✅ | Service 层集成权限检查 |
| 演示测试 | ✅ | permission-management-demo.e2e.ts |
| 移除 `admin` action | ❌ | `realm.admin` 及 `realm.admin:{realm_id}` 待清理 |
| 新增 `dashboard.view` | ❌ | Dashboard 独立权限 |
| 新增 `audit.view` | ❌ | Audit Log 独立权限 |
| 新增 `api_keys.view` | ❌ | API Key 只读权限 |
| `realm.create` → `realm.manage` | ❌ | 统一 Realm 管理权限 |
| 前端菜单/按钮权限对齐 | ❌ | Sidebar 和 QuickNav 按目标权限显示/隐藏 |

**说明**：
- 权限系统采用两层架构：RBAC 元数据层用于定义角色和权限，自研运行时层用于实际权限检查
- 权限检查通过 Policy trait 实现，Service 层使用 `ensure_policy` 辅助函数进行权限验证
- 权限运行时使用 Redis 缓存提升性能，P95 < 50ms

---

## 5. 功能需求

### 5.1 内置角色

| 角色 | 技术标识 | 说明 |
|------|----------|------|
| Realm Admin | `realm-admin` | Realm 管理员，拥有该 Realm 的完整管理权限 |
| User | `user` | 普通用户，仅拥有基本权限 |

### 5.2 realm-admin 权限清单

#### 所有 Realm

| 权限项 | 资源 | 动作 | 说明 |
|--------|------|------|------|
| dashboard.view | dashboard | view | 查看 Dashboard 统计 |
| realm.view | realm | view | 查看 Realm 信息 |
| users.view | users | view | 查看用户 |
| users.manage | users | manage | 用户管理 |
| clients.view | clients | view | 查看客户端应用 |
| clients.manage | clients | manage | 客户端应用管理 |
| roles.view | roles | view | 查看角色 |
| roles.manage | roles | manage | 角色管理 |
| permissions.view | permissions | view | 查看权限 |
| permissions.manage | permissions | manage | 权限管理 |
| policies.view | policies | view | 查看策略 |
| policies.manage | policies | manage | 策略管理 |
| settings.view | settings | view | 查看设置 |
| settings.manage | settings | manage | 设置管理 |
| api_keys.view | api_keys | view | 查看 API Key 列表和详情 |
| api_keys.manage | api_keys | manage | API Key 创建、更新、删除、轮换 |
| billing.view | billing | view | 查看账单、订阅历史、支付配置 |
| billing.manage | billing | manage | 账单管理、支付 Provider 配置管理 |
| points.view | points | view | 查看积分、积分包、积分规则 |
| points.manage | points | manage | 积分管理、积分包管理、Provider 映射管理 |
| audit.view | audit | view | 查看审计日志列表和详情 |

#### Admin Realm 额外权限

| 权限项 | 资源 | 动作 | 说明 |
|--------|------|------|------|
| realm.manage | realm | manage | Realm 创建、更新、删除（仅 admin realm） |

### 5.3 user 权限清单

| 权限项 | 资源 | 动作 | 说明 |
|--------|------|------|------|
| points.view | points | view | 查看自己的积分余额 |

> 用户修改自己的 profile 和 password 在业务逻辑层处理，不需要权限检查。

### 5.4 权限层级

| 已授予的 action | 可通过的请求 action | 说明 |
|---|---|---|
| `manage` | `view`、`create`、`manage` | 唯一的层级 action，向下覆盖 |
| `create` | `create` | 仅自身 |
| `view` | `view` | 仅自身 |

**关键规则**：

1. `manage` 是唯一具有向下隐含能力的 action。授予某资源 `manage` 后，无需再单独授予该资源的 `view` 或 `create`。
2. `create` 不隐含 `view`。如需同时创建和查看，必须分别授予 `create` 和 `view`，或直接授予 `manage`。
3. 所有层级规则仅在**同一 resource 内**生效。`users.manage` 不会授予 `clients.view`。
4. 不使用 `admin` action，不引入特殊 `resource:action` 组合（如 `realm.admin:{realm_id}`）。

### 5.5 已移除的权限

以下权限已废弃，不再初始化和使用：

| 权限项 | 原用途 | 替代方案 |
|--------|--------|---------|
| `realm.admin` | 宽泛的管理端权限 | 各模块具体的 `resource.view` / `resource.manage` |
| `realm.create` | Realm 创建 | `realm.manage`（统一管理权限） |
| `realm.admin:{realm_id}` 特殊策略 | 判断是否能进入管理端 | 具体权限检查 + `Identity::has_access_to_realm` |

### 5.6 前端菜单权限映射

| 菜单 | 权限 |
|-------|------|
| Dashboard | `dashboard.view` |
| Realms | `realm.view` |
| Clients | `clients.view` |
| Users | `users.view` |
| Permissions | `permissions.view` |
| Roles | `roles.view` |
| API Keys | `api_keys.view` |
| Products | `billing.view` |
| Payment Providers | `billing.view` |
| Subscription Plans | `billing.view` |
| Points Packages | `points.view` |
| Points Rules | `points.view` |
| Invoices | `billing.view` |
| Subscription History | `billing.view` |
| Points Wallets | `points.view` |
| Audit Log | `audit.view` |
| Settings | `settings.view` |

### 5.7 按钮级权限

| 页面 | 查看 | 新增/编辑/删除 |
|------|------|---------------|
| Realms | `realm.view` | `realm.manage` |
| Clients | `clients.view` | `clients.manage` |
| Users | `users.view` | `users.manage` |
| Permissions | `permissions.view` | `permissions.manage` |
| Roles | `roles.view` | `roles.manage` |
| Role policy assignment | `roles.view` | `roles.manage` |
| User role assignment | `users.view` | `roles.manage` |
| API Keys | `api_keys.view` | `api_keys.manage` |
| Products / Plans / Invoices / Providers | `billing.view` | `billing.manage` |
| Points Packages / Rules / Wallets | `points.view` | `points.manage` |
| Settings | `settings.view` | `settings.manage` |

### 5.8 Realm 操作权限

| 操作 | 权限 |
|------|------|
| List realms | `realm.view` |
| View realm detail | `realm.view` |
| Create realm | `realm.manage` in admin realm |
| Update realm metadata | `settings.manage` for own realm, or `realm.manage` in admin realm |
| Delete realm | `realm.manage` in admin realm |

---

## 6. API 相关约束

**状态**: 必填

- 每个 API 端点检查具体的 `resource.action` 权限，不使用宽泛的 `realm.admin` 或特殊策略。
- Realm 隔离：权限属于 Realm 级别，跨 Realm 访问必须拒绝。
- 权限层级遵循 5.4 节规则，`manage` 隐含 `view` 和 `create`。
- 只读操作（list、get）检查 `view` 权限；写操作（create、update、delete）检查 `manage` 权限。
- Realm 创建、更新、删除在 admin realm 内检查 `realm.manage`。
- 必须遵守 realm 隔离、权限边界、凭证脱敏和幂等要求。

---

## 7. 前端/交互约束

**状态**: 必填

- 管理端侧边栏菜单根据用户权限动态显示/隐藏，每个菜单项对应明确的 `resource.view` 权限。
- Dashboard 快捷导航根据权限过滤，避免导向无权限页面。
- 按钮级权限控制新增、编辑、删除操作；仅有 `view` 权限时管理按钮不可用。
- Settings 页面：无 `settings.view` 时不可访问；有 `settings.view` 但无 `settings.manage` 时表单只读。
- API Keys 页面：有 `api_keys.view` 但无 `api_keys.manage` 时能查看列表，管理按钮不可用。
- 前端不做 `*` 或其他前端特例判断，权限检查结果以后端为准。

---

## 8. 技术设计承接

**状态**: 必填

- 后端 RBAC 初始化、权限检查中间件、前端权限常量和测试的具体改动方案，参见 `.ai/future/permission_1.md`。
- 接口细节、数据库结构、迁移策略、类型定义和实现步骤，应在 `docs/design/` 或 `.ai/design/` 中承接。
- 如历史实现已经存在，应以现有设计文档、OpenAPI、迁移文件和代码为依据补充，不回写到 PRD 正文。

---

## 9. 相关文件索引

- 相关实现文件请以本功能对应的 `backend/`、`frontend/`、`demo/` 目录和现有设计文档为准。
- 若需补充精确文件清单，应在技术设计文档中维护，避免在 PRD 中混入实现级细节。

---

## 10. 参考资料

- 前端开发指南: `../../spec/frontend/development.md`
- Realm Settings 文档: [docs/prd/core/realm-settings.md](/docs/prd/core/realm-settings.md)
- OAuth 文档: [docs/prd/auth/oauth.md](/docs/prd/auth/oauth.md)
- Dashboard 文档: [docs/prd/core/dashboard.md](/docs/prd/core/dashboard.md)
- Audit 文档: [docs/prd/core/audit.md](/docs/prd/core/audit.md)
- 权限修复方案: `.ai/future/permission_1.md`
