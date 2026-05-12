# Billing 用户故事

**角色代码**: BI
**角色定义**：Realm Admin 负责管理 Realm 的订阅套餐、Product 编目上下文和计费配置。

**故事范围**: US-BI-001 ~ US-BI-006
**创建时间**: 2025-02-01
**状态**: Active

---

## 故事 1：创建订阅套餐 [US-BI-001]

**优先级**: P0

**【用户故事】**
**作为**：Realm Admin
**我希望**：在 Product 上下文中创建订阅套餐，以便定义价格和计费信息
**从而**：为用户提供不同的订阅选项

**【验收标准】**

**场景 1：创建月付套餐**
```gherkin
Given 我是 realm-1 的管理员
And 已配置至少一个支付平台（Creem 或 Stripe）
And 已存在 Product "ai-services"
When 我在 Product "ai-services" 详情页面点击 "Create Plan" 按钮
And 我填写套餐信息：
  | Name         | basic                |
  | Title        | 基础版               |
  | Description  | 适合小型团队          |
  | Type         | monthly              |
  | Price        | 1000                 |
  | Currency     | USD                  |
  | Product ID   | prod_basic_monthly   |
  | Checkout URL | https://app.example.com/billing/checkout |
  | Trial Days   | 14                   |
And 我提交表单
Then 订阅套餐创建成功
And 系统显示成功消息："Plan 'basic' created successfully"
And 套餐列表在 Product "ai-services" 下显示新创建的套餐
And 系统提示我需要为该套餐配置支付平台映射
And 套餐的 Payment Providers 列显示 "Not configured"
```

**场景 2：创建年付套餐**
```gherkin
Given 我是 realm-1 的管理员
And 已配置至少一个支付平台
And 已存在 Product "ai-services"
When 我在 Product "ai-services" 下创建套餐
And 我选择 Type 为 "yearly"
And 我设置 Price 为 "10000"（$100/年）
And 我设置 Trial Days 为 "30"
Then 套餐创建成功
And 套餐显示年付价格和 30 天试用期
And 系统提示我可以为该套餐配置支付平台映射
```

**场景 3：套餐名称唯一性验证**
```gherkin
Given 我是 realm-1 的管理员
And 已存在套餐 "basic"
And 该套餐属于 Product "ai-services"
When 我尝试在同一 Realm 内创建同名套餐 "basic"
Then 系统显示验证错误："Plan name 'basic' already exists"
And 套餐创建失败
```

**场景 4：价格验证**
```gherkin
Given 我是 realm-1 的管理员
And 已存在 Product "ai-services"
When 我在 Product "ai-services" 下创建套餐
And 我设置 Price 为 "0" 或负数
Then 系统显示验证错误："Price must be greater than 0"
```

**场景 5：Checkout URL 验证**
```gherkin
Given 我是 realm-1 的管理员
And 已存在 Product "ai-services"
When 我在 Product "ai-services" 下创建套餐
And 我设置 Checkout URL 为无效 URL（不是 http/https）
Then 系统显示验证错误："Checkout URL must be a valid HTTP/HTTPS URL"
```

**场景 6：创建套餐后引导配置支付平台**
```gherkin
Given 我是 realm-1 的管理员
And 我刚刚创建了套餐 "pro"
And 该套餐属于 Product "ai-services"
And 该套餐尚未配置任何支付平台映射
When 我查看套餐详情
Then 我看到提示信息："This plan has no payment providers configured"
And 我看到 "Add Payment Provider" 按钮
And 点击该按钮可以跳转到支付平台配置页面
```


## 故事 2：编辑订阅套餐 [US-BI-002]

**优先级**: P0

**【用户故事】**
**作为**：Realm Admin
**我希望**：在 Product 上下文中编辑订阅套餐的业务信息，以便更新价格和描述
**从而**：适应市场变化和业务需求

**【验收标准】**

**场景 1：编辑套餐基本信息**
```gherkin
Given 我是 realm-1 的管理员
And 已存在套餐 "basic"
And 该套餐属于 Product "ai-services"
When 我在 Product "ai-services" 下的套餐列表中点击 "Edit" 按钮
And 我修改以下信息：
  | Title       | 基础版（已更新）       |
  | Description | 适合个人和小型团队     |
  | Price       | 1200                   |
And 我保存更改
Then 套餐更新成功
And 系统显示成功消息："Plan updated successfully"
And Product "ai-services" 下的套餐列表显示更新后的信息
```

