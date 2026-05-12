# Herald 用户角色定义参考

> 本文档定义 Herald 系统中的所有用户角色，供编写用户故事时参考。
> 各用户故事文件中的"角色定义"章节应引用本文档，而非重复定义。

---

## 角色层级关系

```
Admin Realm 管理员
    ├── 平台级别权限（仅限 Realm 创建和管理）
    └── ❌ 不能访问其他 Realm 内部资源

Realm Admin
    ├── 单 Realm 权限
    └── 仅管理本 Realm 资源

Regular User
    ├── 基本登录权限
    └── 访问授权资源

Third-party App
    ├── 客户端凭证认证
    └── 服务间调用

Herald 系统（System Actor）
    ├── 系统级定时任务
    ├── Webhook 回调处理
    └── 状态同步
```

---

## 角色总览

| 角色 | 技术标识 | 权限范围 | 初始化/创建方式 |
|------|----------|----------|----------------|
| Admin Realm 管理员 | `realm_admin` | 平台级别（Realm管理）+ Admin Realm内部 | 系统初始化创建 `admin@cas.com` |
| Realm Admin | `realm_admin` | 仅本 Realm | 由 Admin Realm 管理员创建 Realm 时指定 |
| Regular User | `regular` | 个人资源 | 用户注册或由管理员创建 |
| Third-party App | `client_app` | 客户端凭证认证 | 在管理后台注册客户端应用 |
| Herald 系统 | `system` | 系统级任务和回调处理 | N/A（非用户角色） |

---

## 角色详细定义

### Admin Realm 管理员（Admin Realm Admin）

**技术标识**：`realm_admin`
**初始化账号**：`admin@cas.com`（在 admin realm 中）
**权限范围**：平台级别（Realm管理）+ Admin Realm内部资源访问

**权限清单**：

| 权限项 | 说明 |
|--------|------|
| Realm管理 | 创建/查看/删除 Realm |
| 查看Realm列表 | 查看所有Realm的基本信息（ID、名称、创建时间） |
| 指定Realm管理员 | 在创建Realm时指定初始管理员账号 |
| Admin Realm资源访问 | 访问 admin realm 内的用户、角色、配置等资源 |

**边界约束**：
- ❌ 不能访问其他 Realm 内部资源（用户、角色、配置、客户端应用）
- ❌ 不能创建跨 Realm 策略
- ❌ 不能管理其他 Realm 的 RBAC 元数据
- ✅ 仅负责平台级别的 Realm 生命周期管理
- ✅ Realm 创建后，内部管理由该 Realm 的 Realm Admin 负责
- ✅ 可以访问 admin realm 的所有内部资源

**关键变更说明**：
- ❌ 已移除旧的 "Super Admin" 概念和跨 Realm 权限
- ✅ Admin Realm 管理员本质上是一个特殊的 Realm Admin
- ✅ 拥有特殊的平台级权限（创建 Realm）
- ✅ 严格遵循 Realm 隔离原则

---

### Realm Admin（Realm 管理员）

**技术标识**：`realm_admin`
**权限范围**：仅限所属 Realm

**权限清单**：

| 权限项 | 说明 |
|--------|------|
| 用户管理 | 仅本 Realm 的创建/查看/编辑/删除 |
| 角色管理 | 仅为本 Realm 配置角色和权限 |
| 策略管理 | 仅创建本 Realm 的资源访问策略 |
| 客户端应用管理 | 仅查看/管理本 Realm 的客户端应用 |

**边界约束**：
- 无法创建 `resource: "All"` 策略
- 策略必须通过 `validate_policy_for_realm_admin` 校验
- 无法跨 Realm 查看或操作
- 仅能管理本 Realm 的资源和用户

---

### Regular User（普通用户）

**技术标识**：`regular`
**认证方式**：邮箱密码 / OAuth/OIDC / SAML

**能力清单**：

