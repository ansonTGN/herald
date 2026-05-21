# 第三方应用集成产品需求文档 (PRD)

**创建时间**: 2025-01-15
**最后更新**: 2026-05-21
**状态**: 🚧 Partially Implemented（从 Implicit Flow 升级到 Authorization Code + PKCE）

---
## 1. 相关用户故事

> **说明**: 详细的用户故事和验收标准请查看 `../../user-stories/` 目录中的对应文件。

### 1.1 第三方应用开发者用户故事

- 📄 [docs/user-stories/04-third-party-app-user-stories.md](/docs/user-stories/04-third-party-app-user-stories.md)
  - **[US-TP-001] OAuth 授权码登录（Authorization Code + PKCE）** (P0): 作为第三方应用，我希望使用 Authorization Code + PKCE 流程验证用户身份，以便安全获取访问令牌
  - **[US-TP-006] 处理异常情况** (P1): 作为第三方应用，我希望正确处理各种异常情况，以便提供友好体验
  - **[US-TP-007] 会话管理** (P1): 作为第三方应用，我希望管理用户会话，以便实现 SSO 和登出
  - **[US-TP-015] 第三方 Web SPA 发起 SSO 登录** (P0): 作为第三方应用开发者，我希望从 Web SPA 发起 Herald SSO 登录，以便用户无需额外后端即可完成认证
  - **[US-TP-016] 第三方后端用授权码换取令牌** (P0): 作为第三方应用开发者，我希望后端用授权码和 PKCE 验证换取令牌，以便安全完成认证

### 1.2 普通用户用户故事

- 📄 [docs/user-stories/03-regular-user-user-stories.md](/docs/user-stories/03-regular-user-user-stories.md)
  - **[US-RU-008] 访问第三方应用** (P0): 作为普通用户，我希望使用 Herald 账号登录第三方应用，以便获得 SSO 体验
  - **[US-RU-010] 从第三方 Web 应用跳转登录** (P0): 作为普通用户，我希望从第三方应用跳转到 Herald 完成认证后自动返回，以便无缝使用第三方服务

### 1.3 Client App 设置用户故事

- 📄 [docs/user-stories/client-app-settings.md](/docs/user-stories/client-app-settings.md)
  - **[US-TP-008] 配置 Client App 跳转地址白名单** (P0): redirect_uri 白名单精确匹配
  - **[US-TP-010] 启用/禁用 Client App** (P0): 禁用的 Client App 拒绝 OAuth 授权

### 1.4 优先级汇总

| 优先级 | 数量 | 关键故事 |
|--------|------|----------|
| P0 | 5 | Authorization Code + PKCE 流程、Web SPA SSO、令牌交换、第三方跳转登录、白名单配置 |
| P1 | 2 | 异常处理、会话管理 |
| P2 | 0 | - |

---
## 2. 范围界定

### 2.1 包含功能

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

### 2.2 不包含功能 (Out of Scope)

- ❌ **Refresh Token** (原因: 当前不支持令牌刷新)
- ❌ **Token 撤销** (原因: 当前不支持令牌撤销)
- ❌ **OAuth 2.0 Scope 管理** (原因: 没有细粒度 scope 授权页面)
- ❌ **用户主动授权/拒绝授权页面** (原因: 当前授权自动完成，用户无需手动批准)
- ❌ **Implicit Flow** (原因: 已被 OAuth 2.1 废弃，本方案直接替换)

### 2.3 依赖项

- ✅ **用户认证系统** (状态: 已实现) - 提供登录和会话管理
- ✅ **Client App 系统** (状态: 已实现) - OAuth 回调验证、redirect_uri 白名单
- ✅ **权限管理系统** (状态: 已实现) - 权限检查 API
- ✅ **Redis 缓存** (状态: 已实现) - state、authorization_code 存储
- ✅ **TOTP 系统** (状态: 已实现) - TOTP 二次认证
- 🚧 **Token 端点** (状态: 待实现) - authorization_code 换取 access_token

---
## 3. 需求概述

