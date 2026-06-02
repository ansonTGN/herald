# Realm Admin / Regular User 用户故事

> 角色定义见 [docs/user-stories/_roles.md](/docs/user-stories/_roles.md)

## 用户故事

### 故事 1：创建积分包 [US-PP-001]

**优先级**: P0

**【用户故事】**
**作为**：Realm Admin（详见 [docs/user-stories/_roles.md](/docs/user-stories/_roles.md)）
**我希望**：创建积分包商品
**从而**：用户可以购买积分包获得充值积分，而不是订阅

**【验收标准】**

> 验收标准只描述用户动作与可见结果，不写 API 路径、数据表、字段变更、技术实现步骤。

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

### 故事 2：编辑积分包 [US-PP-002]

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

### 故事 3：配置积分包的支付平台映射 [US-PP-003]

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

### 故事 4：查看积分包列表 [US-PP-004]

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

### 故事 5：删除积分包 [US-PP-005]

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

### 故事 6：创建促销积分包 [US-PP-006]

**优先级**: P0

**【用户故事】**
**作为**：Realm Admin（详见 [docs/user-stories/_roles.md](/docs/user-stories/_roles.md)）
**我希望**：创建带折扣价的促销积分包
**从而**：以限时优惠吸引用户购买积分

**【验收标准】**

> 验收标准只描述用户动作与可见结果，不写 API 路径、数据表、字段变更、技术实现步骤。

**场景 1：创建促销积分包**
```gherkin
Given 我是 realm-1 的管理员
When 我在积分包管理页面点击 "Create Package" 按钮
And 我选择包类型为 "促销"
And 我填写积分包信息：
  | Name        | promo-summer-1000 |
  | Title       | 夏日特惠 1000 积分 |
  | Points Amount | 1000           |
  | 售价        | ¥50.00            |
  | 原价（划线价）| ¥100.00          |
  | 促销开始时间 | 2026-06-01 00:00  |
  | 促销结束时间 | 2026-06-30 23:59  |
And 我提交表单
Then 积分包创建成功
And 系统显示成功消息
And 积分包列表显示该包为"促销"类型
And 列表显示折扣信息 "-50%"
```

**场景 2：原价必须大于售价**
```gherkin
Given 我是 realm-1 的管理员
When 我创建促销积分包
And 我设置原价为 ¥50.00，售价为 ¥50.00（原价不大于售价）
And 我提交表单
Then 系统显示验证错误："原价必须大于售价"
And 积分包创建失败
```

**场景 3：促销包不设原价（仅显示角标）**
```gherkin
Given 我是 realm-1 的管理员
When 我创建促销积分包
And 我不填写原价（仅设置促销时段）
And 我提交表单
Then 积分包创建成功
And 前端不显示划线价，仅显示"促销"角标
```

**场景 4：常驻包不允许设置原价**
```gherkin
Given 我是 realm-1 的管理员
When 我创建积分包并选择包类型为 "常驻"
And 我尝试填写原价
Then 系统显示验证错误："常驻包不能设置原价"
```

**场景 5：促销结束时间早于当前时间**
```gherkin
Given 我是 realm-1 的管理员
When 我创建促销积分包
And 我设置促销结束时间为过去的时间
And 我提交表单
Then 系统接受创建但标注该包为"已过期"
And 管理员列表显示"已过期"标签
```

---

### 故事 7：编辑促销积分包 [US-PP-016]

**优先级**: P0

**【用户故事】**
**作为**：Realm Admin（详见 [docs/user-stories/_roles.md](/docs/user-stories/_roles.md)）
**我希望**：修改促销包的折扣价、原价和促销时段
**从而**：灵活调整促销策略

**【验收标准】**

> 验收标准只描述用户动作与可见结果，不写 API 路径、数据表、字段变更、技术实现步骤。

**场景 1：修改促销包折扣价和原价**
```gherkin
Given 我是 realm-1 的管理员
And 已存在促销积分包 "promo-summer-1000"（原价 ¥100，售价 ¥50）
When 我修改售价为 ¥40，原价为 ¥100
And 我保存更改
Then 积分包更新成功
And 前端立即显示更新后的折扣信息 "-60%"
```

**场景 2：促销包转常驻包**
```gherkin
Given 我是 realm-1 的管理员
And 已存在促销积分包 "promo-summer-1000"
When 我将包类型从 "促销" 改为 "常驻"
And 我保存更改
Then 系统自动清除原价和促销时段
And 积分包变为常驻包，前端不再显示折扣信息
```

**场景 3：常驻包转促销包**
```gherkin
Given 我是 realm-1 的管理员
And 已存在常驻积分包 "credits-500"
When 我将包类型从 "常驻" 改为 "促销"
Then 系统要求我填写原价或促销时段
And 我填写原价和促销结束时间后保存
Then 积分包变为促销包
And 前端显示折扣信息
```

**场景 4：修改促销时段**
```gherkin
Given 我是 realm-1 的管理员
And 已存在促销积分包 "promo-summer-1000"（促销结束时间 2026-06-30）
When 我修改促销结束时间为 2026-07-31
And 我保存更改
Then 积分包更新成功
And 促销时长延长
```

---

### 故事 8：用户查看促销积分包 [US-PP-017]

