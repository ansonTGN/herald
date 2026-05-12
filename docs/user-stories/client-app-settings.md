# Client App 设置功能用户故事

> **实现状态**: ✅ 已完成 (2025-01-15)
>
> 本次实现包含以下用户故事的全部功能：
> - 故事 1：配置 Client App 跳转地址白名单 ✅
> - 故事 2：管理 Client App 图标 ✅
> - 故事 3：启用/禁用 Client App ✅
> - 故事 4：配置 Session 有效期策略 ✅

**角色代码**: TP
**故事范围**: US-TP-008 ~ US-TP-011
**创建时间**: 2025-01-15
**状态**: Active

## 角色定义

详见 [`.ai/user_story/_roles.md`](/docs/user-stories/_roles.md)

---

## 用户故事

### 故事 1：配置 Client App 跳转地址白名单 [US-TP-008]

**【用户故事】**
**作为**：Realm 管理员（详见 [`.ai/user_story/_roles.md`](/docs/user-stories/_roles.md)）
**我希望**：为 Client App 配置登录/注册成功后的跳转地址白名单
**从而**：用户在 Herald 完成认证后能安全地跳转回 Client App

**【验收标准】**

**场景 1：成功添加跳转地址**
Given 管理员已登录并进入 Client App 设置页面
When 管理员输入有效的跳转地址（如 "https://app.com/auth/callback"）
And 管理员点击"Add"按钮
And 点击"Save Changes"
Then 系统保存该地址到白名单并显示"Settings updated successfully"

**场景 2：添加无效的 URL**
Given 管理员在设置页面
When 管理员输入格式错误的 URL（如 "not-a-url" 或 "javascript:alert(1)"）
And 点击"Add"按钮
Then 系统提示"Invalid URL format"且不添加该地址

**场景 3：至少需要一个跳转地址**
Given 管理员在设置页面
When 管理员删除所有跳转地址并尝试保存
Then 系统提示"At least one redirect URI is required"

**场景 4：重复的跳转地址**
Given 管理员在设置页面
When 管理员添加已存在的跳转地址
Then 系统提示"Redirect URI already exists"

**场景 5：用户认证时使用白名单地址**
Given Client App 已配置跳转地址白名单
When 用户从 Client App 跳转到 Herald 登录页（携带 redirect_uri 参数）
And 用户完成登录
Then 系统验证 redirect_uri 在白名单中并跳转回该地址

**场景 6：恶意跳转地址被拒绝**
Given Client App 的白名单为 ["https://app.com/callback"]
When 攻击者构造恶意链接（redirect_uri=https://evil.com）
And 用户完成登录
Then 系统检测到地址不在白名单中，返回 400 Bad Request

---

### 故事 2：管理 Client App 图标 [US-TP-009]

**【用户故事】**
**作为**：Realm 管理员（详见 [`.ai/user_story/_roles.md`](/docs/user-stories/_roles.md)）
**我希望**：为 Client App 上传和管理图标
**从而**：在用户选择登录方式时能看到应用图标

**【验收标准】**

**场景 1：成功上传图标**
Given 管理员在 Client App 设置页面
When 管理员输入有效的图标 URL 并保存
Then 系统保存图标 URL 并在列表页显示该图标

**场景 2：删除图标**
Given 管理员已为 Client App 设置了图标
When 管理员清空图标 URL 并保存
Then 系统移除图标配置

---

### 故事 3：启用/禁用 Client App [US-TP-010]

**【用户故事】**
**作为**：Realm 管理员（详见 [`.ai/user_story/_roles.md`](/docs/user-stories/_roles.md)）
**我希望**：能够启用或禁用 Client App
**从而**：临时停止某个应用的 OAuth 集成而不删除配置

**【验收标准】**

**场景 1：禁用 Client App**
Given Client App 当前处于启用状态
When 管理员将 enabled 开关切换为 false
Then 系统禁用该 Client App，用户无法通过该应用登录

**场景 2：重新启用 Client App**
Given Client App 当前处于禁用状态
When 管理员将 enabled 开关切换为 true
Then 系统重新启用该 Client App，用户可以正常登录

---

### 故事 4：配置 Session 有效期策略 [US-TP-011]

**【用户故事】**
**作为**：Realm 管理员（详见 [`.ai/user_story/_roles.md`](/docs/user-stories/_roles.md)）
**我希望**：为 Client App 配置用户 Session 的有效期限和续期策略
**从而**：根据应用的安全要求平衡用户体验和安全性

**【验收标准】**

**场景 1：设置严格的 Session 策略（银行类应用）**
Given 管理员在 Client App 设置页面
When 管理员设置 session_ttl_seconds 为 300（5分钟）
And 设置 session_renewal_ttl_seconds 为 null（不允许续期）
And 点击保存
Then 用户 Session 在 5 分钟后过期，必须重新登录

**场景 2：设置宽松的 Session 策略（企业内部工具）**
Given 管理员在 Client App 设置页面
When 管理员设置 session_ttl_seconds 为 28800（8小时）
And 设置 session_renewal_ttl_seconds 为 28800（允许续期到8小时）
And 点击保存
Then 用户 Session 可持续 8 小时，并可通过续期延长

**场景 3：设置渐进式安全策略**
Given 管理员在 Client App 设置页面
When 管理员设置 session_ttl_seconds 为 300（初始5分钟）
And 设置 session_renewal_ttl_seconds 为 7200（续期后延长到2小时）
And 点击保存
Then 用户首次登录获得 5 分钟 Session，续期后延长到 2 小时

**场景 4：禁止续期时的错误提示**
Given Client App 的 session_renewal_ttl_seconds 为 null
When 用户尝试刷新 Session
Then 系统返回 401 Unauthorized，提示需要重新登录

---

## 备注

### 业务规则
1. **redirect_uris 白名单验证**：
   - 至少包含一个有效的 HTTPS 地址（开发环境允许 HTTP）
   - 验证 URL 格式，禁止 `javascript:` 协议和协议相对 URL `//`
   - OAuth 授权时严格验证 redirect_uri 是否在白名单中

2. **Session 配置规则**：
   - `session_ttl_seconds`：Cookie 初始有效期，默认 1800（30分钟）
   - `session_renewal_ttl_seconds`：续期后的有效期，**NULL 表示不允许续期**
   - 续期时删除旧 Session，生成新 Token

3. **安全考虑**：
   - `client_secret` 创建时自动生成 UUID，只能通过管理接口重新生成
   - 禁用的 Client App 无法完成 OAuth 授权流程
   - redirect_uri 白名单防止开放重定向攻击

### 边界说明
- Realm 管理员只能管理本 Realm 的 Client App 设置
- 删除 Client App 时级联删除其设置（ON DELETE CASCADE）
- 修改设置后立即生效，无需重启服务

---

## 优先级

**US-TP-008 (故事 1)**: P0（关键）- 配置 Client App 跳转地址白名单
**US-TP-009 (故事 2)**: P0（关键）- 管理 Client App 图标
**US-TP-010 (故事 3)**: P0（关键）- 启用/禁用 Client App
**US-TP-011 (故事 4)**: P0（关键）- 配置 Session 有效期策略

---

## 📖 相关PRD

- **Client App 管理**: [docs/prd/integration/client-app.md](/docs/prd/integration/client-app.md)