**场景 2：套餐名称不可修改**
```gherkin
Given 我是 realm-1 的管理员
And 已存在套餐 "basic"
And 该套餐属于 Product "ai-services"
When 我编辑该套餐
Then "Name" 字段为只读或禁用
And 我无法修改套餐名称
And 我可以看到该套餐当前所属的 Product
And 我可以看到该套餐配置的支付平台映射列表
```

**场景 3：启用/禁用套餐**
```gherkin
Given 我是 realm-1 的管理员
And 已存在套餐 "basic" 且状态为 "enabled"
And 该套餐属于 Product "ai-services"
When 我切换套餐的 "Status" 开关为 "disabled"
And 我保存更改
Then 套餐状态更新为 "disabled"
And 新用户无法看到该套餐
And 已订阅用户不受影响
And 该套餐下的所有支付平台映射也一并禁用
```

**场景 4：查看套餐详情和支付平台映射**
```gherkin
Given 我是 realm-1 的管理员
And 套餐 "pro" 属于 Product "ai-services"
And 该套餐已配置 Stripe 支付平台映射
When 我查看套餐 "pro" 的详情
Then 我可以看到套餐的基本信息：
  | Name        | pro              |
  | Title       | 专业版           |
  | Price       | 5000             |
  | Currency    | USD              |
  | Type        | monthly          |
And 我可以看到 "Payment Providers" 配置区域
And 该区域显示所有已配置的支付平台映射：
  | Provider | Status | External Product ID | External Price ID |
  | Stripe   | enabled| prod_pro_monthly    | price_pro_monthly |
And 我可以点击 "Manage Payment Providers" 进入支付平台配置页面
And 我可以看到该套餐所属的 Product 为 "ai-services"
```


## 故事 3：配置 Plan 的支付平台映射 [US-BI-003]

**优先级**: P0

**【用户故事】**
**作为**：Realm Admin
**我希望**：为 Plan 配置一个或多个支付平台映射，以便该套餐可以在不同支付平台上售卖
**从而**：为用户提供更多支付方式选择，并避免为每个平台复制套餐

**【验收标准】**

**场景 1：为 Plan 添加支付平台映射**
```gherkin
Given 我是 realm-1 的管理员
And 已存在套餐 "basic"
And 该套餐属于 Product "ai-services"
And 已配置 Stripe 支付平台
When 我在套餐详情页面点击 "Add Payment Provider" 按钮
And 我选择支付平台为 "Stripe"
And 我填写外部产品信息：
  | External Product ID | prod_basic_monthly   |
  | External Price ID   | price_basic_monthly  |
And 我保存配置
Then 支付平台映射添加成功
And 系统显示成功消息："Payment provider 'Stripe' added to plan 'basic'"
And 套餐详情显示该支付平台映射
```

**场景 2：为同一 Plan 添加多个支付平台**
```gherkin
Given 我是 realm-1 的管理员
And 已存在套餐 "pro"
And 该套餐属于 Product "ai-services"
And 已配置 Stripe 和 Creem 支付平台
When 我为套餐 "pro" 添加 Stripe 支付平台映射
And 我为套餐 "pro" 添加 Creem 支付平台映射
Then 所有支付平台映射都成功添加
And 套餐 "pro" 可以在多个支付平台上使用
And 套餐列表显示 "pro" 支持多个支付平台
```

**场景 3：编辑支付平台映射**
```gherkin
Given 我是 realm-1 的管理员
And 套餐 "basic" 已配置 Stripe 支付平台映射
And 该套餐属于 Product "ai-services"
When 我编辑该支付平台映射
And 我修改 External Price ID 为 "price_basic_updated"
And 我保存更改
Then 支付平台映射更新成功
And 系统显示成功消息："Payment provider mapping updated successfully"
```

**场景 4：启用/禁用支付平台映射**
```gherkin
Given 我是 realm-1 的管理员
And 套餐 "basic" 已配置 Stripe 支付平台映射
And Stripe 映射状态为 "enabled"
When 我禁用 Stripe 支付平台映射
Then Stripe 映射状态更新为 "disabled"
And 新用户无法使用 Stripe 支付该套餐
And 已使用 Stripe 订阅的用户不受影响
```

