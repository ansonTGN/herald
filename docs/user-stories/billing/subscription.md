# Realm Admin 用户故事

> 角色定义见 [docs/user-stories/_roles.md](/docs/user-stories/_roles.md)

## 用户故事

> [US-BI-001 ~ US-BI-005]（创建/编辑/删除订阅套餐、配置支付平台映射、分配套餐到 Client App）已由 Entitlement Mapping 替代，见 [entitlement-mapping.md](entitlement-mapping.md)。

### 故事 6：查看订阅列表 [US-BI-006]

**优先级**: P0

**【用户故事】**
**作为**：Realm Admin（详见 [docs/user-stories/_roles.md](/docs/user-stories/_roles.md)）
**我希望**：查看订阅投影列表，以便了解订阅情况
**从而**：监控业务收入和用户订阅状态

**【验收标准】**

> 验收标准只描述用户动作与可见结果，不写 API 路径、数据表、字段变更、技术实现步骤。

**场景 1：查看所有订阅**
```gherkin
Given 我是 realm-1 的管理员
When 我访问订阅管理页面
Then 我看到订阅投影列表表格
And 表格包含以下列：
  | 列名             | 说明                   |
  | User             | 用户邮箱               |
  | Entitlement Key  | 权益标识               |
  | Payment Provider | 支付平台               |
  | Status           | 订阅状态               |
  | Current Period   | 当前计费周期           |
  | Synced At        | 最后同步时间           |
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

**场景 4：按 Entitlement Key 筛选**
```gherkin
Given 我在订阅管理页面
When 我选择 Entitlement Key 筛选为 "pro-plan"
Then 列表只显示 entitlement_key 为 "pro-plan" 的订阅
```

**场景 5：查看单个订阅详情**
```gherkin
Given 我在订阅列表
When 我点击某个订阅
Then 我看到订阅详情页面
And 页面显示：
  | 字段                | 内容                       |
  | Subscription ID     | sub_1234567890            |
  | User                | user@example.com          |
  | Entitlement Key     | pro-plan                  |
  | Payment Provider    | Stripe                    |
  | Status              | Active                    |
  | Current Period Start| 2025-01-15                |
  | Current Period End  | 2025-02-15                |
  | Cancel At Period End| No                        |
  | Synced At           | 2025-01-15 12:00 UTC      |
```


### 故事 7：第三方应用查询订阅状态（SDK 集成） [US-BI-007]

**优先级**: P0

**【用户故事】**
**作为**：Third-party App（详见 [docs/user-stories/_roles.md](/docs/user-stories/_roles.md)）
**我希望**：通过 SDK 查询用户的订阅状态
**从而**：根据 entitlement_key 做出访问控制决策

**【验收标准】**

> 验收标准只描述用户动作与可见结果，不写 API 路径、数据表、字段变更、技术实现步骤。

**场景 1：查询用户活跃订阅**
```gherkin
Given 我是第三方应用开发者
And 用户已登录我的应用
When 我通过 SDK 查询用户订阅状态
Then 我收到订阅详情，包括：
  | Has Subscription | Yes         |
  | Status           | Active      |
  | Has Access       | Yes         |
  | Entitlement Key  | pro-plan    |
  | Payment Provider | Stripe      |
```

**场景 2：用户无订阅**
```gherkin
Given 我是第三方应用开发者
And 用户 user-2 没有订阅
When 我通过 SDK 查询 user-2 的订阅状态
Then 返回结果显示：
  | Has Subscription | No  |
  | Has Access       | No  |
