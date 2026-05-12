# OAuth Provider 产品需求文档 (PRD)

**创建时间**: 2025-01-10
**状态**: Partially Implemented

---
## 1. 相关用户故事

> **说明**: 详细的用户故事和验收标准请查看 `../../user-stories/` 目录中的对应文件。

### 1.1 Realm Admin 用户故事

- 📄 [docs/user-stories/02-realm-admin-user-stories.md](/docs/user-stories/02-realm-admin-user-stories.md)
  - **[US-RA-008] 配置 Realm 设置** (P0): 作为 Realm Admin，我想要配置 OAuth Provider，以便启用第三方登录

### 1.2 Third-Party App 用户故事

- 📄 [docs/user-stories/04-third-party-app-user-stories.md](/docs/user-stories/04-third-party-app-user-stories.md)
  - **[US-TP-006] 第三方应用授权登录** (P0): 作为第三方应用开发者，我想要使用 OAuth Provider 进行第三方登录，以便快速访问系统

### 1.3 用户故事优先级汇总

| 优先级 | 用户故事数量 | 关键故事 |
|--------|------------|---------|
| P0 | 3 | 配置 OAuth Provider、启用/禁用 Provider、第三方登录 |
| P1 | 0 | - |
| P2 | 0 | - |

---

## 2. 范围界定

### 2.1 包含功能

- ✅ OAuth Provider 配置管理（Google、GitHub、Facebook、Apple）
- ✅ Provider 启用/禁用控制
- ✅ 前端 OAuth 配置表单
- ✅ 前端登录页面 OAuth 按钮
- ✅ Provider 配置 CRUD API
- ✅ 强类型实体（`OAuthProviderConfig`）

### 2.2 不包含功能 (Out of Scope)

- ❌ **后端 HTTP Handlers 完整实现** (原因: 当前为 TODO)
- ❌ **后端路由注册到 HTTP Server** (原因: 待实施)
- ❌ **前端 Provider 配置管理页面** (原因: 待实施)
- ❌ **端到端测试** (原因: 待实施)
- ❌ **OAuth 登录流程完整实现** (原因: 待实施)

### 2.3 依赖项

- ✅ **Realm 系统** (状态: 已实现) - OAuth Config 属于 Realm 级别
- ✅ **权限管理系统** (状态: 已实现) - Realm Admin 权限检查
- ✅ **Client App 系统** (状态: 已实现) - OAuth 回调验证

---

## 3. 需求概述

在 Herald 管理后台中提供 OAuth Provider 配置管理功能，允许管理员为每个 Realm 配置第三方登录提供商（Google、GitHub、Facebook、Apple）。

### 1.1 命名说明

**建议使用 "Provider" 或 "Identity Provider"**：
- ✅ 推荐：**Provider Settings** / **Identity Providers**
- ❌ 不推荐：**OAuth Config** / **OAuth Settings**

**理由**：
- "Provider" 更简洁，符合行业标准（如 Auth0、Okta 等使用 "Identity Providers"）
- 避免与 "OAuth Config" 的技术术语混淆
- 用户体验更好，更易理解

### 1.2 功能定位

**OAuth Provider 系统定位**（独立于 Realm Config）：
- 适用于：管理第三方登录提供商的完整配置
- 复杂度：高（provider 验证、scopes、授权流程）
- 数据模型：强类型结构体（`OAuthProviderConfig`）
- 类型安全：编译期检查

**与 Realm Config 的区别**：
- Realm Config：简单的 key-value 配置（如 Turnstile、Registration）
- OAuth Provider：复杂的结构化配置（独立的 Provider 实体）

---

## 4. 当前实现状态

### ✅ 已完成
- [x] 后端实体定义 (`core/src/domain/oauth/entities.rs`)
- [x] 后端 Service 定义 (`core/src/domain/oauth/config_service.rs`)
- [x] 后端 Repository 接口定义 (`core/src/domain/oauth/ports.rs`)
- [x] 后端 HTTP Handlers 框架 (`api/src/application/http/oauth/`)
- [x] 前端 API 封装 (`frontend/src/lib/api.ts` - `oauthConfigApi`)
- [x] 前端 OAuth 配置表单 (`frontend/src/features/settings/oauth-config-form.tsx`)
- [x] 前端登录页面 OAuth 按钮 (`frontend/src/routes/$realm_id.login.tsx`)

