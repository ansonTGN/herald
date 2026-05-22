# SDK 增强 — 资源管理产品需求文档 (PRD)

**创建时间**: 2026-05-21
**状态**: Draft
**优先级**: P0

---

## 1. 相关用户故事

> 详细故事与验收标准请查看 `docs/user-stories/integration/sdk.md`。

### 1.1 相关故事

- **[US-TP-012]** 通过 SDK 管理 Realm，优先级 P1，来源 `docs/user-stories/integration/sdk.md`
- 角色：Third-Party App
- 摘要：编程式创建、查询列表、查询详情 Realm

- **[US-TP-013]** 通过 SDK 管理用户，优先级 P0，来源 `docs/user-stories/integration/sdk.md`
- 角色：Third-Party App
- 摘要：在指定 Realm 中创建、查询列表、查询详情用户

- **[US-TP-014]** 通过 SDK 管理 Client App，优先级 P1，来源 `docs/user-stories/integration/sdk.md`
- 角色：Third-Party App
- 摘要：编程式创建、查询列表、查询详情 Client App

### 1.2 优先级汇总

| 优先级 | 数量 | 关键故事 |
|--------|------|----------|
| P0 | 1 | 通过 SDK 管理用户 |
| P1 | 2 | 通过 SDK 管理 Realm、通过 SDK 管理 Client App |

---

## 2. 范围界定

### 2.1 包含功能

- SDK 新增 Realm 管理方法：创建、查询列表、查询详情
- SDK 新增用户管理方法：创建、查询列表、查询详情
- SDK 新增 Client App 管理方法：创建、查询列表、查询详情
- 后端 api-ext 模块新增对应的外部 API 端点
- SDK 方法保持与现有风格一致：基于 reqwest、使用 X-API-Key 认证、统一的错误处理
- 新增资源管理端点要求 API Key Principal 具备对应 RBAC 权限

### 2.2 不包含功能 (Out of Scope)

- 权限管理 SDK 方法（角色 CRUD、权限定义、策略管理等）— 保持现有 `check_permission` 不变
- 用户编辑、删除操作
- Client App 编辑、删除、设置管理操作
- Realm 编辑、删除、设置操作
- 前端页面变更
- SDK 缓存策略变更

### 2.3 依赖项

- 现有 api-ext 模块的认证机制（API Key）
- 现有 domain 层的 Realm、User、Client App 领域服务
- 现有 SDK 的 `Client` 结构体和错误处理模式

---

## 3. 需求概述

### 3.1 功能描述

当前 Rust SDK 仅覆盖权限检查、订阅管理和积分系统三类能力。第三方应用开发者若需要通过编程方式管理 Realm、用户和 Client App 等核心资源，只能自行调用内部 API 或登录管理后台手动操作。

本次增强为 SDK 补齐核心资源的管理能力，使第三方应用能够通过 SDK 自动完成用户开通、应用注册和组织（Realm）初始化，降低集成门槛。

### 3.2 关键特性

- **Realm 管理**：创建、查询列表、查询详情
- **用户管理**：创建、查询列表、查询详情（P0）
- **Client App 管理**：创建、查询列表、查询详情
- **与现有 SDK 风格一致**：共享 Client 实例、统一错误类型、API Key 认证
- **统一 Principal 权限语义**：API Key 代表第三方服务端机器凭据；API Key 自身作为 Principal 参与授权，能力由角色/权限决定，资源边界由 Realm 隔离决定

---

## 4. 当前实现状态

| 功能模块 | 状态 | 备注 |
|---------|------|------|
| SDK 权限检查 | ✅ 已实现 | `check_permission` |
| SDK 订阅管理 | ✅ 已实现 | `get_subscription`, `list_plans`, `list_plan_assignments` |
| SDK 积分系统 | ✅ 已实现 | `get_balance`, `consume_points` |
| SDK Realm 管理 | ❌ 未实现 | 需新增后端 ext 端点 + SDK 方法 |
| SDK 用户管理 | ❌ 未实现 | 需新增后端 ext 端点 + SDK 方法 |
| SDK Client App 管理 | ❌ 未实现 | 需新增后端 ext 端点 + SDK 方法 |

