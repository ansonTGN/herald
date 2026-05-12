# 积分系统 - Billing 事件处理用户故事

**角色代码**: PO (Points Owner/Admin) & BI (Billing Integration)
**角色定义**：
- PO: Realm Admin（详见 [docs/user-stories/_roles.md](/docs/user-stories/_roles.md)），负责管理本租户内用户的积分账户
- BI: 系统角色，处理 Billing 事件并执行相应的积分操作

**故事范围**: US-PO-08 ~ US-PO-09, US-BI-009 ~ US-BI-011
**创建时间**: 2026-03-21
**状态**: Active

---

## 故事 1：处理退款积分回收 [US-PO-08]

**优先级**: P0

**【用户故事】**
**作为**：积分系统
**我希望**：当收到退款事件时，自动回收未使用的积分
**从而**：确保积分余额与实际支付金额一致

**【验收标准】**

**场景 1：充值退款按未使用比例回收**
```gherkin
Given 用户 user-1 有 1000 充值积分
And 用户已使用 300 充值积分，剩余 700 充值积分
When 收到 Creem refund.created 事件
And 退款金额为原支付金额的 50%
Then 系统计算应回收积分：1000 × 50% × (700/1000) = 350 积分
And 系统从充值积分中回收 350 积分
And 用户剩余充值积分：700 - 350 = 350 积分
And 创建积分回收记录，记录回收原因和退款 ID
```

**场景 2：会员退款仅回收未使用会员积分**
```gherkin
Given 用户 user-1 有 1000 会员积分（未使用）
And 用户有 500 充值积分
When 收到 Creem refund.created 事件（订阅退款）
Then 系统仅回收未使用的会员积分
And 回收 1000 会员积分
And 保留 500 充值积分
And 创建积分回收记录，记录回收原因和订阅 ID
```

**场景 3：退款事件幂等性**
```gherkin
Given 已处理过退款事件 ch_xxx
And 已回收 350 积分
When 再次收到相同的 refund.created 事件
Then 系统检查幂等键：ch_xxx_timestamp
And 发现已存在相同幂等键的交易记录
Then 系统跳过处理，不重复回收积分
And 返回成功响应（幂等）
```

**场景 4：管理员查看退款回收记录**
```gherkin
Given 我是 realm-1 的管理员
And 已发生多笔退款积分回收
When 我访问积分管理页面的"回收记录"标签
Then 我看到退款回收记录列表
And 列表包含：
  | 回收时间    | 用户      | 回收类型    | 回收金额 | 回收原因        | 关联 ID   |
  | 2026-03-21 | user-1   | refund_revoke | 350     | 退款回收：50% | ch_xxx    |
  | 2026-03-20 | user-2   | refund_revoke | 1000    | 订阅退款        | sub_yyy   |
And 我可以点击查看回收详情
```

**场景 5：已使用积分不回收**
```gherkin
Given 用户 user-1 有 1000 充值积分
And 用户已全部使用 1000 充值积分
When 收到 Stripe charge.refunded 事件
And 退款金额为原支付金额的 100%
Then 系统计算应回收积分：1000 × 100% × (0/1000) = 0 积分
And 系统不执行积分回收操作
And 创建积分回收记录，记录回收金额为 0
```

---

## 故事 2：订阅升级积分处理 [US-BI-009]

**优先级**: P1

**【用户故事】**
**作为**：积分系统
**我希望**：当用户升级订阅时，立即发放差额积分
**从而**：用户可以立即享受新套餐的积分权益

**【验收标准】**

**场景 1：升级立即补发差额积分**
```gherkin
Given 用户 user-1 当前订阅 basic 套餐（500 积分）
And 用户已使用 200 会员积分，剩余 300 会员积分
When 收到 Creem subscription.update 事件
And 用户从 basic 升级到 pro 套餐（1000 积分）
Then 系统立即发放差额积分：1000 - 500 = 500 积分
And 差额积分类型为 subscription_credit
And 差额积分的过期时间与当前周期结束时间一致
And 用户会员积分总额：300（剩余） + 500（差额） = 800 积分
```

**场景 2：升级事件幂等性**
```ghergin
Given 已处理过订阅升级事件 sub_xxx_t1
And 已发放 500 差额积分
When 再次收到相同的 subscription.update 事件
Then 系统检查幂等键：sub_xxx_t1
And 发现已存在相同幂等键的交易记录
Then 系统跳过处理，不重复发放积分
And 返回成功响应（幂等）
```

