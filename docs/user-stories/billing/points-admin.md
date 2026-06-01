# Realm Admin 积分管理用户故事

> 角色定义见 [docs/user-stories/_roles.md](/docs/user-stories/_roles.md)

**边界说明**：当前积分配置仍以 Plan 为主。Product 在当前阶段主要作为 Billing 编目上下文，用于帮助管理员理解某个 Plan 的所属产品线，而不是独立的积分配置层级。

## 用户故事

### 故事 1：配置积分套餐 [US-PO-01]

**优先级**: P0

**【用户故事】**
**作为**：Realm Admin（详见 [docs/user-stories/_roles.md](/docs/user-stories/_roles.md)）
**我希望**：配置每个套餐的积分赠送规则（每期积分、订阅时是否发放）
**从而**：用户订阅后自动获得相应积分

**【验收标准】**

> 验收标准只描述用户动作与可见结果，不写 API 路径、数据表、字段变更、技术实现步骤。

**场景 1：创建积分套餐配置**
```gherkin
Given 我是 realm-1 的管理员
And 已存在订阅套餐 "pro-monthly"
When 我在积分套餐配置页面点击 "Create Config" 按钮
And 我填写配置信息：
  | Plan              | pro-monthly |
  | Points per Period | 1000        |
  | Grant on Subscribe | true       |
  | Grant Period Type | monthly     |
  | Validity Days     | 30          |
  | Max Periods       | 10          |
And 我提交表单
Then 积分套餐配置创建成功
And 系统显示成功消息："Points plan config created successfully"
And 配置列表显示新创建的配置
And 配置列表可显示该 Plan 所属的 Product 上下文
```

**场景 2：查看套餐积分配置**
```gherkin
Given 我是 realm-1 的管理员
And 已存在套餐 "pro-monthly" 的积分配置
When 我在积分套餐配置页面查看该套餐
Then 我看到以下积分配置信息：
  | 字段                    | 值       |
  | 每期发放积分             | 1000     |
  | 订阅时是否发放           | 是       |
  | 发放周期类型             | 月度     |
  | 积分有效期（天）         | 30       |
  | 最大发放期数             | 10       |
And 我可以看到该套餐所属的 Product 名称
```

**场景 3：编辑积分套餐配置**
```gherkin
Given 我是 realm-1 的管理员
And 已存在套餐 "pro-monthly" 的积分配置
When 我编辑该配置：
  | Points per Period  | 1200 |
  | Grant on Subscribe | true |
And 我保存更改
Then 积分套餐配置更新成功
And 新订阅将使用更新后的配置
```

**场景 4：关闭订阅时积分发放**
```gherkin
Given 我是 realm-1 的管理员
And 已存在套餐 "basic-monthly" 的积分配置
And 该套餐已启用订阅时积分发放（Grant on Subscribe = true）
When 我将 "Grant on Subscribe" 设置为 false
And 我保存更改
Then 订阅时不再额外发放积分
And 按周期的定期发放仍正常执行
```

**场景 5：删除积分套餐配置**
```gherkin
Given 我是 realm-1 的管理员
And 已存在套餐 "old-plan" 的积分配置
And 该套餐没有活跃订阅
When 我删除该配置
Then 积分套餐配置删除成功
And 配置列表不再显示该套餐的配置
```


### 故事 2：查看所有用户积分账户 [US-PO-02]

**优先级**: P1

**【用户故事】**
**作为**：Realm Admin（详见 [docs/user-stories/_roles.md](/docs/user-stories/_roles.md)）
**我希望**：查看租户内所有用户的积分账户列表（包括分类余额）
**从而**：了解整体积分使用情况

**【验收标准】**

> 验收标准只描述用户动作与可见结果，不写 API 路径、数据表、字段变更、技术实现步骤。

**场景 1：查看用户积分账户列表（包含分类余额）**
```gherkin
Given 我是 realm-1 的管理员
And realm-1 有 100 个用户
When 我访问积分管理页面
Then 我看到用户积分账户列表
And 列表显示每个用户的积分信息：
  | 用户 ID        | 用户名       | 总余额 | 单位   | 充值积分 | 会员积分 | 累计消耗 | 状态 |
  | user-1         | alice@example.com | 5000 | points | 3000 | 2000 | 5000 | active |
  | user-2         | bob@example.com   | 8000 | points | 6000 | 2000 | 4000 | active |
And 我可以按积分类型排序
And 我可以筛选特定积分类型的用户
```

