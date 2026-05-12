# 用户故事：默认角色和权限保护

## US-RA-009

**角色代码**: RA
**优先级**: P0
**创建时间**: 2025-02-02
**状态**: Active

---

## 用户故事

作为 **系统管理员**，
我希望 **默认的角色和权限不能被删除或修改**，
从而 **确保系统核心功能不受误操作影响，保证系统稳定性**。

## 验收标准

## 用户故事

作为 **系统管理员**，
我希望 **默认的角色和权限不能被删除或修改**，
从而 **确保系统核心功能不受误操作影响，保证系统稳定性**。

## 验收标准

### 场景 1: 不能删除内置角色

**Given** 系统中有以下内置角色：
```
- super-admin (is_builtin = true)
- realm-admin (is_builtin = true)
- user (is_builtin = true)
```

**When** 管理员尝试删除这些角色

**Then** 应该返回 403 Forbidden 错误

**And** 错误消息为 "Cannot delete built-in role"

### 场景 2: 不能修改内置角色名称

**Given** 系统中有内置角色 `realm-admin`

**When** 管理员尝试修改角色名称为 "admin-v2"

**Then** 应该返回 403 Forbidden 错误

**And** 错误消息为 "Cannot change built-in role name"

### 场景 3: 可以修改内置角色描述

**Given** 系统中有内置角色 `realm-admin`

**When** 管理员只修改角色描述

**Then** 应该成功更新

**And** 角色名称保持不变

### 场景 4: 可以删除自定义角色

**Given** 系统中有自定义角色 `content-admin` (is_builtin = false)

**When** 管理员删除该角色

**Then** 应该返回 204 No Content

**And** 角色已被删除

### 场景 5: 不能删除内置权限

**Given** 系统中有以下内置权限：
```
- users.manage (is_builtin = true)
- profile.view (is_builtin = true)
```

**When** 管理员尝试删除这些权限

**Then** 应该返回 403 Forbidden 错误

**And** 错误消息为 "Cannot delete built-in permission"

### 场景 6: API 返回 is_builtin 字段

**Given** 管理员查询角色或权限列表

**When** API 返回数据

**Then** 应该包含 `is_builtin` 字段

**And** 前端可以根据该字段显示"内置"标识或禁用删除按钮

### 场景 7: 不能从内置角色中移除内置权限

**Given** 系统中有以下内置角色和内置权限：
```
- 角色: realm-admin (is_builtin = true)
- 权限: users.manage (is_builtin = true)
```

**When** 管理员尝试从 realm-admin 角色中移除 users.manage 权限

**Then** 应该返回 403 Forbidden 错误

**And** 错误消息为 "Cannot remove built-in permission from built-in role"

**And** 前端应该禁用内置权限的复选框（针对 super-admin 和 realm-admin 角色）

### 场景 8: 不能修改内置权限定义

**Given** 系统中有内置权限 `users.manage` (is_builtin = true)

**When** 管理员尝试修改权限的名称、资源或操作

**Then** 应该返回 403 Forbidden 错误

**And** 错误消息为 "Cannot modify built-in permission definition"

**And** 前端应该禁用内置权限的编辑和删除按钮

### 场景 9: 可以为内置角色添加自定义权限

**Given** 系统中有内置角色 `realm-admin`

**And** 有自定义权限 `reports.view` (is_builtin = false)

**When** 管理员将 `reports.view` 分配给 `realm-admin`

**Then** 应该成功分配

**And** 管理员也可以从 `realm-admin` 中移除该自定义权限

## 技术实现

### 数据库迁移

添加 `is_builtin` 字段到 roles 和 permissions 表：

```sql
ALTER TABLE roles ADD COLUMN is_builtin BOOLEAN NOT NULL DEFAULT FALSE;
ALTER TABLE permissions ADD COLUMN is_builtin BOOLEAN NOT NULL DEFAULT FALSE;
```

标记默认角色和权限为 `is_builtin = true`。

### API 修改

**角色删除 API**:
```rust
pub async fn delete_role(...) {
  // 检查 is_builtin
  let role = query("SELECT is_builtin FROM roles WHERE id = $1")
  if role.is_builtin {
    return Err(Forbidden("Cannot delete built-in role"))
  }
  // 执行删除
}
```

**角色更新 API**:
```rust
pub async fn update_role(...) {
  // 检查 is_builtin 和名称变更
  let role = query("SELECT is_builtin, name FROM roles WHERE id = $1")
  if role.is_builtin && new_name != current_name {
    return Err(Forbidden("Cannot change built-in role name"))
  }
  // 允许修改描述
}
```

### 前端显示

在角色/权限管理页面：
- 内置角色/权限显示"系统内置"标识
- 禁用删除和修改名称按钮
- 允许修改描述

## 相关文件

**数据库迁移**:
- `core/migrations/20260112_add_builtin_flag_to_roles_and_permissions.sql`

**后端 API**:
- `api/src/application/http/role_definitions/delete.rs` - 角色删除保护
- `api/src/application/http/role_definitions/update.rs` - 角色更新保护
- `api/src/application/http/role_definitions/permissions.rs` - 角色权限移除保护
- `api/src/application/http/admin/permission_definitions/delete.rs` - 权限删除保护
- `api/src/application/http/admin/permission_definitions/update.rs` - 权限更新保护
- `api/src/application/http/role_definitions/types.rs` - 类型定义
- `api/src/application/http/admin/permission_definitions/types.rs` - 类型定义

**实体定义**:
- `core/src/entity/roles.rs`
- `core/src/entity/permissions.rs`
- `core/src/domain/authorization/mod.rs`

**前端实现**:
- `frontend/src/lib/types/rbac.ts` - 类型定义（添加 is_builtin 字段）
- `frontend/src/features/role-definitions/index.tsx` - 角色列表页面（显示内置标识、隐藏删除按钮）
- `frontend/src/features/role-definitions/role-permissions-dialog.tsx` - 角色权限对话框（禁用内置权限复选框）
- `frontend/src/utils/rbac.ts` - API 工具函数（错误处理）

## INVEST 原则检查

- **Independent**: 独立的保护逻辑，可单独实现
- **Negotiable**: 哪些角色/权限为内置可调整
- **Valuable**: 防止误操作导致系统不可用
- **Estimable**: 工作量可预估（约 4-6 小时）
- **Small**: 单一功能，边界清晰
- **Testable**: 有明确的成功和失败场景

## 优先级

P0（关键，立即修复）

---

## 📖 相关PRD

- **权限管理**: [docs/prd/auth/permissions.md](/docs/prd/auth/permissions.md)

## 依赖

- 数据库迁移已执行
- 前端角色/权限管理页面已实现
