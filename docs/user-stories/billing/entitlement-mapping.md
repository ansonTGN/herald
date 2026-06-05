# Entitlement Mapping 用户故事

> 角色定义见 [docs/user-stories/_roles.md](/docs/user-stories/_roles.md)

## 用户故事

### 故事 1：查看 Provider Entitlement 映射 [US-EM-001]

**优先级**: P0

**【用户故事】**
**作为**：Realm Admin（详见 [docs/user-stories/_roles.md](/docs/user-stories/_roles.md)）
**我希望**：查看支付方产品/价格到 Herald Entitlement 的映射列表
**从而**：了解每个支付方提供了哪些产品、每个产品映射到什么 entitlement、以及积分策略的同步状态

**【验收标准】**

> 验收标准只描述用户动作与可见结果，不写 API 路径、数据表、字段变更、技术实现步骤。

**场景 1：查看所有 Provider Entitlement 映射**
```gherkin
Given 我是 realm-1 的管理员
And realm-1 已配置 Stripe 和 Creem 支付平台
And 已从 Stripe 同步了 3 个产品、从 Creem 同步了 2 个产品
When 我访问 Billing 管理页面的 "Entitlement Mappings" 区域
Then 我看到所有 provider entitlement 映射列表
And 每条映射显示：
  | Payment Provider | Stripe       |
  | External Product | prod_xxxx    |
  | External Price   | price_yyyy   |
  | Entitlement Key  | pro-plan     |
  | Points Policy    | ✅ Synced    |
  | Synced At        | 2026-06-04   |
  | Enabled          | Yes          |
```

**场景 2：按支付方筛选映射**
```gherkin
Given 我是 realm-1 的管理员
And 存在来自多个支付方的映射
When 我选择支付方筛选 "Stripe"
Then 列表仅显示 Stripe 支付方的映射
```

**场景 3：映射尚未同步**
```gherkin
Given 我是 realm-1 的管理员
And realm-1 已配置支付平台但从未同步过产品
When 我访问 "Entitlement Mappings" 区域
Then 显示空状态提示："No provider products synced yet"
And 显示引导操作："Sync provider products to see available mappings"
```

---

### 故事 2：触发 Provider 产品同步 [US-EM-002]

**优先级**: P1

**【用户故事】**
**作为**：Realm Admin（详见 [docs/user-stories/_roles.md](/docs/user-stories/_roles.md)）
**我希望**：手动触发支付方产品的全量同步
**从而**：确保 Herald 中的 entitlement 映射和积分策略与支付方保持一致

**【验收标准】**

> 验收标准只描述用户动作与可见结果，不写 API 路径、数据表、字段变更、技术实现步骤。

**场景 1：手动触发全量同步**
```gherkin
Given 我是 realm-1 的管理员
And realm-1 已配置 Stripe 支付平台
When 我点击 "Sync Provider Products" 按钮
And 我选择要同步的支付方 "Stripe"
Then 系统开始同步并显示进度指示
When 同步完成
Then 系统显示同步结果：
  | Products Synced  | 5          |
  | Prices Synced    | 12         |
  | Sync Status      | Completed  |
And Entitlement Mappings 列表更新为最新数据
```

**场景 2：同步失败**
```gherkin
Given 我是 realm-1 的管理员
And Stripe API 当前不可用
When 我触发全量同步
Then 系统显示同步失败提示："Failed to sync provider products"
And 显示失败原因和重试建议
And 现有映射数据不受影响，仍可正常使用
```

**场景 3：支付平台未配置**
```gherkin
Given 我是 realm-1 的管理员
And realm-1 未配置任何支付平台
When 我尝试触发同步
Then 系统提示："No payment providers configured. Please configure a payment provider first."
```

---

### 故事 3：Webhook 通过 Metadata 映射订阅 [US-EM-003]

**优先级**: P0

**【用户故事】**
**作为**：System
**我希望**：通过支付方 webhook metadata（而非本地 Product/Plan）将外部订阅映射到 Herald 订阅投影
**从而**：在移除本地 Product/Plan 后仍能正确处理订阅事件

**【验收标准】**

> 验收标准只描述用户动作与可见结果，不写 API 路径、数据表、字段变更、技术实现步骤。

**场景 1：Stripe 订阅激活**
```gherkin
Given Stripe 发送 subscription.active webhook
And webhook metadata 包含：
  | herald_realm_id       | realm-1      |
  | herald_client_app_id  | app-1        |
  | herald_user_id        | user-1       |
  | herald_entitlement_key | pro-plan    |
  | herald_billing_kind   | subscription |
And webhook 签名验证通过
When 系统处理该 webhook
Then Herald 创建/更新订阅投影，关联到 realm-1、app-1、user-1
And 订阅投影的 entitlement_key 为 "pro-plan"
And 订阅状态为 Active
```

