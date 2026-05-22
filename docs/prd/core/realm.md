# Realm 管理产品需求文档 (PRD)

**创建时间**: 2025-01-01
**状态**: Partially Implemented

---
## 1. 相关用户故事

> **说明**: 详细的用户故事和验收标准请查看 `../../user-stories/` 目录中的对应文件。

### 1.1 Admin Realm 用户故事

- 📄 [docs/user-stories/core/admin-realm.md](/docs/user-stories/core/admin-realm.md)
  - **[US-AR-001] 创建 Realm** (P0): 作为 Admin Realm 管理员，我想要创建新的 Realm，以便为不同组织提供独立的认证服务
  - **[US-AR-002] 查看 Realm 列表** (P0): 作为 Admin Realm 管理员，我想要查看所有 Realm，以便管理系统中的组织
  - **[US-AR-003] 查看 Realm 详情** (P1): 作为 Admin Realm 管理员，我想要查看 Realm 详情，以便了解 Realm 的配置信息
  - **[US-AR-004] Realm 创建权限控制** (P0): 作为 Admin Realm 管理员，只有我有 realm.create 权限，Realm Admin 无法创建 Realm

### 1.2 Realm Admin 用户故事

- 📄 [docs/user-stories/core/realm-admin.md](/docs/user-stories/core/realm-admin.md)
  - **[US-RA-001] Realm 隔离访问** (P0): 作为 Realm Admin，我想要只能访问自己 Realm 的资源，以便保证数据隔离

### 1.3 用户故事优先级汇总

| 优先级 | 用户故事数量 | 关键故事 |
|--------|------------|---------|
| P0 | 3 | 创建 Realm、查看 Realm 列表、Realm 隔离访问 |
| P1 | 1 | 查看 Realm 详情 |
| P2 | 0 | - |

---

## 2. 范围界定

### 2.1 包含功能

- ✅ 多租户隔离（完全的数据和权限隔离）
- ✅ 通过 URL 中的 realm_id 参数进行上下文切换
- ✅ Realm 创建功能
- ✅ Realm 列表查看
- ✅ 基于 realm 的数据隔离和权限控制
- ✅ 前端路由设计（使用 `/$realmId/*` 路径）
- ✅ 认证集成（登录时需要指定 realm_id）
- ✅ **多 Realm 导航访问**（新增）
  - 每个 Realm 拥有独立的 UI 界面
  - 通过 URL 路径 `/$realmId/*` 访问特定 Realm 的管理界面
  - Admin Realm 管理员可以切换到其他 Realm 验证配置

### 2.2 不包含功能 (Out of Scope)

- ❌ **Realm 删除功能** (原因: 数据库不支持级联删除，删除 realm 会导致数据孤立)
- ❌ **Realm 详情页** (原因: 前端没有 realm 信息展示和编辑页面)
- ❌ **Realm 列表独立页面** (原因: 前端没有专门的 realm 列表管理页面)
- ❌ **Realm 编辑功能** (原因: Realm 创建后不允许修改关键配置)

### 2.3 依赖项

- ✅ **用户认证系统** (状态: 已实现) - 提供登录和会话管理
- ✅ **权限管理系统** (状态: 已实现) - 提供基于 Realm 的权限检查
- ✅ **数据库系统** (状态: 已实现) - PostgreSQL 数据存储
- ❌ **级联删除支持** (状态: 未实现) - 数据库外键约束不支持级联删除

---

## 3. 需求概述

Realm（域）是 Herald 系统中的多租户隔离单位，每个用户、客户端应用、角色、配置都属于一个特定的 realm。本文档描述 Realm 的增加、切换和查看等功能需求。

**关键特性**：
- 完全的多租户隔离
- 通过 URL 中的 realm_id 参数进行上下文切换
- 基于 realm 的数据隔离和权限控制
- Realm 管理功能（创建、查看、编辑）

**注意**：
- ❌ 当前项目**不提供** realm 删除功能
- ❌ 当前数据库**不支持**级联删除

---

## 4. 当前实现状态

### 2.1 已实现功能

- ✅ **后端实体层**：Realm 实体定义 (`core/src/entity/realm.rs`, `core/src/domain/realm/mod.rs`)
- ✅ **后端 Repository 层**：Realm 数据库操作接口和实现 (`RealmRepository` trait, `PostgresRealmRepository`)
- ✅ **后端 Service 层**：Realm 业务逻辑 (`RealmService` trait, `RealmServiceImpl`)
- ✅ **后端 HTTP API**：RESTful API 接口 (`api/src/application/http/realm/mod.rs`)
- ✅ **前端数据层**：Realm API 调用函数 (`frontend/src/data/realms.ts`)
- ✅ **前端类型定义**：TypeScript 类型定义 (`frontend/src/lib/types/realm.ts`)
- ✅ **前端路由设计**：使用 `/$realmId/*` 路径进行 realm 隔离
- ✅ **前端认证集成**：登录时需要指定 realm_id