提供基于 **Authorization Code + PKCE** 的 OAuth 2.1 标准流程，允许第三方 Web 应用通过 Herald 系统验证用户身份。

### 3.1 核心特性

- **Authorization Code + PKCE**: 第三方 SPA 生成 code_verifier/code_challenge，通过授权码流程安全交换令牌，无需暴露 client_secret 给前端
- **Redirect 回调模式**: 授权成功后通过 `?code=xxx&state=xxx` 重定向回第三方 callback 地址
- **State 校验**: Redis 存储 state 关联 client、code_challenge 信息，防止 CSRF 和伪造
- **精确白名单匹配**: redirect_uri 白名单校验改为 origin + port 完全一致，防止开放重定向绕过
- **TOTP 兼容**: TOTP 二次认证流程中保持 OAuth 上下文，认证完成后同样返回 redirectTo

### 3.2 与旧 Implicit Flow 的区别

| 特性 | 旧 Implicit Flow | 新 Authorization Code + PKCE |
|------|-----------------|-------------------------------|
| 步骤数 | 2 步（授权 → 直接用 token） | 3 步（授权 → 换取令牌 → 使用令牌） |
| Token 获取 | URL Fragment `#token=xxx` | 查询参数 `?code=xxx`，后端用 code 换 token |
| 前端是否接触 token | 是（前端直接获得 token） | 否（前端只获得 code，后端换 token） |
| 安全性 | 中（token 暴露在浏览器） | 高（token 只在服务端交换） |
| PKCE 保护 | 无 | 有（code_verifier/code_challenge） |
| OAuth 标准兼容 | OAuth 2.0 Implicit（已废弃） | OAuth 2.1 推荐模式 |

### 3.3 适用场景

**适合使用 Authorization Code + PKCE 的场景**：
- 第三方 SPA（单页应用）
- 第三方 Web 应用（有后端服务）
- 需要高安全性的应用
- 需要 OAuth 2.1 标准兼容的场景

**仍可使用 Cookie 共享模式的场景**：
- 与 Herald 同域的内部应用
- 参考教程: `docs/tutorials/third-party-integration.md`

---
## 4. 当前实现状态

### 4.1 已完成功能

#### 后端（旧 Implicit Flow 雏形，需升级）
- ✅ Client App CRUD 和 redirect_uri 白名单
- ✅ authorize 端点（接收 clientId、redirectUri、state、responseType）
- ✅ Redis state 存储（TTL 5 分钟）
- ✅ login OAuth 分支（接收 oauthClientId、redirectUri、state）
- ⚠️ redirect_uri 白名单使用 `starts_with`（有安全隐患，需改为精确匹配）
- ⚠️ authorize 跳转路径与前端路由不匹配
- ⚠️ login OAuth 分支直接返回 token（需改为返回 authorization_code）

#### 前端
- ✅ 登录页面 OAuth 参数检测（clientId）
- ⚠️ search schema 缺少 OAuth 上下文字段（oauthClientId、redirectUri、state）

### 4.2 待完成功能

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

### 4.3 实施计划

参考 `.ai/future/fix_4.md` 中的详细实施方案。

---
## 5. 功能需求

### 5.1 授权请求（authorize）

- 接收第三方 SPA 发起的授权请求，参数包含 client_id、redirect_uri、state、response_type=code、code_challenge、code_challenge_method
- 校验 Client App 存在、启用且 redirect_uri 在白名单中（精确匹配）
- 将 state、client_id、redirect_uri、code_challenge 存入 Redis（TTL 5 分钟）
- 重定向到 Herald 登录页，携带 OAuth 上下文参数

### 5.2 用户认证 + 授权码生成

- 用户在 Herald 登录页输入凭据（密码或 TOTP）
- 登录成功后校验 Redis 中的 state（完整性校验）
- 生成 authorization_code 存入 Redis（关联 code_challenge、client_id、redirect_uri，TTL 5 分钟）
- 删除原 state（防重放）
- 返回 JSON 包含 redirectTo 指向第三方 callback 地址（携带 code 和 state 参数）

### 5.3 令牌交换（token endpoint）