**优先级**: P0

**【用户故事】**
**作为**：Regular User（详见 [docs/user-stories/_roles.md](/docs/user-stories/_roles.md)）
**我希望**：在购买页面看到带有折扣标识的促销包
**从而**：快速识别优惠商品，享受更优惠的价格

**【验收标准】**

> 验收标准只描述用户动作与可见结果，不写 API 路径、数据表、字段变更、技术实现步骤。

**场景 1：促销包显示折扣角标和划线价**
```gherkin
Given 我是已登录用户
And 当前 Realm 有促销积分包 "夏日特惠"（原价 ¥100，售价 ¥50）
When 我访问积分购买页面
Then 我看到 "夏日特惠" 积分包卡片显示折扣角标 "-50%"
And 卡片显示划线价 ¥100.00 和实际售价 ¥50.00
```

**场景 2：促销包仅显示角标（无原价）**
```gherkin
Given 我是已登录用户
And 当前 Realm 有促销积分包（未设置原价）
When 我访问积分购买页面
Then 我看到该积分包卡片显示"促销"角标
And 卡片不显示划线价，只显示售价
```

**场景 3：已过期促销包不在用户购买页显示**
```gherkin
Given 我是已登录用户
And 当前 Realm 有促销积分包 "夏日特惠"
And 该促销包已过期（促销结束时间已过）
When 我访问积分购买页面
Then 我看不到 "夏日特惠" 积分包
```

**场景 4：未开始的促销包不在用户购买页显示**
```gherkin
Given 我是已登录用户
And 当前 Realm 有促销积分包 "国庆特惠"
And 该促销包尚未开始（促销开始时间未到）
When 我访问积分购买页面
Then 我看不到 "国庆特惠" 积分包
```

**场景 5：促销包排序优先**
```gherkin
Given 我是已登录用户
And 当前 Realm 有多个积分包（含促销包和常驻包）
When 我访问积分购买页面
Then 有效的促销包排在常驻包前面
And 常驻包按原有排序规则排列
```

**场景 6：购买促销包流程与常驻包一致**
```gherkin
Given 我是已登录用户
And 当前 Realm 有促销积分包 "夏日特惠"（售价 ¥50）
When 我选择购买 "夏日特惠" 积分包
And 我完成支付
Then 系统按售价 ¥50 扣费
And 系统发放对应积分数到我的账户
And 购买流程与常驻积分包完全一致
```

**场景 7：促销包显示限时标签**
```gherkin
Given 我是已登录用户
And 当前 Realm 有促销积分包 "夏日特惠"
And 该促销包设置了结束时间
When 我访问积分购买页面
Then 我看到 "夏日特惠" 卡片显示"限时"标签
And 标签显示剩余时间或结束日期
```

---

### 故事 9：促销包自动过期 [US-PP-018]

**优先级**: P1

**【用户故事】**
**作为**：Realm Admin（详见 [docs/user-stories/_roles.md](/docs/user-stories/_roles.md)）
**我希望**：促销包到期后自动在用户侧隐藏
**从而**：无需手动下架过期促销

**【验收标准】**

> 验收标准只描述用户动作与可见结果，不写 API 路径、数据表、字段变更、技术实现步骤。

**场景 1：促销包到期自动隐藏**
```gherkin
Given 当前 Realm 有促销积分包 "夏日特惠"
And 促销结束时间为 2026-06-30 23:59
When 当前时间超过 2026-06-30 23:59
And 普通用户访问积分购买页面
Then 该促销包不再显示在购买列表中
```

**场景 2：管理员查看已过期促销包**
```gherkin
Given 我是 realm-1 的管理员
And 有促销包已过期
When 我访问积分包管理页面
Then 我仍然可以看到已过期的促销包
And 该包显示"已过期"标签
And 我可以编辑该包延长促销时间
```

**场景 3：延长促销时间使过期包重新可见**
```gherkin
Given 我是 realm-1 的管理员
And 有促销包已过期（标注"已过期"）
When 我修改促销结束时间为未来日期
And 我保存更改
Then 该促销包不再显示"已过期"标签
And 普通用户可以再次看到该促销包
```

**场景 4：促销包的启用/禁用独立于过期状态**
```gherkin
Given 我是 realm-1 的管理员
And 有促销包已过期
When 我查看该包的启用/禁用状态
Then 该包的启用/禁用状态未改变
And 过期仅影响用户侧可见性，不影响启用/禁用状态
```

---

## 用户故事优先级汇总

| 优先级 | 用户故事数量 | 关键故事 |
|--------|------------|---------|
| P0 | 7 | 创建积分包、编辑积分包、配置支付平台映射、查看积分包列表、创建促销积分包、编辑促销积分包、用户查看促销积分包 |
| P1 | 2 | 删除积分包、促销包自动过期 |
| P2 | 0 | - |

---

## 相关文档

- **PRD**: `docs/prd/billing/unified-purchase.md` - 统一购买架构产品需求文档
- **PRD**: `docs/prd/billing/points.md` - 积分系统产品需求文档
- **用户故事**: `points-admin-manage.md` - 积分管理用户故事
- **用户故事**: `points-user-view.md` - 积分查询用户故事