### 2.2 未实现功能

- ❌ **Realm 详情页**：前端没有 realm 信息展示和编辑页面
- ❌ **Realm 列表独立页面**：前端没有专门的 realm 列表管理页面

**说明**：
- Realm 删除功能**已禁用**，Repository 和 Service 层都会返回错误 "Realm deletion is not supported"
- 原因：数据库没有级联删除外键约束，删除 realm 会导致 client_app、roles、permissions 等表的数据孤立
- **认证实现**：HTTP 层已通过 `require_session` 从请求的 cookie 中获取 session（token 为 "X-Auth"），并从 session 构建 `Identity` 对象
- **权限实现**：✅ Service 层已实现基于 `RealmPolicy` 的权限检查，使用 `ensure_policy` 辅助函数
- **角色说明**：权限检查完全通过 RBAC Policy 实现，无需在 Identity 中存储角色信息

---

## 5. 功能需求

### 3.1 Realm 导航与访问

#### 3.1.1 路由设计

每个 Realm 拥有独立的前端路由：

**示例 URL**：

**导航说明**：
- **Admin Realm 特权**: Admin Realm 管理员拥有 `realm.create` 权限，可以创建新 Realm
- **Realm 访问**: 用户登录后通过 URL 访问特定 Realm 的资源
- **权限隔离**: 后端验证用户是否有权访问目标 Realm 的资源（403 if unauthorized）

### 3.2 Realm 列表

#### 3.2.1 获取用户可访问的 Realm 列表

**Query Parameters**:
- `user_id`: string (optional) - 筛选指定用户可访问的 realms（用于管理员查看）

**说明**：
- 如果不传 `user_id`，返回当前用户可访问的所有 realms
- 如果传递 `user_id`（需要管理员权限），返回指定用户可访问的 realms

#### 3.2.2 Realm 列表页面（可选）

**路由**：`/$realmId/realms`（未来扩展）

**功能**：
- 表格展示所有 realm
- 显示 realm 名称、ID、创建时间
- 支持创建、编辑 realm（需主管理员权限）
- 删除功能暂不提供

### 3.3 Realm 创建

#### 3.3.1 创建入口

**位置**：Realms 管理页面（`/admin/realms`）

**导航方式**：
- 在左侧导航菜单中点击 "Realms" 菜单项
- 仅 Admin Realm 显示此菜单项
- 点击页面右上角的 "Create Realm" 按钮

**UI 设计**：
- Realms 管理页面显示所有 Realm 的列表表格
- 页面右上角有 "Create Realm" 按钮
- 点击按钮打开对话框显示创建表单

#### 3.3.2 创建表单

**表单字段**：
| Realm ID | string | 否 | 字母数字，3-36 个字符，全局唯一。若不指定则使用 UUID v4 自动生成 |
| Name | string | 是 | 最小 1 个字符 |
| Admin Email | string | 是 | 邮箱格式 |
| Admin Password | string | 是 | 最少 8 个字符 |

**Realm ID 说明**：
- 用户可以指定自定义的 Realm ID（如 `myapp`、`production`）
- 如果用户不指定 Realm ID，系统将使用 UUID v4 自动生成
- 自定义 ID 必须满足：字母数字、3-36 个字符、全局唯一
- 禁止使用保留词：`admin`、`system`、`api`、`www`

**创建后的初始化**：
1. 创建 realm 实体
2. **级联创建默认 web-console client app**：
   - `id`: 自动生成 UUID v7
   - `realm_id`: 新创建的 realm ID
   - `client_id`: 固定为 `"admin-web-console"`
   - `name`: `"Web Console"`
   - `description`: `"Default web console client"`
3. **⭐ 创建管理员用户并分配角色（必需）**：
   - 使用提供的 email 和 password 创建用户
   - 自动将用户分配 `realm-admin` 角色
   - 如果创建失败，回滚整个 realm 创建
