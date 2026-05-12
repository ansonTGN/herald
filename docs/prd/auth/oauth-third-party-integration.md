# 第三方应用集成产品需求文档 (PRD)

**创建时间**: 2025-01-15
**状态**: Implemented

---
## 1. 相关用户故事

> **说明**: 详细的用户故事和验收标准请查看 `../../user-stories/` 目录中的对应文件。

### 1.1 第三方应用开发者用户故事

- 📄 [docs/user-stories/04-third-party-app-user-stories.md](/docs/user-stories/04-third-party-app-user-stories.md)
  - **[US-TP-006] 第三方应用授权登录** (P0): 作为第三方应用开发者，我想要使用简化的 OAuth 流程，以便快速集成用户认证
  - **[US-TP-007] 授权码交换** (P0): 作为第三方应用开发者，我想要验证 Session Token，以便确认用户身份并检查用户权限，以便控制资源访问

### 1.2 用户故事优先级汇总

| 优先级 | 用户故事数量 | 关键故事 |
|--------|------------|---------|
| P0 | 3 | 简化的 OAuth 流程、Session Token 验证、权限检查 |
| P1 | 0 | - |
| P2 | 0 | - |

---

## 2. 范围界定

### 2.1 包含功能

- ✅ 简化的 OAuth 授权流程（2 步：授权 → 直接使用令牌）
- ✅ Session Token 通过 URL Fragment 传递（`#token=xxx`）
- ✅ Session IP 绑定（防止令牌被盗用）
- ✅ State Token 保护（防止 CSRF 攻击，5分钟有效期）
- ✅ 灵活的 Session TTL（OAuth 流程 10 分钟，正常登录 30 分钟）

### 2.2 不包含功能 (Out of Scope)

- ❌ **标准 OAuth 2.0 授权码流程** (原因: 本文档提供简化的替代方案)
- ❌ **Token Exchange 端点** (原因: 直接返回 Session Token，无需换取 access_token)
- ❌ **Refresh Token** (原因: 当前不支持令牌刷新)
- ❌ **Token 撤销** (原因: 当前不支持令牌撤销)

### 2.3 依赖项

- ✅ **用户认证系统** (状态: 已实现) - 提供登录和会话管理
- ✅ **Client App 系统** (状态: 已实现) - OAuth 回调验证
- ✅ **权限管理系统** (状态: 已实现) - 权限检查 API
- ✅ **Redis 缓存** (状态: 已实现) - Session Token 存储

---

## 3. 需求概述

提供简化的 OAuth 流程，允许第三方应用通过 Herald 系统验证用户身份，而无需实现完整的 OAuth 2.0 授权码流程。

### 1.1 核心特性

- ✅ **简化流程**：直接返回 Session Token，无需后端 Token Exchange
- ✅ **URL Fragment 传递**：Token 通过 `#token=xxx` 传递，避免服务器日志泄露
- ✅ **Session IP 绑定**：Session Token 绑定客户端 IP，防止令牌被盗用
- ✅ **State Token 保护**：使用 State Token 防止 CSRF 攻击（5分钟有效期）
- ✅ **灵活的 Session TTL**：OAuth 流程 10 分钟，正常登录 30 分钟

### 1.2 与标准 OAuth 2.0 的区别

| 特性 | 标准 OAuth 2.0 授权码流程 | 简化 OAuth 流程 |
|------|------------------------|---------------|
| 步骤数 | 3 步（授权 → 换取令牌 → 使用令牌） | 2 步（授权 → 直接使用令牌） |
| Token 获取 | 后端用 code 换取 access_token | 直接在 URL Fragment 中返回 |
| Token 类型 | JWT (Bearer Token) | Session Token (存储在 Redis) |
| Token 传递 | Query Parameter (`?code=xxx`) | URL Fragment (`#token=xxx`) |
| 后端验证 | 需要 Token Exchange 端点 | 直接验证 Session Token |
| 安全性 | 高（标准流程） | 中高（适用于特定场景） |

### 1.3 适用场景

**适合使用简化 OAuth 流程的场景**：
- 单页应用（SPA）
- 移动应用
- 快速原型开发
- 内部工具集成
- 不需要复杂权限管理的场景

**不适合使用简化 OAuth 流程的场景**：
- 需要高安全性的金融应用
- 需要刷新令牌（Refresh Token）的场景
- 需要撤销令牌的场景
- 需要标准 OAuth 2.0 兼容性的场景

---

## 4. 当前实现状态

### 8.1 已完成功能 ✅

#### 后端实现
- ✅ Session Token 生成和验证
- ✅ State Token 生成、存储和验证
- ✅ Session IP 绑定
- ✅ Redirect URI 白名单验证
- ✅ URL Fragment Token 传递

#### 前端实现
- ✅ 登录页面 OAuth 参数检测（`$realm_id.login.tsx`）
- ✅ OAuth 流程自动识别
- ✅ TypeScript 类型定义
- ✅ API 调用函数

### 8.2 待完成功能 ⏳

