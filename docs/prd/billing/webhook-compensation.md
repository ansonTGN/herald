# Webhook 补偿机制 产品需求文档 (PRD)

**创建时间**: 2026-06-09
**优先级**: P1

---

## 1. 相关用户故事

> 详细故事与验收标准请查看 `docs/user-stories/` 中对应文档。

### 1.1 故事引用

- `[US-WC-001]` 定时检测并补偿缺失的 Webhook 事件，优先级 P0，来源 `docs/user-stories/billing/webhook-compensation.md`
  - 角色：Herald 系统
  - 摘要：定时从支付方拉取近期事件，与本地记录对比，补处理缺失的事件

- `[US-WC-002]` 补偿处理保持幂等性，优先级 P0，来源 `docs/user-stories/billing/webhook-compensation.md`
  - 角色：Herald 系统
  - 摘要：补偿处理与 webhook 处理共享相同的幂等保证，防止重复副作用

- `[US-PA-002]` 场景 5 — 主动查询平台状态（回调未到达时的补偿），优先级 P0，来源 `docs/user-stories/billing/payment-attempt.md`
  - 角色：Herald 系统
  - 摘要：单笔支付尝试级别的主动查询补偿

- `[US-EM-003]` Webhook 通过 Metadata 映射订阅，优先级 P0，来源 `docs/user-stories/billing/entitlement-mapping.md`
  - 角色：Herald 系统
  - 摘要：webhook 通过 metadata 将外部订阅映射到 Herald 订阅投影

### 1.2 优先级汇总

| 优先级 | 数量 | 关键故事 |
|--------|------|----------|
| P0 | 4 | 定时检测补偿缺失事件、补偿幂等性保证（本功能）；主动查询平台状态补偿、Webhook metadata 映射订阅（交叉引用） |
| P1 | 0 | - |
| P2 | 0 | - |

---

## 2. 范围界定

### 2.1 包含功能

- 后台定时 Job 定期从 Stripe/Creem 拉取近期事件
- 按 realm 遍历，对每个配置了支付平台的 realm 分别对账
- 以外部事件 ID 为键，对比 provider 事件与本地支付事件记录
- 对 provider 存在但 Herald 缺失的事件，执行与 webhook handler 相同的业务逻辑
- 覆盖 Stripe 全事件类型（订阅生命周期、退款、争议等）
- 覆盖 Creem 全事件类型（通过交易查询 + 订阅状态查询组合对账）
- 补偿处理与 webhook 共享幂等保证（基于支付事件记录去重）
- 每次对账运行输出统计日志（拉取数、缺失数、补处理成功数、失败数）
- 数据不一致时写 Error 日志（含 realm_id、event_id、本地状态、provider 状态）

### 2.2 不包含功能 (Out of Scope)

- **报警通知**：不对数据不一致或补偿失败发送报警
- **自动数据修复**：不对检测到的不一致数据执行自动修复
- **手动触发补偿**：不提供管理界面手动触发对账
- **补偿状态管理界面**：不提供前端页面展示对账历史和结果
- **Creem Events API**：Creem 不提供事件列表 API，不在本功能范围内
- **新增数据库表/列**：使用现有支付事件和领域配置表，不新增迁移

### 2.3 依赖项

- **Worker 框架** — 已有的定时 Job 调度机制
- **Stripe Events API** — 支持时间范围和事件类型过滤，保留 30 天
- **Creem Transactions API** — 支付交易列表
- **Creem Subscription Search API** — 订阅状态列表
- **现有 Webhook 处理逻辑** — 事件路由和业务处理函数
- **支付事件记录** — 事件幂等判断的数据源

---

## 3. 需求概述

### 3.1 功能描述

当 Stripe/Creem 事件已发生但 webhook 未到达 Herald（网络故障、provider 侧发送失败、Herald 宕机等），系统没有主动发现和补处理的机制。本功能通过后台定时 Job 调用 provider API 拉取近期事件，与本地支付事件记录对比，对缺失的事件执行补偿处理。

这是现有 webhook 处理的可靠性增强，不引入新的业务逻辑或用户可见行为。

### 3.2 关键特性

- 按 realm 隔离，每个 realm 使用各自的 API key 独立对账
- Stripe 通过 Events API 直接拉取事件列表
- Creem 通过交易查询 + 订阅状态查询组合推导缺失事件
- 补偿处理复用 webhook 处理的业务逻辑，保持行为一致性
- 基于现有幂等机制防止重复处理
- 可观测的统计日志用于诊断

---

## 4. 业务规则与状态

### 4.1 业务规则

**补偿触发规则**：
- 定时执行，间隔可通过环境变量配置
- 仅对配置了 Stripe/Creem API key 的 realm 执行对账
- 串行遍历 realm，避免并发拉取造成 provider API 限流

**事件对比规则**：
- 以外部事件 ID 为唯一键对比 provider 事件与本地支付事件记录
- provider 侧存在但 Herald 缺失的事件 → 补处理
- provider 侧存在且 Herald 已存在 → 跳过（幂等）
- provider 侧存在但 Herald 状态不一致 → 写 Error 日志，不做自动修复

