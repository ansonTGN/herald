# Realm Admin 用户故事

**角色代码**: RA
**角色定义**：Realm Admin 是特定 Realm 的管理员，负责管理该 Realm 下的用户、客户端应用、角色、权限和订阅套餐。

**故事范围**: US-RA-001 ~ US-RA-012
**创建时间**: 2025-02-01
**状态**: Active

---

## 故事 1：Realm 隔离访问 [US-RA-001]

**优先级**: P0

**【用户故事】**
**作为**：Realm Admin
**我希望**：只能访问自己 Realm 的资源，以便保证数据隔离
**从而**：确保多租户系统的安全性和数据隔离

**【验收标准】**

**场景 1：访问自己 Realm 的资源**
```gherkin
Given 我是 realm-1 的管理员
When 我访问 realm-1 的用户管理页面（/realm-1/users）
Then 我可以看到 realm-1 的所有用户
And 我可以管理这些用户
```

**场景 2：不能访问其他 Realm 的资源**
```gherkin
Given 我是 realm-1 的管理员
When 我尝试访问 realm-2 的用户管理页面（/realm-2/users）
Then 系统返回 403 Forbidden
And 显示错误消息："Access denied: You do not have permission to access this realm"
```

**场景 3：API 跨 Realm 访问被拒绝**
```gherkin
Given 我是 realm-1 的管理员
And 我的 session 属于 realm-1
When 我通过 API 访问 realm-2 的资源
  When 我通过 API 访问 realm-2 的资源
Then 系统返回 403 Forbidden
And 响应体包含错误信息：
  | error | "Cross-realm access denied" |
```


## 故事 2：角色定义管理 [US-RA-002]

**优先级**: P0

**【用户故事】**
**作为**：Realm Admin
**我希望**：管理角色定义，以便定义不同的用户角色
**从而**：灵活控制用户权限

**【验收标准】**

**场景 1：创建角色定义**
```gherkin
Given 我是 realm-1 的管理员
When 我在角色管理页面点击 "Create Role" 按钮
And 我填写角色信息：
  | Name        | user-admin       |
  | Description | 用户管理员        |
And 我提交表单
Then 角色定义创建成功
And 系统显示成功消息："Role 'user-admin' created successfully"
```

**场景 2：查看角色定义列表**
```gherkin
Given 我是 realm-1 的管理员
When 我访问角色管理页面
Then 我看到角色定义列表
And 列表包含默认角色：
  | Name         | Description     |
  | realm-admin  | Realm 管理员     |
  | user         | 普通用户        |
And 列表包含我创建的自定义角色
```

**场景 3：编辑角色定义**
```gherkin
Given 我是 realm-1 的管理员
And 已存在角色 "user-admin"
When 我编辑该角色的描述为 "高级用户管理员"
Then 角色定义更新成功
And 列表显示更新后的描述
```

**场景 4：删除角色定义**
```gherkin
Given 我是 realm-1 的管理员
And 已存在角色 "temp-role" 且未被使用
When 我删除该角色
Then 角色定义删除成功
And 列表不再显示该角色
```


## 故事 3：权限定义管理 [US-RA-003]

**优先级**: P0

**【用户故事】**
**作为**：Realm Admin
**我希望**：管理权限定义，以便定义系统权限
**从而**：精确控制用户可访问的资源

**【验收标准】**

**场景 1：创建权限定义**
```gherkin
Given 我是 realm-1 的管理员
When 我在权限管理页面点击 "Create Permission" 按钮
And 我填写权限信息：
  | Name        | users.delete       |
  | Resource    | users              |
  | Action      | delete             |
  | Description | 删除用户权限        |
And 我提交表单
Then 权限定义创建成功
```

**场景 2：查看权限定义列表**
```gherkin
Given 我是 realm-1 的管理员
When 我访问权限管理页面
Then 我看到权限定义列表
And 列表包含默认权限（realm-admin 的 17 项权限）
And 列表包含我创建的自定义权限
```


## 故事 4：为角色分配权限 [US-RA-004]

**优先级**: P0

**【用户故事】**
**作为**：Realm Admin
**我希望**：为角色分配权限，以便控制角色可访问的资源
**从而**：实现基于角色的权限控制（RBAC）

**【验收标准】**