---

## 5. 功能需求

### 5.1 核心需求

1. **Realm 管理** — US-TP-012
   - 创建新 Realm，返回 Realm ID 和基本信息
   - 查询可见 Realm 列表
   - 查询指定 Realm 详情

2. **用户管理** — US-TP-013（P0）
   - 在指定 Realm 中创建用户，返回用户 ID 和状态
   - 查询指定 Realm 的用户列表
   - 查询指定 Realm 中单个用户的详情

3. **Client App 管理** — US-TP-014
   - 在指定 Realm 中创建 Client App，返回 Client ID 和 Secret
   - 查询指定 Realm 的 Client App 列表
   - 查询指定 Realm 中单个 Client App 的详情

### 5.2 验收目标

- 3 个用户故事的全部验收场景通过
- SDK 新增方法与现有方法风格一致（方法命名、错误处理、参数模式）
- 所有新增 ext 端点遵循 Realm 隔离原则：API Key 只能操作所属 Realm 的资源
- 所有新增资源管理端点要求 API Key Principal 具备对应权限
- Realm 创建需额外校验 API Key Principal 属于 admin realm 且具备 `realm:create`
- SDK 单元测试覆盖全部新增方法（使用 wiremock mock）
- 后端集成测试覆盖关键场景（创建成功、重复邮箱、跨 Realm 拒绝）

---

## 6. API 相关约束

**状态**: 必填

### 访问控制原则

- 所有新增端点使用现有 API Key 认证机制（`X-API-Key` 请求头）
- **API Key 语义**：API Key 只有一种身份语义，代表第三方服务端机器凭据；API Key 自身作为 Principal 参与授权，不按 Key 类型拆分。
- **权限模型**：本次使用统一 Principal + RBAC 模型。API Key 不携带 `runtime` / `management` scope；能力由 Principal 的角色和 role policy 决定。
- **Realm 隔离**：用户和 Client App 操作仅限 API Key 所属 Realm
- **Realm 创建特权**：创建 Realm 的端点需校验 API Key Principal 在 admin realm 具备 `realm:create` 权限，普通 Realm 的 API Key 不可创建 Realm
- **Principal 绑定**：API Key 以自身唯一标识作为 Principal ID，复用现有角色绑定机制，不引入独立的 Principal 管理表。
- 复用现有 `handle_response` 统一错误处理模式

### 接口能力边界

- Realm：创建、列表、详情（需 `realm:create/list/view`；创建还需 admin realm 权限）
- User：创建、列表、详情（需 `users:create/view`，限本 Realm）
- Client App：创建、列表、详情（需 `clients:create/view`，限本 Realm）

具体端点设计由 `/t-design sdk-improve` 产出。

---

## 7. 前端/交互约束

**状态**: 不适用

本次变更仅涉及 SDK 和后端 ext API，无前端页面变更。

---

## 8. 技术设计承接

**状态**: 必填

需要 `/t-design sdk-improve` 产出技术设计文档，涵盖：
- 后端 ext API 端点定义和路由规划
- SDK 新增方法的签名和类型定义
- 认证与权限校验方案（特别是 API Key Principal -> RBAC permission 与 Realm 创建的权限模型）

---

## 9. 相关文件索引

### 9.1 后端文件

- `backend/sdk/src/lib.rs` — SDK 主文件，需新增 Realm/User/Client App 方法
- `backend/api-ext/src/` — 外部 API 模块，需新增 handler 和路由
- `backend/sdk/README.md` — SDK 文档，需更新使用示例

### 9.2 前端文件

- 不适用

---

## 10. 参考资料

- 用户故事：`docs/user-stories/integration/sdk.md`
- 相关 PRD：`docs/prd/auth/oauth.md`（现有 ext API）
- 相关 PRD：`docs/prd/core/realm.md`（Realm 管理）
- 相关 PRD：`docs/prd/core/users.md`（用户管理）
- 相关 PRD：`docs/prd/integration/client-app.md`（Client App 管理）
- SDK 源码：`backend/sdk/src/lib.rs`
