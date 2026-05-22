# OAuth 与第三方集成产品需求文档 (PRD)

**创建时间**: 2025-01-10
**最后更新**: 2026-05-21
**状态**: Partially Implemented

---

## 1. 相关用户故事

> **说明**: 详细的用户故事和验收标准请查看 `docs/user-stories/` 目录中的对应文件。

### 1.1 Realm Admin 用户故事

- 📄 [docs/user-stories/core/realm-admin.md](/docs/user-stories/core/realm-admin.md)
  - **[US-RA-008] 配置 Realm 设置** (P0): 作为 Realm Admin，我想要配置 OAuth Provider，以便启用第三方登录

### 1.2 第三方应用开发者用户故事

- 📄 [docs/user-stories/auth/third-party-app.md](/docs/user-stories/auth/third-party-app.md)
  - **[US-TP-001] OAuth 授权码登录（Authorization Code + PKCE）** (P0): 作为第三方应用，我希望使用 Authorization Code + PKCE 流程验证用户身份，以便安全获取访问令牌
  - **[US-TP-002] 验证用户登录状态** (P0): 作为第三方应用,我想要验证用户的登录状态和身份,从而保护应用资源
  - **[US-TP-003] 检查用户权限** (P0): 作为第三方应用,我想要检查用户是否有权限访问特定资源,从而实现细粒度的访问控制
  - **[US-TP-006] 第三方应用授权登录** (P0): 作为第三方应用开发者，我想要使用 OAuth Provider 进行第三方登录，以便快速访问系统
  - **[US-TP-006] 处理异常情况** (P1): 作为第三方应用，我希望正确处理各种异常情况，以便提供友好体验
  - **[US-TP-007] 会话管理** (P1): 作为第三方应用，我希望管理用户会话，以便实现 SSO 和登出
  - **[US-TP-008] 第三方 API 认证** (P0): 作为第三方应用,我想要使用 API Key 认证调用 Herald 第三方接口,从而安全地集成 Herald 系统
  - **[US-TP-009] 查询订阅状态** (P0): 作为第三方应用,我想要能够查询客户端应用的订阅状态,从而根据订阅状态提供相应的功能和体验
  - **[US-TP-015] 第三方 Web SPA 发起 SSO 登录** (P0): 作为第三方应用开发者，我希望从 Web SPA 发起 Herald SSO 登录，以便用户无需额外后端即可完成认证
  - **[US-TP-016] 第三方后端用授权码换取令牌** (P0): 作为第三方应用开发者，我希望后端用授权码和 PKCE 验证换取令牌，以便安全完成认证

### 1.3 普通用户用户故事

- 📄 [docs/user-stories/core/regular-user.md](/docs/user-stories/core/regular-user.md)
  - **[US-RU-008] 访问第三方应用** (P0): 作为普通用户，我希望使用 Herald 账号登录第三方应用，以便获得 SSO 体验
  - **[US-RU-010] 从第三方 Web 应用跳转登录** (P0): 作为普通用户，我希望从第三方应用跳转到 Herald 完成认证后自动返回，以便无缝使用第三方服务

### 1.4 主管理员用户故事

- 📄 [docs/user-stories/core/admin-realm.md](/docs/user-stories/core/admin-realm.md)
  - **API Key 管理** (P0): 作为主管理员,我想要创建和管理第三方 API Keys,从而控制第三方访问

### 1.5 Client App 设置用户故事

- 📄 [docs/user-stories/auth/client-app-settings.md](/docs/user-stories/auth/client-app-settings.md)
  - **[US-TP-008] 配置 Client App 跳转地址白名单** (P0): redirect_uri 白名单精确匹配
  - **[US-TP-010] 启用/禁用 Client App** (P0): 禁用的 Client App 拒绝 OAuth 授权

### 1.6 用户故事优先级汇总

| 优先级 | 用户故事数量 | 关键故事 |
|--------|------------|---------|
| P0 | 12 | 配置 OAuth Provider、Authorization Code + PKCE 流程、Web SPA SSO、令牌交换、API Key 认证、权限检查、订阅查询、第三方跳转登录、白名单配置 |
| P1 | 2 | 异常处理、会话管理 |
| P2 | 0 | - |

