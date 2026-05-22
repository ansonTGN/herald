# Realm Admin 用户故事 - OAuth Provider 配置（扩展）

**角色代码**: RA
**角色定义**：Realm Admin 是特定 Realm 的管理员，负责管理该 Realm 下的用户、客户端应用、角色、权限、订阅套餐和 OAuth Provider 配置。

**故事范围**: US-RA-010（OAuth Provider 配置管理）
**创建时间**: 2025-02-12
**状态**: Active

---

## 故事 10：OAuth Provider 配置管理 [US-RA-010]

**优先级**: P0

**【用户故事】**
**作为**：Realm Admin
**我希望**：管理 OAuth Provider 配置（Google、GitHub、Facebook、Apple），以便用户可以使用第三方登录
**从而**：提供灵活的多渠道登录选项

**【验收标准】**

**场景 1：添加 OAuth Provider 配置**
```gherkin
Given 我是 realm-1 的管理员
And 我在 Settings -> Providers 页面
When 我点击 "Add Provider" 按钮
And 我选择 Provider Type 为 "Google"
And 我填写 OAuth 配置：
  | Client ID     | google-client-id-123 |
  | Client Secret | google-client-secret-456 |
  | Scopes        | userinfo.email, userinfo.profile |
  | Enabled       | true |
And 我提交表单
Then Provider 配置创建成功
And Provider 列表显示 Google Provider
```

**场景 2：添加 GitHub Provider 配置**
```gherkin
Given 我是 realm-1 的管理员
And 我在 Settings -> Providers 页面
When 我添加 GitHub Provider 配置：
  | Provider Type | GitHub    |
  | Client ID     | github-client-id-789 |
  | Client Secret | github-client-secret-012 |
  | Scopes        | user:email |
Then Provider 配置创建成功
```

**场景 3：启用/禁用 Provider**
```gherkin
Given 我是 realm-1 的管理员
And 已配置 Google Provider
When 我禁用 Google Provider
Then Google Provider 状态变为 "Disabled"

When 我重新启用 Google Provider
Then Google Provider 状态变为 "Enabled"
```

**场景 4：编辑 Provider 配置**
```gherkin
Given 我是 realm-1 的管理员
And 已配置 GitHub Provider
When 我编辑 GitHub Provider 配置
And 修改 Client ID 为 "github-client-id-updated"
And 保存更改（Client Secret 留空表示不更新）
Then Provider 配置更新成功
And 列表显示新的 Client ID
```

**场景 5：删除 Provider 配置**
```gherkin
Given 我是 realm-1 的管理员
And 已配置 GitHub Provider
When 我删除 GitHub Provider
And 确认删除
Then Provider 配置删除成功
And 列表不再显示该 Provider
```

---

## 📖 相关PRD

- **OAuth Provider**: [docs/prd/auth/oauth.md](/docs/prd/auth/oauth.md)
- **OAuth 第三方集成**: [docs/prd/auth/oauth.md](/docs/prd/auth/oauth.md)

---