4. **⭐ 自动初始化默认 RBAC（必需）**：
   - 创建默认角色定义（`realm-admin`, `user`）
     - `realm-admin`: Realm管理员角色，拥有当前Realm下所有权限
     - `user`: 普通用户角色，拥有基础访问权限
   - 创建默认权限定义（17项）
     - realm-admin: 17项权限（`realm.view`, `realm.admin`, `realm.create`, `users.view`, `users.manage`, `clients.view`, `clients.manage`, `roles.view`, `roles.manage`, `permissions.view`, `permissions.manage`, `policies.view`, `policies.manage`, `settings.view`, `settings.manage`, `billing.view`, `billing.manage`）
     - user: 0项权限（用户修改自己的 profile 和 password 不需要权限检查）
   - 创建角色权限关联（`role_permissions` 表）
   - 创建 RBAC 运行时策略（`role_policies` 表）
     - realm-admin: 多条策略（realm:admin, realm:create, users:manage, clients:manage 等）
     - user: 3条精细策略（`profile:view`, `profile:update`, `password:change`）
   - **实现位置**: `core/src/infrastructure/realm/mod.rs:87-113`
4. 返回创建的 realm信息

**⭐ RBAC 初始化说明**：
- 新创建的 Realm 会**自动初始化**完整的 RBAC 基础设施
- 包括角色定义、权限定义、角色权限关联和 RBAC 策略
- 详细信息请参考 `docs/prd/permissions.md` 的"2.5 默认角色和权限"章节

**重要说明**：
- 每个 Realm 创建时会自动创建一个 admin-web-console client app
- web-console client app 的 `client_id` 固定为 `"admin-web-console"`
- 该 client app 用于 Web 控制台的登录和权限管理
- 用户登录时需要指定 `client_id` 参数（使用外部标识符）

**⚠️ 注意**：
- Realm 创建时**必须指定管理员用户的 email 和 password**
- 创建后会**自动创建管理员用户并分配 realm-admin 角色**
- Realm 创建时会**自动初始化完整的 RBAC 基础设施**
- 创建完成后，创建的管理员用户可以立即登录使用
- 详细的默认角色和权限说明请参考 `docs/prd/permissions.md` 的"2.5 默认角色和权限"章节

#### 3.3.3 创建流程

1. 点击 "Create Realm"
2. 弹出对话框
3. 输入 realm 信息
4. 前端验证（ID 唯一性、格式）
5. 调用 API 创建 realm
6. 创建成功后：
   - 显示成功提示
   - 自动切换到新创建的 realm（可选）
   - 刷新 realm 列表

### 3.4 Realm 详情与编辑

#### 3.4.1 Realm 详情页面

**路由**：`/$realmId/settings/realm`（未来扩展，可整合到 Settings）

**显示信息**：
- Realm ID
- Realm 名称
- 描述
- 创建时间
- 更新时间

**编辑功能**：
- 修改名称
- 修改描述
- Realm ID 不可修改

**字段说明**：
- `name` 字段必填（在 HTTP 层 `UpdateRealmValidator` 中）
- `id` 字段不可修改
- 更新后会自动更新 `updated_at` 时间戳

### 3.5 Realm 删除

**当前项目不提供 Realm 删除功能**。

**实现状态**：
- ✅ Repository 层已禁用：`PostgresRealmRepository::delete_realm` 返回错误
- ✅ Service 层已禁用：`RealmServiceImpl::delete_realm` 返回错误
- ✅ 前端数据层已禁用：`deleteRealm` 函数抛出错误

**原因**：
- 数据库不支持级联删除外键约束
- 删除 realm 会导致 client_app、roles、permissions 等表的数据孤立
- Realm 删除属于高危操作，需要完善的数据迁移和清理机制

**错误响应**：

**未来扩展方向**（如需实现）：

2. 数据清理工具：开发 realm 数据清理脚本
3. 审计日志：记录删除操作和影响范围
4. 备份恢复：删除前自动备份相关数据

### 3.6 Realm 管理用户旅程

本章节描述 Admin Realm 管理员创建和管理 Realm 的完整用户旅程。

#### 3.6.1 创建新 Realm

**目标用户**：Admin Realm 管理员

**前置条件**：
- 已登录到 Admin Realm
- 拥有 Realm 创建权限（`realm.create` 或 `realms.manage` 策略）

**操作流程**：
1. 在左侧导航菜单中点击 "Realms" 菜单项
2. 导航到 `/admin/realms` 页面
3. 点击页面右上角的 "Create Realm" 按钮
4. 在对话框中填写 Realm 信息：
   - **Realm ID**（可选）：留空则自动生成 UUID v7，或输入自定义 ID（3-36 字符，字母数字）
   - **Name**（必填）：Realm 显示名称（3-50 字符）
5. 点击 "Create Realm" 提交
6. 系统自动创建：
   - Realm 实体
   - 默认 RBAC（角色、权限、策略）
   - admin-web-console 客户端应用
7. 创建成功后显示确认消息