---

## 2. 范围界定

### 2.1 包含功能

**OAuth Provider 配置管理:**
- ✅ OAuth Provider 配置管理（Google、GitHub、Facebook、Apple）
- ✅ Provider 启用/禁用控制
- ✅ 前端 OAuth 配置表单
- ✅ 前端登录页面 OAuth 按钮
- ✅ Provider 配置 CRUD API
- ✅ 强类型实体（`OAuthProviderConfig`）

**第三方应用 OAuth 集成 (Authorization Code + PKCE):**
- ✅ Authorization Code + PKCE 流程（OAuth 2.1 推荐模式）
- ✅ 第三方 SPA 通过 code_challenge 发起授权请求
- ✅ 用户在 Herald 登录页完成认证后生成 authorization_code
- ✅ authorization_code 通过 redirect_uri 查询参数回传第三方（`?code=xxx&state=xxx`）
- ✅ 第三方后端用 authorization_code + code_verifier 换取 access_token
- ✅ Redis state 校验（防 CSRF）和 authorization_code 一次性使用（防重放）
- ✅ redirect_uri 白名单精确匹配（origin + port 一致）
- ✅ TOTP 二次认证流程中保持 OAuth 上下文
- ✅ 前端登录页透传 OAuth 参数（oauthClientId、redirectUri、state）
- ✅ 前端处理后端返回的 redirectTo 跳转第三方 callback

**第三方 API 接入:**
- ✅ 第三方 API 认证系统（X-API-Key header）
- ✅ API Key 数据模型（client_api_keys 表）
- ✅ API Key 使用统计（last_used_at, usage_count）
- ✅ Realm 隔离（API Key 绑定到 realm）
- ✅ OpenAPI 文档集成（third tag）

### 2.2 不包含功能 (Out of Scope)

**OAuth Provider:**
- ❌ **后端 HTTP Handlers 完整实现** (原因: 当前为 TODO)
- ❌ **后端路由注册到 HTTP Server** (原因: 待实施)
- ❌ **前端 Provider 配置管理页面** (原因: 待实施)
- ❌ **端到端测试** (原因: 待实施)
- ❌ **OAuth 登录流程完整实现** (原因: 待实施)

**第三方应用 OAuth 集成:**
- ❌ **Refresh Token** (原因: 当前不支持令牌刷新)
- ❌ **Token 撤销** (原因: 当前不支持令牌撤销)
- ❌ **OAuth 2.0 Scope 管理** (原因: 没有细粒度 scope 授权页面)
- ❌ **用户主动授权/拒绝授权页面** (原因: 当前授权自动完成，用户无需手动批准)
- ❌ **Implicit Flow** (原因: 已被 OAuth 2.1 废弃，本方案直接替换)

**第三方 API 接入:**
- ❌ API Key 管理界面 (原因: 后续优化 Phase 1 功能)
- ❌ 速率限制 (原因: 后续优化 Phase 1 功能)
- ❌ 审计日志 (原因: 后续优化 Phase 1 功能)
- ❌ Scope 验证 (原因: 后续优化 Phase 2 功能)
- ❌ API Key 轮换 (原因: 后续优化 Phase 2 功能)
- ❌ Webhooks 支持 (原因: 后续优化 Phase 3 功能)
- ❌ GraphQL 支持 (原因: 后续优化 Phase 3 功能)

### 2.3 依赖项

- ✅ **Realm 系统** (状态: 已实现) - OAuth Config 属于 Realm 级别；API Key 绑定到 realm
- ✅ **权限管理系统** (状态: 已实现) - Realm Admin 权限检查；权限检查 API
- ✅ **Client App 系统** (状态: 已实现) - OAuth 回调验证、redirect_uri 白名单
- ✅ **用户认证系统** (状态: 已实现) - 提供登录和会话管理
- ✅ **Redis 缓存** (状态: 已实现) - state、authorization_code 存储
- ✅ **TOTP 系统** (状态: 已实现) - TOTP 二次认证
- ✅ **订阅系统** (状态: 已实现) - 订阅状态查询
- ✅ **Session Token 验证** (状态: 已实现) - 第三方 API 中的 session token 校验
- 🚧 **Token 端点** (状态: 待实现) - authorization_code 换取 access_token

