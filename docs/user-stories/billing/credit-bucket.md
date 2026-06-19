# Credit Bucket 用户故事

> 角色定义以 [docs/user-stories/_roles.md](../_roles.md) 为准。

## 用户故事

### 故事 1：管理 Credit Bucket 目录 US-CB-001

**优先级**: P0

**【用户故事】**
**作为**：Realm Admin（详见 [docs/user-stories/_roles.md](../_roles.md)）
**我希望**：在本 Realm 内创建、编辑、启用/禁用多个 Credit Bucket，并指定其中一个为注册积分接收池
**从而**：为不同业务线或应用群配置独立的积分池目录与消费隔离边界

**【验收标准】**

> 验收标准只描述用户动作与可见结果，不写 API 路径、数据表、字段变更、技术实现步骤。

**场景 1：创建并管理 Bucket**
```gherkin
Given Realm Admin 已登录管理后台并进入 Credit Bucket 管理页
When 管理员创建一个新 Bucket（填写名称、可选展示顺序）并保存
Then 该 Bucket 出现在本 Realm 的 Bucket 列表中，默认为启用状态
And 管理员可编辑其名称与展示顺序
And 管理员可将某个 Bucket 标记为注册积分接收池，同一 Realm 同时仅有一个注册接收池
```

**场景 2：禁用 Bucket 不影响已持有池**
```gherkin
Given 某 Bucket 已存在且关联了套餐/积分包
When 管理员禁用该 Bucket
Then 该 Bucket 不再对新用户可见可购
And 已持有该 Bucket 积分池的用户仍可消费其剩余积分
```

### 故事 2：为 Bucket 绑定 Client App 覆盖集 US-CB-002

**优先级**: P0

**【用户故事】**
**作为**：Realm Admin（详见 [docs/user-stories/_roles.md](../_roles.md)）
**我希望**：为每个 Bucket 指定它覆盖的 Client App 集合
**从而**：限定只有被覆盖的 Client App 才能消费该 Bucket 积分池内的积分

**【验收标准】**

**场景 1：绑定覆盖集**
```gherkin
Given 一个已创建的 Bucket
When 管理员为其选择一个或多个 Client App 作为覆盖集并保存
Then 该 Bucket 显示已覆盖的 Client App 列表
And 至少绑定一个 Client App 才能保存成功
```

**场景 2：注册接收池也需显式配置覆盖集**
```gherkin
Given Realm 内某个 Bucket 被指定为接收注册/免费积分的池
When 管理员未为其配置任何 Client App 覆盖集
Then 该池内的注册/免费积分不可被任何应用消费
And 管理员为其配置覆盖集后，仅被覆盖的应用可消费该池积分
```

### 故事 3：将套餐/积分包归属到 Bucket US-CB-003

**优先级**: P0

**【用户故事】**
**作为**：Realm Admin（详见 [docs/user-stories/_roles.md](../_roles.md)）
**我希望**：把可购买的套餐或积分包归属到某个 Bucket
**从而**：用户购买该套餐/积分包后，积分进入对应 Bucket 的积分池

**【验收标准】**

**场景 1：归属套餐到 Bucket**
```gherkin
Given 一个已创建的 Bucket 和一个已配置的套餐/积分包
When 管理员将该套餐/积分包归属到此 Bucket 并保存
Then 该套餐/积分包显示其归属的 Bucket
And 一个 Bucket 可归属多个套餐/积分包
```

**场景 2：未归属的套餐不可购**
```gherkin
Given 某套餐/积分包未归属任何 Bucket
When 用户尝试购买该套餐/积分包
Then 系统阻止购买并提示该套餐未配置积分池
```

### 故事 4：购买 Bucket 套餐/积分包 US-CB-004

**优先级**: P0

**【用户故事】**
**作为**：Regular User（详见 [docs/user-stories/_roles.md](../_roles.md)）
**我希望**：购买某个 Bucket 对应的套餐或积分包，积分进入该 Bucket 的积分池
**从而**：为特定应用群充值独立、隔离的积分

**【验收标准】**

**场景 1：购买并入账到 Bucket 池**
```gherkin
Given 用户已登录，且某 Bucket 对应的套餐/积分包可购
When 用户完成该套餐/积分包的支付
Then 用户获得该 Bucket 的积分池，积分到账
And 该积分仅能被此 Bucket 覆盖的 Client App 消费
```

**场景 2：同时持有多个 Bucket**
```gherkin
Given 用户已持有 Bucket A 的积分池
When 用户再购买 Bucket B 对应的套餐/积分包并支付成功
Then 用户同时持有 Bucket A 与 Bucket B 两个独立积分池
And 两个池的余额互相独立、互不影响
```

### 故事 5：查看按 Bucket 分组的积分余额 US-CB-005