**场景 5：删除支付平台映射**
```gherkin
Given 我是 realm-1 的管理员
And 套餐 "basic" 已配置 Stripe 支付平台映射
And 该支付平台映射下无活跃订阅
When 我删除该支付平台映射
Then 支付平台映射删除成功
And 系统显示成功消息："Payment provider mapping deleted successfully"
And 套餐详情不再显示该支付平台
```

**场景 6：无法删除有活跃订阅的支付平台映射**
```gherkin
Given 我是 realm-1 的管理员
And 套餐 "basic" 已配置 Stripe 支付平台映射
And 该支付平台映射下有 5 个活跃订阅
When 我尝试删除该支付平台映射
Then 系统显示错误消息："Cannot delete payment provider mapping with active subscriptions"
And 显示活跃订阅数量："5 active subscriptions"
And 支付平台映射删除失败
```

**场景 7：查看支付平台映射列表**
```gherkin
Given 我是 realm-1 的管理员
And 套餐 "pro" 属于 Product "ai-services"
And 套餐 "pro" 已配置多个支付平台映射
When 我查看套餐 "pro" 的详情
Then 我看到支付平台映射列表：
  | Provider | External Product ID | External Price ID | Status |
  | Stripe   | prod_pro_monthly    | price_pro_monthly | enabled|
  | Creem    | prod_pro_creem      | price_pro_creem   | disabled|
And 我可以看到每个映射的状态
```

**场景 8：验证支付平台唯一性**
```gherkin
Given 我是 realm-1 的管理员
And 套餐 "basic" 已配置 Stripe 支付平台映射
When 我尝试为同一套餐再次添加 Stripe 支付平台映射
Then 系统显示验证错误："Payment provider 'Stripe' already configured for this plan"
And 支付平台映射添加失败
```


## 故事 4：删除订阅套餐 [US-BI-004]

**优先级**: P0

**【用户故事】**
**作为**：Realm Admin
**我希望**：在 Product 上下文中删除订阅套餐，以便移除不再需要的套餐
**从而**：保持套餐列表的整洁

**【验收标准】**

**场景 1：删除无订阅的套餐**
```gherkin
Given 我是 realm-1 的管理员
And 已存在套餐 "legacy-plan" 且无活跃订阅
And 该套餐属于 Product "legacy-product"
When 我在 Product "legacy-product" 下的套餐列表中点击 "Delete" 按钮
And 我确认删除
Then 套餐删除成功
And 系统显示成功消息："Plan deleted successfully"
And Product "legacy-product" 下的套餐列表不再显示该套餐
And 该套餐的所有支付平台映射也一并删除
```

**场景 2：无法删除有活跃订阅的套餐**
```gherkin
Given 我是 realm-1 的管理员
And 已存在套餐 "basic" 且有 10 个活跃订阅
And 该套餐属于 Product "ai-services"
When 我尝试删除该套餐
Then 系统显示错误消息："Cannot delete plan with active subscriptions"
And 显示活跃订阅数量："10 active subscriptions"
And 套餐删除失败
```

**场景 3：删除已取消订阅的套餐**
```gherkin
Given 我是 realm-1 的管理员
And 已存在套餐 "old-plan"
And 该套餐的所有订阅都已取消（无 active 状态订阅）
And 该套餐属于 Product "legacy-product"
When 我删除该套餐
Then 套餐删除成功
And 该套餐的所有支付平台映射也一并删除
```


## 故事 5：分配套餐到 Client App [US-BI-005]

**优先级**: P0

**【用户故事】**
**作为**：Realm Admin
**我希望**：将套餐分配到 Client App，以便控制哪些应用可以提供哪些订阅
**从而**：实现不同应用的差异化套餐策略

**【验收标准】**

**场景 1：分配套餐到单个 Client App**
```gherkin
Given 我是 realm-1 的管理员
And 已存在套餐 "basic"
And 该套餐属于 Product "ai-services"
And 该套餐已配置 Stripe 支付平台映射
And 已存在 Client App "mobile-app"
When 我在套餐列表中点击 "Assign" 按钮
And 我选择 "mobile-app"
And 我保存分配
Then 套餐分配成功
And 系统显示成功消息："Plan assigned to mobile-app"
And "mobile-app" 的用户可以看到 "basic" 套餐
And 用户可以选择使用 Stripe 支付
```

