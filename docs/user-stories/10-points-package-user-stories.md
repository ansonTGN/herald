# 积分包（PointsPackage）用户故事

**角色代码**: PP (Points Package)
**角色定义**：Realm Admin 负责管理积分包商品；Regular User 购买积分包获得充值积分（topup_credit）。

**故事范围**: US-PP-001 ~ US-PP-005
**创建时间**: 2026-04-08
**状态**: Active

**依赖关系**:
- US-PP-001 无前置依赖，可优先实现
- US-PP-002 和 US-PP-003 依赖 US-PP-001（需要先创建积分包）
- US-PP-004 依赖 US-PP-001（需要存在积分包才能查看）
- US-PP-005 无前置依赖，但建议在 US-PP-001 之后实现
- 依赖 Product/Plan 编目系统（docs/user-stories/product-management.md）
- 依赖支付平台配置（docs/user-stories/07-payment-provider-user-stories.md）

---

## 故事 1：创建积分包 [US-PP-001]

**优先级**: P0

**【用户故事】**
**作为**：Realm Admin（详见 [docs/user-stories/_roles.md](/docs/user-stories/_roles.md)）
**我希望**：创建积分包（PointsPackage）商品
**从而**：用户可以购买积分包获得充值积分（topup_credit），而不是订阅

**【验收标准】**

**场景 1：创建新的积分包**
```gherkin
Given 我是 realm-1 的管理员
When 我在积分包管理页面点击 "Create Package" 按钮
And 我填写积分包信息：
  | Name        | credits-500          |
  | Title       | 500 积分包            |
  | Description | 适合个人用户的基础积分包 |
  | Points Amount | 500                 |
  | Price       | 5000                 |
  | Currency    | CNY                  |
  | Sort Order  | 1                    |
And 我提交表单
Then 积分包创建成功
And 系统显示成功消息："Points package 'credits-500' created successfully"
And 积分包列表显示新创建的积分包
And 系统提示我需要为该积分包配置支付平台映射
And 积分包的 Payment Providers 列显示 "Not configured"
```

**场景 2：积分包名称唯一性验证**
```gherkin
Given 我是 realm-1 的管理员
And 已存在积分包 "credits-500"
When 我尝试创建同名积分包 "credits-500"
Then 系统显示验证错误："Points package name 'credits-500' already exists in this realm"
And 积分包创建失败
```

**场景 3：积分数必须为正数**
```gherkin
Given 我是 realm-1 的管理员
When 我创建积分包
And 我设置 Points Amount 为 "0" 或负数
Then 系统显示验证错误："Points amount must be greater than 0"
```

**场景 4：价格必须为正数**
```gherkin
Given 我是 realm-1 的管理员
When 我创建积分包
And 我设置 Price 为 "0" 或负数
Then 系统显示验证错误："Price must be greater than 0"
```

---

## 故事 2：编辑积分包 [US-PP-002]

**优先级**: P0

**【用户故事】**
**作为**：Realm Admin
**我希望**：编辑积分包信息
**从而**：更新积分包的价格和描述

**【验收标准】**

**场景 1：编辑积分包基本信息**
```gherkin
Given 我是 realm-1 的管理员
And 已存在积分包 "credits-500"
When 我在积分包列表中点击 "Edit" 按钮
And 我修改以下信息：
  | Title       | 500 积分包（已更新）   |
  | Description | 适合新用户的入门积分包 |
  | Price       | 4800                   |
And 我保存更改
Then 积分包更新成功
And 系统显示成功消息："Points package updated successfully"
And 积分包列表显示更新后的信息
```

**场景 2：积分包名称不可修改**
```gherkin
Given 我是 realm-1 的管理员
And 已存在积分包 "credits-500"
When 我编辑该积分包
Then "Name" 字段为只读或禁用
And 我无法修改积分包名称
```

**场景 3：启用/禁用积分包**
```gherkin
Given 我是 realm-1 的管理员
And 已存在积分包 "credits-500" 且状态为 "enabled"
When 我切换积分包的 "Status" 开关为 "disabled"
And 我保存更改
Then 积分包状态更新为 "disabled"
And 新用户无法看到该积分包
And 已购买的用户不受影响
And 该积分包下的所有支付平台映射也一并禁用
```

---

## 故事 3：配置积分包的支付平台映射 [US-PP-003]

**优先级**: P0

**【用户故事】**
**作为**：Realm Admin
**我希望**：为积分包配置一个或多个支付平台映射
**从而**：用户可以在不同支付平台上购买积分包

**【验收标准】**

**场景 1：为积分包添加支付平台映射**
```gherkin
Given 我是 realm-1 的管理员
And 已存在积分包 "credits-500"
And 已配置微信支付平台
When 我在积分包详情页面点击 "Add Payment Provider" 按钮
And 我选择支付平台为 "WeChat Pay"
And 我填写外部商品信息：
  | External Package ID | pkg_credits_500 |
And 我保存配置
Then 支付平台映射添加成功
And 系统显示成功消息："Payment provider 'WeChat Pay' added to package 'credits-500'"
And 积分包详情显示该支付平台映射
```

