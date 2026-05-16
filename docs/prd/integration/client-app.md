# Client Apps 菜单产品需求文档 (PRD)

**创建时间**: 2025-01-05
**状态**: Implemented

---
## 1. 相关用户故事

> **说明**: 详细的用户故事和验收标准请查看 `../../user-stories/` 目录中的对应文件。

### 1.1 Third-Party App 用户故事

- 📄 [docs/user-stories/04-third-party-app-user-stories.md](/docs/user-stories/04-third-party-app-user-stories.md)
  - **[US-TP-001] 创建 Client App** (P0): 作为 Realm Admin，我想要创建新的 Client App，以便添加新的接入应用
  - **[US-TP-002] 查看 Client App 列表** (P0): 作为 Realm Admin，我想要查看所有 Client App，以便管理系统中的应用
  - **[US-TP-003] 查看 Client App 详情** (P0): 作为 Realm Admin，我想要查看 Client App 详情，以便了解应用配置
  - **[US-TP-004] 编辑 Client App** (P0): 作为 Realm Admin，我想要编辑 Client App 配置，以便更新应用设置
  - **[US-TP-005] 删除 Client App** (P0): 作为 Realm Admin，我想要删除 Client App，以便移除不再使用的应用

### 1.2 Client App Settings 用户故事

- 📄 [docs/user-stories/client-app-settings.md](/docs/user-stories/client-app-settings.md)
  - **配置 OAuth 2.0 设置** (P0): 作为 Realm Admin，我想要配置 OAuth 2.0 设置（redirect_uris、client_secret、enabled），以便确保应用安全接入
  - **配置会话设置** (P0): 作为 Realm Admin，我想要配置会话 TTL 和滑动续期策略，以便在用户活跃时自动延长会话
  - **配置应用外观** (P1): 作为 Realm Admin，我想要配置应用图标，以便提升用户体验
  - **重新生成 Client Secret** (P0): 作为 Realm Admin，我想要重新生成 Client Secret，以便在密钥泄露时更新凭证

### 1.3 优先级汇总

| 优先级 | 数量 | 关键故事 |
|--------|------|----------|
| P0 | 7 | Client App 管理、创建/编辑/删除 Client App、配置 OAuth 2.0 设置、配置会话设置（含滑动续期）、重新生成 Client Secret |
| P1 | 1 | 配置应用外观 |
| P2 | 0 | - |

---

## 2. 范围界定

### 2.1 包含功能

- ✅ Client App 列表展示（分页）
- ✅ 创建 Client App（支持自定义 client_id，双 ID 系统）
- ✅ 编辑 Client App（名称、描述、启用状态）
- ✅ 删除 Client App（需要二次确认）
- ✅ OAuth 2.0 配置（redirect_uris、client_secret、enabled）
- ✅ 会话配置（session_ttl_seconds、session_renewal_ttl_seconds）
- ✅ 滑动会话续期（Identity 中间件在受保护 API 请求时自动续期）
- ✅ 应用外观配置（icon_url）
- ✅ Client Secret 重新生成
- ✅ Client App 快速切换（启用/禁用）
- ✅ URL 安全验证（禁止 javascript: 协议、协议相对 URL）

### 2.2 不包含功能 (Out of Scope)

- ❌ **Client App 作用域管理** (原因: 没有 OAuth 2.0 scope 管理功能)
- ❌ **Client App 访问日志** (原因: 没有记录 Client App 访问日志)
- ❌ **Client App 统计** (原因: 没有 Client App 使用统计)
- ❌ **Client App 模板功能** (原因: 没有预定义的 Client App 模板)
- ❌ **批量导入/导出 Client App** (原因: 没有批量操作功能)

### 2.3 依赖项

- ✅ **用户认证系统** (状态: 已实现) - 提供登录和会话管理
- ✅ **权限管理系统** (状态: 已实现) - Realm Admin 权限检查
- ✅ **Realm 系统** (状态: 已实现) - Client App 属于 Realm 级别
- ✅ **OAuth 2.0 系统** (状态: 已实现) - 支持 OAuth 2.0 授权流程

---

## 3. 需求概述

在 Herald 管理后台的左侧导航栏添加 **Client Apps** 菜单项，用于展示和管理系统中的客户端应用。Client App 是指接入 Herald 系统的客户端应用程序（如 Web 应用、移动应用、第三方服务等）。