**场景 2：分配套餐到多个 Client App**
```gherkin
Given 我是 realm-1 的管理员
And 已存在套餐 "pro"
And 该套餐属于 Product "ai-services"
And 该套餐已配置多个支付平台映射
And 已存在多个 Client App：
  | mobile-app  |
  | web-app     |
  | desktop-app |
When 我分配套餐到所有 Client App
Then 套餐分配成功
And 所有三个应用的用户都可以看到 "pro" 套餐
And 所有应用的用户都可以选择该套餐支持的支付平台
```

**场景 3：查看套餐分配状态**
```gherkin
Given 我是 realm-1 的管理员
And 套餐 "pro" 属于 Product "ai-services"
When 我查看套餐 "pro" 的详情
Then 我看到该套餐的分配状态：
  | Assigned Apps   | Unassigned Apps |
  | mobile-app      | admin-portal    |
  | web-app         | api-service     |
And 我可以看到哪些应用已分配，哪些未分配
And 我可以看到该套餐所属的 Product 为 "ai-services"
And 我可以看到该套餐支持的支付平台列表
```

**场景 4：移除套餐分配**
```gherkin
Given 我是 realm-1 的管理员
And 套餐 "basic" 已分配给 "mobile-app"
And 套餐 "basic" 属于 Product "ai-services"
When 我移除该分配
Then 分配移除成功
And "mobile-app" 的新用户无法看到 "basic" 套餐
And 已订阅用户不受影响
```


## 故事 6：查看订阅列表 [US-BI-006]

**优先级**: P0

**【用户故事】**
**作为**：Realm Admin
**我希望**：查看订阅列表，以便了解订阅情况
**从而**：监控业务收入和用户订阅状态

**【验收标准】**

**场景 1：查看所有订阅**
```gherkin
Given 我是 realm-1 的管理员
When 我访问订阅管理页面
Then 我看到订阅列表表格
And 表格包含以下列：
  | 列名             | 说明                   |
  | User             | 用户邮箱               |
  | Product          | 产品名称               |
  | Plan             | 套餐名称               |
  | Payment Provider | 支付平台               |
  | Status           | 订阅状态               |
  | Billing Period   | 计费周期               |
  | Started At       | 订阅开始时间           |
  | Next Billing At  | 下次计费时间           |
And 列表显示所有订阅（active, canceled, past_due 等）
And 每个订阅显示其使用的支付平台
```

**场景 2：筛选订阅状态**
```gherkin
Given 我在订阅管理页面
When 我选择状态筛选为 "active"
Then 列表只显示活跃订阅
When 我选择状态筛选为 "canceled"
Then 列表只显示已取消订阅
```

**场景 3：筛选支付平台**
```gherkin
Given 我在订阅管理页面
When 我选择支付平台筛选为 "Stripe"
Then 列表只显示通过 Stripe 支付的订阅
```

**场景 4：查看单个订阅详情**
```gherkin
Given 我在订阅列表
When 我点击某个订阅
Then 我看到订阅详情页面
And 页面显示：
  | 字段                | 内容                       |
  | Subscription ID     | sub_1234567890            |
  | User                | user@example.com          |
  | Product             | AI 服务                    |
  | Plan                | Pro (monthly)             |
  | Payment Provider    | Stripe                    |
  | Status              | Active                    |
  | Current Period Start| 2025-01-15                |
  | Current Period End  | 2025-02-15                |
  | Cancel At Period End| No                        |
```


## 故事 7：第三方应用查询套餐状态（SDK 集成） [US-BI-007]

**优先级**: P0

**【用户故事】**
**作为**：Third-party App（详见 [docs/user-stories/_roles.md](/docs/user-stories/_roles.md)）
**我希望**：通过 SDK 查询用户的订阅和套餐状态，以及可用的支付平台选项
**从而**：根据订阅状态控制应用功能访问，并为用户提供支付平台选择

**【验收标准】**

