# Device Code 登录产品需求文档 (PRD)

**创建时间**: 2026-05-14
**状态**: Draft
**优先级**: P0

---

## 1. 相关用户故事

> 详细故事与验收标准请查看 `docs/user-stories/` 中对应文档。

### 1.1 相关故事

- `[US-DC-001]` CLI 工具发起设备授权，优先级 P0，来源 `docs/user-stories/15-device-code-user-stories.md`
  - 角色：Third-Party App
  - 摘要：CLI 通过 Device Authorization Grant 请求 device_code 和 user_code
- `[US-DC-002]` 用户在验证页面完成授权，优先级 P0，来源 `docs/user-stories/15-device-code-user-stories.md`
  - 角色：Regular User
  - 摘要：用户在 Herald 验证页面输入 user_code 并完成登录授权
- `[US-DC-003]` CLI 工具轮询获取令牌，优先级 P0，来源 `docs/user-stories/15-device-code-user-stories.md`
  - 角色：Third-Party App
  - 摘要：CLI 按 interval 轮询令牌端点，用户授权后获得 access token
- `[US-DC-004]` Realm Admin 配置 Device Code Grant，优先级 P1，来源 `docs/user-stories/15-device-code-user-stories.md`
  - 角色：Realm Admin
  - 摘要：管理员为 Client App 启用或禁用 Device Code Grant
- `[US-DC-005]` 设备验证页面 API，优先级 P1，来源 `docs/user-stories/15-device-code-user-stories.md`
  - 角色：Third-Party App
  - 摘要：开放 API 供第三方应用构建自定义设备码验证体验

### 1.2 优先级汇总

| 优先级 | 数量 | 关键故事 |
|--------|------|----------|
| P0 | 3 | 设备授权请求、用户验证授权、令牌轮询 |
| P1 | 2 | Client App 配置、验证页面 API |
| P2 | 0 | - |

---

## 2. 范围界定

### 2.1 包含功能

- 设备授权端点（Device Authorization Request，RFC 8628 §3.1、§3.2）
- 令牌轮询端点（Device Access Token Request，RFC 8628 §3.4、§3.5）
- Herald 前端设备验证页面（`/{realmId}/device` 路由）
- `verification_uri_complete` 支持（URL 中嵌入 user_code）
- Device Code Grant 在 Client App 中的启用/禁用配置
- 设备验证 API（供第三方应用自定义验证流程）
- 协议规定的全部错误码：`authorization_pending`、`slow_down`、`expired_token`、`access_denied`

### 2.2 不包含功能 (Out of Scope)

- ❌ **QR 码生成**（可在后续迭代中添加，CLI 工具可自行生成）
- ❌ **Refresh Token**（当前系统不支持 token 刷新，与现有 OAuth 一致）
- ❌ **Scope 管理**（当前系统无 OAuth scope 管理，与现有 OAuth 一致）
- ❌ **PKCE**（Device Code Flow 不适用 PKCE，RFC 8628 未要求）
- ❌ **标准授权码流程改造**（本功能为独立 grant_type，不影响现有流程）

### 2.3 依赖项

- ✅ **Client App 系统**（状态: 已实现）— 复用 client_id 和 Client App 配置模型
- ✅ **Session Token 系统**（状态: 已实现）— 复用 Session Token 生成与验证
- ✅ **缓存/存储基础设施**（状态: 已实现）— 用于存储 device_code 等临时状态
- ✅ **用户认证系统**（状态: 已实现）— 验证页面复用登录能力
- ✅ **权限管理系统**（状态: 已实现）— 复用 RBAC 权限检查

---

## 3. 需求概述

### 3.1 功能描述

为 Herald 新增 OAuth 2.0 Device Authorization Grant（RFC 8628）支持，主要服务于 CLI 工具认证场景。

在 CLI 工具等无浏览器或输入受限的环境中，用户无法通过传统的授权码流程完成 OAuth 认证。Device Code Flow 通过将认证过程分离到用户的浏览器（手机或电脑）上，使 CLI 工具能在终端环境下安全完成用户认证。

**核心价值**：为第三方 CLI 应用提供标准化、安全的认证方式，降低集成门槛，提升用户体验。

### 3.2 关键特性

- **RFC 8628 完整合规**：实现协议规定的全部端点、参数和错误码
- **复用现有架构**：复用 Client App 模型和 Session Token 机制
- **双通道验证**：Herald 提供默认验证页面，同时开放 API 供第三方自定义
- **安全防护**：短生命周期码、轮询限速、展示 Client App 名称防钓鱼

### 3.3 协议流程概览

```
CLI 工具                          Herald                      用户浏览器
  |                                 |                              |
  |-- POST /device_authorization -->|                              |
  |<-- device_code, user_code, -----|                              |
  |    verification_uri, interval --|                              |
  |                                 |                              |
  |  显示: "访问 verification_uri   |                              |
  |   输入 user_code: ABCD-1234"    |                              |
  |                                 |<-- 访问 /{realmId}/device 页面 -|
  |                                 |<-- 输入 user_code 并登录 -----|
  |                                 |-- 授权确认页面 -------------->|
  |                                 |<-- 点击"授权" ----------------|
  |                                 |                              |
  |-- POST /token (polling) ------->|                              |
  |<-- authorization_pending -------|                              |
  |                                 |-- (用户完成授权)              |
  |-- POST /token (polling) ------->|                              |
  |<-- access_token ----------------|                              |
```

---

## 4. 当前实现状态