---

## 3. OAuth Provider 配置

在 Herald 管理后台中提供 OAuth Provider 配置管理功能，允许管理员为每个 Realm 配置第三方登录提供商（Google、GitHub、Facebook、Apple）。

### 3.1 命名说明

**建议使用 "Provider" 或 "Identity Provider"**：
- ✅ 推荐：**Provider Settings** / **Identity Providers**
- ❌ 不推荐：**OAuth Config** / **OAuth Settings**

**理由**：
- "Provider" 更简洁，符合行业标准（如 Auth0、Okta 等使用 "Identity Providers"）
- 避免与 "OAuth Config" 的技术术语混淆
- 用户体验更好，更易理解

### 3.2 功能定位

**OAuth Provider 系统定位**（独立于 Realm Config）：
- 适用于：管理第三方登录提供商的完整配置
- 复杂度：高（provider 验证、scopes、授权流程）
- 数据模型：强类型结构体（`OAuthProviderConfig`）
- 类型安全：编译期检查

**与 Realm Config 的区别**：
- Realm Config：简单的 key-value 配置（如 Turnstile、Registration）
- OAuth Provider：复杂的结构化配置（独立的 Provider 实体）

### 3.3 导航菜单配置

**建议**：将 Provider 配置集成到 Settings 菜单中，作为独立的 Tab：

**Settings 页面 Tabs**：
- Turnstile
- Registration
- **Providers** ← 新增

### 3.4 Provider 配置管理页面

#### 3.4.1 页面布局

创建 `frontend/src/features/settings/provider-config-page.tsx`：

#### 3.4.2 Provider 列表

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

#### 3.4.3 Provider 配置表单

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

### 3.5 OAuth 登录流程

#### 3.5.1 登录页面 Provider 按钮

**参考**：`frontend/src/routes/$realm_id.login.tsx:37-128`

**当前实现**：
- ✅ 动态加载启用的 Providers（`oauthApi.listProviders`）
- ✅ 显示 Provider 登录按钮
- ✅ 点击后调用 `oauthApi.initiateLogin`

#### 3.5.2 OAuth 登录流程

**流程图**：

#### 3.5.3 State Token 管理策略

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

#### 3.5.4 Session 管理策略

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

#### 3.5.5 OAuth 异常场景处理

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

#### 3.5.6 Client Secret 更新逻辑

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

## 4. 第三方应用 OAuth 集成（Authorization Code + PKCE）

提供基于 **Authorization Code + PKCE** 的 OAuth 2.1 标准流程，允许第三方 Web 应用通过 Herald 系统验证用户身份。

### 4.1 核心特性

- **Authorization Code + PKCE**: 第三方 SPA 生成 code_verifier/code_challenge，通过授权码流程安全交换令牌，无需暴露 client_secret 给前端
- **Redirect 回调模式**: 授权成功后通过 `?code=xxx&state=xxx` 重定向回第三方 callback 地址
- **State 校验**: Redis 存储 state 关联 client、code_challenge 信息，防止 CSRF 和伪造
- **精确白名单匹配**: redirect_uri 白名单校验改为 origin + port 完全一致，防止开放重定向绕过
- **TOTP 兼容**: TOTP 二次认证流程中保持 OAuth 上下文，认证完成后同样返回 redirectTo

### 4.2 与旧 Implicit Flow 的区别