**重要变更**：Client App 现在包含两种 ID
  - `id`: UUID (内部主键，用于数据库关联和 role_policies)
  - `client_id`: string (外部标识符，必填，3-36字符，仅字母数字)

### 3.1 会话滑动续期机制

当 Client App 配置了 `session_renewal_ttl_seconds` 时，Identity 中间件在处理受保护 API 请求时自动执行滑动续期：

1. **触发条件**：Redis 中 Session 的剩余 TTL 低于 `session_renewal_ttl_seconds` 的一半（`<= renewal_ttl / 2`）
2. **续期行为**：将 Redis TTL 刷新为 `session_renewal_ttl_seconds`，并在响应中追加新的 `Set-Cookie` 头以同步浏览器 Cookie 的 Max-Age
3. **不续期的情况**：`session_renewal_ttl_seconds` 为 null 时，中间件不执行任何续期操作，Session 按原始 TTL 自然过期
4. **配置固化**：续期策略在 Session 创建时固化（写入 Redis SessionData），后续对 Client App 配置的修改只影响新创建的 Session

典型场景：
- **严格策略**（银行应用）：`session_ttl=300, renewal_ttl=null` → 5 分钟过期，无续期
- **宽松策略**（企业工具）：`session_ttl=28800, renewal_ttl=28800` → 用户持续活跃时 Session 永不过期
- **渐进策略**：`session_ttl=300, renewal_ttl=7200` → 首次登录 5 分钟，首次续期后延长到 2 小时

---

## 4. 当前实现状态

### 4.1 已实现功能

- ✅ **后端实体层**：Client App 实体定义（双 ID 系统）
- ✅ **后端 Repository 层**：Client App 数据库操作接口和实现
- ✅ **后端 Service 层**：Client App 业务逻辑和权限检查
- ✅ **后端 HTTP API**：Client App CRUD RESTful API
- ✅ **滑动会话续期**：Identity 中间件自动检测 Redis TTL 并续期，同步 Set-Cookie
- ✅ **前端数据层**：Client App API 调用函数
- ✅ **前端类型定义**：TypeScript 类型定义（支持双 ID）
- ✅ **前端导航菜单**：Client Apps 菜单项已配置
- ✅ **前端 Client Apps 列表页面**：列表、创建、编辑、删除
- ✅ **前端 OAuth 2.0 配置**：redirect_uris、client_secret、enabled、icon_url、session_ttl_seconds 等
- ✅ **演示测试**：client-app-settings-demo.e2e.ts

### 4.2 未实现功能

- ❌ **Client App 作用域管理**：没有实现 OAuth 2.0 scope 管理
- ❌ **Client App 访问日志**：没有记录 Client App 访问日志
- ❌ **Client App 统计**：没有 Client App 使用统计

**说明**：
- Client App 采用双 ID 系统：UUID 作为内部主键，`client_id` 作为外部标识符
- API 调用使用 `client_id`（外部标识符），Session/RBAC 中使用 `id`（UUID）
- Client App 默认创建为 `admin-web-console`，用于 Web 控制台登录

---

## 5. 功能需求

### 5.1 导航菜单配置

导航菜单已配置完成（参考 `frontend/src/data/navigation.ts`）：

### 5.2 Client Apps 列表页面

#### 5.2.1 路由定义

创建 `frontend/src/routes/$realmId/clients.tsx` 路由文件：

#### 5.2.2 页面布局

Client Apps 列表页面应包含以下元素：

1. **页面标题区域**
   - 标题: "Client Apps"
   - 描述: "Manage client applications"

2. **操作区域**
   - 右上角 "Add Client App" 按钮
   - 点击后弹出创建 Client App 对话框

3. **Client Apps 列表表格**
   - 显示 Client App 基本信息
   - 支持分页
   - 支持编辑、删除等操作

#### 5.2.3 表格列定义

| 列名 | 说明 | 数据来源 |
|------|------|----------|
| Icon | Client App 图标 | `client_app.icon_url` |
| Client ID | Client App 外部标识符 | `client_app.client_id` |
| Name | Client App 名称 | `client_app.name` |
| Description | Client App 描述 | `client_app.description` |
| Redirect URIs | 跳转地址白名单（显示计数和预览） | `client_app.redirect_uris` |
| Session TTL | Session 有效期（分钟格式） | `client_app.session_ttl_seconds` |
| Status | 启用/禁用状态 | `client_app.enabled` |
| Actions | 操作按钮 | - |

#### 5.2.4 操作列功能