**权限边界**：
- ✅ Admin Realm 管理员可以创建任意数量的 Realm
- ❌ 其他 Realm 的管理员不能创建新 Realm
- ❌ Admin Realm 管理员创建 Realm 后，**不能切换**到新 Realm 的内部资源

#### 3.6.2 查看 Realm 列表

**目标用户**：Admin Realm 管理员

**操作流程**：
1. 在左侧导航菜单中点击 "Realms" 菜单项
2. 导航到 `/admin/realms` 页面
3. 查看 Realm 列表表格，显示：
   - Realm ID
   - Realm 名称
   - 创建时间
   - 更新时间

**权限说明**：
- ✅ Admin Realm 管理员可以查看所有 Realm
- ❌ 其他 Realm 的管理员无法访问此页面

#### 3.6.3 导航与访问说明

**URL 结构**：

**导航方式**：
- **左侧菜单**：根据当前 realm 和用户权限动态显示菜单项
- **直接访问**：通过 URL 直接访问特定 realm 的页面
- **权限检查**：后端验证用户是否有权访问目标 realm 的资源

**关键限制**：
- ✅ **多 Realm 导航访问**：Admin Realm 管理员创建 Realm 后，可使用该 Realm 的管理员账号登录访问其管理界面
- ✅ **严格权限隔离**：用户只能访问自己被授权的 Realm 资源
- ✅ **跨 Realm 管理**：Admin Realm 管理员可以管理所有 Realm（通过 /admin/realms 页面）

### 6.1 权限模型

#### 权限级别

1. **主管理员（Super Admin）**
   - 可以创建新的 realm
   - 完全管理 admin realm
   - 不能管理其他 realm 的资源
   - 不能删除 realm（当前限制）
   - 默认主管理员：第一个注册的用户或通过配置文件指定

2. **次管理员（Realm Admin）**
   - 可以管理特定 realm 的用户和配置
   - 可以编辑 realm 的名称
   - 不能删除 realm
   - 创建 realm 的用户自动成为该 realm 的管理员

3. **普通用户（Realm User）**
   - 只能访问被授权的 realm
   - 可以在所属 realm 中进行被授权的操作
   - 不能管理 realm 设置

#### 权限要求

- **查看 Realm 列表**：所有已认证用户（只返回用户所属的 realms）
- **切换 Realm**：用户需要有访问目标 realm 的权限（通过用户-realm 关联表检查）
- **创建 Realm**：需要主管理员权限
- **编辑 Realm**：需要次管理员权限（本 realm）
- **删除 Realm**：不提供该功能（当前限制）

### 6.2 防护措施

1. **Realm ID 验证**
   - 格式：仅字母数字，3-36 个字符
   - 唯一性：全局唯一
   - 保留词：禁止使用 "admin", "system" 等保留词

2. **权限隔离**
   - Realm 级别的数据隔离
   - 用户只能访问被授权的 realm

3. **删除限制**
   - 当前项目**不支持** realm 删除
   - 避免数据孤立和一致性问题

---

## 6. API 相关约束

**状态**: 必填

- 仅说明 Realm、用户、设置等核心管理能力的访问边界、角色要求和数据隔离原则，不在 PRD 中维护端点列表、请求响应字段或实现细节。
- 必须遵守 realm 隔离、权限校验、敏感信息脱敏和关键操作审计要求。
- 详细接口契约、验证规则和错误模型应在技术设计、接口说明或代码中维护。

---

## 7. 前端/交互约束

**状态**: 必填

- 仅保留菜单入口、页面可见性、表单校验期望、操作反馈和角色差异，不写路由代码、组件实现或类型定义。
- 核心管理流程需确保敏感操作有明确确认与结果反馈，且不同角色看到的操作入口和数据范围保持一致。


## 8. 相关文件索引

- 相关实现文件请以本功能对应的 `backend/`、`frontend/`、`demo/` 目录和现有设计文档为准。
- 若需补充精确文件清单，应在技术设计文档中维护，避免在 PRD 中混入实现级细节。

---

## 9. 参考资料

- Realm 实体定义: `core/src/entity/realm.rs`
- Realm Service: `core/src/domain/realm/services.rs`
- Realm Repository: `core/src/infrastructure/realm/mod.rs`
- Realm Policy: `core/src/domain/common/policies.rs`
- **Client App 管理: `docs/prd/client-app.md`** ⭐ 重要：包含双 ID 系统详细说明
- 现有功能参考:
  - Users 功能: `docs/prd/users.md`
  - Settings 功能: `docs/prd/realm-settings.md`
  - OAuth 配置: `docs/prd/auth/oauth.md`
  - Permission 管理: `docs/prd/permissions.md`

