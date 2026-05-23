# Admin Realm 管理员 用户故事

> 角色定义见 [docs/user-stories/_roles.md](/docs/user-stories/_roles.md)

## 用户故事

### 故事 1：创建 Realm [US-AR-001]

**优先级**: P0

**【用户故事】**
**作为**：Admin Realm 管理员（详见 [docs/user-stories/_roles.md](/docs/user-stories/_roles.md)）
**我希望**：创建新的 Realm，以便为不同组织提供独立的认证服务
**从而**：支持多租户业务场景

**【验收标准】**

> 验收标准只描述用户动作与可见结果，不写 API 路径、数据表、字段变更、技术实现步骤。

**场景 1：正常创建 Realm 成功**
```gherkin
Given 我是 Admin Realm 的管理员
And 我拥有 realm.manage 权限
When 我在 Realms 管理页面点击 "Create Realm" 按钮
And 我填写 Realm 信息：
  | Realm ID | myapp                  |
  | Name     | My Application         |
  | Email    | admin@myapp.com        |
  | Password | SecurePassword123      |
And 我提交表单
Then Realm 创建成功
And 系统显示成功消息："Realm 'myapp' created successfully"
And Realm 列表显示新创建的 Realm
And 新 Realm 自动创建默认角色和权限配置
And 新 Realm 自动创建管理控制台客户端应用
And 提供的管理员用户自动分配管理员角色
```

**场景 2：Realm ID 必填验证**
```gherkin
Given 我是 Admin Realm 的管理员
When 我在创建 Realm 表单中留空 "Realm ID" 字段
And 我填写其他必填字段并提交
Then 系统显示验证错误："Realm ID is required"
And Realm 创建失败
```

**场景 3：Realm ID 格式验证**
```gherkin
Given 我是 Admin Realm 的管理员
And 我在创建 Realm 表单中输入无效的 Realm ID
When 我提交表单
Then 系统显示验证错误：
  | 错误类型 | 说明                     |
  | 格式错误 | "Realm ID must be alphanumeric" |
  | 长度错误 | "Realm ID must be 3-36 characters" |
  | 保留词   | "Realm ID cannot be a reserved word" |
  | 已存在   | "Realm ID already exists"            |
```

**场景 4：密码强度验证**
```gherkin
Given 我是 Admin Realm 的管理员
And 我在创建 Realm 表单中输入弱密码
When 我提交表单
Then 系统显示验证错误："Password must be at least 8 characters"
```

---

### 故事 2：查看 Realm 列表 [US-AR-002]

**优先级**: P0

**【用户故事】**
**作为**：Admin Realm 管理员（详见 [docs/user-stories/_roles.md](/docs/user-stories/_roles.md)）
**我希望**：查看所有 Realm，以便管理系统中的组织
**从而**：了解系统整体情况

**【验收标准】**

> 验收标准只描述用户动作与可见结果，不写 API 路径、数据表、字段变更、技术实现步骤。

**场景 1：查看所有 Realm（分页显示）**
```gherkin
Given 我是 Admin Realm 的管理员
When 我访问 Realms 管理页面
Then 我看到 Realm 列表表格
And 表格包含以下列：
  | 列名         | 说明               |
  | Realm ID     | Realm 的唯一标识符 |
  | Name         | Realm 显示名称     |
  | Created At   | 创建时间           |
  | Updated At   | 更新时间           |
And 表格默认每页显示 25 个 Realm
And 我可以查看分页控件
```

**场景 2：Realm 列表排序**
```gherkin
Given 我在 Realms 管理页面
When 我点击 "Realm ID" 列标题
Then Realm 列表按 Realm ID 升序排序
And 当我再次点击 "Realm ID" 列标题
Then Realm 列表按 Realm ID 降序排序
And 我可以点击任意列标题进行排序：
  | Realm ID   | 按 Realm ID 排序 |
  | Name       | 按名称排序       |
  | Created At | 按创建时间排序   |
  | Updated At | 按更新时间排序   |
```

**场景 3：搜索 Realm**
```gherkin
Given 我在 Realms 管理页面
When 我在搜索框中输入 "test"
Then 表格仅显示 Realm ID 或名称包含 "test" 的 Realm
And 当我清空搜索框
Then 表格显示所有 Realm
```

