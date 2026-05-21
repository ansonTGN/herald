# 普通用户用户故事

**角色代码**: RU
**角色定义**: 普通用户是通过注册或由管理员创建的用户，可以访问第三方应用和 OAuth 登录。

**故事范围**: US-RU-001 ~ US-RU-010
**创建时间**: 2025-02-01
**状态**: Active

---

## 用户故事

### 故事 1：账号注册 [US-RU-001]

**【用户故事】**
**作为**：普通用户
**我希望**：能够在开启注册的Realm中注册账号
**从而**：获得系统访问权限

**【验收标准】**

**场景 1a：正常注册成功（不需要邮箱验证）**
Given Realm realm-1已开启用户自注册功能
And require_email_verification 配置为 false 或未设置
When 用户访问`/realm-1/register`页面
And 输入邮箱、密码（符合密码策略）、确认密码、昵称
And 完成Turnstile验证（若启用）
Then 注册成功，账号状态为"Normal"（status=1）
And 用户可以立即登录
And 页面跳转到 Dashboard 或 redirect 页面

**场景 1b：注册成功需要邮箱验证**
Given Realm realm-1已开启用户自注册功能
And require_email_verification 配置为 true
When 用户访问`/realm-1/register`页面
And 输入邮箱、密码（符合密码策略）、确认密码、昵称
And 完成Turnstile验证（若启用）
Then 注册成功，账号状态为"WaitVerified"（status=0）
And 系统发送验证邮件到用户邮箱
And 页面跳转到`/realm-1/verify-email`页面
And 显示"请查收验证邮件"的提示信息
And 用户无法登录，直到完成邮箱验证

**场景 1c：邮箱验证成功**
Given 用户账号状态为"WaitVerified"（status=0）
And 邮箱中包含验证链接
When 用户点击邮件中的验证链接
Then 验证成功，账号状态更新为"Normal"（status=1）
And 用户可以正常登录
And 显示"邮箱验证成功"的提示信息

**场景 1d：重新发送验证邮件**
Given 用户在邮箱验证页面
When 用户点击"重新发送验证邮件"按钮
And 距离上次发送超过 60 秒
Then 系统重新发送验证邮件到用户邮箱
And 显示"验证邮件已发送"的提示信息

**场景 1e：重新发送验证邮件频率限制（失败场景）**
Given 用户在邮箱验证页面
And 距离上次发送不足 60 秒
When 用户点击"重新发送验证邮件"按钮
Then 系统提示"请等待 60 秒后再试"
And 不发送新的验证邮件

**场景 1f：未验证账号尝试登录（失败场景）**
Given 用户账号状态为"WaitVerified"（status=0）
When 用户在登录页面输入邮箱和密码
Then 系统提示"邮箱未验证或账号未激活"
And 登录失败
And 用户停留在登录页面

**场景 2：邮箱格式验证失败（失败场景）**
Given 用户在注册页面
When 输入无效邮箱格式"invalid-email"
Then 系统提示"请输入有效的邮箱地址"

**场景 3：密码不符合策略（失败场景）**
Given Realm配置密码策略为最少8位、必须包含大小写字母、数字和特殊字符
When 用户在注册页面输入密码"Pass123"
Then 系统提示"密码必须包含大小写字母、数字和特殊字符"

**场景 4：两次密码不一致（失败场景）**
Given 用户在注册页面
When 输入密码"Password123!"和确认密码"Password456!"
Then 系统提示"两次密码不一致"

**场景 5：邮箱已存在（失败场景）**
Given 邮箱"user@example.com"已注册
When 用户使用相同邮箱再次注册
Then 系统提示"该邮箱已被注册"

**场景 6：Realm未开启注册（失败场景）**
Given Realm realm-2未开启用户自注册功能
When 用户访问`/realm-2/register`页面
Then 系统提示"该Realm未开启注册功能"或隐藏注册入口

**场景 7：Turnstile验证失败（失败场景）**
Given Realm realm-1开启了Turnstile验证
When 用户未完成Turnstile验证就提交注册表单
Then 系统提示"请完成人机验证"

---

### 故事 2：账号登录 [US-RU-002]

**【用户故事】**
**作为**：普通用户
**我希望**：能够使用邮箱和密码登录系统
**从而**：访问授权的第三方应用

**【验收标准】**

**场景 1：正常登录成功**
Given 用户已注册账号user@example.com，密码为Password123!
When 用户访问`/realm-1/login`页面
And 输入正确的邮箱和密码
And 完成Turnstile验证（若启用）
Then 登录成功，系统设置Session Cookie并重定向