**场景 1：查询订阅详情**
```gherkin
Given 我是第三方应用开发者
And 用户已登录我的应用
When 我通过 SDK 调用查询订阅详情接口
Then 我收到订阅详情，包括：
  | id                     | sub_1234567890            |
  | status                 | active                    |
  | product                | AI 服务                    |
  | plan                   | Pro (monthly)             |
  | payment_provider       | stripe                   |
  | current_period_start    | 2025-01-15               |
  | current_period_end      | 2025-02-15               |
And 我可以根据 status 和 plan 控制功能访问
And 我可以识别使用的支付平台
```

**场景 2：查询可用套餐列表（包含支付平台信息）**
```gherkin
Given 我是第三方应用开发者
When 我通过 SDK 调用查询套餐列表接口
Then 我收到该 Realm 的所有可用套餐
And 每个套餐包含可用的支付平台列表：
  | product     | name  | title    | price | currency | payment_providers      |
  | AI 服务     | basic | 基础版   | 1000  | USD      | [stripe]                |
  | AI 服务     | pro   | 专业版   | 5000  | USD      | [stripe, creem]         |
And 我可以在应用中展示套餐选项
And 我可以看到每个套餐支持的多个支付平台
```

**场景 3：查询套餐分配**
```gherkin
Given 我是第三方应用开发者
When 我通过 SDK 调用查询套餐分配接口
Then 我收到分配给该 Client App 的所有套餐
And 每个套餐包含其支持的支付平台列表
And 我可以只展示已分配的套餐给用户
```

**场景 4：根据订阅状态控制功能**
```gherkin
Given 我是第三方应用开发者
And 我查询到用户的订阅状态为 "active"
And 套餐为 "pro"
And 支付平台为 "stripe"
When 用户访问高级功能
Then 我允许用户访问
When 用户访问高级功能
And 订阅状态为 "canceled" 或 "past_due"
Then 我拒绝访问并提示续费
```

**场景 5：展示支付平台选择**
```gherkin
Given 我是第三方应用开发者
And 我查询到套餐 "pro" 支持多个支付平台
And 支付平台列表为 [stripe, creem]
When 用户选择订阅该套餐
Then 我在支付页面展示支付平台选择：
  | Stripe   | 推荐使用    |
  | Creem    | 测试平台    |
And 用户可以选择其中一个支付平台进行支付
```

**场景 6：根据支付平台状态过滤套餐**
```gherkin
Given 我是第三方应用开发者
And 我查询到套餐 "basic" 支持的支付平台为 [stripe]
And Stripe 的配置已禁用
When 新用户尝试查看可用套餐
Then 套餐 "basic" 仍然显示
And 套餐显示为不可用（无可用支付平台）
And 我可以提示用户："Stripe 支付暂时不可用"
```

**场景 7：处理套餐无可用支付平台的情况**
```gherkin
Given 我是第三方应用开发者
And 我查询到套餐 "legacy" 支持的支付平台为 [stripe]
And Stripe 的配置已禁用
When 新用户尝试查看可用套餐
Then 套餐 "legacy" 仍然显示
And 支付平台选择显示为空
And 我显示提示信息："该套餐暂无可用支付平台，请稍后再试"
And 我禁用该套餐的订阅按钮
```


## 故事 8：查看订阅变更历史 [US-BI-008]

**优先级**: P1

**【用户故事】**
**作为**：Realm Admin
**我希望**：查看所有用户的订阅变更历史，以便监控和管理订阅情况
**从而**：了解订阅的变更轨迹和趋势

**【验收标准】**

**场景 1：查看全局订阅变更历史**
```gherkin
Given 我是 realm-1 的管理员
When 我访问订阅变更历史页面
Then 我看到订阅变更历史列表
And 列表包含以下列：
  | 列名             | 说明                   |
  | Timestamp        | 变更时间               |
  | Event Type       | 变更类型               |
  | User             | 用户邮箱               |
  | Subscription     | 订阅 ID                |
  | Plan             | 套餐名称               |
  | Changes          | 变更摘要               |
And 列表按时间倒序排列
```

**场景 2：按变更类型筛选**
```gherkin
Given 我在订阅变更历史页面
When 我选择变更类型筛选为 "upgraded"
Then 列表只显示升级事件
When 我选择变更类型筛选为 "canceled"
Then 列表只显示取消事件
```