### ⚠️ 部分实现
- [ ] 后端 HTTP Handlers 完整实现（当前为 TODO）
- [ ] **后端 HTTP Handlers 框架已定义**：`api/src/application/http/oauth/` 目录下的处理器已定义，但核心业务逻辑返回 "Not implemented"
  - `config.rs` - OAuth 配置 CRUD（已定义，待实现）
  - `providers.rs` - Provider 列表（已实现）
  - `login.rs` - OAuth 登录（已定义框架，待实现）
  - `callback.rs` - OAuth 回调（已定义框架，待实现）
- [ ] 后端路由注册到 HTTP Server（状态需确认）
- [ ] 前端 Provider 配置管理页面

### ❌ 待实施
- [ ] 端到端测试
- [ ] OAuth 登录流程完整实现

---

## 5. 功能需求

### 2.1 导航菜单配置

**建议**：将 Provider 配置集成到 Settings 菜单中，作为独立的 Tab：

**Settings 页面 Tabs**：
- Turnstile
- Registration
- **Providers** ← 新增

### 2.2 Provider 配置管理页面

#### 2.2.1 页面布局

创建 `frontend/src/features/settings/provider-config-page.tsx`：

#### 2.2.2 Provider 列表

创建 `frontend/src/features/settings/provider-list.tsx`：

**表格列定义**：

| 列名 | 说明 | 数据来源 |
|------|------|----------|
| Provider | Provider 名称 | `provider_type.display_name()` |
| Client ID | OAuth Client ID | `client_id` |
| Status | 启用状态 | `enabled` |
| Scopes | 授权范围 | `scopes` (显示前 3 个) |
| Actions | 操作按钮 | - |

**状态显示**：
- **Enabled**: 绿色 Badge
- **Disabled**: 灰色 Badge

**操作按钮**：
- **Edit**: 编辑 Provider 配置
- **Toggle**: 启用/禁用 Provider
- **Delete**: 删除 Provider 配置（需要二次确认）

#### 2.2.3 Provider 配置表单

创建 `frontend/src/features/settings/provider-config-dialog.tsx`：

**表单字段**：
| Provider Type | Select | 是 | - | Google, GitHub, Facebook, Apple |
| Client ID | string | 是 | 最小 1 个字符 | OAuth Client ID |
| Client Secret | string | 是 | 最小 1 个字符 | OAuth Client Secret（编辑时可选） |
| Scopes | string[] | 否 | - | OAuth 授权范围（默认值） |
| Enabled | boolean | 否 | - | 是否启用（默认 true） |

**默认 Scopes**（参考 `core/src/domain/oauth/entities.rs:59-69`）：

**表单验证**（Zod Schema）：

**注意事项**：
- 编辑模式下 `Client Secret` 为可选（留空表示不更新）
- 前端不应显示已存储的 `Client Secret`（安全考虑）
- 表单提示："Leave empty to keep existing secret"

### 2.3 OAuth 登录流程

#### 2.3.1 登录页面 Provider 按钮

**参考**：`frontend/src/routes/$realm_id.login.tsx:37-128`

**当前实现**：
- ✅ 动态加载启用的 Providers（`oauthApi.listProviders`）
- ✅ 显示 Provider 登录按钮
- ✅ 点击后调用 `oauthApi.initiateLogin`

**UI 示例**：

#### 2.3.2 OAuth 登录流程

**流程图**：

#### 2.3.3 State Token 管理策略

**目的**：防止 CSRF 攻击

**实现要求**：

1. **State Token 生成**
   - 格式：UUID v7 (时间有序，防止碰撞)
   - 代码：`generate_uuid_v7().to_string()`