```

**场景 3：查询性能不依赖 Provider API**
```gherkin
Given 第三方应用查询用户订阅状态
And 当前 Stripe API 响应缓慢或不可用
When SDK 返回订阅查询结果
Then 结果来自 Herald 本地订阅投影
And 查询速度不受 Stripe API 影响
```

**场景 4：根据订阅状态控制功能**
```gherkin
Given 我是第三方应用开发者
And 我查询到用户的订阅状态为 "active"
And entitlement_key 为 "pro-plan"
When 用户访问高级功能
Then 我允许用户访问
When 订阅状态为 "canceled" 或 "past_due"
Then 我拒绝访问并提示续费
```


### 故事 8：查看订阅变更历史 [US-BI-008]

**优先级**: P1

**【用户故事】**
**作为**：Realm Admin（详见 [docs/user-stories/_roles.md](/docs/user-stories/_roles.md)）
**我希望**：查看所有用户的订阅变更历史，以便监控和管理订阅情况
**从而**：了解订阅的变更轨迹和趋势

**【验收标准】**

> 验收标准只描述用户动作与可见结果，不写 API 路径、数据表、字段变更、技术实现步骤。

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
  | Entitlement Key  | 权益标识               |
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

**场景 4：按 Entitlement Key 筛选**
```gherkin
Given 我在订阅变更历史页面
When 我选择 entitlement_key 筛选为 "pro-plan"
Then 列表只显示该 entitlement 相关的变更历史
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
  | 字段              | 内容                                    |
  | Event ID         | evt_1234567890                         |
  | Event Type       | Upgraded                                |
  | Timestamp        | 2025-01-20 10:30:00 UTC                |
  | Actor            | user@example.com                       |
  | Previous Entitlement | basic-plan                         |
  | New Entitlement  | pro-plan                                |
  | Previous State   | { "status": "active", "entitlement_key": "basic-plan" } |
  | New State        | { "status": "active", "entitlement_key": "pro-plan" } |
```


### 故事 9：查看自己的订阅变更历史 [US-BI-009]

**优先级**: P1

**【用户故事】**
**作为**：Regular User（详见 [docs/user-stories/_roles.md](/docs/user-stories/_roles.md)）
**我希望**：查看我的订阅变更历史，以便了解订阅的变更轨迹
**从而**：追踪我的订阅状态变化

**【验收标准】**

> 验收标准只描述用户动作与可见结果，不写 API 路径、数据表、字段变更、技术实现步骤。

**场景 0：个人中心 Subscription 入口按活跃 Entitlement 映射显示**
```gherkin
Given 我是 regular-user-1
And 当前 Realm 存在已启用的 Entitlement Mapping
When 我打开个人中心
Then 侧边栏显示 "Subscription" 菜单
When 当前 Realm 不存在已启用的 Entitlement Mapping
Then 侧边栏不显示 "Subscription" 菜单
And 我直接访问订阅历史页面时被引导回个人资料页
```

**场景 1：查看订阅历史时间线**
```gherkin
Given 我是 regular-user-1
And 当前 Realm 存在已启用的 Entitlement Mapping
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
  | 时间              | 类型      | 变更描述                              |
  | 2025-01-20 10:30  | Upgraded  | Entitlement: basic-plan → pro-plan   |
  | 2025-01-15 08:00  | Created   | 订阅创建成功（basic-plan）           |
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
And 我通过 Client App "app-a" 关联了订阅
When 我访问订阅历史
Then 我只能查看自己通过 Client App 关联的订阅历史
And 我无法查看其他用户的订阅历史记录
```


## 用户故事优先级汇总

| 优先级 | 用户故事数量 | 关键故事 |
|--------|------------|---------|
| P0 | 2 | 查看订阅列表、第三方应用查询订阅状态（SDK） |
| P1 | 2 | 查看订阅变更历史（Realm Admin）、查看自己的订阅变更历史（Regular User） |

---

## 相关文档

- **PRD**: [docs/prd/billing/subscription.md](/docs/prd/billing/subscription.md) - Billing 订阅计费产品需求文档
- **PRD**: [docs/prd/billing/product_reduce.md](/docs/prd/billing/product_reduce.md) - Product/Plan 本地模型废弃与 Entitlement 映射
- **用户故事**: [docs/user-stories/billing/entitlement-mapping.md](/docs/user-stories/billing/entitlement-mapping.md) - Entitlement Mapping 用户故事（替代 US-BI-001 ~ US-BI-005）
- **用户故事**: [docs/user-stories/billing/points-admin.md](/docs/user-stories/billing/points-admin.md) - 积分管理用户故事