**场景 2：为同一积分包添加多个支付平台**
```gherkin
Given 我是 realm-1 的管理员
And 已存在积分包 "credits-1000"
And 已配置微信支付和 Stripe 支付平台
When 我为积分包 "credits-1000" 添加微信支付映射
And 我为积分包 "credits-1000" 添加 Stripe 支付映射
Then 所有支付平台映射都成功添加
And 积分包 "credits-1000" 可以在两个支付平台上购买
And 积分包列表显示 "credits-1000" 支持多个支付平台
```

**场景 3：验证支付平台唯一性**
```gherkin
Given 我是 realm-1 的管理员
And 积分包 "credits-500" 已配置微信支付映射
When 我尝试为同一积分包再次添加微信支付映射
Then 系统显示验证错误："Payment provider 'WeChat Pay' already configured for this package"
And 支付平台映射添加失败
```

**场景 4：启用/禁用支付平台映射**
```gherkin
Given 我是 realm-1 的管理员
And 积分包 "credits-500" 已配置微信支付映射
And 微信支付映射状态为 "enabled"
When 我禁用微信支付映射
Then 微信支付映射状态更新为 "disabled"
And 新用户无法使用微信支付购买该积分包
And 已购买的用户不受影响
```

---

## 故事 4：查看积分包列表 [US-PP-004]

**优先级**: P0

**【用户故事】**
**作为**：Realm Admin
**我希望**：查看所有积分包列表
**从而**：管理积分包商品

**【验收标准】**

**场景 1：查看所有积分包**
```gherkin
Given 我是 realm-1 的管理员
When 我访问积分包管理页面
Then 我看到积分包列表按 sort_order 排序
And 每个积分包显示：
  | Name        | credits-500    |
  | Title       | 500 积分包      |
  | Points      | 500            |
  | Price       | ¥50.00         |
  | Status      | Enabled        |
  | Providers   | WeChat, Stripe |
  | Sort Order  | 1              |
```

**场景 2：按状态筛选积分包**
```gherkin
Given 我是 realm-1 的管理员
And 存在多个积分包，状态包括 enabled 和 disabled
When 我选择状态筛选 "Enabled"
Then 列表仅显示 enabled 状态的积分包
When 我选择状态筛选 "All"
Then 列表显示所有积分包
```

**场景 3：查看积分包下的支付平台映射**
```gherkin
Given 我是 realm-1 的管理员
And 积分包 "credits-1000" 配置了多个支付平台
When 我点击该积分包的展开按钮
Then 我看到该积分包的所有支付平台映射：
  | Provider | External Package ID | Status |
  | WeChat   | pkg_credits_1000_wp | enabled|
  | Stripe   | pkg_credits_1000_st | enabled|
```

---

## 故事 5：删除积分包 [US-PP-005]

**优先级**: P1

**【用户故事】**
**作为**：Realm Admin
**我希望**：删除不再需要的积分包
**从而**：保持积分包目录整洁

**【验收标准】**

**场景 1：删除无购买记录的积分包**
```gherkin
Given 我是 realm-1 的管理员
And 已存在积分包 "legacy-package"
And 该积分包没有购买记录
When 我在积分包列表中点击 "Delete" 按钮
And 我确认删除
Then 积分包删除成功
And 系统显示成功消息："Points package deleted successfully"
And 积分包列表不再显示该积分包
And 该积分包的所有支付平台映射也一并删除
```

**场景 2：无法删除有购买记录的积分包**
```gherkin
Given 我是 realm-1 的管理员
And 已存在积分包 "credits-500"
And 该积分包有 10 个购买记录
When 我尝试删除该积分包
Then 系统显示错误消息："Cannot delete points package with existing purchases"
And 显示购买记录数量："10 purchases"
And 积分包删除失败
```

**场景 3：删除前确认**
```gherkin
Given 我是 realm-1 的管理员
And 我点击删除积分包按钮
Then 系统显示确认对话框
And 对话框显示警告信息："This will permanently delete the points package. Are you sure?"
And 我必须点击 "Confirm" 才能执行删除
```

---

## 用户故事优先级汇总

| 优先级 | 用户故事数量 | 关键故事 |
|--------|------------|---------|
| P0 | 4 | 创建积分包、编辑积分包、配置支付平台映射、查看积分包列表 |
| P1 | 1 | 删除积分包 |
| P2 | 0 | - |

---

## 与订阅套餐的区别

| 特性 | 订阅套餐（Plan） | 积分包（PointsPackage） |
|------|-----------------|----------------------|
| 购买对象 | 订阅权益 | 充值积分（topup_credit） |
| 履约结果 | 创建 Subscription + 发放 subscription_credit | 仅发放 topup_credit |
| 是否续费 | 是（定期） | 否（一次性） |
| 积分类型 | subscription_credit | topup_credit |
| 积分过期 | 有（随订阅周期） | 无（长期有效） |

---

## 相关文档

- **PRD**: `docs/prd/billing/unified-purchase.md` - 统一购买架构产品需求文档（待创建）
- **PRD**: `docs/prd/billing/points.md` - 积分系统产品需求文档
- **用户故事**: `points-admin-manage.md` - 积分管理用户故事
- **用户故事**: `points-user-view.md` - 积分查询用户故事
- **技术方案**: `.ai/future/order.md` - 统一购买架构技术方案