**场景 3：用户查看升级积分发放记录**
```gherkin
Given 我是 user-1
And 我的订阅刚刚从 basic 升级到 pro
When 我访问"我的积分"页面的"交易历史"标签
Then 我看到一条新的交易记录：
  | 交易时间     | 交易类型       | 金额 | 描述               | 积分类型              |
  | 2026-03-21  | subscription_upgrade | 500 | 订阅升级补差积分 | subscription_credit |
And 我可以看到当前周期的会员积分余额增加了
```

---

## 故事 3：订阅降级积分处理 [US-BI-010]

**优先级**: P1

**【用户故事】**
**作为**：积分系统
**我希望**：当用户降级订阅时，当前周期已发积分不回收
**从而**：用户在当前周期内不受降级影响

**【验收标准】**

**场景 1：降级下周期生效，不回收当前积分**
```gherkin
Given 用户 user-1 当前订阅 pro 套餐（1000 积分）
And 当前周期已发放 1000 会员积分
And 用户已使用 300 会员积分，剩余 700 会员积分
When 收到 Creem subscription.update 事件
And 用户从 pro 降级到 basic 奶餐（500 积分）
Then 系统不回收当前周期的 700 会员积分
And 用户当前周期继续使用 700 会员积分
And 下周期按 basic 套餐发放 500 积分
And 创建订阅变更记录，记录降级事件
```

**场景 2：降级事件幂等性**
```gherkin
Given 已处理过订阅降级事件 sub_xxx_t1
And 已记录降级事件
When 再次收到相同的 subscription.update 事件
Then 系统检查幂等键：sub_xxx_t1
And 发现已存在相同幂等键的交易记录
Then 系统跳过处理，不重复记录
And 返回成功响应（幂等）
```

**场景 3：用户查看降级说明**
```gherkin
Given 我是 user-1
And 我的订阅刚刚从 pro 降级到 basic
When 我访问"我的订阅"页面
Then 我看到降级说明：
  "您的套餐已从 Pro 降级到 Basic"
  "当前周期（2026-03-15 至 2026-04-15）继续享受 Pro 套餐权益"
  "下周期（2026-04-15 起）将按 Basic 套餐发放积分"
And 我可以看到当前周期的会员积分余额保持不变
```

---

## 故事 4：订阅取消积分处理 [US-BI-011]

**优先级**: P1

**【用户故事】**
**作为**：积分系统
**我希望**：当用户取消订阅时，根据取消模式处理积分回收
**从而**：默认取消保留积分到周期结束，立即取消回收未使用会员积分

**【验收标准】**

**场景 1：默认取消保留积分到周期结束**
```gherkin
Given 用户 user-1 有活跃订阅
And 当前周期已发放 1000 会员积分
And 用户已使用 300 会员积分，剩余 700 会员积分
When 收到 Creem subscription.canceled 事件
And 取消模式为默认取消（周期结束）
Then 系统设置会员积分的过期时间为周期结束时间
And 当前周期继续有效，用户继续使用 700 会员积分
And 下周期不再发放会员积分
And 周期结束后，未使用的会员积分过期
```

**场景 2：立即取消回收未使用会员积分**
```gherkin
Given 用户 user-1 有活跃订阅
And 当前周期已发放 1000 会员积分（未使用）
And 用户有 500 充值积分
When 收到 Creem subscription.canceled 事件
And 取消模式为立即取消
Then 系统回收未使用的 1000 会员积分
And 系统不回收 500 充值积分
And 创建积分回收记录，记录回收原因和订阅 ID
And 用户剩余 500 充值积分
```

**场景 3：立即取消已使用会员积分不回收**
```gherkin
Given 用户 user-1 有活跃订阅
And 当前周期已发放 1000 会员积分
And 用户已全部使用 1000 会员积分
And 用户有 500 充值积分
When 收到 Creem subscription.canceled 事件
And 取消模式为立即取消
Then 系统不回收会员积分（已全部使用）
And 系统不回收充值积分
And 创建积分回收记录，记录回收金额为 0
And 用户剩余 500 充值积分
```

**场景 4：取消事件幂等性**
```gherkin
Given 已处理过订阅取消事件 sub_xxx_t1
And 已设置会员积分过期时间或回收积分
When 再次收到相同的 subscription.canceled 事件
Then 系统检查幂等键：sub_xxx_t1
And 发现已存在相同幂等键的交易记录
Then 系统跳过处理，不重复设置过期时间或回收积分
And 返回成功响应（幂等）
```

**场景 5：用户查看取消说明**
```gherkin
Given 我是 user-1
And 我的订阅刚刚取消
When 我访问"我的订阅"页面
Then 如果是默认取消，我看到：
  "您的订阅已取消"
  "当前周期（2026-03-15 至 2026-04-15）继续享受套餐权益"
  "会员积分将在周期结束时过期"
And 如果是立即取消，我看到：
  "您的订阅已立即取消"
  "未使用的会员积分已被回收"
  "充值积分保留，可继续使用"
```