| 能力项 | 说明 |
|--------|------|
| 登录 | 密码登录或第三方登录（Google/GitHub等） |
| 个人信息管理 | 查看昵称、更新邮箱 |
| 密码管理 | 修改密码、重置密码（通过邮箱链接） |
| 资源访问 | 访问被授权的应用资源（基于 OAuth/OIDC） |

**边界约束**：
- 无法访问管理后台（`/admin/*` 路径）
- 无法管理其他用户
- 资源访问需基于授权策略
- 初始状态为 `WaitVerified`，需验证邮箱后变为 `Normal`

---

### Third-party App（第三方应用）

**技术标识**：`client_app`
**认证方式**：Client Credentials（客户端凭证）

**能力清单**：

| 能力项 | 说明 |
|--------|------|
| 服务间调用 | 使用 `client_id` + `client_secret` 获取 token |
| API 访问 | 携带 token 调用受保护的 API |
| 多 Realm 支持 | 可配置多个 Realm 的访问权限 |

**边界约束**：
- 无法访问管理后台
- 无用户会话概念（无 UI 登录）
- 需预先在管理后台注册客户端应用（生成 `client_id`/`client_secret`）
- Token 有效期限制

---

### Herald 系统（System Actor）

**技术标识**：`system`
**说明**：系统级 Actor，代表 Herald 后台自动执行的定时任务、回调处理、状态同步等非用户触发的行为。不对应真实用户，仅在用户故事中作为角色使用。

**典型场景**：
- Webhook 回调处理（支付回调、OAuth 回调等）
- 定时任务（过期订单清理、补偿查询等）
- 系统间状态同步

---

## 代码实现映射

| 用户故事角色 | 前端枚举 (`frontend/src/auth.tsx:8`) | 后端校验 (`api/src/application/http/admin/middleware.rs`) |
|--------------|-------------------------------------|--------------------------------------------------------|
| Admin Realm 管理员 | `UserRole.RealmAdmin` + 特殊权限检查 | `require_realm_admin_for_admin_realm()` |
| Realm Admin | `UserRole.RealmAdmin` | `require_realm_admin()` |
| Regular User | `UserRole.Regular` | - |
| Third-party App | - | 客户端凭证流程（`/oauth/token`） |

---

## 使用指南

编写新用户故事时：

1. **在用户故事文件开头引用本文档**：
   ```markdown
   **作为**：Admin Realm 管理员（详见 [docs/user-stories/_roles.md](/docs/user-stories/_roles.md)）
   ```

2. **避免重复定义**：不要在每个故事文件中重复完整的角色定义

3. **更新角色定义时**：仅修改本文档，确保全局一致性

4. **新增角色时**：
   - 在本文档添加新角色定义
   - 同步更新前端 `frontend/src/auth.tsx` 的 `UserRole` 枚举
   - 同步更新后端 `api/src/application/http/admin/middleware.rs` 的校验逻辑

---

## 架构迁移说明

### 旧架构（已废弃）

```
Super Admin (跨 Realm 权限)
    ├── admin realm
    ├── realm-1
    ├── realm-2
    └── ❌ 可以访问所有 Realm 的内部资源（违反隔离原则）
```

**问题**：
- ❌ Super Admin 概念破坏了 Realm 隔离
- ❌ "All" 权限策略作为通配符，违反精确匹配原则
- ❌ 跨 Realm 权限导致安全和审计问题

### 新架构（当前）

```
Admin Realm 管理员 (特殊 Realm Admin)
    ├── 平台级权限（创建 Realm）
    ├── Admin Realm 内部权限（realm-admin 角色）
    └── ❌ 不能访问其他 Realm 内部资源

Realm-1 Admin (realm-1 的 realm-admin)
    └── 仅能访问 realm-1

Realm-2 Admin (realm-2 的 realm-admin)
    └── 仅能访问 realm-2
```

**优点**：
- ✅ 严格的 Realm 隔离
- ✅ 精确的权限匹配（无通配符）
- ✅ 清晰的审计边界
- ✅ 每个 Realm 独立管理
