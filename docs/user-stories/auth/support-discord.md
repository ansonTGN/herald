# Discord OAuth 登录 用户故事

> 角色定义见 [docs/user-stories/_roles.md](/docs/user-stories/_roles.md)
> 既有覆盖：`docs/user-stories/auth/oauth-extension.md`（US-OE-001 Provider 配置管理）与 `docs/user-stories/core/regular-user.md`（US-RU-003 OAuth 第三方登录）描述通用行为；本文件只补齐 Discord 专属验收场景。

## 租户管理员故事

### 故事 1：Discord OAuth Provider 配置 [US-SD-001]

**优先级**: P1

**【用户故事】**
**作为**：Realm Admin（详见 [docs/user-stories/_roles.md](/docs/user-stories/_roles.md)）
**我希望**：为本 Realm 配置并启用 Discord OAuth Provider
**从而**：用户可以使用 Discord 账号登录本 Realm 的应用

**【验收标准】**

> 验收标准只描述用户动作与可见结果，不写 API 路径、数据表、字段变更、技术实现步骤。

**场景 1：添加 Discord Provider 配置**
```gherkin
Given 我是 realm-1 的管理员
And 我在 Settings -> Providers 页面
When 我点击 "Add Provider" 按钮
And 我选择 Provider Type 为 "Discord"
And 我填写 OAuth 配置：
  | Client ID     | discord-client-id-123 |
  | Client Secret | discord-client-secret-456 |
  | Scopes        | identify, email |
  | Enabled       | true |
And 我提交表单
Then Provider 配置创建成功
And Provider 列表显示 Discord Provider
```

**场景 2：Scopes 默认值与调整**
```gherkin
Given 我是 realm-1 的管理员
And 我在添加 Discord Provider 的表单中
When 我未手动修改 Scopes
Then Scopes 默认填充为 identify 与 email

When 我把 Scopes 修改为仅 identify 并保存
Then 配置保存成功
```

**场景 3：启用/禁用 Discord Provider**
```gherkin
Given 我是 realm-1 的管理员
And 已配置并启用 Discord Provider
When 我禁用 Discord Provider
Then Discord Provider 状态变为 "Disabled"
And 该 Realm 登录页不再显示 Discord 登录按钮

When 我重新启用 Discord Provider
Then Discord Provider 状态变为 "Enabled"
And 登录页重新显示 Discord 登录按钮
```

**场景 4：删除 Discord Provider 配置**
```gherkin
Given 我是 realm-1 的管理员
And 已配置 Discord Provider
When 我删除 Discord Provider 并确认删除
Then 配置删除成功
And 列表不再显示该 Provider
```

## 租户用户故事

### 故事 2：使用 Discord 账号登录 [US-SD-002]

**优先级**: P1

**【用户故事】**
**作为**：Regular User（详见 [docs/user-stories/_roles.md](/docs/user-stories/_roles.md)）
**我希望**：在登录页点击 "Discord" 按钮使用 Discord 账号登录
**从而**：无需为本服务记忆额外密码

**【验收标准】**

**场景 1：首次 Discord 登录自动创建账号（realm 开放注册）**
```gherkin
Given realm-1 已配置并启用 Discord Provider
And realm-1 开启自动注册
And 我持有邮箱已验证的 Discord 账号，且该 Discord 账号从未在本 Realm 登录过
When 我在登录页点击 "Discord" 按钮
And 我在 Discord 授权页同意授权
Then 系统自动为我创建本 Realm 账号并完成登录
And 我的账号资料显示 Discord 返回的显示名与头像
```

**场景 2：已验证邮箱关联既有账号**
```gherkin
Given realm-1 已启用 Discord Provider
And 我已有本 Realm 账号，绑定邮箱 me@example.com
And 我的 Discord 账号邮箱为 me@example.com 且已在 Discord 完成验证
When 我使用该 Discord 账号完成 Discord 授权登录
Then 我登录到既有账号（不新建账号）
```

**场景 3：未验证邮箱不得关联或建号**
```gherkin
Given realm-1 已启用 Discord Provider
And 我的 Discord 账号邮箱未在 Discord 完成验证
When 我完成 Discord 授权
Then 系统不将该邮箱关联到任何既有账号
And 系统不为我创建新账号
And 我看到明确的失败提示
```

**场景 4：注册关闭时不自动建号**
```gherkin
Given realm-1 已启用 Discord Provider
And realm-1 关闭自动注册
And 我的 Discord 凭据未匹配到已有账号
When 我完成 Discord 授权
Then 系统不创建新账号
And 我看到注册未开放的提示
```

**场景 5：Discord 未返回邮箱时明确拒绝**
```gherkin
Given realm-1 的 Discord Provider 配置丢失获取邮箱所需的授权范围（或 Discord 未返回邮箱）
When 我完成 Discord 授权
Then 登录被拒绝
And 我看到说明邮箱缺失原因的明确错误提示
```

---

## 相关文档

- **PRD**: [Discord OAuth 登录 PRD](/docs/prd/auth/support-discord.md)
- **正式基线**: [OAuth 与第三方集成 PRD](/docs/prd/auth/oauth.md)