---

## 故事 5：异步任务失败积分退回 [US-PO-09]

**优先级**: P1

**【用户故事】**
**作为**：第三方应用开发者
**我希望**：当异步任务（如图片生成）失败时，系统能自动退回已消费的积分
**从而**：用户不会因为任务失败而损失积分

**【验收标准】**

**场景 1：异步任务成功，积分不退回**
```gherkin
Given 用户 user-1 有 1000 积分
And 用户发起生成图片任务，消耗 100 积分
And 系统创建交易记录 txn_001，交易类型为 consume
When 图片生成成功
Then 系统不执行任何积分退回操作
And 用户剩余积分：900
And 交易历史显示一条消费记录：-100 积分
```

**场景 2：异步任务失败，自动退回积分**
```gherkin
Given 用户 user-1 有 1000 积分
And 用户发起生成图片任务，消耗 100 积分
And 系统创建交易记录 txn_001，交易类型为 consume
And 原消费的积分为 topup_credit 类型
When 图片生成失败
And 系统调用积分补偿接口
Then 系统创建新的积分记录，类型为 system_grant
And 补偿积分数：100
And 补偿积分的 external_ref_id 为 "refund:txn_001"
And 用户剩余积分：1000（原余额）
And 交易历史显示：
  - 消费记录：-100 积分（consume）
  - 补偿记录：+100 积分（system_grant）
```

**场景 3：退回积分的幂等性**
```gherkin
Given 已处理过任务失败，txn_001 已退回 100 积分
And 存在 external_ref_id 为 "refund:txn_001" 的交易记录
When 再次收到相同的任务失败通知
Then 系统检查是否已存在退回记录
Then 发现已存在 external_ref_id 为 "refund:txn_001" 的记录
Then 系统跳过处理，不重复退回积分
And 返回成功响应（幂等）
```

**场景 4：管理员查看积分补偿记录**
```gherkin
Given 我是 realm-1 的管理员
And 已发生多笔异步任务失败积分退回
When 我访问积分管理页面的"交易历史"标签
And 筛选交易类型为 "system_grant"
Then 我看到积分补偿记录列表
And 列表包含：
  | 交易时间     | 用户      | 交易类型      | 金额 | 描述                     | 关联 ID          |
  | 2026-03-25  | user-1   | system_grant  | 100  | 任务失败补偿：生成图片   | refund:txn_001  |
  | 2026-03-25  | user-2   | system_grant  | 50   | 任务失败补偿：视频处理   | refund:txn_002  |
And 我可以点击查看原始消费记录
```

**场景 5：用户查看积分退回记录**
```gherkin
Given 我是 user-1
And 我的生成图片任务失败，积分已退回
When 我访问"我的积分"页面的"交易历史"标签
Then 我看到两条相关交易记录：
  | 时间        | 类型          | 金额 | 说明               |
  | 2026-03-25  | consume       | -100 | 生成图片           |
  | 2026-03-25  | system_grant  | +100 | 任务失败补偿：生成图片 |
And 我可以清楚地看到原始消费和补偿的对应关系
```

**场景 6：不同类型积分的退回**
```gherkin
Given 用户 user-1 有：
  | topup_credit | subscription_credit |
  | 500         | 500                 |
And 用户发起任务，优先消耗 subscription_credit
And 系统消耗 100 subscription_credit
When 任务失败
Then 系统退回 100 subscription_credit（而非 topup_credit）
And 用户余额恢复为：
  | topup_credit | subscription_credit |
  | 500         | 500                 |
```

---

## 用户故事优先级汇总

| 优先级 | 用户故事数量 | 关键故事 |
|--------|------------|---------|
| P0 | 1 | US-PO-08: 处理退款积分回收 |
| P1 | 4 | US-BI-009: 订阅升级积分处理, US-BI-010: 订阅降级积分处理, US-BI-011: 订阅取消积分处理, US-PO-09: 异步任务失败积分退回 |
| P2 | 0 | - |

---

## 相关文档

- **PRD**: `docs/prd/billing/points.md` - 积分系统产品需求文档
- **PRD**: `docs/prd/billing/billing.md` - Billing 订阅计费产品需求文档
- **验收标准**: `.ai/future/points-accept.md` - 积分系统验收指南
- **用户故事**: `points-admin-manage.md` - Tenant Admin 积分管理用户故事
- **用户故事**: `points-user-view.md` - Tenant User 积分查询用户故事