| 特性 | 旧 Implicit Flow | 新 Authorization Code + PKCE |
|------|-----------------|-------------------------------|
| 步骤数 | 2 步（授权 → 直接用 token） | 3 步（授权 → 换取令牌 → 使用令牌） |
| Token 获取 | URL Fragment `#token=xxx` | 查询参数 `?code=xxx`，后端用 code 换 token |
| 前端是否接触 token | 是（前端直接获得 token） | 否（前端只获得 code，后端换 token） |
| 安全性 | 中（token 暴露在浏览器） | 高（token 只在服务端交换） |
| PKCE 保护 | 无 | 有（code_verifier/code_challenge） |
| OAuth 标准兼容 | OAuth 2.0 Implicit（已废弃） | OAuth 2.1 推荐模式 |

### 4.3 适用场景

**适合使用 Authorization Code + PKCE 的场景**：
- 第三方 SPA（单页应用）
- 第三方 Web 应用（有后端服务）
- 需要高安全性的应用
- 需要 OAuth 2.1 标准兼容的场景

**仍可使用 Cookie 共享模式的场景**：
- 与 Herald 同域的内部应用
- 参考教程: `docs/tutorials/third-party-integration.md`

### 4.4 功能需求

#### 4.4.1 授权请求（authorize）

- 接收第三方 SPA 发起的授权请求，参数包含 client_id、redirect_uri、state、response_type=code、code_challenge、code_challenge_method
- 校验 Client App 存在、启用且 redirect_uri 在白名单中（精确匹配）
- 将 state、client_id、redirect_uri、code_challenge 存入 Redis（TTL 5 分钟）
- 重定向到 Herald 登录页，携带 OAuth 上下文参数

#### 4.4.2 用户认证 + 授权码生成

- 用户在 Herald 登录页输入凭据（密码或 TOTP）
- 登录成功后校验 Redis 中的 state（完整性校验）
- 生成 authorization_code 存入 Redis（关联 code_challenge、client_id、redirect_uri，TTL 5 分钟）
- 删除原 state（防重放）
- 返回 JSON 包含 redirectTo 指向第三方 callback 地址（携带 code 和 state 参数）

#### 4.4.3 令牌交换（token endpoint）

- 接收第三方后端发起的令牌交换请求（authorization_code + code_verifier）
- 校验 code 有效、未使用、未过期
- 校验 client_id 和 redirect_uri 匹配
- 验证 code_verifier 的 SHA256 值匹配存储的 code_challenge（PKCE 校验）
- 校验通过后创建 session，返回 access_token
- 使用后删除 authorization_code（防重放）

#### 4.4.4 安全约束

- **redirect_uri 白名单精确匹配**: 只允许 origin + port 完全一致下的路径差异，不允许前缀匹配
- **authorization_code 一次性使用**: 使用后立即从 Redis 删除
- **state 一次性使用**: 校验后立即从 Redis 删除
- **PKCE 校验**: code_verifier 的 SHA256 必须匹配 code_challenge
- **普通登录不受影响**: 无 OAuth 参数时，行为与现有完全一致

#### 4.4.5 TOTP + OAuth 流程

- TOTP 临时会话中保存 OAuth 上下文（oauth_client_id、redirect_uri、state）
- TOTP 验证成功后检查临时会话中的 OAuth 字段
- 有 OAuth 字段时走同样的 authorization_code 生成 + redirectTo 逻辑

---

## 5. 第三方 API 接入

### 5.1 功能描述

第三方应用通过 API Key 认证接入 Herald 系统，实现用户登录状态验证、权限检查和订阅状态查询等功能。

### 5.2 关键特性

- **专用认证**: 使用 `X-API-Key` header 认证，而非 session token
- **Realm 隔离**: API Key 绑定到特定 realm，防止跨租户访问
- **使用统计**: 记录 API Key 的使用次数和最后使用时间
- **OpenAPI 文档**: 新增 `third` tag 标识第三方接口

### 5.3 当前问题

1. **缺少统一前缀**: 第三方接口没有统一的 URL 前缀标识
2. **缺少专门认证**: 现有接口使用 session token 认证，不适合第三方集成
3. **缺少订阅查询**: 没有公开的 API 供第三方查询用户订阅状态
4. **OpenAPI 文档**: 缺少 `third` tag 标识第三方接口

### 5.4 功能需求