- 接收第三方后端发起的令牌交换请求（authorization_code + code_verifier）
- 校验 code 有效、未使用、未过期
- 校验 client_id 和 redirect_uri 匹配
- 验证 code_verifier 的 SHA256 值匹配存储的 code_challenge（PKCE 校验）
- 校验通过后创建 session，返回 access_token
- 使用后删除 authorization_code（防重放）

### 5.4 安全约束

- **redirect_uri 白名单精确匹配**: 只允许 origin + port 完全一致下的路径差异，不允许前缀匹配
- **authorization_code 一次性使用**: 使用后立即从 Redis 删除
- **state 一次性使用**: 校验后立即从 Redis 删除
- **PKCE 校验**: code_verifier 的 SHA256 必须匹配 code_challenge
- **普通登录不受影响**: 无 OAuth 参数时，行为与现有完全一致

### 5.5 TOTP + OAuth 流程

- TOTP 临时会话中保存 OAuth 上下文（oauth_client_id、redirect_uri、state）
- TOTP 验证成功后检查临时会话中的 OAuth 字段
- 有 OAuth 字段时走同样的 authorization_code 生成 + redirectTo 逻辑

---
## 6. API 相关约束

**状态**: 必填

- 本功能涉及三个关键能力：authorize（授权请求）、login OAuth 分支（认证 + 授权码生成）、token（令牌交换）
- 必须遵守 realm 隔离、Client App 身份校验、凭证脱敏和防重放要求
- redirect_uri 校验必须精确匹配白名单（禁止前缀匹配）
- authorization_code 和 state 必须一次性使用（Redis 删除而非标记）
- PKCE 的 code_challenge 必须使用 S256 方法（SHA256）
- 详细端点契约和请求响应 schema 应在技术设计文档中维护

---
## 7. 前端/交互约束

**状态**: 必填

- 登录页 search schema 须支持 OAuth 上下文参数（oauthClientId、redirectUri、state）
- OAuth 参数完整（三项都存在）时提交登录须一并传给后端
- OAuth 参数不完整时显示错误提示，不静默降级为普通登录
- 后端返回 redirectTo 时直接跳转第三方 callback（不经 getSafeRedirect，安全由后端白名单保证）
- TOTP 完成后同样支持 redirectTo 跳转
- 无 OAuth 参数时行为与现有普通登录完全一致

---
## 8. 技术设计承接

**状态**: 必填

- 详细实施方案见 `.ai/future/fix_4.md`
- 接口细节、Redis key 设计、前端 search schema 变更、路由注册应在技术设计文档中承接
- 如历史实现已经存在，应以现有代码和 OpenAPI 规范为依据补充

---
## 9. 相关文件索引

### 9.1 后端文件
- `backend/api-oauth/src/authorize.rs` — 授权端点（需升级）
- `backend/api-oauth/src/token.rs` — 令牌端点（待新建）
- `backend/api-oauth/src/lib.rs` — 模块注册
- `backend/api-auth/src/login.rs` — 登录 OAuth 分支（需修复）
- `backend/api-auth/src/verify_totp.rs` — TOTP 验证（需加 OAuth 上下文）

### 9.2 前端文件
- `frontend/src/lib/schemas/search-params.ts` — search schema（需扩展）
- `frontend/src/routes/$realmId/auth/login.tsx` — 登录页（需透传 OAuth 参数）

---
## 10. 参考资料

- **实施方案**: `.ai/future/fix_4.md`
- **Client Apps 管理**: [docs/prd/integration/client-app.md](/docs/prd/integration/client-app.md)
- **权限验证**: [docs/prd/auth/permissions.md](/docs/prd/auth/permissions.md)
- **OAuth Provider 配置**: [docs/prd/auth/oauth-provider.md](/docs/prd/auth/oauth-provider.md)
- **TOTP**: [docs/prd/auth/totp.md](/docs/prd/auth/totp.md)
- **第三方接入教程**: `docs/tutorials/third-party-integration.md`
- **用户故事**: [docs/user-stories/04-third-party-app-user-stories.md](/docs/user-stories/04-third-party-app-user-stories.md)