**场景 3：按用户筛选**
```gherkin
Given 我在订阅变更历史页面
When 我输入用户邮箱 "user@example.com"
Then 列表只显示该用户的变更历史
```

**场景 4：按套餐筛选**
```gherkin
Given 我在订阅变更历史页面
When 我选择套餐 "pro"
Then 列表只显示该套餐相关的变更历史
```

**场景 5：按时间段筛选**
```gherkin
Given 我在订阅变更历史页面
When 我设置时间范围为 "2025-01-01" 到 "2025-01-31"
Then 列表只显示该时间段内的变更历史
```

**场景 6：查看变更详情**
```gherkin
Given 我在订阅变更历史列表
When 我点击某条变更记录
Then 我看到变更详情对话框
And 对话框显示：
  | 字段              | 内容                       |
  | Event ID         | evt_1234567890            |
  | Event Type       | Upgraded                   |
  | Timestamp        | 2025-01-20 10:30:00 UTC   |
  | Actor            | user@example.com          |
  | Changes          | Plan: basic → pro         |
  | Previous State   | { "status": "active", "plan": "basic" } |
  | New State        | { "status": "active", "plan": "pro" } |
```


## 故事 9：查看自己的订阅变更历史 [US-BI-009]

**优先级**: P1

**【用户故事】**
**作为**：Regular User
**我希望**：查看我的订阅变更历史，以便了解订阅的变更轨迹
**从而**：追踪我的订阅状态变化

**【验收标准】**

**场景 1：查看订阅历史时间线**
```gherkin
Given 我是 regular-user-1
And 我有活跃订阅 "sub_1234567890"
When 我访问我的订阅详情页面
And 我点击 "History" 标签
Then 我看到订阅历史时间线
And 时间线按时间倒序排列
```

**场景 2：时间线显示完整变更记录**
```gherkin
Given 我在订阅历史时间线页面
Then 我看到以下变更事件（按时间倒序）：
  | 时间              | 类型      | 变更描述                    |
  | 2025-01-20 10:30  | Upgraded  | 套餐: basic → pro         |
  | 2025-01-15 08:00  | Created   | 订阅创建成功（basic 套餐） |
And 每个事件显示操作者
And 升级事件显示变更前后对比
```

**场景 3：显示变更类型标签**
```gherkin
Given 我在订阅历史时间线页面
Then 我看到不同的变更类型有不同颜色的标签：
  | 类型        | 颜色   |
  | Created     | 绿色   |
  | Upgraded    | 蓝色   |
  | Downgraded  | 橙色   |
  | Canceled    | 红色   |
  | Renewed     | 绿色   |
```

**场景 4：查看变更详情**
```gherkin
Given 我在订阅历史时间线页面
When 我点击某个变更事件
Then 我看到该事件的详细信息
And 信息包括：事件类型、时间、操作者、变更详情、变更前后状态
```

**场景 5：无历史记录的订阅**
```gherkin
Given 我是 regular-user-1
And 我有订阅 "sub_9876543210"
And 该订阅是刚创建的，没有变更记录
When 我访问该订阅的历史页面
Then 我看到提示信息："No history available for this subscription"
```

**场景 6：权限隔离**
```gherkin
Given 我是 regular-user-1
When 我尝试访问用户 regular-user-2 的订阅历史
Then 我收到权限错误："Access denied"
And 我无法查看其他用户的历史记录
```


## 用户故事优先级汇总

| 优先级 | 用户故事数量 | 关键故事 |
|--------|------------|---------|
| P0 | 7 | 创建/编辑订阅套餐、配置支付平台映射、删除套餐、分配套餐到 Client App、查看订阅列表、第三方应用查询套餐状态（SDK） |
| P1 | 2 | 查看订阅变更历史（Realm Admin）、查看自己的订阅变更历史（Regular User） |
| P2 | 0 | - |

---

## 相关文档

- **PRD**: `docs/prd/billing/billing.md` - Billing 订阅计费产品需求文档
- **PRD**: `docs/prd/billing/subscription-history.md` - Subscription History 订阅变更历史产品需求文档
- **SDK 文档**: `backend/sdk/src/lib.rs` - Rust SDK 源码
- **技术设计**: `.ai/design/fix-permission-and-sdk-impl.md` - SDK API 扩展技术设计