**场景 2：按用户名或邮箱搜索**
```gherkin
Given 我是 realm-1 的管理员
And realm-1 有多个用户
When 我在搜索框输入 "alice"
Then 我只看到用户名或邮箱包含 "alice" 的用户
And 搜索结果正确
```

**场景 3：查看账户状态**
```gherkin
Given 我是 realm-1 的管理员
And 存在账户状态不同的用户
When 我查看积分账户列表
Then 我可以看到每个账户的状态：
  | user-1 | active  |
  | user-2 | frozen  |
  | user-3 | closed  |
```

**场景 4：分页查看账户列表**
```gherkin
Given 我是 realm-1 的管理员
And realm-1 有 1000 个用户
When 我查看积分账户列表（每页 20 条）
Then 我看到第 1 页的 20 个用户
And 可以翻页查看其他用户
And 页面显示总页数和当前页码
```

**场景 5：查看用户积分账本明细**
```gherkin
Given 我是 realm-1 的管理员
And 已存在用户 "user-1"
And user-1 有多笔积分来源
When 我在积分账户列表中点击 user-1
And 我点击"积分账本"标签
Then 我看到 user-1 的积分账本明细
And 列表显示每笔积分的完整生命周期：
  | 积分类型 | 来源类型 | 发放金额 | 已使用 | 剩余 | 回收金额 | 状态 | 过期时间 |
  | 会员积分 | 订阅续费 | 1000 | 300 | 700 | 0 | active | 2026-04-15 |
  | 充值积分 | 充值购买 | 2000 | 500 | 1500 | 0 | active | 永久有效 |
And 我可以按积分类型筛选
And 我可以按状态筛选
And 我可以按过期时间排序
And 我可以点击查看某笔积分的完整消费分摊记录
```


### 故事 3：查看用户积分交易历史 [US-PO-03]

**优先级**: P1

**【用户故事】**
**作为**：Realm Admin（详见 [docs/user-stories/_roles.md](/docs/user-stories/_roles.md)）
**我希望**：查看任意用户的积分交易历史
**从而**：审计和问题排查

**【验收标准】**

> 验收标准只描述用户动作与可见结果，不写 API 路径、数据表、字段变更、技术实现步骤。

**场景 1：查看用户的所有交易记录（包含新的交易类型）**
```gherkin
Given 我是 realm-1 的管理员
And 已存在用户 "user-1"
And user-1 有 50 条积分交易记录
When 我在积分账户列表中点击 user-1
Then 我看到 user-1 的交易历史列表
And 列表显示每笔交易的详细信息：
  | 交易 ID | 交易类型 | 积分类型 | 金额 | 交易后余额 | 描述 | 时间 |
  | txn-001 | 订阅续费 | 会员积分 | 1000 | 1000 | 订阅 pro 套餐续费 | 2026-03-13 12:00 |
  | txn-002 | 消耗 | 会员积分 | -100 | 900 | 调用 AI API | 2026-03-13 12:30 |
  | txn-003 | 退款回收 | 充值积分 | -350 | 550 | 退款回收：50% 退款 | 2026-03-13 13:00 |
  | txn-004 | 过期回收 | 会员积分 | -200 | 350 | 会员积分过期 | 2026-03-15 00:00 |
And 我可以看到退款、过期等新的交易类型
```

**场景 2：按交易类型筛选（包含新的交易类型）**
```gherkin
Given 我是 realm-1 的管理员
And user-1 有多种类型的交易记录
When 我选择只查看"退款回收"类型的交易
Then 我只看到退款回收类型的交易记录
And 其他类型的交易不显示
When 我选择只查看"过期回收"类型的交易
Then 我只看到过期回收类型的交易记录
And 我可以选择的交易类型包括：
  | 充值 | 订阅赠送 | 订阅续费 | 订阅升级 |
  | 消耗 | 退款回收 | 过期回收 | 取消回收 | 调整 |
```