#### 5.4.1 API Key 认证系统

**功能描述**: 提取并验证 `X-API-Key` header，查询数据库验证 API Key 有效性，更新使用统计，注入 `ThirdPartyIdentity` 到 request extensions。

**验收标准**:
- ✅ 提取 `X-API-Key` header
- ✅ 哈希并查询数据库验证
- ✅ 检查 API Key 是否启用和未过期
- ✅ 更新使用统计（last_used_at, usage_count）
- ✅ 注入 `ThirdPartyIdentity` 到 request extensions
- ✅ 无效/缺失 API Key 返回 401 Unauthorized
- ✅ 过期/禁用 API Key 返回 401 Unauthorized
- ✅ 支持 Realm 隔离（API Key 只能访问所属 realm 的资源）

#### 5.4.2 权限检查 API

**功能描述**: 第三方应用使用 API Key 和用户 session token 验证用户是否有权限访问特定资源。

**验收标准**:
- ✅ 接受 API Key 认证（`X-API-Key` header）
- ✅ 验证 session token 有效性
- ✅ 检查用户是否有指定权限（基于 `rules` 数组）
- ✅ 返回权限检查结果（`allowed`, `user_id`）
- ✅ 支持 batch 权限检查（多个 rules）
- ✅ 无效 session token 返回 `{"allowed": false}`
- ✅ API Key 无效返回 401 Unauthorized

#### 5.4.3 订阅状态查询 API

**功能描述**: 第三方应用使用 API Key 查询指定客户端应用的订阅状态。

**验收标准**:
- ✅ 接受 API Key 认证（`X-API-Key` header）
- ✅ 接受 client_app_id（URL 参数）
- ✅ 验证客户端应用存在
- ✅ 查询订阅状态
- ✅ 返回订阅信息（status, tier, plan_name）
- ✅ 无订阅时返回 free tier 信息
- ✅ 客户端应用不存在返回 404 Not Found
- ✅ API Key 无效返回 401 Unauthorized

#### 5.4.4 数据模型

**client_api_keys 表**:
- ✅ `id` (UUID): API Key 唯一标识
- ✅ `name` (VARCHAR(255)): API Key 名称
- ✅ `api_key_hash` (VARCHAR(255)): API Key 哈希值（不存储明文）
- ✅ `realm_id` (VARCHAR(36)): 所属租户 ID
- ✅ `enabled` (BOOLEAN): 是否启用
- ✅ `expires_at` (TIMESTAMP): 过期时间（NULL 表示永不过期）
- ✅ `created_at` (TIMESTAMP): 创建时间
- ✅ `last_used_at` (TIMESTAMP): 最后使用时间
- ✅ `usage_count` (INTEGER): 使用次数

**索引**:
- ✅ `idx_client_api_keys_realm` on `realm_id`
- ✅ `idx_client_api_keys_key` on `api_key_hash`

---

## 6. 当前实现状态

### 6.1 OAuth Provider 配置

#### 已完成
- [x] 后端实体定义 (`core/src/domain/oauth/entities.rs`)
- [x] 后端 Service 定义 (`core/src/domain/oauth/config_service.rs`)
- [x] 后端 Repository 接口定义 (`core/src/domain/oauth/ports.rs`)
- [x] 后端 HTTP Handlers 框架 (`api/src/application/http/oauth/`)
- [x] 前端 API 封装 (`frontend/src/lib/api.ts` - `oauthConfigApi`)
- [x] 前端 OAuth 配置表单 (`frontend/src/features/settings/oauth-config-form.tsx`)
- [x] 前端登录页面 OAuth 按钮 (`frontend/src/routes/$realm_id.login.tsx`)

#### 部分实现
- [ ] 后端 HTTP Handlers 完整实现（当前为 TODO）
- [ ] **后端 HTTP Handlers 框架已定义**：`api/src/application/http/oauth/` 目录下的处理器已定义，但核心业务逻辑返回 "Not implemented"
  - `config.rs` - OAuth 配置 CRUD（已定义，待实现）
  - `providers.rs` - Provider 列表（已实现）
  - `login.rs` - OAuth 登录（已定义框架，待实现）
  - `callback.rs` - OAuth 回调（已定义框架，待实现）