**场景 2：密码错误（失败场景）**
Given 用户账号user@example.com已注册
When 用户输入错误密码wrongpassword
Then 系统提示"邮箱或密码错误"，停留在登录页面

**场景 3：邮箱不存在（失败场景）**
Given 邮箱notexist@example.com未注册
When 用户使用该邮箱登录
Then 系统提示"邮箱或密码错误"

**场景 4：账号被禁用（失败场景）**
Given 用户账号user@example.com的状态为"Forbidden"（status=2）
When 用户尝试登录
Then 系统提示"账号已被禁用，请联系管理员"

**场景 5：Turnstile验证失败（失败场景）**
Given Realm realm-1开启了Turnstile验证
When 用户未完成Turnstile验证就点击登录
Then 系统提示"请完成人机验证"

---

### 故事 3：OAuth 第三方登录 [US-RU-003]

**【用户故事】**
**作为**：普通用户
**我希望**：能够使用第三方账号（Google、GitHub、Facebook、Apple）登录
**从而**：无需记忆额外密码

**【验收标准】**

**场景 1：Google登录成功**
Given Realm realm-1已配置Google OAuth Provider
When 用户在登录页面点击"Continue with Google"
And 在Google页面授权
Then OAuth回调成功，用户登录系统

**场景 2：GitHub登录成功**
Given Realm realm-1已配置GitHub OAuth Provider
When 用户在登录页面点击"Continue with GitHub"
And 在GitHub页面授权
Then OAuth回调成功，用户登录系统

**场景 3：Facebook登录成功**
Given Realm realm-1已配置Facebook OAuth Provider
When 用户在登录页面点击"Continue with Facebook"
And 在Facebook页面授权
Then OAuth回调成功，用户登录系统

**场景 4：Apple登录成功**
Given Realm realm-1已配置Apple OAuth Provider
When 用户在登录页面点击"Continue with Apple"
And 在Apple页面授权
Then OAuth回调成功，用户登录系统

**场景 5：用户拒绝授权（失败场景）**
Given 用户点击"Continue with Google"
When 在Google授权页面点击"取消"
Then 系统提示"授权失败，请重试"

**场景 6：OAuth Provider未启用（失败场景）**
Given Realm realm-2未配置Google OAuth Provider
When 用户访问登录页面
Then 不显示"Continue with Google"按钮

**场景 7：State Token验证失败（失败场景）**
Given 用户发起Google OAuth登录
When OAuth回调时state token无效或已过期
Then 系统提示"登录链接已过期，请重新登录"

**场景 8：OAuth登录时Email冲突**
Given Google账号返回邮箱user@example.com
And 该邮箱已在本Realm注册
Then 系统自动关联OAuth账户到已有用户

---

### 故事 4：修改个人密码 [US-RU-004]

**【用户故事】**
**作为**：普通用户
**我希望**：能够修改自己的登录密码
**从而**：定期更新密码保护账户安全

**【验收标准】**

**场景 1：正常修改密码成功**
Given 用户已登录
When 用户访问个人资料页面
And 输入当前密码、新密码（符合密码策略）、确认密码并提交
Then 密码修改成功，下次登录需使用新密码

**场景 2：当前密码错误（失败场景）**
Given 用户已登录
When 输入错误的当前密码
Then 系统提示"当前密码错误"

**场景 3：新密码不符合策略（失败场景）**
Given Realm配置密码策略
When 用户在密码修改页面输入不符合策略的新密码
Then 系统提示"密码必须符合安全策略"

**场景 4：两次新密码不一致（失败场景）**
Given 用户在密码修改页面
When 输入新密码"NewPass123!"和确认密码"NewPass456!"
Then 系统提示"两次新密码不一致"

---

### 故事 5：查看个人资料 [US-RU-005]

**【用户故事】**
**作为**：普通用户
**我希望**：能够查看自己的个人资料
**从而**：了解自己的账户信息

**【验收标准】**

**场景 1：查看个人资料成功**
Given 用户已登录
When 用户访问个人资料页面
Then 显示邮箱、昵称、账号状态、创建时间

**场景 2：Email为只读字段**
Given 用户在个人资料页面
Then Email字段为只读，不可修改

**场景 3：账号状态为只读字段**
Given 用户在个人资料页面
Then 状态字段为只读，显示"Normal"、"Forbidden"等

---

### 故事 6：修改个人昵称 [US-RU-006]

**【用户故事】**
**作为**：普通用户
**我希望**：能够修改自己的昵称
**从而**：个性化显示名称

**【验收标准】**