**场景 1：为角色分配权限**
```gherkin
Given 我是 realm-1 的管理员
And 已存在角色 "user-admin"
And 已存在权限 "users.view" 和 "users.manage"
When 我在角色详情页面选择 "user-admin" 角色
And 我为该角色分配权限：
  | users.view   |
  | users.manage |
And 我保存更改
Then 权限分配成功
And "user-admin" 角色拥有 users.view 和 users.manage 权限
```

**场景 2：查看角色的权限**
```gherkin
Given 我是 realm-1 的管理员
When 我查看 "user-admin" 角色的详情
Then 我看到该角色拥有的所有权限
And 权限列表包含：
  | Resource | Action  |
  | users    | view    |
  | users    | manage  |
```

**场景 3：移除角色权限**
```gherkin
Given 我是 realm-1 的管理员
And "user-admin" 角色拥有 "users.delete" 权限
When 我移除该权限
Then "user-admin" 角色不再拥有 "users.delete" 权限
And 该角色的用户无法删除用户
```

**场景 4：权限层级自动生效**
```gherkin
Given 我是 realm-1 的管理员
And 已存在角色 "user-admin"
And 已存在权限 "users.manage"
When 我为该角色仅分配 "users.manage" 权限
And 我保存更改
Then "user-admin" 角色自动拥有以下权限：
  | users.view    |
  | users.manage  |
And "user-admin" 角色可以执行所有用户操作：创建、查看、编辑、删除、重置密码等
```

**场景 5：低权限不覆盖高权限**
```gherkin
Given 我是 realm-1 的管理员
And 已存在角色 "viewer"
And 已存在权限 "users.view"
When 我为该角色仅分配 "users.view" 权限
And 我保存更改
Then "viewer" 角色仅拥有：
  | users.view    |
And "viewer" 角色不能访问：
  | users.manage  |
And "viewer" 角色不能执行任何修改操作（编辑、删除、创建等）
```

**场景 6：特殊权限不参与层级**
```gherkin
Given 我是 realm-1 的管理员
And 已存在角色 "super-admin"
When 我为该角色仅分配 "users.admin" 权限
Then 该角色仅拥有：
  | users.admin |
And 该角色不能访问：
  | users.view |
```


## 故事 5：查看角色权限 [US-RA-005]

**优先级**: P0

**【用户故事】**
**作为**：Realm Admin
**我希望**：查看角色的权限，以便了解角色能力
**从而**：更好地管理和审计权限

**【验收标准】**

**场景 1：查看单个角色的权限**
```gherkin
Given 我是 realm-1 的管理员
When 我在角色列表中点击 "realm-admin" 角色
Then 我看到该角色的详情页面
And 页面显示：
  | 字段         | 内容                         |
  | Name         | realm-admin                  |
  | Description  | Realm 管理员                  |
  | Permissions  | 17 项权限列表                |
```

**场景 2：批量查看所有角色的权限**
```gherkin
Given 我是 realm-1 的管理员
When 我访问角色管理页面
Then 我看到角色列表
And 每个角色显示其权限数量
And 我可以点击角色查看详细权限
```


## 故事 6：用户角色分配 [US-RA-006]

**优先级**: P0

**【用户故事】**
**作为**：Realm Admin
**我希望**：为用户分配角色，以便控制用户权限
**从而**：实现灵活的用户权限管理

**【验收标准】**

**场景 1：为用户分配角色**
```gherkin
Given 我是 realm-1 的管理员
And 已存在用户 "user@example.com"
And 已存在角色 "user-admin"
When 我在用户详情页面访问 "Roles" 标签
And 我为该用户分配 "user-admin" 角色
And 我保存更改
Then 角色分配成功
And 用户 "user@example.com" 拥有 "user-admin" 角色的所有权限
```

**场景 2：为用户分配多个角色**
```gherkin
Given 我是 realm-1 的管理员
And 已存在用户 "advanced-user@example.com"
And 已存在角色 "user-admin" 和 "billing-admin"
When 我为该用户分配两个角色：
  | user-admin    |
  | billing-admin |
And 我保存更改
Then 角色分配成功
And 用户拥有两个角色的所有权限（并集）
```

**场景 3：移除用户角色**
```gherkin
Given 我是 realm-1 的管理员
And 用户 "user@example.com" 拥有 "user-admin" 角色
When 我移除该角色
Then 用户不再拥有 "user-admin" 角色的权限
And 用户只保留剩余角色的权限
```

**场景 4：查看用户的角色**
```gherkin
Given 我是 realm-1 的管理员
When 我查看用户 "user@example.com" 的详情
Then 我看到该用户的角色列表
And 角色列表显示：
  | Role         | Assigned At  |
  | user-admin   | 2025-01-15   |
```