- [ ] 后端路由注册到 HTTP Server（状态需确认）
- [ ] 前端 Provider 配置管理页面

#### 待实施
- [ ] 端到端测试
- [ ] OAuth 登录流程完整实现

### 6.2 第三方应用 OAuth 集成（旧 Implicit Flow 雏形，需升级）

#### 已完成功能

**后端:**
- ✅ Client App CRUD 和 redirect_uri 白名单
- ✅ authorize 端点（接收 clientId、redirectUri、state、responseType）
- ✅ Redis state 存储（TTL 5 分钟）
- ✅ login OAuth 分支（接收 oauthClientId、redirectUri、state）
- ⚠️ redirect_uri 白名单使用 `starts_with`（有安全隐患，需改为精确匹配）
- ⚠️ authorize 跳转路径与前端路由不匹配
- ⚠️ login OAuth 分支直接返回 token（需改为返回 authorization_code）

**前端:**
- ✅ 登录页面 OAuth 参数检测（clientId）
- ⚠️ search schema 缺少 OAuth 上下文字段（oauthClientId、redirectUri、state）

#### 待完成功能

- ❌ authorize 支持 PKCE 参数（code_challenge、code_challenge_method）
- ❌ authorize 只接受 `response_type=code`，修正跳转路径
- ❌ redirect_uri 白名单改为精确匹配
- ❌ login 加 Redis state 校验 + authorization_code 生成 + JSON redirectTo 返回
- ❌ TOTP 临时会话加 OAuth 上下文字段
- ❌ TOTP verify 后返回 redirectTo
- ❌ 新建 token 端点（authorization_code + code_verifier → access_token）
- ❌ 前端 search schema 扩展 OAuth 参数
- ❌ 前端登录提交透传 OAuth 字段
- ❌ 前端处理 redirectTo 跳转第三方 callback
- ❌ 第三方 Web SSO 教程

#### 实施计划

参考 `.ai/future/fix_4.md` 中的详细实施方案。

### 6.3 第三方 API 接入

| 功能模块 | 状态 | 备注 |
|---------|------|------|
| API Key 认证中间件 | ❌ 待实施 | 中间件文件待创建 |
| OpenAPI 文档更新 (third tag) | ❌ 待实施 | 需添加第三方接口的 OpenAPI tag |
| 场景测试 | ❌ 待实施 | 需编写完整测试用例 |

---

## 7. API 相关约束

**状态**: 必填

- 仅说明认证、授权、验证、回调或账号绑定等能力边界，不在 PRD 中展开端点、请求响应 schema、状态码矩阵。
- 必须遵守 realm 隔离、权限边界、凭证脱敏和幂等要求；涉及回调时需满足回调来源校验、重放防护和错误可恢复性。
- 若存在第三方身份提供商或支付/消息回调，应在技术设计或接口说明中维护详细契约，PRD 只保留业务约束和兼容性要求。
- redirect_uri 校验必须精确匹配白名单（禁止前缀匹配）
- authorization_code 和 state 必须一次性使用（Redis 删除而非标记）
- PKCE 的 code_challenge 必须使用 S256 方法（SHA256）
- 第三方接入必须遵守 Client App/第三方身份校验、凭证脱敏、回调安全和可观测性要求
- 详细端点契约、认证方式和错误模型应下沉到技术设计、接口说明或 SDK 文档

---

## 8. 前端/交互约束

**状态**: 必填