**场景 1：正常修改昵称成功**
Given 用户已登录，当前昵称为"User"
When 用户进入个人资料页面
And 修改昵称为"John Doe"并保存
Then 昵称更新成功

**场景 2：昵称为可选字段**
Given 用户在个人资料页面
When 清空昵称并保存
Then 更新成功，昵称为空

**场景 3：昵称长度限制（失败场景）**
Given 用户在个人资料页面
When 输入昵称超过50个字符
Then 系统提示"昵称最多50个字符"

---

### 故事 7：退出登录 [US-RU-007]

**【用户故事】**
**作为**：普通用户
**我希望**：能够安全退出登录
**从而**：保护账户安全

**【验收标准】**

**场景 1：正常退出登录成功**
Given 用户已登录
When 用户点击"Logout"按钮
Then 系统清除Session Cookie并重定向到登录页面

**场景 2：退出后无法访问受保护资源（失败场景）**
Given 用户已退出登录
When 尝试访问需要认证的页面
Then 系统重定向到登录页面

---

### 故事 8：访问第三方应用 [US-RU-008]

**【用户故事】**
**作为**：普通用户（详见 [docs/user-stories/_roles.md](/docs/user-stories/_roles.md)）
**我希望**：能够使用 Herald 账号登录第三方应用
**从而**：获得单点登录（SSO）体验

**【验收标准】**

**场景 1：通过 OAuth 重定向登录第三方应用**
Given 第三方应用已接入 Herald 的 realm-1
And 用户在第三方应用页面点击"使用 Herald 登录"
When 用户被重定向到 Herald 登录页面
And 用户输入正确的邮箱和密码登录成功
Then Herald 生成授权码并将用户重定向回第三方应用
And 第三方应用使用授权码完成认证

**场景 2：Session 过期后重新登录**
Given 用户的 Session 已过期
When 用户访问第三方应用
Then 第三方应用将用户重定向到 Herald 登录页面

**场景 3：已登录用户直接完成 SSO（失败场景）**
Given 用户已在 Herald 登录
When 用户从第三方应用跳转到 Herald 授权端点
Then 用户无需再次输入凭据即可完成认证（需 Herald 支持静默授权，当前版本仍需登录）

---

### 故事 9：认证重定向流程 [US-RU-009]

**【用户故事】**
**作为**：所有用户（Admin Realm管理员、Realm管理员、普通用户）
**我希望**：系统能够根据我的认证状态和权限智能重定向到适当的页面
**从而**：获得流畅的用户体验

**【验收标准】**

**场景 1：未认证用户访问根URL**
Given 用户未登录
When 用户访问 `http://localhost:3000/`
Then 重定向到 `/admin/auth/login`（使用'admin' realm）

**场景 2：未认证用户访问受保护路由**
Given 用户未登录
When 用户访问 `/${realmId}/manage` 或 `/${realmId}/user/profile`
Then 重定向到 `/${realmId}/auth/login`，并携带 redirect 参数

**场景 3：拥有管理权限的用户登录重定向**
Given 用户拥有管理权限（users.*、roles.*、permissions.*、clients.*、realms.*、client_apps.* 任意权限）
When 用户登录成功
Then 重定向到 `/${realmId}/manage`

**场景 4：无管理权限的用户登录重定向**
Given 用户没有管理权限
When 用户登录成功
Then 重定向到 `/${realmId}/user/profile`

**场景 5：拥有管理权限的用户访问realm根路径**
Given 用户已登录且拥有管理权限
When 用户访问 `/${realmId}` 或 `/${realmId}/`
Then 重定向到 `/${realmId}/manage`

**场景 6：无管理权限的用户访问realm根路径**
Given 用户已登录且没有管理权限
When 用户访问 `/${realmId}` 或 `/${realmId}/`
Then 重定向到 `/${realmId}/user/profile`

**场景 7：无管理权限的用户访问管理后台（权限拒绝）**
Given 用户已登录但没有管理权限
When 用户尝试访问 `/${realmId}/manage`
Then 重定向到 `/${realmId}/user/profile`

**场景 8：退出登录并重定向**
Given 用户已登录
When 用户点击退出登录按钮
Then 清除Session Cookie并重定向到登录页面

**业务规则**：
1. 根URL (`/`) 始终重定向到 'admin' realm 的登录页面 (`/admin/auth/login`)
2. 登录成功后的重定向取决于用户拥有的权限，而非具体角色
3. 管理权限包括：
   - `users.view`, `users.create`, `users.update`, `users.delete`
   - `roles.view`, `roles.create`, `roles.update`, `roles.delete`
   - `permissions.view`, `permissions.create`, `permissions.update`, `permissions.delete`
   - `clients.view`, `clients.create`, `clients.update`, `clients.delete`
   - `realms.create`, `realms.update`
   - `client_apps.view`, `client_apps.create`, `client_apps.update`, `client_apps.delete`