**场景 3：按时间范围筛选**
```gherkin
Given 我是 realm-1 的管理员
And user-1 有跨越多月的交易记录
When 我选择时间范围 "2026-03-01 到 2026-03-31"
Then 我只看到该时间范围内的交易记录
And 3 月之外的交易不显示
```

**场景 4：按 Client App 筛选**
```gherkin
Given 我是 realm-1 的管理员
And user-1 的积分消耗来自不同的 Client App
When 我选择按 Client App "app-001" 筛选
Then 我只看到来自 app-001 的消耗交易
And 来自其他 Client App 的交易不显示
```

**场景 5：分页查看交易历史**
```gherkin
Given 我是 realm-1 的管理员
And user-1 有 500 条交易记录
When 我查看交易历史（每页 20 条）
Then 我看到第 1 页的 20 条交易
And 可以翻页查看其他交易
And 页面显示总交易数和当前页码
```


### 故事 4：管理积分套餐配置 [US-PO-04]

**优先级**: P2

**【用户故事】**
**作为**：Realm Admin（详见 [docs/user-stories/_roles.md](/docs/user-stories/_roles.md)）
**我希望**：创建、编辑、删除积分套餐配置
**从而**：灵活调整积分赠送策略

**【验收标准】**

> 验收标准只描述用户动作与可见结果，不写 API 路径、数据表、字段变更、技术实现步骤。

**场景 1：批量创建多个套餐配置**
```gherkin
Given 我是 realm-1 的管理员
And 已存在多个订阅套餐：
  | 套餐名称  |
  | 基础月付  |
  | 专业月付  |
  | 企业版    |
When 我为每个套餐创建积分配置：
  | 套餐名称  | 每期积分 | 订阅时发放 | 周期类型  |
  | 基础月付  | 500      | 是         | monthly   |
  | 专业月付  | 1000     | 是         | monthly   |
  | 企业版    | 2000     | 是         | monthly   |
Then 所有套餐配置创建成功
And 配置列表显示所有套餐
```

**场景 2：编辑已有配置**
```gherkin
Given 我是 realm-1 的管理员
And 已存在套餐 "pro-monthly" 的配置
When 我编辑该配置：
  | 字段               | 原值  | 新值  |
  | Points per Period  | 1000  | 1500  |
  | Grant on Subscribe | true  | false |
And 我保存更改
Then 配置更新成功
And 新的订阅和续费将使用新配置
And 历史交易记录不受影响
```

**场景 3：删除套餐配置（无活跃订阅）**
```gherkin
Given 我是 realm-1 的管理员
And 已存在套餐 "old-plan" 的配置
And 该套餐没有活跃订阅
When 我删除该配置
Then 配置删除成功
And 配置列表不再显示该套餐
```

**场景 4：删除套餐配置（有活跃订阅）**
```gherkin
Given 我是 realm-1 的管理员
And 已存在套餐 "pro-monthly" 的配置
And 该套餐有 10 个活跃订阅
When 我尝试删除该配置
Then 系统显示警告消息："该套餐有 10 个活跃订阅，无法删除配置"
And 配置未被删除
```

**场景 5：批量编辑多个套餐配置**
```gherkin
Given 我是 realm-1 的管理员
And 已存在多个套餐配置
When 我选择多个套餐
And 我批量修改 "Max Periods" 为 20
Then 所有选中的套餐配置更新成功
And 所有套餐的最大发放期数都是 20
```


### 故事 5：查看套餐充值引导 [US-PO-05]

**优先级**: P2

**【用户故事】**
**作为**：Realm Admin（详见 [docs/user-stories/_roles.md](/docs/user-stories/_roles.md)）
**我希望**：查看套餐的积分配置信息
**从而**：向用户说明积分赠送规则

**【验收标准】**

> 验收标准只描述用户动作与可见结果，不写 API 路径、数据表、字段变更、技术实现步骤。

**场景 1：查看单个套餐的充值引导**
```gherkin
Given 我是 realm-1 的管理员
And 已存在套餐 "pro-monthly" 及其积分配置
When 我查看该套餐的积分引导信息
Then 我看到清晰的充值说明：
  | 套餐名称           | 专业版（月付）                  |
  | 每期发放积分       | 1000                           |
  | 订阅时是否额外发放 | 是                             |
  | 发放周期           | 月度                           |
  | 积分有效期         | 30 天                          |
  | 最大发放期数       | 10                             |
```