- **Edit**: 编辑 Client App 信息
- **Delete**: 删除 Client App（需要二次确认）

### 5.3 创建 Client App 功能

#### 5.3.1 创建 Client App 对话框

使用 Shadcn/ui 的 `Dialog` 组件创建表单，采用 **Tabs 布局**，包含以下字段：
| **Basic 标签页** ||||||
| Client ID | string | **是** | 仅字母数字，3-36 个字符 | - |
| Name | string | 是 | 最小 1 个字符，最大 36 个字符 | - |
| Description | string | 否 | 最大 255 个字符 | - |
| **Redirect URIs 标签页** ||||||
| Redirect URIs | string[] | **是** | 至少一个有效 URL，禁止 javascript: 和协议相对 URL | - |
| **Security 标签页** ||||||
| Enabled | boolean | 否 | - | true |
| Session TTL (seconds) | number | 否 | 最小 60 秒，最大 86400 秒（24小时） | 1800 (30分钟) |
| Session Renewal TTL (seconds) | number | 否 | 最小 60 秒，最大 604800 秒（7天），可为 null（禁止续期），需 >= Session TTL | - |
| **Appearance 标签页** ||||||
| Icon URL | string | 否 | 有效 URL | - |

**注意**:
- `realm_id` 从路由参数自动获取
- `client_id` 是必填字段，用于外部标识
- 系统会自动生成 `client_secret` (UUID)，只在创建时返回一次
- 表单使用 TanStack Form + Zod 验证

#### 5.3.2 表单验证

使用 Zod schema 进行验证（详见 `frontend/src/lib/schemas/client-app-forms.ts`）：

### 5.4 编辑 Client App 功能

#### 5.4.1 编辑 Client App 对话框

使用相同的表单结构，预填充现有数据，并添加以下选项：
| **所有创建字段** | - | - | - |
| Regenerate Secret | boolean | 否 | - |

**注意**:
- 编辑模式下 Client ID 不可修改（作为系统标识，只读显示）
- 重新生成 Secret 时会显示警告消息
- 新 Secret 只在重新生成后返回一次

---

## 6. API 相关约束

**状态**: 必填

- 仅说明第三方接入、Client App、API Key、回调或外部能力暴露的边界，不在 PRD 中展开端点、schema、签名格式或 SDK 类型定义。
- 必须遵守 realm 隔离、Client App/第三方身份校验、凭证脱敏、回调安全和可观测性要求。
- 详细接入契约、认证方式和错误模型应下沉到技术设计、接口说明或 SDK 文档。

---

## 7. 前端/交互约束

**状态**: 必填

- 仅保留接入配置入口、凭证展示规则、状态反馈和帮助说明，不写 SDK 调用示例、前端实现代码或接口调试步骤。
- 涉及第三方接入时，需明确哪些流程由 Herald 后台完成，哪些流程在第三方应用或外部平台完成。
- Session Renewal TTL 字段允许设置为 null 或留空，表示禁止续期；设置值必须 >= Session TTL。
- 续期行为由后端中间件自动完成，前端无需主动调用续期接口。

---

## 8. 技术设计承接

**状态**: 必填

- 接口细节、数据库结构、迁移策略、类型定义、调度方案、SDK 设计和实现步骤，应在 `docs/design/`、`.ai/design/`、接口说明或代码中承接。
- 如历史实现已经存在，应以现有设计文档、OpenAPI、迁移文件和代码为依据补充，不回写到 PRD 正文。

---

## 9. 相关文件索引

- 相关实现文件请以本功能对应的 `backend/`、`frontend/`、`demo/` 目录和现有设计文档为准。
- 若需补充精确文件清单，应在技术设计文档中维护，避免在 PRD 中混入实现级细节。

---

## 10. 参考资料

- 前端开发指南: `../../spec/frontend/development.md`
- Client App 实体定义: `core/src/domain/client/entities.rs`
- Client App API 实现: `api/src/application/http/admin/client_app.rs`
- Users 功能参考: `docs/frontend/users.md`
- 现有代码参考:
  - 数据层: `frontend/src/data/users.ts`
  - 工具层: `frontend/src/utils/users.ts`
  - 页面组件: `frontend/src/features/users/index.tsx`
  - 表格组件: `frontend/src/features/users/user-table.tsx`
  - 表单组件: `frontend/src/features/users/user-form.tsx`
  - 路由配置: `frontend/src/routes/$realmId/users.tsx`
  - 类型定义: `frontend/src/lib/types.ts`