#### 测试
- ⏳ 后端单元测试（OAuth 授权流程）
- ⏳ 前端组件测试（OAuth 参数检测）
- ⏳ E2E 测试（完整 OAuth 流程）
- ⏳ 性能测试（API 响应时间）

#### 文档
- ⏳ 集成示例代码
- ⏳ API 文档更新
- ⏳ 用户故事更新

---

## 5. 功能需求

### 2.1 OAuth 授权端点

#### 2.1.1 发起授权请求

**Query Parameters**:
- `client_id` (required): Client App 标识符
- `redirect_uri` (required): 授权成功后的跳转地址（必须在 Client App 配置的白名单中）
- `state` (required): CSRF 保护令牌（由第三方应用生成）

**响应**:
- **302 重定向**到 Herald 登录页面，携带以下参数：
  - `client_id`: Client App 标识符
  - `redirect_uri`: 回调地址
  - `state`: State Token

**登录页面 URL 示例**:

#### 2.1.2 用户登录并授权

**流程说明**:
1. 用户访问登录页面
2. 用户输入用户名/密码并登录
3. Herald 系统验证用户凭据
4. Herald 系统验证 `client_id` 和 `redirect_uri` 是否匹配
5. Herald 系统生成 Session Token
6. Herald 系统重定向到 `redirect_uri#token=xxx&state=xxx`

**Session Token 格式**:

**示例**: `realm-1_0191a2b3c4d5e6f7g8h9i0j1k2l3m4n5_1736899200`

#### 2.1.3 Token 传递方式

**重要**: Token 通过 **URL Fragment** 传递，而不是 Query Parameter。

**原因**:
- URL Fragment 不会发送到服务器
- 避免 Token 被记录在服务器访问日志中
- 提高安全性

**回调 URL 示例**:

### 2.2 Session Token 验证

#### 2.2.1 Session Token 结构

**格式**: `{realm-id}_{uuidv7}_{timestamp}`

**组成部分**:
- `realm-id`: Realm 标识符
- `uuidv7`: 时间排序的 UUID（用户会话唯一标识）
- `timestamp`: Unix 时间戳（秒）

**示例**: `realm-1_0191a2b3c4d5e6f7g8h9i0j1k2l3m4n5_1736899200`

#### 2.2.2 Session Token 验证流程

**响应**:

**验证逻辑**:
1. 从 Redis 中加载 Session（使用 Token 作为 Key）
2. 验证 Session 是否过期
3. 验证客户端 IP 是否匹配（IP 绑定）
4. 验证 `client_id` 是否匹配
5. 使用 RBAC 验证用户权限

### 2.3 State Token 管理

#### 2.3.1 State Token 生成

**格式**: UUID v7

**存储位置**: Redis

**Key 格式**: `oauth:state:{state_token}`

**Value 格式**:

**过期时间**: 300 秒（5 分钟）

#### 2.3.2 State Token 验证

**验证时机**: 用户登录成功后，在重定向到 `redirect_uri` 之前

**验证逻辑**:
1. 从 Redis 中获取 State Token 数据
2. 验证 State Token 是否存在且未过期
3. 验证 `client_id` 和 `redirect_uri` 是否匹配
4. 验证成功后立即删除 State Token（一次性使用）

**验证失败处理**: 返回 400 错误，显示 "Invalid or expired state token"

### 2.4 Session 安全机制

#### 2.4.1 Session IP 绑定

**目的**: 防止 Session Token 被盗用

**实现**:
- Session 创建时记录客户端 IP
- 每次 Session 验证时检查客户端 IP 是否匹配
- IP 不匹配时返回 403 Forbidden

**IP 获取优先级**:
1. `X-Forwarded-For` Header（代理/负载均衡器）
2. `X-Real-IP` Header（Nginx）
3. 直接连接的远程 IP

#### 2.4.2 Session TTL 配置

**两种 Session TTL**:
- **OAuth 流程**: 600 秒（10 分钟）
- **正常登录**: 1800 秒（30 分钟）

**原因**: OAuth 流程通常用于第三方应用短期授权，使用较短的 TTL 提高安全性。

#### 2.4.3 Session 存储

**存储位置**: Redis

**Key 格式**: `session:{realm_id}:{session_token}`

**Value 格式**:

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

### 9.1 相关文档
- **Client Apps 管理**: `docs/prd/client-app.md`
- **权限验证**: `docs/prd/permissions.md`
- **OAuth Provider 配置**: `docs/prd/oauth-provider.md`
- **用户故事**: [docs/user-stories/04-third-party-app-user-stories.md](/docs/user-stories/04-third-party-app-user-stories.md)

### 9.2 实现文件索引

#### 后端文件

#### 前端文件
- 登录页面: `frontend/src/routes/$realm_id.login.tsx`
- API 调用: `frontend/src/lib/api.ts`
- 类型定义: `frontend/src/lib/types/auth.ts`
- 路由守卫: `frontend/src/lib/route-guards.ts`