**场景 2：查看所有套餐的充值引导列表**
```gherkin
Given 我是 realm-1 的管理员
And 已存在多个套餐配置
When 我查看套餐充值引导页面
Then 我看到所有套餐的充值规则对比：
  | 套餐名称  | 每期积分 | 订阅时发放 | 周期 | 最大期数 |
  | 基础版    | 500      | 是         | 月度 | 5       |
  | 专业版    | 1000     | 是         | 月度 | 10      |
  | 企业版    | 2000     | 是         | 月度 | 无限制  |
```

**场景 3：导出充值引导文档**
```gherkin
Given 我是 realm-1 的管理员
And 已存在多个套餐配置
When 我点击 "Export Guide" 按钮
Then 系统生成充值引导文档
And 文档包含所有套餐的积分规则
And 我可以下载或分享该文档给用户
```

**场景 4：向用户展示充值规则（通过分享链接）**
```gherkin
Given 我是 realm-1 的管理员
And 已存在套餐 "pro-monthly" 的积分配置
When 我生成套餐充值规则的分享链接
Then 链接包含套餐的积分配置信息
And 我可以将链接分享给用户
And 用户点击链接可以查看充值规则
```

---

### 故事 6：配置 Realm 默认积分策略 [US-PO-06]

**优先级**: P0

**【用户故事】**
**作为**：Realm Admin（详见 [docs/user-stories/_roles.md](/docs/user-stories/_roles.md)）
**我希望**：配置免费用户的默认积分策略
**从而**：灵活控制免费用户的积分权益，优化产品转化率

**【验收标准】**

> 验收标准只描述用户动作与可见结果，不写 API 路径、数据表、字段变更、技术实现步骤。

**场景 1：查看 Realm 默认配置**
```gherkin
Given 我是 realm-1 的管理员
When 我访问"Realm 配置"页面
Then 我看到默认积分配置：
  | 配置项 | 当前值 |
  | 注册初始积分 | 1000 |
  | 每日积分数 | 50 |
  | 每日积分周期类型 | daily |
  | 每日积分有效期 | 1 天 |
And 我看到配置影响说明："配置仅影响新注册用户，不影响现有用户"
```

**场景 2：修改注册初始积分数**
```gherkin
Given 我是 realm-1 的管理员
And 当前注册初始积分为 1000
When 我将"注册初始积分"修改为 1500
And 我点击"保存"按钮
Then 配置更新成功
And 系统显示消息："配置已更新，新注册用户将获得 1500 初始积分"
And 现有用户的积分不受影响
```

**场景 3：修改每日积分数**
```gherkin
Given 我是 realm-1 的管理员
And 当前每日积分为 50
When 我将"每日积分数"修改为 100
And 我点击"保存"按钮
Then 配置更新成功
And 新注册用户每日获得 100 积分
And 现有免费用户的每日积分仍为 50（不受影响）
```

**场景 4：修改每日积分有效期**
```gherkin
Given 我是 realm-1 的管理员
And 当前每日积分有效期为 1 天
When 我将"每日积分有效期"修改为 2 天
And 我点击"保存"按钮
Then 配置更新成功
And 新注册用户的每日积分有效期为 2 天
And 现有免费用户的每日积分有效期仍为 1 天（不受影响）
```

**场景 5：禁用每日积分**
```gherkin
Given 我是 realm-1 的管理员
And 当前每日积分数为 50
When 我将"每日积分数"设置为 0
And 我点击"保存"按钮
Then 配置更新成功
And 新注册用户不会获得每日积分（每日积分数为 0 即禁用）
And 现有免费用户的每日积分调度不受影响（继续发放）
```

**场景 6：配置验证（不允许负数）**
```gherkin
Given 我是 realm-1 的管理员
When 我尝试将"注册初始积分"设置为 -100
And 我点击"保存"按钮
Then 系统显示错误："积分数量必须大于等于 0"
And 配置未更新
```

