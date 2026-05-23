# Audit 审计日志 用户故事

> 角色定义见 [docs/user-stories/_roles.md](/docs/user-stories/_roles.md)

## 用户故事

### 故事 1：查看 Realm 审计日志 [US-AU-001]

**优先级**: P0

**【用户故事】**
**作为**：Realm Admin（详见 [docs/user-stories/_roles.md](/docs/user-stories/_roles.md)）
**我希望**：在管理后台查看当前 Realm 下所有核心操作的审计日志
**从而**：能够追溯系统内关键变更，排查安全事件和操作纠纷

**【验收标准】**

> 验收标准只描述用户动作与可见结果，不写 API 路径、数据表、字段变更、技术实现步骤。

**场景 1：正常查看审计日志列表**
```gherkin
Given Realm Admin 已登录管理后台
And 当前 Realm 存在若干审计日志记录
When 管理员进入审计日志页面
Then 系统按操作时间倒序展示审计日志列表
And 每条记录至少显示操作时间、操作者、操作类型、操作目标和操作结果
```

**场景 2：Realm 隔离**
```gherkin
Given Realm Admin A 已登录管理后台
And 另一个 Realm B 存在审计日志
When 管理员 A 查看审计日志
Then 管理员 A 只能看到自己所属 Realm 的审计日志
And 无法看到 Realm B 的任何审计记录
```

**场景 3：无审计日志时**
```gherkin
Given Realm Admin 已登录管理后台
And 当前 Realm 暂无审计日志记录
When 管理员进入审计日志页面
Then 系统显示"暂无审计日志"的空状态提示
```

---

### 故事 2：按条件筛选审计日志 [US-AU-002]

**优先级**: P0

**【用户故事】**
**作为**：Realm Admin（详见 [docs/user-stories/_roles.md](/docs/user-stories/_roles.md)）
**我希望**：按事件类型、操作者和时间范围筛选审计日志
**从而**：快速定位特定类型的操作记录或特定时间段的变更

**【验收标准】**

> 验收标准只描述用户动作与可见结果，不写 API 路径、数据表、字段变更、技术实现步骤。

**场景 1：按事件类型筛选**
```gherkin
Given Realm Admin 已登录管理后台
And 审计日志中包含多种事件类型（如用户管理、角色变更、权限变更等）
When 管理员选择"角色变更"事件类型筛选
Then 列表仅显示角色相关的审计日志
```

**场景 2：按时间范围筛选**
```gherkin
Given Realm Admin 已登录管理后台
And 审计日志中存在不同时间的记录
When 管理员选择特定时间范围（如最近7天）
Then 列表仅显示该时间范围内的审计日志
```

**场景 3：按操作者筛选**
```gherkin
Given Realm Admin 已登录管理后台
And 审计日志中存在不同操作者的记录
When 管理员输入操作者标识进行搜索
Then 列表仅显示匹配操作者的审计日志
```

**场景 4：组合筛选无结果**
```gherkin
Given Realm Admin 已登录管理后台
When 管理员设置多个筛选条件的组合，且该组合无匹配记录
Then 系统显示"无匹配的审计日志"的空状态提示
```

---

### 故事 3：查看审计日志详情 [US-AU-003]

**优先级**: P1

**【用户故事】**
**作为**：Realm Admin（详见 [docs/user-stories/_roles.md](/docs/user-stories/_roles.md)）
**我希望**：查看某条审计日志的详细信息
**从而**：了解操作的具体变更内容和上下文

**【验收标准】**

> 验收标准只描述用户动作与可见结果，不写 API 路径、数据表、字段变更、技术实现步骤。

**场景 1：查看日志详情**
```gherkin
Given Realm Admin 已登录管理后台
And 审计日志列表中存在记录
When 管理员点击某条审计日志
Then 系统展示该条日志的完整详情
And 详情包含操作时间、操作者信息、操作类型、目标对象、变更详情和操作结果
```

**场景 2：变更详情展示**
```gherkin
Given Realm Admin 正在查看某条审计日志详情
And 该条记录包含变更前后的对比信息
When 系统展示详情
Then 管理员能看到变更的关键信息（如角色名称变更、权限授予/撤销）
```

---

### 故事 4：查看 Admin Realm 审计日志 [US-AU-004]

**优先级**: P0

**【用户故事】**
**作为**：Admin Realm 管理员（详见 [docs/user-stories/_roles.md](/docs/user-stories/_roles.md)）
**我希望**：在管理后台查看 Admin Realm 下的审计日志
**从而**：掌握平台级操作的变更历史，包括 Realm 创建等平台管理操作

**【验收标准】**

> 验收标准只描述用户动作与可见结果，不写 API 路径、数据表、字段变更、技术实现步骤。

**场景 1：查看平台级审计日志**
```gherkin
Given Admin Realm 管理员已登录管理后台
When 管理员进入审计日志页面
Then 系统展示 Admin Realm 的审计日志
And 包含 Realm 创建、RBAC 初始化等平台级操作的记录
```

**场景 2：仅可查看 Admin Realm 数据**
```gherkin
Given Admin Realm 管理员已登录管理后台
When 管理员查看审计日志
Then 管理员只能看到 Admin Realm 的审计日志
And 无法看到其他 Realm 的审计记录
```

---

### 故事 5：系统自动记录核心操作 [US-AU-005]

**优先级**: P0

**【用户故事】**
**作为**：Herald 系统（详见 [docs/user-stories/_roles.md](/docs/user-stories/_roles.md)）
**我希望**：在核心操作发生时自动记录审计事件
**从而**：保证所有关键变更可追溯，满足安全合规要求

**【验收标准】**

> 验收标准只描述用户动作与可见结果，不写 API 路径、数据表、字段变更、技术实现步骤。

**场景 1：用户管理操作记录**
```gherkin
Given 系统正常运行
When Realm Admin 执行用户创建、更新、删除操作
Then 系统自动记录对应的审计事件
And 审计事件包含操作者、目标用户、操作类型和操作结果
```

**场景 2：RBAC 变更操作记录**
```gherkin
Given 系统正常运行
When Realm Admin 执行角色创建/删除/更新、权限授予/撤销、角色分配/取消操作
Then 系统自动记录对应的审计事件
And 审计事件包含变更前后的关键信息
```

**场景 3：Realm 管理操作记录**
```gherkin
Given 系统正常运行
When Admin Realm 管理员执行 Realm 创建操作
Then 系统自动记录对应的审计事件
And 审计事件包含新 Realm 的基本信息
```

**场景 4：认证事件记录**
```gherkin
Given 系统正常运行
When 用户执行登录、登出操作，或登录失败
Then 系统自动记录对应的认证审计事件
And 审计事件包含操作者、认证方式和操作结果
```

**场景 5：操作失败也记录**
```gherkin
Given 系统正常运行
When 用户执行核心操作但操作失败（如权限不足、参数错误）
Then 系统仍然记录该操作尝试的审计事件
And 审计事件的操作结果标记为失败
```