2. **State Token 存储**
   - 存储位置：Redis (推荐) 或 内存缓存
   - Key 格式：`oauth:state:{state_token}`
   - Value 格式：JSON `{ realm_id: string, provider_type: string, redirect_uri?: string, created_at: timestamp }`
   - 过期时间：5 分钟 (300 秒)

3. **State Token 验证**
   - 在 OAuth 回调时验证 state 是否存在且未过期
   - 验证成功后立即删除 state token (一次性使用)

4. **Redis 示例**

#### 2.3.4 Session 管理策略

**Session 类型**：JWT (JSON Web Token)

**Cookie 配置**：
- 名称：`cas_session`
- 值：JWT token
- 属性：
  - `httpOnly`: true (防止 XSS 攻击)
  - `secure`: true (生产环境，HTTPS only)
  - `sameSite`: "Lax" (防止 CSRF 攻击)
  - `path`: "/"
  - `maxAge`: 7 天 (604800 秒)

**JWT Payload**：

**示例代码**：

#### 2.3.5 OAuth 异常场景处理

**必须处理的异常场景**：

1. **用户拒绝授权**
   - 现象：OAuth provider 返回 `error=access_denied`
   - 处理：显示友好错误信息，引导用户使用其他登录方式

2. **State Token 验证失败**
   - 现象：state 不存在或已过期
   - 处理：显示 "登录链接已过期，请重新发起登录"

3. **授权码无效或过期**
   - 现象：用 code 换取 token 时失败
   - 处理：显示 "授权失败，请重新登录"

4. **获取用户信息失败**
   - 现象：access token 有效但无法获取用户信息
   - 处理：显示 "无法获取用户信息，请联系管理员"

5. **Email 冲突处理**
   - 现象：OAuth 返回的 email 已存在于系统中
   - 处理：自动关联 OAuth 账户到已有用户 (需要验证用户当前未登录)

6. **Provider 配置被禁用/删除**
   - 现象：用户点击登录按钮，但 Provider 已被禁用
   - 处理：在 `listProviders` API 中过滤掉禁用的 Provider

**错误响应格式**：

#### 2.3.6 Client Secret 更新逻辑

**编辑模式下 Client Secret 的处理规则**：

| 前端传递 | 后端处理 | 说明 |
|---------|---------|------|
| `undefined` | 不更新 `client_secret` 字段 | 保持原值 |
| `null` | 不更新 `client_secret` 字段 | 保持原值 |
| `""` (空字符串) | 不更新 `client_secret` 字段 | 前端表单留空表示不更新 |
| `"new-secret"` | 更新 `client_secret` 为新值 | 用户输入了新密钥 |

**前端实现**：

**后端实现**：

---

## 6. API 相关约束

**状态**: 必填

- 仅说明认证、授权、验证、回调或账号绑定等能力边界，不在 PRD 中展开端点、请求响应 schema、状态码矩阵。
- 必须遵守 realm 隔离、权限边界、凭证脱敏和幂等要求；涉及回调时需满足回调来源校验、重放防护和错误可恢复性。
- 若存在第三方身份提供商或支付/消息回调，应在技术设计或接口说明中维护详细契约，PRD 只保留业务约束和兼容性要求。

---

## 7. 前端/交互约束

**状态**: 必填

- 仅保留页面入口、关键用户路径、状态反馈、权限可见性和异常提示要求，不写组件实现步骤或前端类型定义。
- 认证相关流程应优先保证成功/失败状态清晰、回跳路径明确、敏感信息不回显，并对首次配置、失效、锁定、重试等场景提供稳定反馈。

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
- Realm Settings 文档: `docs/prd/realm-settings.md`
- OAuth 实体定义: `core/src/domain/oauth/entities.rs`
- OAuth Service: `core/src/domain/oauth/config_service.rs`
- OAuth HTTP Handlers: `api/src/application/http/oauth/`
- 前端 OAuth API: `frontend/src/lib/api.ts`
- 前端 OAuth 配置表单: `frontend/src/features/settings/oauth-config-form.tsx`
- 登录页面 OAuth 实现: `frontend/src/routes/$realm_id.login.tsx`