4. 拥有任意管理权限的用户可访问管理后台，无管理权限的用户将被重定向到个人中心

---

### 故事 10：从第三方 Web 应用跳转登录 [US-RU-010]

**【用户故事】**
**作为**：普通用户（详见 [docs/user-stories/_roles.md](/docs/user-stories/_roles.md)）
**我希望**：从第三方 Web 应用跳转到 Herald 完成认证后自动返回第三方应用
**从而**：无需在第三方应用中输入密码，无缝使用第三方服务

**【验收标准】**

**场景 1：从第三方跳转登录成功后自动返回**
Given 用户在第三方应用页面点击"使用 Herald 登录"
And Herald 登录页面显示了 OAuth 上下文参数（来源应用名称等）
When 用户输入正确的邮箱和密码完成登录
Then 页面自动跳转回第三方应用的 callback 地址
And 第三方应用完成认证，用户可以正常使用服务

**场景 2：TOTP 二次认证后自动返回第三方**
Given 用户已启用 TOTP 二次认证
And 用户从第三方应用跳转到 Herald 登录
When 用户输入正确的密码后，系统要求输入 TOTP 验证码
And 用户输入正确的 TOTP 验证码
Then TOTP 验证通过后页面自动跳转回第三方应用的 callback 地址

**场景 3：登录失败停留在 Herald 登录页**
Given 用户从第三方应用跳转到 Herald 登录
When 用户输入错误的密码
Then 页面停留在 Herald 登录页，显示错误提示
And OAuth 上下文参数保持不变，用户可以重试

**场景 4：OAuth 参数不完整时显示错误**
Given 用户通过某种方式访问到 Herald 登录页
And URL 中只有部分 OAuth 参数（如只有 oauthClientId 但缺少 redirectUri 和 state）
When 页面加载时
Then 系统显示错误提示，告知用户 OAuth 参数不完整
And 不静默降级为普通登录

---

## 备注

### 业务规则
1. 普通用户只能通过前端注册页面注册（若Realm开启）
2. 无管理权限的用户不能访问管理后台页面（`/{realmId}/dashboard`、`/{realmId}/users`、`/{realmId}/roles` 等）
3. 普通用户只能访问第三方应用（通过Client App接入）
4. 普通用户的状态包括：
   - `WaitVerified` (0): 待验证
   - `Normal` (1): 正常
   - `Forbidden` (2): 禁用
   - `Invalid` (3): 无效
5. 普通用户只能修改自己的昵称和密码
6. 普通用户不能修改邮箱和状态

### 与其他角色的区别
| 功能 | 拥有管理权限的用户 | 无管理权限的用户（普通用户） |
|------|---------|---------|
| 访问管理后台 | ✅ | ❌（重定向到个人中心） |
| 注册账号 | ❌（管理员创建） | ✅（若Realm开启） |
| 管理用户 | ✅（根据权限范围） | ❌ |
| 管理角色权限 | ✅（根据权限范围） | ❌ |
| 管理客户端应用 | ✅（根据权限范围） | ❌ |
| 配置系统设置 | ✅（根据权限范围） | ❌ |
| 访问第三方应用 | ✅ | ✅ |
| 修改个人资料 | ✅ | ✅ |
| OAuth登录 | ✅ | ✅ |

### 访问路径说明
- 注册页面：`/{realmId}/register`
- 登录页面：`/{realmId}/login`
- 管理后台：`/{realmId}/dashboard`、`/{realmId}/users`、`/{realmId}/roles`、`/{realmId}/permissions`、`/{realmId}/settings` 等（需要管理权限）
- 个人中心：`/{realmId}/profile`、`/{realmId}/profile/security`（所有登录用户可访问）
- 第三方应用：由接入方定义，通过Herald验证身份

**说明**：用户是否能访问管理后台取决于其拥有的权限（users.*、roles.*、permissions.*、client.*、realms.*、billing.*），而非具体角色。拥有任意管理权限的用户可以访问管理后台，无管理权限的用户将被重定向到个人中心。

---

## 优先级

**故事 1**: P0（关键）- 个人资料管理是基本用户需求
**故事 2**: P0（关键）- 修改密码是安全功能
**故事 3**: P1（重要）- OAuth 第三方登录增强便利性

---

## 📖 相关PRD

- **用户管理**: [docs/prd/core/users.md](/docs/prd/core/users.md)