**场景 2：Metadata 缺失 entitlement_key**
```gherkin
Given Stripe 发送 subscription.active webhook
And webhook metadata 缺少 herald_entitlement_key
When 系统处理该 webhook
Then 系统记录错误诊断："Missing herald_entitlement_key in webhook metadata"
And 订阅投影更新失败
And 错误对管理员可见
```

**场景 3：Checkout 创建时验证 Metadata**
```gherkin
Given 系统为用户发起 Stripe Checkout
When Checkout Session metadata 未包含 herald_realm_id 或 herald_entitlement_key
Then 系统拒绝创建 Checkout 并提示缺少必填 metadata
```

**场景 4：幂等处理**
```gherkin
Given 已处理过 Stripe event "evt_123"
When 系统再次收到相同 event "evt_123"
Then 系统识别为重复事件并跳过处理
And 订阅投影状态不变
```

---

### 故事 4：基于 Entitlement 应用积分策略 [US-EM-004]

**优先级**: P0

**【用户故事】**
**作为**：System
**我希望**：在订阅事件发生时基于 entitlement_key 查询和应用积分策略
**从而**：在移除 plan_id 后仍能正确发放、续期和回收积分

**【验收标准】**

> 验收标准只描述用户动作与可见结果，不写 API 路径、数据表、字段变更、技术实现步骤。

**场景 1：首次订阅积分发放**
```gherkin
Given 用户 user-1 在 realm-1 首次订阅 entitlement "pro-plan"
And provider sync 缓存中 "pro-plan" 的积分策略为：
  | Points Per Period | 1000 |
  | Grant On Subscribe | Yes  |
  | Validity Days     | 30   |
When 系统处理订阅激活事件
Then user-1 获得 1000 积分，有效期 30 天
And 积分发放记录与 entitlement_key "pro-plan" 关联
```

**场景 2：续费积分发放**
```gherkin
Given 用户 user-1 已订阅 entitlement "pro-plan"
And "pro-plan" 的续费积分策略为：
  | Points Per Period | 500  |
  | Max Periods       | 12   |
  | Grant Period Type | monthly |
When 系统处理续费事件
Then user-1 获得 500 积分
And 续费发放次数 +1
```

**场景 3：Entitlement 无积分策略**
```gherkin
Given 用户订阅 entitlement "basic-plan"
And "basic-plan" 在 provider sync 缓存中无积分策略配置
When 系统处理订阅激活事件
Then 系统跳过积分发放
And 记录诊断："No points policy found for entitlement 'basic-plan'"
```

**场景 4：取消订阅积分回收**
```gherkin
Given 用户 user-1 通过 entitlement "pro-plan" 获得了积分
When 用户取消订阅
Then 系统根据回收规则处理积分
And 积分回收记录与 entitlement_key "pro-plan" 关联
```

---

### 故事 5：SDK 通过 Entitlement 查询订阅状态 [US-EM-005]

**优先级**: P0

**【用户故事】**
**作为**：Third-Party App（详见 [docs/user-stories/_roles.md](/docs/user-stories/_roles.md)）
**我希望**：通过 entitlement_key 查询用户订阅状态
**从而**：在不依赖本地 Plan 的情况下做出访问控制决策

**【验收标准】**

> 验收标准只描述用户动作与可见结果，不写 API 路径、数据表、字段变更、技术实现步骤。

**场景 1：查询用户活跃订阅**
```gherkin
Given 用户 user-1 在 realm-1 有一个活跃订阅
And 订阅的 entitlement_key 为 "pro-plan"
When 第三方应用通过 SDK 查询 user-1 的订阅状态
Then 返回结果显示：
  | Has Subscription | Yes         |
  | Status           | Active      |
  | Has Access       | Yes         |
  | Entitlement Key  | pro-plan    |
  | Payment Provider | Stripe      |
```

**场景 2：用户无订阅**
```gherkin
Given 用户 user-2 在 realm-1 没有订阅
When 第三方应用通过 SDK 查询 user-2 的订阅状态
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

---

### 故事 6：查看订阅投影列表 [US-EM-006]

**优先级**: P0

**【用户故事】**
**作为**：Realm Admin（详见 [docs/user-stories/_roles.md](/docs/user-stories/_roles.md)）
**我希望**：查看 Realm 内所有订阅投影列表
**从而**：了解用户的订阅状态、entitlement 和支付方信息

**【验收标准】**

> 验收标准只描述用户动作与可见结果，不写 API 路径、数据表、字段变更、技术实现步骤。

**场景 1：查看所有订阅投影**
```gherkin
Given 我是 realm-1 的管理员
And realm-1 有多个用户的订阅来自不同支付方
When 我访问 Billing 管理页面的 "Subscriptions" 区域
Then 我看到所有订阅投影列表
And 每条订阅显示：
  | User             | user-1          |
  | Entitlement Key  | pro-plan        |
  | Payment Provider | Stripe          |
  | Status           | Active          |
  | Current Period   | Jun 1 - Jul 1   |
  | Synced At        | 2026-06-04      |