**优先级**: P0

**【用户故事】**
**作为**：Regular User（详见 [docs/user-stories/_roles.md](../_roles.md)）
**我希望**：在积分页面看到我持有的每个 Bucket 的独立余额，以及跨全部 Bucket 的合计
**从而**：清楚每个应用群可用积分和总体积分

**【验收标准】**

**场景 1：多 Bucket 余额展示**
```gherkin
Given 用户同时持有多个 Bucket 的积分池
When 用户打开积分余额页
Then 页面按 Bucket 分组显示每个池的余额（按积分类型分桶）
And 页面显示跨全部 Bucket 的积分合计
```

**场景 2：注册积分所在 Bucket 余额可见**
```gherkin
Given 用户持有注册/免费等系统发放的积分（位于被指定接收的 Bucket）
When 用户查看积分余额页
Then 该 Bucket 的余额与其他 Bucket 一并分组展示
```

### 故事 6：查看 Bucket 维度的交易历史 US-CB-006

**优先级**: P1

**【用户故事】**
**作为**：Regular User（详见 [docs/user-stories/_roles.md](../_roles.md)）
**我希望**：在交易历史中看到每笔交易所属的 Bucket，并能按 Bucket 筛选
**从而**：核对每个积分池的收支明细

**【验收标准】**

**场景 1：交易记录显示 Bucket 并支持筛选**
```gherkin
Given 用户有多个 Bucket 的积分收支记录
When 用户打开交易历史页
Then 每条记录显示其所属的 Bucket
And 用户可按 Bucket 筛选交易记录
```

### 故事 7：SDK 按 Client App 跨 Bucket 消费 US-CB-007

**优先级**: P0

**【用户故事】**
**作为**：Third-Party App（详见 [docs/user-stories/_roles.md](../_roles.md)）
**我希望**：通过 SDK 消费积分时，系统按当前 Client App 自动从覆盖该应用的 Bucket 积分池中扣减
**从而**：在不感知 Bucket 的情况下，按应用授权范围正确消费用户积分

**【验收标准】**

**场景 1：跨多个 Bucket 池消费**
```gherkin
Given 用户持有两个 Bucket（Bucket A、Bucket B），两者都覆盖当前 Client App，且 Bucket A 内积分更早过期
When 第三方应用代表用户消费一定积分
Then 系统按过期时间优先，跨这两个 Bucket 池扣减，原子完成且不超额
```

**场景 2：无可用池或余额不足**
```gherkin
Given 用户名下没有任何覆盖当前 Client App 的 Bucket 池，或覆盖池合计余额不足
When 第三方应用代表用户消费积分
Then 系统拒绝消费并返回余额不足/无可用积分池的明确提示
```

**场景 3：应用越权消费**
```gherkin
Given 某 Bucket 未覆盖当前 Client App
When 第三方应用尝试消费该 Bucket 池内的积分
Then 系统拒绝消费，该 Bucket 池不被扣减
```

### 故事 8：订阅生命周期按 Bucket 池发放与回收 US-CB-008

**优先级**: P0

**【用户故事】**
**作为**：Herald 系统（详见 [docs/user-stories/_roles.md](../_roles.md)）
**我希望**：订阅的首次发放、续费、升级、降级、取消、退款的积分都准确定位到该订阅绑定的 Bucket 积分池
**从而**：保证积分发放与回收始终回到正确的池，不串池

**【验收标准】**

**场景 1：续费发放回原池**
```gherkin
Given 用户已订阅某 Bucket 对应的套餐，且首期积分已入该 Bucket 池
When 订阅续费成功
Then 新一期积分继续进入同一 Bucket 池
```

**场景 2：取消/退款回收回原池**
```gherkin
Given 用户已订阅某 Bucket 对应的套餐
When 订阅被取消或发生退款
Then 系统仅在该订阅绑定的 Bucket 池内回收未使用的会员积分，不影响其他 Bucket 池
```

**场景 3：升级在原池回收旧套餐并发新套餐**
```gherkin
Given 用户已订阅某 Bucket 对应的套餐，且当前周期会员积分已入该 Bucket 池
When 用户在该订阅上升级到更高档套餐
Then 系统在该订阅绑定的同一 Bucket 池内回收旧套餐未使用的会员积分并发新套餐积分
And 注册初始积分保留
And 不影响其他 Bucket 池
```

**场景 4：降级保留当前周期，下周期回原池发放**
```gherkin
Given 用户已订阅某 Bucket 对应的套餐，且当前周期会员积分已发到该 Bucket 池
When 用户在该订阅上降级到更低档套餐
Then 当前周期已发积分保留
And 下周期按新套餐发放到同一 Bucket 池
And 不影响其他 Bucket 池
```