**补偿处理规则**：
- 补偿处理执行与 webhook handler 相同的业务逻辑（订阅同步、积分发放等）
- 跳过 HTTP 层逻辑（签名验证、Redis 幂等检查），仅依赖 DB 层支付事件记录做幂等判断
- 补偿处理失败不阻塞后续事件，记录失败日志后继续

**Stripe 对账规则**：
- 调用 Stripe Events API 按时间范围和事件类型过滤
- 分页拉取，控制每页大小以避免 rate limit
- Stripe Events API 事件默认保留 30 天，对账间隔远小于 30 天

**Creem 对账规则**：
- 使用交易查询 API 查询近期交易（支付、退款、争议）
- 使用订阅状态查询 API 查询订阅状态
- 从交易和订阅数据推导缺失的事件类型（与本地订阅状态对比）
- 如订阅状态查询不支持过滤参数，则全量拉取后本地过滤

**日志规则**：
- 每次对账运行记录统计日志：拉取事件数、缺失数、补处理成功数、补处理失败数
- 数据不一致时写 Error 日志，包含 realm_id、event_id、本地状态、provider 状态
- 补偿处理失败时写 Error 日志，包含事件详情和错误原因

### 4.2 关键状态与异常

| 场景 | 处理方式 |
|------|---------|
| Provider API 不可用 | 记录 Error 日志，跳过该 realm，继续处理其他 realm |
| Provider API rate limit | 控制请求频率和分页大小，必要时跳过等待下次运行 |
| 事件 payload 不完整 | 记录 Error 日志，跳过该事件，不阻塞后续 |
| 补偿处理业务失败 | 记录 Error 日志，继续处理后续事件 |
| 本地数据不一致 | 写 Error 日志（含详细信息），不做自动修复 |
| 无支付平台配置的 realm | 跳过，不报错 |

---

## 5. 功能需求

### 5.1 核心需求

**Stripe 事件补偿**：
- 定时调用 Stripe Events API 拉取上一个时间段的全部订阅/支付相关事件
- 按 realm 遍历，使用各自 API key
- 与本地支付事件记录对比，识别缺失事件
- 对缺失事件执行与 webhook 相同的业务处理

**Creem 事件补偿**：
- 定时调用 Creem 交易查询 API 拉取近期交易
- 定时调用 Creem 订阅状态查询 API 拉取订阅状态
- 从交易和订阅数据推导缺失事件类型
- 对缺失事件执行补偿处理

**幂等保证**：
- 补偿处理复用 webhook 的 DB 层幂等机制
- 已处理的事件（含 webhook 已处理和前次补偿已处理）自动跳过

**可观测性**：
- 每次运行输出结构化统计日志
- 数据不一致和补偿失败记录 Error 级别日志

### 5.2 验收目标

- Stripe 事件缺失时，补偿 Job 能在下一个周期内检测并补处理
- Creem 事件缺失时，补偿 Job 能通过交易 + 订阅组合检测并补处理
- 已通过 webhook 正常处理的事件不会被补偿 Job 重复处理
- 补偿处理后订阅状态、积分余额与直接通过 webhook 处理的结果一致
- 每次对账运行产生统计日志，包含拉取数、缺失数、成功数、失败数
- 数据不一致时产生 Error 日志，不触发自动修复或报警

---

## 6. API 相关约束

**适用性**: 不适用

本功能是后台定时 Job，不暴露任何 HTTP API。对账通过调用 provider 外部 API 实现，不涉及 Herald API 端点。

---

## 7. 前端/交互约束

**适用性**: 不适用

本功能无前端界面、无用户交互。所有行为通过后台 Job 自动执行，结果通过日志可观测。

---

## 8. 已确认决策

- **Error 日志优先**：遇到不一致数据时先写 Error 日志，不做报警和自动数据修复
- **无报警**：不发送报警通知，后续版本根据日志观测结果决定报警策略
- **无自动修复**：检测到不一致后仅记录日志，不做自动修复，后续版本考虑
- **复用 webhook 逻辑**：补偿处理复用现有 webhook 业务逻辑，不引入新的业务规则
- **DB 层幂等**：补偿路径跳过 Redis 幂等检查，仅依赖 DB 层支付事件记录做幂等
- **不新增数据库迁移**：使用现有支付事件和领域配置表
- **无前端**：不提供补偿状态管理界面和手动触发入口

---

## 9. 参考资料

- 用户故事：`docs/user-stories/billing/webhook-compensation.md`
- 相关 PRD：`docs/prd/billing/subscription.md`（Webhook 处理规则、幂等处理）
- 相关 PRD：`docs/prd/billing/stripe-payment.md`（Stripe 支付集成）
- 相关用户故事：`docs/user-stories/billing/payment-attempt.md`（单笔支付尝试补偿）
- 相关用户故事：`docs/user-stories/billing/entitlement-mapping.md`（Webhook metadata 映射）
