# Device Code 登录用户故事

**角色代码**: DC
**涉及角色**: Third-Party App (CLI 工具)、Regular User、Realm Admin
**角色定义**: 见 [docs/user-stories/_roles.md](_roles.md)

**故事范围**: US-DC-001 ~ US-DC-005
**创建时间**: 2026-05-14
**状态**: Active

---

## 用户故事

### 故事 1：CLI 工具发起设备授权 [US-DC-001]

**优先级**: P0

**【用户故事】**
**作为**：第三方应用（CLI 工具）
**我希望**：通过 Device Authorization Grant 向 Herald 请求设备授权码
**从而**：在无浏览器或输入受限的环境中安全完成用户认证

**【验收标准】**

**场景 1：成功发起设备授权请求**
Given 第三方应用 "my-cli" 已在 realm-1 中注册为 Client App 且已启用
When CLI 调用设备授权端点，携带 client_id="my-cli"
Then 系统返回 `device_code`、`user_code`（8 字符，格式 `XXXX-XXXX`）、`verification_uri`、`expires_in`（900 秒）和 `interval`（5 秒）

**场景 2：用户码格式与可读性**
Given 系统返回了 `user_code`
When 用户查看 CLI 输出
Then `user_code` 为 8 字符的大写字母和数字组合，以连字符分隔（如 `ABCD-1234`），不包含易混淆字符（0/O、1/I/L）

**场景 3：Client App 已禁用（失败场景）**
Given Client App "my-cli" 的 enabled 状态为 false
When CLI 调用设备授权端点
Then 系统返回错误：`{"error": "client_app_disabled"}`

**场景 4：Client App 不存在（失败场景）**
Given client_id "nonexistent" 未在系统中注册
When CLI 调用设备授权端点
Then 系统返回错误：`{"error": "invalid_client"}`

**场景 5：设备授权码过期**
Given 设备授权请求已超过 `expires_in`（900 秒）
When CLI 使用该 `device_code` 轮询令牌端点
Then 系统返回错误：`{"error": "expired_token"}`

---

### 故事 2：用户在验证页面完成授权 [US-DC-002]

**优先级**: P0

**【用户故事】**
**作为**：普通用户
**我希望**：在 Herald 验证页面输入设备码并完成登录授权
**从而**：授权 CLI 工具以我的身份访问受保护资源

**【验收标准】**

**场景 1：成功输入设备码并授权**
Given 用户在浏览器访问 Herald 的设备验证页面（如 `https://auth.example.com/{realmId}/device`）
When 用户输入正确的 `user_code`（如 `ABCD-1234`）并提交
Then 系统提示用户登录（如未登录），登录后显示授权确认页面，展示请求授权的 Client App 名称
When 用户点击"授权"
Then 系统提示"授权成功，请返回 CLI 工具"
And 同时 CLI 工具通过轮询获取到 access token

**场景 2：通过 verification_uri_complete 直接授权**
Given CLI 工具提供了 `verification_uri_complete`（URL 中已嵌入 `user_code`）
When 用户通过二维码或链接访问该 URL
Then 系统自动填入 `user_code`，用户只需登录并确认授权

**场景 3：设备码无效或已过期（失败场景）**
Given 用户输入的 `user_code` 不存在或已过期
When 用户提交验证
Then 系统提示"设备码无效或已过期，请在 CLI 工具中重新获取"

**场景 4：设备码已被使用（失败场景）**
Given 用户已经使用该 `user_code` 完成了授权
When 另一个用户尝试使用相同的 `user_code`
Then 系统提示"设备码已使用"

**场景 5：用户拒绝授权**
Given 用户在授权确认页面看到 Client App 信息
When 用户点击"拒绝"
Then 系统提示"授权已拒绝"，CLI 工具轮询时收到 `access_denied` 错误

---

### 故事 3：CLI 工具轮询获取令牌 [US-DC-003]

**优先级**: P0

**【用户故事】**
**作为**：第三方应用（CLI 工具）
**我希望**：按照协议规定间隔轮询令牌端点，直到用户完成授权
**从而**：获得 access token 用于后续 API 调用

**【验收标准】**

**场景 1：用户完成授权后获取令牌**
Given 用户已在验证页面完成授权
And CLI 以 `interval`（5 秒）间隔轮询令牌端点
When 轮询请求携带 `grant_type=urn:ietf:params:oauth:grant-type:device_code` 和有效的 `device_code`
Then 系统返回 access token

**场景 2：用户尚未完成授权（等待中）**
Given 用户还未在验证页面完成授权
When CLI 轮询令牌端点
Then 系统返回 `{"error": "authorization_pending"}`，CLI 应继续轮询