**场景 4：分页导航**
```gherkin
Given 我在 Realms 管理页面
And 系统中有超过 25 个 Realm
When 我点击 "Next" 按钮
Then 表格显示下一页的 25 个 Realm
And 当我点击 "Previous" 按钮
Then 表格显示上一页的 25 个 Realm
And 我可以跳转到指定页码
```

---

### 故事 3：查看 Realm 详情 [US-AR-003]

**优先级**: P1

**【用户故事】**
**作为**：Admin Realm 管理员（详见 [docs/user-stories/_roles.md](/docs/user-stories/_roles.md)）
**我希望**：查看 Realm 详情，以便了解 Realm 的配置信息
**从而**：更好地管理 Realm

**【验收标准】**

> 验收标准只描述用户动作与可见结果，不写 API 路径、数据表、字段变更、技术实现步骤。

**场景 1：查看 Realm 基本信息**
```gherkin
Given 我是 Admin Realm 的管理员
When 我在 Realm 列表中点击某个 Realm
Then 我看到 Realm 详情页面
And 页面显示以下信息：
  | 字段       | 说明           |
  | Realm ID   | Realm 唯一标识 |
  | Name       | Realm 名称     |
  | Created At | 创建时间       |
  | Updated At | 更新时间       |
```

---

### 故事 4：Realm 创建权限控制 [US-AR-004]

**优先级**: P0

**【用户故事】**
**作为**：Admin Realm 管理员（详见 [docs/user-stories/_roles.md](/docs/user-stories/_roles.md)）
**我希望**：只有拥有 Realm 管理权限的 Admin Realm 用户才能创建新 Realm
**从而**：防止未授权用户创建 Realm，保证系统安全

**【验收标准】**

> 验收标准只描述用户动作与可见结果，不写 API 路径、数据表、字段变更、技术实现步骤。

**场景 1：拥有权限的用户创建 Realm 成功**
```gherkin
Given 我是 Admin Realm 的管理员
And 我拥有 realm.manage 权限
When 我尝试创建新 Realm
Then Realm 创建成功
```

**场景 2：无权限的 Realm Admin 无法创建 Realm**
```gherkin
Given 我是 Realm Admin（来自非 admin realm）
And 我没有 realm.manage 权限
When 我尝试通过管理界面创建新 Realm
Then 系统显示错误："权限不足"
And 我无法访问创建 Realm 功能
```

**场景 3：Realms 导航菜单权限控制**
```gherkin
Given 我是 Realm Admin（非 admin realm）
And 我没有 realm.manage 权限
When 我登录系统
Then 左侧导航菜单中不显示 "Realms" 菜单项
```

**场景 4：直接访问 URL 权限检查**
```gherkin
Given 我是 Realm Admin（非 admin realm）
And 我没有 realm.manage 权限
When 我直接访问 Realms 管理页面
Then 系统返回权限不足提示或重定向到无权限页面
```

---

### 故事 5：访问新创建的 Realm [US-AR-005]

**优先级**: P0

**【用户故事】**
**作为**：Admin Realm 管理员（详见 [docs/user-stories/_roles.md](/docs/user-stories/_roles.md)）
**我希望**：创建 Realm 后能够访问该 Realm 的管理界面
**从而**：验证 Realm 配置、设置权限、管理 Realm 用户

**【验收标准】**

> 验收标准只描述用户动作与可见结果，不写 API 路径、数据表、字段变更、技术实现步骤。

**场景 1：创建 Realm 后访问其 Dashboard**
```gherkin
Given 我是 Admin Realm 的管理员
And 我创建了新 Realm "test-realm-123"
When 我使用该 Realm 的管理员账号登录
  | Email           | admin@test-realm-123.com |
  | Password        | password123              |
  | Realm           | test-realm-123           |
And 我访问该 Realm 的 Dashboard
Then 我看到 Dashboard 页面加载成功
And 页面显示 Realm 名称 "test-realm-123"
And 我可以访问该 Realm 的管理功能
```

**场景 2：验证新 Realm 自动创建的默认配置**
```gherkin
Given 我是 Admin Realm 的管理员
And 我创建了新 Realm "myapp"
And 我使用该 Realm 的管理员账号登录
When 我访问该 Realm 的角色管理页面
Then 我看到默认角色存在：
  | realm-admin |
  | user        |
And 我看到默认权限配置正确
```