## 故事 7：权限策略管理 [US-RA-007]

**优先级**: P0

**【用户故事】**
**作为**：Realm Admin
**我希望**：管理权限策略，以便定义资源访问规则
**从而**：实现细粒度的权限控制

**【验收标准】**

**场景 1：创建权限策略**
```gherkin
Given 我是 realm-1 的管理员
When 我在权限策略页面点击 "Create Policy" 按钮
And 我填写策略信息：
  | Role    | user-admin   |
  | Resource| users        |
  | Action  | manage       |
And 我提交表单
Then 权限策略创建成功
And 拥有 "user-admin" 角色的用户可以管理用户资源
```

**场景 2：查看权限策略列表**
```gherkin
Given 我是 realm-1 的管理员
When 我访问权限策略页面
Then 我看到权限策略列表
And 列表包含默认策略：
  | Role         | Resource | Action  |
  | realm-admin  | *        | *       |
And 列表包含我创建的自定义策略
```

**场景 3：删除权限策略**
```gherkin
Given 我是 realm-1 的管理员
And 已存在策略："user-admin 可以 manage users"
When 我删除该策略
Then 策略删除成功
And 拥有 "user-admin" 角色的用户无法再管理用户资源
```


## 故事 8：订阅套餐管理 [US-RA-008]

**优先级**: P0

**【用户故事】**
**作为**：Realm Admin
**我希望**：管理订阅套餐，以便为用户提供不同的订阅选项
**从而**：实现灵活的订阅计费模式

**【验收标准】**

**场景 1：创建订阅套餐**
```gherkin
Given 我是 realm-1 的管理员
When 我在 Billing 管理页面点击 "Create Plan" 按钮
And 我填写套餐信息：
  | Name         | basic                |
  | Title        | 基础版               |
  | Description  | 适合小型团队          |
  | Type         | monthly              |
  | Price        | 1000                 |
  | Currency     | USD                  |
And 我提交表单
Then 订阅套餐创建成功
And 套餐列表显示新创建的套餐
```

**场景 2：分配套餐到 Client App**
```gherkin
Given 我是 realm-1 的管理员
And 已存在套餐 "basic"
And 已存在 Client App "mobile-app"
When 我在套餐管理页面点击 "Assign" 按钮
And 我选择 "mobile-app"
And 我保存分配
Then 套餐分配成功
And "mobile-app" 的用户可以看到 "basic" 套餐
```

**场景 3：查看订阅列表**
```gherkin
Given 我是 realm-1 的管理员
When 我访问订阅管理页面
Then 我看到订阅列表
And 列表包含：
  | User       | Plan    | Status  | Billing Period   |
  | user@...   | basic   | active  | 2025-01 - 2025-02|
```


## 故事 9：权限层级验证 [US-RA-009]

**优先级**: P0

**【用户故事】**
**作为**：Realm Admin
**我希望**：系统能自动应用权限层级规则，以便简化角色权限配置
**从而**：减少手动配置多个权限的工作量

**【验收标准】**

**场景 1：Manage 权限自动覆盖所有操作**
```gherkin
Given 我是 realm-1 的管理员
And 已存在角色 "admin"
When 我为该角色仅分配 "users.manage" 权限
And 用户 "admin-user" 拥有 "admin" 角色
When 该用户尝试执行以下操作：
  | 操作 | 预期结果 |
  | 查看用户列表 | ✅ 成功 |
  | 编辑用户信息 | ✅ 成功 |
  | 删除用户 | ✅ 成功 |
  | 管理用户 | ✅ 成功 |
  | 重置用户密码 | ✅ 成功 (manage 覆盖所有操作) |
Then 所有符合预期的操作都能正常执行
```

**场景 2：验证层级关系不影响低权限角色**
```gherkin
Given 我是 realm-1 的管理员
And 已存在角色 "viewer"
And 我为该角色仅分配 "users.view" 权限
And 用户 "viewer-user" 拥有 "viewer" 角色
When 该用户尝试执行以下操作：
  | 操作 | 预期结果 |
  | 查看用户列表 | ✅ 成功 |
  | 编辑用户信息 | ❌ 失败 (view 不包含 manage) |
  | 删除用户 | ❌ 失败 |
Then 只有查看操作能正常执行
```