```

**场景 2：按 entitlement 或状态筛选**
```gherkin
Given 我是 realm-1 的管理员
When 我选择 Entitlement 筛选 "pro-plan"
Then 列表仅显示 entitlement_key 为 "pro-plan" 的订阅
When 我选择状态筛选 "Active"
Then 列表仅显示活跃状态的订阅
```

**场景 3：查看订阅变更历史**
```gherkin
Given 我是 realm-1 的管理员
And 用户 user-1 的订阅有变更历史
When 我点击订阅 "user-1" 的详情
Then 我看到该订阅的完整变更时间线
And 每条变更记录显示：
  | Event Type        | upgraded     |
  | Entitlement       | pro-plan     |
  | Previous Entitlement | basic-plan   |
  | Changed At        | 2026-06-01   |
```

---

## 业务规则总结

### Provider Ownership 边界
1. **商业目录**：支付方拥有 Product、Price、Checkout、Customer billing、Subscription lifecycle、Invoice/payment
2. **Herald 职责**：Realm/Client App 边界、User binding、Access entitlement projection、Webhook 幂等、积分策略和账本、SDK 读模型
3. **Herald 不维护**：独立的本地商业目录或可编辑的订阅套餐

### Metadata 契约
1. **统一前缀**：所有 Herald metadata key 使用 `herald_` 前缀
2. **必填 metadata**：`herald_realm_id`、`herald_client_app_id`、`herald_user_id`、`herald_entitlement_key`
3. **计费类型**：`herald_billing_kind` 值为 `subscription` 或 `points_package`
4. **Stripe 分层**：稳定映射放 Product/Price metadata，请求特定信息放 Checkout Session/Subscription metadata
5. **Creem**：metadata 写入 checkout 请求，后续 webhook 返回该 metadata

### Entitlement 映射规则
1. **Source of truth**：Herald 本地 provider-to-entitlement mapping 为 entitlement 映射和积分策略的 source of truth；Stripe Product/Price metadata 可作为导入来源
2. **本地角色**：Herald 维护 provider-to-entitlement 映射作为 allowlist 和同步缓存
3. **同步机制**：支持 webhook 触发增量同步和管理员手动触发全量同步
4. **降级策略**：同步失败时本地缓存可独立服务 webhook；同步失败不应静默降级为默认策略

### 订阅投影规则
1. **投影语义**：Subscription 是支付方订阅状态的本地投影，不是 Herald 拥有的订阅
2. **字段边界**：保留 realm_id、client_app_id、user_id、entitlement_key、status、period 信息、provider metadata
3. **移除字段**：不再维护 plan_id、本地 tier、本地 billing_period

### 积分策略规则
1. **策略来源**：积分策略的 source of truth 是 Herald 本地 mapping/entitlement policy；Stripe Product/Price metadata 可作为导入来源，Creem 必须在 Herald 中配置
2. **策略查询**：按 entitlement_key 从本地配置查询
3. **覆盖场景**：首次订阅发放、续费发放、取消回收、退款回收、升级/降级处理
4. **管理员编辑**：管理员可在 Herald 中查看和修改积分策略；修改的是 Herald 业务规则，不回写 provider

### 废弃规则
1. **Product CRUD**：本地 Product 管理页面和接口废弃并移除
2. **Plan CRUD**：本地 SubscriptionPlan 管理页面和接口废弃并移除
3. **关联表**：subscription_plan_payment_provider、client_app_subscription_plan、points_plan_configs 废弃并移除
4. **plan_id 外键**：全链路从 plan_id 迁移到 entitlement_key 后移除

---

## 相关文档

- **PRD**: [docs/prd/billing/product_reduce.md](/docs/prd/billing/product_reduce.md) - Product & Subscription Model Reduction 产品需求文档
- **PRD**: [docs/prd/billing/subscription.md](/docs/prd/billing/subscription.md) - 订阅计费 PRD（需更新）
- **PRD**: [docs/prd/billing/points.md](/docs/prd/billing/points.md) - 积分系统 PRD（需更新）
- **PRD**: [docs/prd/billing/product-catalog.md](/docs/prd/billing/product-catalog.md) - Product 编目管理 PRD（将废弃）
- **技术研究**: [.ai/tech-research/product_reduce.md](/.ai/tech-research/product_reduce.md) - 技术预研报告
- **需求来源**: Product and Subscription Local Model Reduction — 移除本地 Product/Plan 商业目录，将目录和订阅生命周期交给支付方
