# SDK 资源管理用户故事

> 角色定义见 [docs/user-stories/_roles.md](/docs/user-stories/_roles.md)

**角色代码**: TP（Third-Party App — SDK 使用方）
**故事范围**: US-TP-012 ~ US-TP-014
**创建时间**: 2026-05-21
**状态**: Active

---

## 统一权限约束

- API Key 代表 Third-Party App 的服务端机器凭据。
- API Key 的能力由 scope 决定，本故事只区分 `runtime` 与 `management`。
- 本文件中的 SDK 资源管理能力均要求 API Key 具备 `management` scope。
- API Key 仍受 Realm 隔离约束，只能操作其所属 Realm 的资源。
- 创建 Realm 额外要求 API Key 属于 admin realm。

---

### 故事 1：通过 SDK 管理 Realm [US-TP-012]

**优先级**: P1

**【用户故事】**
**作为**：Third-Party App（详见 [docs/user-stories/_roles.md](/docs/user-stories/_roles.md)）
**我希望**：通过 SDK 编程式管理 Realm（创建、查询列表、查询详情）
**从而**：在我的平台为不同组织自动开通和管理独立的认证服务

**【验收标准】**

**场景 1：创建 Realm 成功**
```gherkin
Given SDK 已使用具备 management scope 且属于 admin realm 的 API Key 初始化
When 调用 SDK 提供名称和 Realm 管理员信息创建 Realm
Then 返回新创建的 Realm 信息（包含 Realm ID 和名称）
```

**场景 2：查询 Realm 列表**
```gherkin
Given SDK 已使用具备 management scope 的 API Key 初始化
And 系统中存在多个可见 Realm
When 调用 SDK 查询 Realm 列表
Then 返回所有可见 Realm 的列表（包含 ID、名称、状态等基本信息）
```

**场景 3：查询 Realm 详情**
```gherkin
Given SDK 已使用具备 management scope 的 API Key 初始化
And 指定 ID 的 Realm 存在
When 调用 SDK 查询该 Realm 详情
Then 返回 Realm 的完整信息（名称、状态、管理员、创建时间等）
```

**场景 4：创建 Realm 权限不足**
```gherkin
Given SDK 使用的 API Key 不具备 management scope 或不属于 admin realm
When 调用 SDK 创建 Realm
Then 返回权限不足错误
```

**场景 5：Realm 不存在**
```gherkin
Given 指定 ID 的 Realm 不存在
When 调用 SDK 查询该 Realm 详情
Then 返回未找到错误
```

---

### 故事 2：通过 SDK 管理用户 [US-TP-013]

**优先级**: P0

**【用户故事】**
**作为**：Third-Party App
**我希望**：通过 SDK 在指定 Realm 中管理用户（创建、查询列表、查询详情）
**从而**：在我的应用中自动完成用户注册开通和信息查询

**【验收标准】**

**场景 1：创建用户成功**
```gherkin
Given SDK 已使用具备 management scope 的 API Key 初始化，且 API Key 归属于目标 Realm
When 调用 SDK 提供邮箱、密码和昵称创建用户
Then 返回新用户信息（包含用户 ID、邮箱、状态）
```

**场景 2：查询用户列表**
```gherkin
Given SDK 已使用具备 management scope 的 API Key 初始化，且 API Key 归属于目标 Realm
And 目标 Realm 中存在多个用户
When 调用 SDK 查询用户列表
Then 返回用户列表（包含用户 ID、邮箱、昵称、状态）
```

**场景 3：查询用户详情**
```gherkin
Given SDK 已使用具备 management scope 的 API Key 初始化，且 API Key 归属于目标 Realm
And 指定 ID 的用户存在于目标 Realm 中
When 调用 SDK 查询用户详情
Then 返回用户完整信息（ID、邮箱、昵称、状态、角色、创建时间）
```

**场景 4：邮箱重复**
```gherkin
Given 目标 Realm 中已存在相同邮箱的用户
When 调用 SDK 使用该邮箱创建用户
Then 返回明确的错误提示，说明邮箱已被注册
```

**场景 5：跨 Realm 拒绝**
```gherkin
Given API Key 具备 management scope 且归属于 Realm-A
When 调用 SDK 尝试在 Realm-B 中创建或查询用户
Then 返回权限不足错误
```

**场景 6：缺少管理权限**
```gherkin
Given SDK 使用的 API Key 不具备 management scope
When 调用 SDK 在目标 Realm 中创建或查询用户
Then 返回权限不足错误
```

---

### 故事 3：通过 SDK 管理 Client App [US-TP-014]

**优先级**: P1

**【用户故事】**
**作为**：Third-Party App
**我希望**：通过 SDK 在指定 Realm 中管理 Client App（创建、查询列表、查询详情）
**从而**：自动注册新的接入应用并查询已有应用信息

**【验收标准】**

**场景 1：创建 Client App 成功**
```gherkin
Given SDK 已使用具备 management scope 的 API Key 初始化，且 API Key 归属于目标 Realm
When 调用 SDK 提供名称和回调地址创建 Client App
Then 返回新 Client App 信息（包含 Client ID、Client Secret、名称）
```

**场景 2：查询 Client App 列表**
```gherkin
Given SDK 已使用具备 management scope 的 API Key 初始化，且 API Key 归属于目标 Realm
And 目标 Realm 中存在多个 Client App
When 调用 SDK 查询 Client App 列表
Then 返回 Client App 列表（包含 ID、名称、状态、创建时间）
```

**场景 3：查询 Client App 详情**
```gherkin
Given SDK 已使用具备 management scope 的 API Key 初始化，且 API Key 归属于目标 Realm
And 指定 ID 的 Client App 存在于目标 Realm 中
When 调用 SDK 查询 Client App 详情
Then 返回 Client App 完整信息（ID、名称、回调地址、状态、Client ID）
```

**场景 4：缺少必要参数**
```gherkin
Given 调用参数缺少名称
When 调用 SDK 创建 Client App
Then 返回参数校验错误
```

**场景 5：Client App 不存在**
```gherkin
Given 指定 ID 的 Client App 不存在
When 调用 SDK 查询 Client App 详情
Then 返回未找到错误
```

**场景 6：缺少管理权限**
```gherkin
Given SDK 使用的 API Key 不具备 management scope
When 调用 SDK 创建或查询 Client App
Then 返回权限不足错误
```