**场景 7：配置验证（有效期必须大于 0）**
```gherkin
Given 我是 realm-1 的管理员
When 我尝试将"每日积分有效期"设置为 0
And 我点击"保存"按钮
Then 系统显示错误："有效期必须大于 0 天"
And 配置未更新
```

**场景 8：查看配置历史**（待实现）
```gherkin
Given 我是 realm-1 的管理员
And 我已多次修改配置
When 我访问"配置历史"页面
Then 我看到配置变更历史：
  | 修改时间 | 配置项 | 旧值 | 新值 |
  | 2026-03-23 12:00 | 注册初始积分 | 1000 | 1500 |
  | 2026-03-22 10:00 | 每日积分数 | 50 | 100 |
And 我可以看到谁在何时修改了配置

> 注意：此场景尚未实现，后端暂不支持配置变更历史记录。
```

---

### 故事 7：查看免费用户积分统计 [US-PO-07]

**优先级**: P1

**【用户故事】**
**作为**：Realm Admin（详见 [docs/user-stories/_roles.md](/docs/user-stories/_roles.md)）
**我希望**：查看免费用户的积分统计数据
**从而**：了解免费用户的使用情况和转化率

**【验收标准】**

> 验收标准只描述用户动作与可见结果，不写 API 路径、数据表、字段变更、技术实现步骤。

**场景 1：查看免费用户概览统计**
```gherkin
Given 我是 realm-1 的管理员
And realm-1 有 1000 个免费用户
When 我访问"免费用户统计"页面
Then 我看到以下统计数据：
  | 统计项 | 数值 |
  | 总免费用户数 | 1000 |
  | 活跃用户数（近 7 天） | 800 |
  | 累计发放注册初始积分 | 1,000,000 |
  | 累计发放每日积分 | 40,000 |
  | 平均每用户每日积分 | 50 |
  | 转化率（免费→付费） | 15% |
And 数据实时更新
```

**场景 2：按时间范围筛选统计数据**
```gherkin
Given 我是 realm-1 的管理员
When 我选择时间范围"最近 30 天"
Then 我看到近 30 天的统计数据：
  | 统计项 | 数值 |
  | 新增免费用户 | 200 |
  | 发放注册初始积分 | 200,000 |
  | 发放每日积分 | 10,000 |
  | 转化为付费用户 | 30 |
And 转化率显示为 15%（30/200）
```

**场景 3：查看免费用户增长趋势**
```gherkin
Given 我是 realm-1 的管理员
When 我查看"用户增长趋势"图表
Then 我看到折线图显示每日新增免费用户数量
And 图表横轴为日期（最近 30 天）
And 图表纵轴为用户数量
And 我可以鼠标悬停查看具体日期的用户数
```

**场景 4：查看积分发放趋势**
```gherkin
Given 我是 realm-1 的管理员
When 我查看"积分发放趋势"图表
Then 我看到两条曲线：
  - 注册初始积分发放趋势
  - 每日积分发放趋势
And 图表横轴为日期（最近 30 天）
And 图表纵轴为积分数
And 我可以鼠标悬停查看具体日期的发放数量
```

**场景 5：查看转化率趋势**
```gherkin
Given 我是 realm-1 的管理员
When 我查看"转化率趋势"图表
Then 我看到折线图显示每日转化率（免费用户升级为付费用户的比例）
And 图表显示近 30 天的转化率变化
And 我可以看到转化率是否有提升或下降趋势
And 图表显示平均转化率（如 15%）
```

**场景 6：导出统计数据**
```gherkin
Given 我是 realm-1 的管理员
When 我点击"导出数据"按钮
Then 系统生成 CSV 文件
And 文件包含以下字段：
  - 日期
  - 新增免费用户数
  - 活跃免费用户数
  - 发放注册初始积分
  - 发放每日积分
  - 转化用户数
  - 转化率
And 我可以下载或分享该文件
```

**场景 7：查看单个免费用户的积分详情**
```gherkin
Given 我是 realm-1 的管理员
And 已存在免费用户 "user-1"
When 我在免费用户列表中点击 user-1
Then 我看到该用户的积分详情：
  | 积分类型 | 余额 | 有效期 | 发放时间 |
  | 注册初始积分 | 1000 | 永久有效 | 2026-03-23 15:30 |
  | 每日免费积分 | 50 | 明天 15:30 过期 | 2026-03-24 15:30 |
And 我看到该用户的积分使用统计：
  - 累计获得积分：1050
  - 累计消耗积分：200
  - 当前余额：850
And 我看到该用户是否已升级为付费用户
```

