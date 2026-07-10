# Realm 自定义域名（Custom Domain）用户故事

> 角色定义见 [docs/user-stories/_roles.md](/docs/user-stories/_roles.md)

## 用户故事

### 故事 1：为本 Realm 配置自定义登录域名 [US-CD-001]

**优先级**: P0

**【用户故事】**
**作为**：Realm Admin（详见 [docs/user-stories/_roles.md](/docs/user-stories/_roles.md)）
**我希望**：在管理后台为本 Realm 配置一个自定义登录域名（如 `login.acme.com`），并按系统指引将该域名 CNAME 到 Herald 指定的 hostname
**从而**：为本 Realm 建立自有品牌登录域名配置，并让 Herald 仅对已注册的该域名授权签发证书

**【验收标准】**

> 验收标准只描述用户动作与可见结果，不写 API 路径、数据表、字段变更、技术实现步骤。

**场景 1：成功配置并保存自定义域名草稿**
```gherkin
Given 我是 realm-1 的 Realm Admin
And 我已进入管理后台 Settings 页面的自定义域名配置入口
When 我填入一个尚未被其他 Realm 占用的自定义域名
And 我保存配置
Then 配置保存成功并显示成功反馈
And 配置仅关联到 realm-1，不影响其他 Realm
```

**场景 2：系统提供 CNAME 指引并展示生效状态**
```gherkin
Given realm-1 已配置自定义域名 login.acme.com
When 我查看该自定义域名配置
Then 系统向我展示需要 CNAME 到的 Herald 指定 hostname
And 系统展示当前域名生效状态（如 CNAME 是否已正确指向、TLS 是否就绪）
```

**场景 3：自定义域名全局唯一**
```gherkin
Given login.acme.com 已被 realm-1 配置占用
When 我尝试在 realm-2 配置相同的 login.acme.com
Then 系统拒绝该配置并提示该域名已被占用
```

**场景 4：仅 Realm Admin 可配置**
```gherkin
Given 我是 realm-1 的普通用户（Regular User）
When 我尝试访问管理后台的自定义域名配置入口
Then 系统拒绝访问
And 我看不到该配置入口
```

---

### 故事 2：自定义域名配置的草稿与发布 [US-CD-003]

**优先级**: P1

**【用户故事】**
**作为**：Realm Admin（详见 [docs/user-stories/_roles.md](/docs/user-stories/_roles.md)）
**我希望**：自定义域名配置遵循草稿/发布/恢复生命周期，未发布前不影响当前已生效的自定义域名配置
**从而**：避免配置错误（如填错域名）立即影响已生效的自定义域名，并能在出错后快速回退

**【验收标准】**

> 验收标准只描述用户动作与可见结果，不写 API 路径、数据表、字段变更、技术实现步骤。

**场景 1：未发布草稿不影响当前已生效的配置**
```gherkin
Given realm-1 当前已发布生效自定义域名 login.acme.com
And 我在管理后台编辑了一份新的自定义域名草稿（如 login2.acme.com）但尚未发布
When 我查看当前已生效的自定义域名配置
Then 当前生效的配置仍为 login.acme.com
And 不会因为存在草稿而改变已生效的自定义域名
```

**场景 2：发布后域名注册映射更新**
```gherkin
Given realm-1 已保存一份自定义域名草稿 login2.acme.com
When 我发布该草稿
Then 系统将 login2.acme.com 置为当前生效的自定义域名
And 该域名被注册到 Herald 的域名注册映射中
And 证书授权门控据此开始为 login2.acme.com 授权
```

**场景 3：恢复上一版自定义域名配置**
```gherkin
Given realm-1 刚刚发布了一个错误的自定义域名配置
When 我执行恢复上一版操作
Then 系统将已生效的自定义域名配置恢复到上一版
And 域名注册映射同步回滚到上一版
```

---

### 故事 3：未授权域名访问的拒绝 [US-CD-005]

**优先级**: P1

**【用户故事】**
**作为**：Regular User（详见 [docs/user-stories/_roles.md](/docs/user-stories/_roles.md)）
**我希望**：访问一个未在任何 Realm 注册的自定义域名时，Herald 不会为该域名授权签发证书
**从而**：避免 Herald 为未授权域名签发证书，防止证书滥用与钓鱼

**【验收标准】**

> 验收标准只描述用户动作与可见结果，不写 API 路径、数据表、字段变更、技术实现步骤。

**场景 1：未注册自定义域名不被授权签发证书**
```gherkin
Given evil.com 未在任何 Realm 注册为自定义域名
When 证书签发流程向 Herald 查询 evil.com 是否可签发
Then Herald 不授权为 evil.com 签发证书
```

**场景 2：已注册并生效的域名被授权签发证书**
```gherkin
Given login.acme.com 已在 realm-1 注册并发布生效
When 证书签发流程向 Herald 查询 login.acme.com 是否可签发
Then Herald 授权为 login.acme.com 签发证书
```

**场景 3：证书授权查询不泄露 Realm 身份**
```gherkin
Given 任意域名（无论是否注册）
When 证书签发流程向 Herald 查询该域名是否可签发
Then Herald 的响应仅表明是否授权
And 不泄露该域名关联的 Realm 身份或其他信息
```

---

## 未来范围（Deferred）

以下故事原属本 feature 的完整设想，但因 host→realm 解析机制尚未交付，列为未来范围，不在当前已发布能力中：

- **终端用户在自定义域名下完成 auth 流**：终端用户在租户的自定义域名（如 `login.acme.com`）下完成登录/注册及其他 auth 流，URL 保持该自有品牌域名，收到的邮件链接也指向该自定义域名。依赖 host→realm 解析先落地。
- **自定义域名与 path-based canonical 域名并存**：配置自定义域名后，原有 `herald.com/{realmId}` 路径入口仍可用，两条入口同时可用，不强制迁移。依赖 host→realm 解析先落地。