- 仅保留页面入口、关键用户路径、状态反馈、权限可见性和异常提示要求，不写组件实现步骤或前端类型定义。
- 认证相关流程应优先保证成功/失败状态清晰、回跳路径明确、敏感信息不回显，并对首次配置、失效、锁定、重试等场景提供稳定反馈。
- 登录页 search schema 须支持 OAuth 上下文参数（oauthClientId、redirectUri、state）
- OAuth 参数完整（三项都存在）时提交登录须一并传给后端
- OAuth 参数不完整时显示错误提示，不静默降级为普通登录
- 后端返回 redirectTo 时直接跳转第三方 callback（不经 getSafe_redirect，安全由后端白名单保证）
- TOTP 完成后同样支持 redirectTo 跳转
- 无 OAuth 参数时行为与现有普通登录完全一致
- 涉及第三方接入时，需明确哪些流程由 Herald 后台完成，哪些流程在第三方应用或外部平台完成

---

## 9. 技术设计承接

**状态**: 必填

- 接口细节、数据库结构、迁移策略、类型定义、调度方案、SDK 设计和实现步骤，应在 `docs/design/`、`.ai/design/`、接口说明或代码中承接。
- 如历史实现已经存在，应以现有设计文档、OpenAPI、迁移文件和代码为依据补充，不回写到 PRD 正文。
- 详细实施方案见 `.ai/future/fix_4.md`
- Redis key 设计、前端 search schema 变更、路由注册应在技术设计文档中承接

---

## 10. 相关文件索引

### 10.1 OAuth Provider 配置

- OAuth 实体定义: `core/src/domain/oauth/entities.rs`
- OAuth Service: `core/src/domain/oauth/config_service.rs`
- OAuth HTTP Handlers: `api/src/application/http/oauth/`
- 前端 OAuth API: `frontend/src/lib/api.ts`
- 前端 OAuth 配置表单: `frontend/src/features/settings/oauth-config-form.tsx`
- 登录页面 OAuth 实现: `frontend/src/routes/$realm_id.login.tsx`

### 10.2 第三方应用 OAuth 集成

**后端文件:**
- `backend/api-oauth/src/authorize.rs` — 授权端点（需升级）
- `backend/api-oauth/src/token.rs` — 令牌端点（待新建）
- `backend/api-oauth/src/lib.rs` — 模块注册
- `backend/api-auth/src/login.rs` — 登录 OAuth 分支（需修复）
- `backend/api-auth/src/verify_totp.rs` — TOTP 验证（需加 OAuth 上下文）

**前端文件:**
- `frontend/src/lib/schemas/search-params.ts` — search schema（需扩展）
- `frontend/src/routes/$realmId/auth/login.tsx` — 登录页（需透传 OAuth 参数）

### 10.3 第三方 API 接入

**后端文件:**
- 状态: ✅ 已创建
- 状态: ❌ 待创建 — 路由模块导出
- 状态: ❌ 待创建 — API Key 认证中间件
- 状态: ❌ 待创建 — 权限检查 API
- 状态: ❌ 待创建 — 订阅状态 API
- 状态: ❌ 待修改 — 添加 `pub mod third;`
- 状态: ❌ 待修改 — 集成 third 路由、添加 OpenAPI 配置

**测试文件:**
- 状态: ❌ 待创建 — API Key 认证、权限检查、订阅查询场景测试

---

## 11. 参考资料

- 前端开发指南: `../../spec/frontend/development.md`
- Realm Settings 文档: `docs/prd/realm-settings.md`
- **实施方案**: `.ai/future/fix_4.md`
- **Client Apps 管理**: [docs/prd/integration/client-app.md](/docs/prd/integration/client-app.md)
- **权限验证**: [docs/prd/auth/permissions.md](/docs/prd/auth/permissions.md)
- **TOTP**: [docs/prd/auth/totp.md](/docs/prd/auth/totp.md)
- **第三方接入教程**: `docs/tutorials/third-party-integration.md`
- **用户故事**: [docs/user-stories/auth/third-party-app.md](/docs/user-stories/auth/third-party-app.md)
- **后端开发指南**: [spec/backend/development.md](/spec/backend/development.md)
- **测试指南**: [spec/backend/testing.md](/spec/backend/testing.md)
- **设计文档**: `../../.ai/future/third.md`（待补充）
- **类似系统**:
  - Keycloak Admin API: https://www.keycloak.org/docs-api/latest/rest_api/
  - Stripe API: https://stripe.com/docs/api