**场景 3：特殊权限不参与层级**
```gherkin
Given 我是 realm-1 的管理员
And 已存在角色 "realm-creator"
And 我为该角色仅分配 "realm.create" 权限
And 用户拥有 "realm-creator" 角色
When 该用户尝试执行以下操作：
  | 操作 | 预期结果 |
  | 创建新 Realm | ✅ 成功 |
  | 查看 Realm 列表 | ❌ 失败 (create 不包含 view) |
Then 只有创建操作能正常执行
```


## 故事 10：查看 Dashboard 用户活跃概览 [US-RA-010]

**优先级**: P1

**【用户故事】**
**作为**：Realm Admin
**我希望**：在 Admin Dashboard 首屏看到本 Realm 的用户核心指标（总用户数、新增用户数、活跃用户数）
**从而**：快速了解 Realm 的用户增长与活跃趋势，无需进入多个子页面分别查看

**【验收标准】**

**场景 1：Dashboard 正常展示用户指标**
```gherkin
Given 我是 realm-1 的管理员
And realm-1 下有用户和近期的登录活动
When 我访问 realm-1 的管理后台首页
Then 页面首屏显示 3 张指标卡片：总用户数、最近 7 天新增用户数、最近 7 天活跃用户数
And 每张卡片显示对应的数值
```

**场景 2：新 Realm 无数据时的空态**
```gherkin
Given 我是新创建的 realm-2 的管理员
And realm-2 下没有任何用户
When 我访问 realm-2 的管理后台首页
Then 3 张指标卡片均显示数值 0
```

## 故事 11：查看 Dashboard 认证趋势图 [US-RA-011]

**优先级**: P1

**【用户故事】**
**作为**：Realm Admin
**我希望**：在 Dashboard 上看到最近 30 天的认证趋势图（登录成功和失败次数）
**从而**：发现异常登录波动，及时响应安全问题

**【验收标准】**

**场景 1：正常展示认证趋势**
```gherkin
Given 我是 realm-1 的管理员
And realm-1 在最近 30 天内有认证事件记录
When 我访问 realm-1 的管理后台首页
Then 页面显示最近 30 天的认证趋势图
And 图表包含两条线：登录成功次数和登录失败次数
And 图表按天聚合，X 轴为日期
```

**场景 2：无认证事件时的空态**
```gherkin
Given 我是 realm-1 的管理员
And realm-1 在最近 30 天内没有任何认证事件
When 我访问 realm-1 的管理后台首页
Then 趋势图区域显示"暂无数据"提示
```

## 故事 12：通过 Dashboard 快捷导航跳转 [US-RA-012]

**优先级**: P1

**【用户故事】**
**作为**：Realm Admin
**我希望**：在 Dashboard 下方保留原有的管理功能导航入口
**从而**：从 Dashboard 快速跳转到用户管理、角色管理等子页面

**【验收标准】**

**场景 1：快捷导航正常跳转**
```gherkin
Given 我是 realm-1 的管理员
When 我访问 realm-1 的管理后台首页
Then 页面下方显示快捷导航入口（Users、Roles、Permissions、Client Apps、Realms、Settings）
And 点击任一导航项可跳转到对应管理页面
```

**场景 2：原有导航入口不丢失**
```gherkin
Given Dashboard 重设计前已有 6 张导航卡片
When Dashboard 重设计上线后
Then 所有原有导航入口仍可通过页面下方快捷导航区域访问
And 每个导航入口的跳转目标不变
```


## 用户故事优先级汇总

| 优先级 | 用户故事数量 | 关键故事 |
|--------|------------|---------|
| P0 | 9 | Realm 隔离访问、角色定义管理、权限定义管理、为角色分配权限、查看角色权限、用户角色分配、权限策略管理、订阅套餐管理、权限层级验证 |
| P1 | 3 | 查看 Dashboard 用户活跃概览、查看 Dashboard 认证趋势图、通过 Dashboard 快捷导航跳转 |
| P2 | 0 | - |

---

## 相关文档

- **PRD**: `docs/prd/auth/permissions.md` - 权限与角色管理产品需求文档
- **PRD**: `docs/prd/billing/billing.md` - Billing 订阅计费产品需求文档
- **技术设计**: `.ai/design/fix-permission-and-sdk-impl.md` - Realm 创建权限修复技术设计
- **用户故事**: `docs/user-stories/builtin_protection.md` - 默认角色和权限保护
- **PRD**: `docs/prd/core/dashboard-redesign.md` - Dashboard 重设计产品需求文档