---


### 故事 8：主动发放积分 [US-PO-08]

**优先级**: P0

**【用户故事】**
**作为**：Realm Admin（详见 [docs/user-stories/_roles.md](/docs/user-stories/_roles.md)）
**我希望**：主动向指定用户发放积分，并可设置有效期和发放原因
**从而**：灵活运营，奖励用户或补偿异常情况

**【验收标准】**

> 验收标准只描述用户动作与可见结果，不写 API 路径、数据表、字段变更、技术实现步骤。

**场景 1：向单个用户发放积分**
```gherkin
Given 我是 realm-1 的管理员
And 已存在用户 "user-1"
When 我在积分管理页面点击 "Grant Points" 按钮
And 我填写发放信息：
  | 用户     | user-1           |
  | 积分数量 | 100              |
  | 有效期   | 30 天            |
  | 发放原因 | "Event reward"   |
And 我提交表单
Then 系统向 user-1 发放 100 积分
And 系统显示成功消息："Successfully granted 100 points to user-1"
And user-1 的积分余额增加 100
And 交易历史中新增一笔类型为 "发放" 的记录，原因为 "Event reward"
And 这批积分将在 30 天后过期
```

**场景 2：发放积分并设置永久有效**
```gherkin
Given 我是 realm-1 的管理员
And 已存在用户 "user-1"
When 我向 user-1 发放积分
And 我设置有效期为 "永久有效"
And 我提交表单
Then 系统向 user-1 发放积分
And 这批积分永久有效（无过期时间）
```

**场景 3：积分数量必须为正数**
```gherkin
Given 我是 realm-1 的管理员
When 我尝试发放积分
And 我设置积分数量为 0 或负数
Then 系统显示验证错误："积分数量必须大于 0"
And 发放操作未执行
```

**场景 4：用户不存在**
```gherkin
Given 我是 realm-1 的管理员
And 指定的用户 "nonexistent-user" 不存在
When 我尝试向该用户发放积分
Then 系统显示错误："User not found"
And 发放操作未执行
```

**场景 5：跨 Realm 用户不可发放**
```gherkin
Given 我是 realm-1 的管理员
And 用户 "user-other-realm" 属于 realm-2
When 我尝试向 user-other-realm 发放积分
Then 系统显示权限不足错误
And 发放操作未执行
```

**场景 6：发放原因必填**
```gherkin
Given 我是 realm-1 的管理员
When 我尝试发放积分但不填写发放原因
Then 系统显示验证错误："发放原因为必填项"
And 发放操作未执行
```

---

## 用户故事优先级汇总

| 优先级 | 用户故事数量 | 关键故事 |
|--------|------------|---------|
| P0 | 3 | US-PO-01: 配置积分套餐, US-PO-06: 配置 Realm 默认积分策略, US-PO-08: 主动发放积分 |
| P1 | 3 | US-PO-02: 查看所有用户积分账户, US-PO-03: 查看用户积分交易历史, US-PO-07: 查看免费用户积分统计 |
| P2 | 2 | US-PO-04: 管理积分套餐配置, US-PO-05: 查看套餐充值引导 |

---

## 相关文档

- **PRD**: [docs/prd/billing/points.md](/docs/prd/billing/points.md) - 积分系统产品需求文档
- **用户故事**: [docs/user-stories/billing/points-user.md](/docs/user-stories/billing/points-user.md) - 用户积分查询用户故事
- **用户故事**: [docs/user-stories/billing/points-free-user.md](/docs/user-stories/billing/points-free-user.md) - 免费用户积分用户故事
- **依赖 PRD**: [docs/prd/billing/subscription.md](/docs/prd/billing/subscription.md) - Billing 订阅计费产品需求文档
- **依赖 PRD**: [docs/prd/billing/points-grant.md](/docs/prd/billing/points-grant.md) - 积分发放产品需求文档