**场景 3：轮询过快需要降速**
Given CLI 轮询频率高于 `interval` 要求
When CLI 在间隔时间内再次请求
Then 系统返回 `{"error": "slow_down"}`，CLI 应将轮询间隔增加 5 秒后继续

**场景 4：设备码过期（失败场景）**
Given `device_code` 已超过 `expires_in` 有效期
When CLI 轮询令牌端点
Then 系统返回 `{"error": "expired_token"}`，CLI 应引导用户重新发起设备授权

**场景 5：用户拒绝授权（失败场景）**
Given 用户在验证页面拒绝了授权
When CLI 轮询令牌端点
Then 系统返回 `{"error": "access_denied"}`，CLI 应提示用户授权被拒绝

---

### 故事 4：Realm Admin 配置 Device Code Grant [US-DC-004]

**优先级**: P1

**【用户故事】**
**作为**：Realm Admin
**我希望**：为 Client App 启用或禁用 Device Code Grant
**从而**：按需控制哪些应用支持 CLI 设备码登录

**【验收标准】**

**场景 1：为 Client App 启用 Device Code Grant**
Given Realm Admin 在 Client App 设置页面
When 管理员将 Device Code Grant 设置为"启用"
Then 该 Client App 可以发起设备授权请求

**场景 2：为 Client App 禁用 Device Code Grant**
Given Realm Admin 在 Client App 设置页面
When 管理员将 Device Code Grant 设置为"禁用"
Then 该 Client App 发起设备授权请求时，系统返回 `{"error": "unauthorized_client"}`

**场景 3：默认状态**
Given 新创建的 Client App
When 管理员查看 Device Code Grant 配置
Then 默认为"禁用"状态，需手动启用

---

### 故事 5：设备验证页面 API [US-DC-005]

**优先级**: P1

**【用户故事】**
**作为**：第三方应用（CLI 工具）
**我希望**：通过 API 验证和确认设备码，而不仅依赖 Herald 提供的验证页面
**从而**：在自己的应用中构建自定义的设备码验证体验

**【验收标准】**

**场景 1：通过 API 验证设备码**
Given 拥有有效的 `user_code`
When 调用设备码验证 API，携带已登录用户的 session token 和 `user_code`
Then 系统验证 `user_code` 有效后，返回需要授权的 Client App 信息

**场景 2：通过 API 确认授权**
Given 设备码已验证通过
When 调用设备码确认授权 API，携带用户的 session token 和确认信息
Then 系统完成授权绑定，CLI 工具下次轮询时获得 access token

**场景 3：API 验证无效设备码（失败场景）**
Given `user_code` 不存在或已过期
When 调用设备码验证 API
Then 系统返回错误

---

## 备注

### 业务规则
1. **Device Code Grant 是 OAuth 2.0 的扩展授权类型**（RFC 8628），适用于无浏览器或输入受限的设备
2. `user_code` 格式为 8 字符（`XXXX-XXXX`），使用 base-20 编码排除易混淆字符
3. `device_code` 对用户不可见，仅用于后端轮询
4. `device_code` 和 `user_code` 的默认有效期为 900 秒（15 分钟）
5. 默认轮询间隔为 5 秒
6. Device Code Grant 需在 Client App 配置中显式启用
7. Device Code Grant 不需要 `redirect_uri`，适用于无浏览器环境
8. 复用现有 Client App 模型和 Session Token 机制

### 安全注意事项
1. `device_code` 应使用高强度随机值，不可猜测
2. 限制单个 Client App 的并发设备授权请求数量
3. 用户应只输入自己发起的 `user_code`，防范钓鱼攻击（参考 Storm-2372 攻击模式）
4. 验证页面应展示请求授权的 Client App 名称，帮助用户确认
5. 已使用或已过期的 `user_code` 应立即失效
6. 授权确认后 `device_code` 应标记为已使用，防止重放

### 与现有 OAuth 的关系
- 复用 Client App 注册和权限模型
- 复用 Session Token 生成和验证机制
- Device Code Grant 作为新增 grant_type，不影响现有授权码流程
- 验证页面是 Herald 前端新增的独立页面

### 相关 PRD
- OAuth 第三方集成: [docs/prd/auth/oauth-third-party-integration.md](/docs/prd/auth/oauth-third-party-integration.md)
- Client App 管理: [docs/prd/integration/client-app.md](/docs/prd/integration/client-app.md)
- OAuth Provider: [docs/prd/auth/oauth-provider.md](/docs/prd/auth/oauth-provider.md)