| 功能模块 | 状态 | 备注 |
|---------|------|------|
| 设备授权端点 | ❌ 未实现 | 需新建 |
| 令牌轮询端点 | ❌ 未实现 | 需新建，新增 grant_type |
| 前端验证页面 | ❌ 未实现 | 需新建 `/{realmId}/device` 路由 |
| Client App 配置扩展 | ❌ 未实现 | 需添加 Device Code Grant 启用/禁用字段 |
| 验证页面 API | ❌ 未实现 | 需新建 |
| user_code 生成 | ❌ 未实现 | base-20 编码，排除易混淆字符 |
| device_code 状态存储 | ❌ 未实现 | 复用现有存储基础设施 |

---

## 5. 功能需求

### 5.1 核心需求

1. **设备授权请求**：CLI 工具通过 `client_id` 请求 `device_code` 和 `user_code`，响应包含 `verification_uri`、`verification_uri_complete`、`expires_in`（默认 900 秒）、`interval`（默认 5 秒）
2. **用户验证授权**：用户在 Herald 验证页面输入 `user_code`、登录、查看 Client App 名称并确认授权
3. **令牌轮询**：CLI 工具以指定间隔轮询令牌端点，系统返回 `authorization_pending`、`slow_down`、`expired_token`、`access_denied` 或 access token
4. **Client App 配置**：Realm Admin 可为每个 Client App 独立启用或禁用 Device Code Grant

### 5.2 验收目标

- P0 场景（US-DC-001 ~ US-DC-003）全部通过，CLI 工具可完成完整的设备码认证流程
- P1 场景（US-DC-004 ~ US-DC-005）通过，管理员可配置、第三方可自定义验证页面
- 与现有授权码流程互不干扰

### 5.3 user_code 生成规则

- 长度：8 字符，格式 `XXXX-XXXX`（4+4，连字符分隔）
- 字符集：base-20 编码，排除易混淆字符（0、O、1、I、L）
- 有效字符：`A B C D E F G H J K M N P Q R S T V W X Y Z 2 3 4 5 6 7 8 9`
- 大小写：统一大写显示，验证时不区分大小写
- 唯一性：生成时检查与当前未过期的 user_code 不重复

### 5.4 device_code 安全与生命周期

- 高强度随机性，不可猜测或枚举
- 有效期：默认 900 秒（15 分钟），过期后不可使用
- 状态由 pending 转为终态（authorized / denied / expired）后不可逆
- 存储与数据结构设计详见 `.ai/design/device-code.md`

---

## 6. API 相关约束

**状态**: 必填

- 设备授权端点需验证 `client_id` 有效且 Client App 已启用 Device Code Grant
- 令牌轮询端点需正确实现 RFC 8628 §3.5 规定的全部错误响应
- 轮询端点需对 `slow_down` 错误正确累加间隔（每次 +5 秒）
- 验证页面 API 需要求用户已登录（session 认证）
- 所有端点遵守 realm 隔离原则
- 不需要 `redirect_uri` 参数（与授权码流程的关键区别）
- 不需要 `client_secret`（适用于 public client / CLI 场景）

---

## 7. 前端/交互约束

**状态**: 必填

### 验证页面（`/{realmId}/device`）

- **入口**：Herald 前端新增 `/{realmId}/device` 路由，与 realm 绑定（登录跳转、API 调用均基于路径中的 realmId）
- **输入**：用户输入 `user_code`，8 字符输入框（自动格式化为 `XXXX-XXXX`）
- **授权确认**：显示请求授权的 Client App 名称和图标（如果配置了 icon_url），用户点击"授权"或"拒绝"
- **状态反馈**：
  - 输入无效/过期码：提示"设备码无效或已过期"
  - 已登录用户直接看到授权确认页面
  - 未登录用户先跳转 `/{realmId}/auth/login`，登录后回到验证页面
  - 授权成功：提示"授权成功，请返回 CLI 工具"
  - 授权拒绝：提示"授权已拒绝"
- **URL 预填**：通过 `verification_uri_complete` 访问时，`user_code` 自动填入输入框

### Client App 设置

- 在现有 Client App 设置页面中新增 Device Code Grant 启用/禁用开关
- 默认为禁用状态

---

## 8. 技术设计承接

**状态**: 必填

- 接口细节（端点路径、请求/响应参数、数据存储结构）应在 `.ai/design/device-code.md` 中承接
- 数据库变更（Client App 表新增字段）应在技术设计中说明
- 实现步骤和代码结构应在技术设计中规划

---

## 9. 相关文件索引

### 9.1 后端文件（现有参考）
- OAuth 授权处理: `backend/api/src/application/http/oauth/`
- Client App 实体: `backend/domain/src/client/`
- Session 管理: 复用现有 session 机制

### 9.2 前端文件（现有参考）
- 登录页面: `frontend/src/routes/$realm_id.login.tsx`
- Client App 设置: `frontend/src/features/settings/`

---

## 10. 参考资料

- 用户故事：`docs/user-stories/15-device-code-user-stories.md`
- RFC 8628 — OAuth 2.0 Device Authorization Grant: https://datatracker.ietf.org/doc/html/rfc8628
- 相关 PRD：`docs/prd/auth/oauth-third-party-integration.md`
- 相关 PRD：`docs/prd/integration/client-app.md`
- Auth0 Device Authorization Flow: https://auth0.com/docs/get-started/authentication-and-authorization-flow/device-authorization-flow
- WorkOS Device Authorization Grant 实践指南: https://workos.com/blog/oauth-device-authorization-grant
