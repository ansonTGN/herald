# 积分系统 产品需求文档 (PRD)

**创建时间**: 2026-03-13
**状态**: Implemented
**优先级**: P0
**最后更新**: 2026-03-31

---
## 1. 相关用户故事

> **说明**: 详细的用户故事和验收标准请查看 `../../user-stories/` 目录中的对应文件。

### 1.1 用户故事

详细的用户故事和验收标准请查看 `../../user-stories/` 目录中的对应文件：

**Tenant Admin 积分管理**: [points-admin-manage.md](/docs/user-stories/points-admin-manage.md)
- US-PO-01: 配置积分套餐
- US-PO-02: 查看所有用户积分账户
- US-PO-03: 查看用户积分交易历史
- US-PO-04: 管理积分套餐配置
- US-PO-05: 查看套餐充值引导
- US-PO-06: 配置 Realm 默认积分策略（新增）
- US-PO-07: 查看免费用户积分统计（新增）

**Tenant User 积分查询**: [points-user-view.md](/docs/user-stories/points-user-view.md)
- US-PU-01: 查看我的积分余额
- US-PU-02: 查看我的交易历史
- US-PU-03: 筛选交易记录

**免费用户积分体验**: [points-free-user.md](/docs/user-stories/points-free-user.md)（新增）
- US-FU-01: 注册时获得初始积分（永久有效）
- US-FU-02: 定期自动获得免费积分（支持 once/daily/weekly/monthly）
- US-FU-03: 升级到付费套餐时保留注册初始积分

### 1.2 用户故事优先级汇总

| 优先级 | 用户故事数量 | 关键故事 |
|--------|------------|---------|
| P0 | 2 | US-PO-01, US-PU-01 |
| P1 | 3 | US-PO-02, US-PO-03, US-PU-02 |
| P2 | 3 | US-PO-04, US-PO-05, US-PU-03 |

---

## 2. 范围界定

### 2.1 包含功能

- ✅ 积分账户管理（创建、查询）
- ✅ 积分余额查询
- ✅ 积分消耗/扣除（SDK API）
- ✅ 积分充值（套餐兑换积分）
- ✅ 积分交易历史记录
- ✅ 积分套餐配置（套餐与积分兑换比例）
- ✅ 前端积分管理页面
- ✅ 前端积分充值页面
- ✅ 积分 SDK（供第三方应用调用消耗积分）
- ✅ **积分发放周期管理**（新增）：支持灵活的积分发放策略（一次性/每日/每周/每月）
- ✅ **积分有效期管理**（新增）：支持积分过期时间配置（永久有效/N天有效）
- ✅ **积分发放调度系统**（新增）：定时任务自动发放积分
- ✅ **免费用户积分系统**（新增）：注册初始积分、定期免费积分（支持 once/daily/weekly/monthly）
- ✅ **Realm 默认配置**（新增）：管理员配置免费用户的积分策略（含周期类型）

### 2.2 不包含功能 (Out of Scope)

- ❌ **积分转账/赠送** (原因: 暂不包含用户间积分转移)
- ❌ **积分提现** (原因: 积分仅供内部消费，不支持提现为现金)
- ❌ **积分等级/会员系统** (原因: 首版不实现积分等级体系)
- ❌ **积分商城** (原因: 商城系统不在本次实现范围内)

**注意**: 积分过期机制已纳入本版本需求（见 5.10 节），不再属于"不包含功能"。

### 2.3 依赖项

- ⚠️ **billing (订阅计费)** (状态: 部分实现)
  - 依赖原因: 需要与套餐系统打通，支持通过购买套餐获取积分
  - 影响范围: 积分充值、套餐配置

---

## 3. 需求概述

### 3.1 功能描述

积分系统是 Herald 多租户认证与授权系统中的虚拟货币子系统，为每个用户提供独立的积分账户管理能力。系统与订阅计费套餐深度集成，支持通过购买套餐定期自动获得积分，第三方应用可以通过授权 SDK 消耗积分来实现按次/按量计费场景。

当前 PRD 基线仍以 `Plan` 级积分配置为主。随着 Billing 编目向 `Realm -> Product -> Plan` 演进，Points 只要求与 Product 语义兼容；是否升级为 Product 分层积分规则，属于后续版本能力，不作为本 PRD 的当前必做范围。

### 3.2 关键特性

- **用户级别独立账户**: 每个用户（User）拥有独立的积分账户，支持个人积分管理
- **SDK 授权消费接口**: 提供基于 API Key 授权的 Rust SDK，供第三方应用安全调用积分消耗接口
- **套餐定期自动充值**: 支持配置套餐与积分的兑换比例，按订阅周期（月/年）自动充值积分
- **交易追溯**: 完整的积分交易历史记录，支持审计和对账
- **管理界面**: 提供租户管理员管理积分套餐、查看积分报表的功能
- **用户自助查询**: 用户可查询个人积分余额和交易历史

---

## 4. 当前实现状态

| 功能模块 | 状态 | 备注 |
|---------|------|------|
| 积分交易记录数据模型 | ✅ | 已完整实现（包含账本、消费分摊、回收记录等） |
| 积分充值 API（套餐兑换） | ✅ | 已实现（Webhook 处理器 + Service 层） |
| 积分套餐配置管理 API | ✅ | 已实现（CRUD + Realm 配置） |
| **前端积分管理页面** | ✅ | **已完整实现**（管理员和用户页面） |
| **前端积分充值页面** | ✅ | **已完整实现**（套餐配置和管理） |
| **积分系统 Demo 测试** | ✅ | **已完整实现**（847行管理员测试 + 用户测试） |

**实现说明**：

- ✅ **后端数据库表完整**：
  - `points_accounts`, `points_transactions`, `points_credit_ledger`
  - `points_consumption_allocations`, `points_revocation_records`
  - `points_plan_configs`, `realm_default_configs`, `user_points_configs`
  - `points_grant_schedules`, `points_grant_records`, `idempotency_keys`

- ✅ **Domain 层完整**：`backend/core/src/domain/points/` 包含完整的实体、服务、DTOs、端口定义
  - 账户查询、交易历史、套餐配置、Realm 配置、用户配置等

- ✅ **前端已完整实现**：
  - 管理员积分管理页面：`frontend/src/routes/$realmId/manage/points.tsx`
  - 用户积分查询页面：`frontend/src/routes/$realmId/user/points.tsx`
  - 数据层 fixtures：`frontend/src/fixtures/points-*.fixture.ts`
  - 表单 schemas：`frontend/src/lib/schemas/points-forms.ts`
  - MSW mock handlers：`frontend/src/test/mocks/handlers/points.ts`

- ✅ **测试已完整实现**：
  - 管理员测试：`demo/e2e/billing-admin/points-admin-comprehensive-demo.e2e.ts`（847 行，覆盖 US-PO-01 ~ US-PO-07）
  - 用户测试：`demo/e2e/regular-user/points-user-comprehensive-demo.e2e.ts`（覆盖 US-PU-01 ~ US-PU-03）
  - 测试辅助工具：`demo/e2e/helpers/points-helpers.ts`, `points-constants.ts`

**实现完整性**：✅ 积分系统已完整实现，包括后端、前端、测试

---

## 5. 功能需求

### 5.1 积分账户管理

每个用户拥有独立的积分账户，积分余额以整数形式存储，余额单位固定为 `points`（积分），不使用法币 `currency` 表示。账户与 User 实体一对一关联，支持 realm 级别隔离。

**功能点**:
- 用户创建时自动初始化积分账户
- 积分账户关联 User ID 和 Realm ID，支持多租户隔离
- 支持查询当前积分余额
- 积分账户和余额接口返回 `unit = "points"` 表示余额单位
- 支持账户状态管理（正常/冻结/关闭）

### 5.2 积分消耗（SDK）

第三方应用通过授权 SDK 消耗积分，实现按次计费、按量计费等场景。

**功能点**:
- SDK 提供消耗积分的异步接口，需要 API Key 授权
- 使用 Identity::ThirdParty(ClientApiKey) 机制进行认证
- 支持原子性扣减积分（数据库乐观锁，防止超扣）
- 积分不足时返回明确错误
- 每次 SDK 调用记录交易历史（包含：交易 ID、用户 ID、Client App ID、消耗数量、时间戳、说明）

**认证要求**:
- SDK 调用必须携带有效的 API Key（X-API-Key header）
- API Key 必须属于目标 realm
- 仅 ThirdParty 身份可调用消耗接口
- 严格检查 realm 访问权限，防止跨 realm 消耗

**SDK 接口示例**:

#### 5.2.1 异步任务积分补偿

**业务场景**

某些业务场景采用"预扣费"模式：用户发起异步任务（如图片生成、视频处理、AI 推理等）时先消耗积分，任务执行失败后需要退回已消费的积分。

**典型流程**:
1. 用户发起异步任务
2. 系统调用 `consume_points` 预扣积分（如 100 积分）
3. 异步任务开始执行...
4. 如果任务失败，系统调用补偿接口退回积分
5. 用户积分余额恢复

**补偿机制**

使用现有的 `grant_points_internal` 方法实现积分补偿，无需新增 API：

**关键特性**:

- ✅ **关联追溯**: 通过 `external_ref_id = "refund:{transaction_id}"` 关联原始消费交易
- ✅ **类型一致**: 补偿积分的类型与原消费类型一致（topup_credit / subscription_credit）
- ✅ **过期继承**: 补偿积分的过期时间与原积分保持一致
- ✅ **幂等性**: 检查是否已存在相同 `external_ref_id` 的补偿记录，避免重复退回
- ✅ **审计完整**: 交易历史清晰记录原始消费和补偿的对应关系

**幂等性处理**:

**审计要求**:

- 所有补偿操作必须创建交易记录
- 交易记录的 `description` 应包含失败原因
- 通过 `external_ref_id` 可追溯到原始消费交易
- 支持按交易类型筛选查询补偿记录

**实现位置**:

- **核心方法**: `backend/core/src/domain/points/service.rs:grant_points_internal`
- **调用方**: 异步任务处理逻辑（由业务方实现）
- **交易类型**: `system_grant`（CreditSourceType）
- **参考用户故事**: `docs/user-stories/points-billing-events.md` - US-PO-08

**注意事项**:

1. **积分类型匹配**: 补偿时必须使用原消费的积分类型，避免混乱
2. **过期时间处理**: 如果原积分有过期时间，补偿积分应继承相同的过期时间
3. **部分失败场景**: 如果任务部分成功，可按比例退回积分
4. **业务方责任**: 判断任务失败并调用补偿接口是业务方的责任

### 5.3 积分充值（套餐兑换）

与订阅计费套餐深度集成，支持灵活的积分发放策略和有效期管理。

**当前边界说明**：

Billing 引入 Product 后，Points 需要保证语义上能够承接 `Product -> Plan` 的新编目关系；但当前正式配置维度仍以 Plan 为主，不在本 PRD 中要求同步落地 `Product + Plan + Client App` 的多层规则体系。

**功能点**:
- 支持配置积分发放周期类型（`grant_period_type`）：once / daily / weekly / monthly
- 支持配置每次发放积分数（`points_per_period`）
- 支持配置积分有效期（`validity_days`）：0=永久有效，>0=N天有效
- 支持订阅时立即发放（`grant_on_subscribe`）
- 支持设置最大发放期数（`max_periods`）：null=无限期发放
- 订阅创建时触发首次充值（`subscription.paid` webhook）
- 定时任务按周期自动发放积分
- 为未来 Product 级规则扩展保留语义空间，但不要求当前版本落地

**充值场景**:
- 一次性发放（once）：订阅时发放一次，不创建调度
- 每日发放（daily）：按订阅时间点每天同一时间发放
- 每周发放（weekly）：按订阅时间点每7天发放一次
- 每月发放（monthly）：按订阅日期每月同一天发放（月末特殊处理）

**发放周期计算规则**:
- **once**: 订阅时发放一次，不创建调度
- **daily**: 按订阅时间点计算，每天同一时间发放（如订阅于 15:30，则每天 15:30 发放）
- **weekly**: 按订阅时间点计算，每7天发放一次（如订阅于周一 15:30，则每7天后的 15:30 发放）
- **monthly**: 按订阅日期计算，每月同一天发放（如订阅于3月23日，则每月23日发放；特殊：若订阅于31日，则月末月份的最后一天发放）

**有效期计算规则**:
- **validity_days = 0**: 永久有效（expires_at = null）
- **validity_days = 1**: 发放后24小时过期
- **validity_days = 7**: 发放后7天过期
- **validity_days = 30**: 发放后30天过期

计算公式:

**Webhook 集成**:
- `subscription.paid` 事件：处理首次订阅积分充值，创建发放调度
- 定时任务：按周期扫描待发放调度，执行积分发放

### 5.4 积分交易历史查询

租户管理员可以查询所有用户的积分交易历史，普通用户仅能查询自己的交易历史。

**功能点**:
- 支持按时间范围筛选
- 支持按交易类型筛选（充值、消耗）
- 支持按用户 ID 筛选（仅管理员可用）
- 支持按 Client App ID 筛选（追踪消耗来源）
- 分页查询支持
- 权限隔离：用户只能查询自己的记录，管理员可查询全租户记录

### 5.5 积分套餐配置管理

租户管理员配置每个套餐对应的积分赠送规则和发放策略。

**功能点**:
- 配置积分发放周期类型（`grant_period_type`）：once / daily / weekly / monthly
- 配置每次发放积分数（`points_per_period`）
- 配置积分有效期（`validity_days`）：0=永久有效，>0=N天有效
- 配置订阅时是否立即发放（`grant_on_subscribe`）
- 配置最大发放期数（`max_periods`）：null=无限期发放
- 支持修改配置（仅影响新订阅和续费，不影响历史记录）

**配置示例**:

**月付套餐 + 每日发放 + 当天有效**:

**工作流程**:
1. 用户订阅成功（15:30）
2. Webhook 触发首次发放（100积分，过期时间=明天15:30）
3. 创建调度记录（next_grant_time = 明天15:30）
4. 定时任务每天15:30执行，发放100积分
5. 昨天的积分自动过期

**年付套餐 + 一次性发放 + 永久有效**:

### 5.5.1 Product 兼容约束（后续扩展预留）

随着 Product 概念的引入，Points 需要兼容新的 Billing 编目语义，但当前版本不将 5 层积分规则系统定义为 P0 核心范围。

当前约束如下：

1. 当前正式配置对象仍是 Plan。
2. Product 的引入不改变现有积分字段语义，如首次发放、续费发放、周期类型、有效期等。
3. 当 Plan 归属于 Product 后，积分规则查询和展示需要能够正确关联到所属 Product 上下文。
4. 如果未来出现 Product 级默认规则、按 Client App 差异化规则、规则优先级覆盖和冲突检测需求，应在后续 PRD 版本中单独升级。

**后续可扩展方向**：

- Product 级默认积分规则
- Product + Plan 组合规则
- Product + Plan + Client App 差异化规则
- 多层规则优先级匹配
- 冲突检测与规则预览

### 5.5.2 免费用户积分系统（P0 核心 - 新增）

免费用户积分系统为未付费用户提供基础的积分体验，包括注册时的初始积分赠送和每日自动积分发放。

**功能点**:
- **注册初始积分**：用户注册时自动获得一次性积分（永久有效）
- **定期免费积分**：免费用户按配置周期自动获得积分（支持 once/daily/weekly/monthly，不可累积）
- **Realm 默认配置**：管理员配置免费用户的积分策略（含周期类型）
- **独立于订阅系统**：免费用户不需要创建订阅记录
- **升级平滑过渡**：免费用户升级到付费套餐时，注册初始积分保留

**核心思想**:
- 免费用户**不需要创建订阅**，直接使用独立的积分配置
- 通过 `user_points_configs` 表管理免费用户积分
- 完全独立的积分发放调度系统，支持多种周期类型
- 管理员通过 Realm 配置控制免费用户的积分策略

**架构优势**:
- ✅ 简单直接，无需订阅系统参与
- ✅ 避免 $0 订阅污染订阅表
- ✅ 性能更好（无需查询订阅表）
- ✅ 业务语义清晰（"订阅"仅代表付费关系）
- ✅ 灵活配置周期类型，与付费积分统一语义

**业务规则**:
- 注册初始积分永久有效（`validity_days = 0`）
- 积分类型为 `registration_credit`（新增类型）
- 升级到付费套餐后保留，不受订阅变更影响
- 定期积分按配置有效期（`validity_days`），通常为 1/7/30 天
- 不累积：上一期未用完的积分在本期过期，不累积到下一期
- 积分类型为 `free_periodic_credit`（新增类型，原 free_daily_credit）
- 支持 4 种周期类型：once/daily/weekly/monthly

**发放流程**:

**免费用户升级规则**:
- 注册初始积分保留（`registration_credit`，永久有效）
- 定期积分立即回收（`free_periodic_credit`，升级时全部回收）
- 不删除 `user_points_configs` 记录（保留历史）
- 停用定期积分调度（设置 `free_periodic_points_amount = 0`）

**详细 PRD**: 参见 `docs/prd/billing/points-free-user.md`

### 5.6 积分类型分离（P0 核心）

系统必须支持两种积分类型的分离管理，这是积分系统的核心设计原则。

#### 5.6.1 积分类型定义

**充值积分（topup_credit）**：
- 用户主动购买获得
- 长期有效，默认不过期（除非产品配置特殊规则）
- 可用于所有消费场景

**会员积分（subscription_credit）**：
- 通过订阅套餐获得
- 按订阅周期发放，周期结束自动过期
- 优先于充值积分消费

**注册初始积分（registration_credit）**（新增）：
- 用户注册时自动获得
- 永久有效（`expires_at = null`）
- 升级到付费套餐后保留，不受订阅变更影响
- 每个用户只能获得一次

**免费定期积分（free_periodic_credit）**（新增）：
- 免费用户按配置周期自动获得（once/daily/weekly/monthly）
- 按配置有效期（通常为 1/7/30 天）
- 不累积：上一期未用完的积分在本期过期
- 升级到付费套餐后立即回收

#### 5.6.2 数据模型要求

**强制要求**：
- 数据库必须存在 `credit_type` 字段区分积分类型
- 所有积分发放、消费、回收操作必须明确指定积分类型
- 账户余额必须按类型分别显示和统计

**数据存储**：
- `points_credit_ledger` 表的 `credit_type` 字段：topup_credit / subscription_credit / registration_credit / free_periodic_credit
- `points_accounts` 表的聚合字段：`topup_balance`、`subscription_balance`、`registration_balance`、`free_periodic_balance`
- `points_transactions` 表的 `credit_type` 字段：记录每笔交易涉及的积分类型

#### 5.6.3 业务规则

**发放规则**：
- 订阅首次赠送：发放 `subscription_credit`
- 订阅续费：发放 `subscription_credit`
- 订阅升级补差：发放 `subscription_credit`
- 直接充值购买：发放 `topup_credit`
- 用户注册：发放 `registration_credit`（新增）
- 免费用户定期发放：发放 `free_periodic_credit`（新增，支持 once/daily/weekly/monthly）

**查询规则**：
- 用户查询余额时，必须显示两种类型的分别余额
- 管理员查询账户时，必须显示按类型分桶的积分明细

### 5.7 积分消费优先级（P0 核心）

当用户同时拥有多种类型的积分时，系统必须按照过期时间优先消费即将过期的积分。

#### 5.7.1 消费优先级规则

**强制规则**：按过期时间优先消费
- 查询所有可用积分（WHERE status = 'active' AND remaining_amount > 0）
- 按 `expires_at` 升序排序（NULL 排最后，表示永久有效）
- 即将过期的积分优先消费

**优势**：
- 逻辑简单，不需要复杂的类型判断
- 自动适配所有积分类型（包括未来新增类型）
- 符合用户利益（优先消费即将过期的积分，避免损失）

**示例**：
- 用户余额：free_periodic_credit（今天过期）= 50，subscription_credit（下月过期）= 500，registration_credit（永不过期）= 1000，topup_credit（永不过期）= 1000
- 消费 300 积分：先扣除 free_periodic_credit 的 50，再扣除 subscription_credit 的 250
- 消费 1500 积分：先扣除 free_periodic_credit 的 50，再扣除 subscription_credit 的 500，再扣除 registration_credit 的 950

#### 5.7.2 实现要求

**原子性要求**：
- 单次消费必须原子性地完成跨类型的积分扣减
- 不允许部分扣减导致数据不一致

**审计要求**：
- 每次消费必须记录 `points_consumption_allocations` 表
- 记录消费了哪些 `ledger` 项，每个 `ledger` 项消费了多少
- 支持审计追溯：某次消费消耗了哪些来源的积分

**API 要求**：
- 不需要调用方指定消费哪种类型的积分
- 系统根据可用余额自动计算消费分摊

### 5.8 退款积分回收（P0 核心）

当支付平台处理退款时，Herald 必须执行积分回收操作。

#### 5.8.1 责任界定

**支付平台（Creem）职责**：
- 处理金额退款
- 发送 `refund.created` webhook 事件

**Herald（业务系统）职责**：
- 接收 `refund.created` webhook 事件
- 根据退款金额回收未使用的积分
- 记录积分回收审计日志

#### 5.8.2 退款积分回收规则

**充值退款（topup_credit 退款）**：
- 按未使用比例回收 `topup_credit`
- 已使用部分不回收
- 优先回收最晚获得的充值积分（FIFO）

**会员退款（subscription_credit 退款）**：
- 仅回收未使用的 `subscription_credit`
- 已使用部分不回收
- 不回收充值积分

**示例**：
- 充值 1000 积分，使用 300 积分，剩余 700 积分
- 退款 50%（500 积分对应金额）
- 回收计算：500 × (700/1000) = 350 积分
- 最终剩余：1000 - 300 - 350 = 350 积分

#### 5.8.3 实现要求

**Webhook 处理**：
- 接收并处理 `refund.created` 事件
- 根据退款金额计算应回收积分数量

**回收操作**：
- 更新 `points_credit_ledger` 表：增加 `revoked_amount`，更新 `status`
- 创建 `points_revocation_record` 记录

**幂等性**：
- 使用 Stripe 事件的 `id` 作为幂等键
- 重复事件不重复回收积分

### 5.9 订阅生命周期积分处理（P1）

当用户的订阅发生变更时，积分系统需要处理相应的积分发放和回收。

#### 5.9.1 订阅升级

**规则**：升级立即生效，回收老积分，发放新积分

**积分处理**：
- 回收旧套餐的所有 `subscription_credit` 积分（包括一次性发放和周期性发放的积分）
- 发放新套餐的积分（根据新套餐配置：`grant_period_type`, `points_per_period`, `validity_days`）
- 积分过期周期重算（从升级时刻重新计算）
- 注册初始积分（`registration_credit`）保留，不受影响

**示例**：
- 用户从 basic（一次性 500 积分，永不过期）升级到 pro（每日 100 积分，当天有效）
- 回收 basic 的 500 积分
- 发放 pro 的 100 积分（有效期从升级时刻重新计算）
- 注册初始积分 1000 保留

**业务价值**：
- 逻辑清晰：老套餐积分清零，新套餐积分重新开始
- 避免积分混乱：不同套餐的积分规则完全隔离
- 过期时间准确：新积分的过期时间从升级时刻计算

**实现要求**：
- Webhook 事件：`subscription.update`
- 判断 `previous_attributes.plan_id` 与 `current.plan_id`
- 回收老套餐的 `subscription_credit`（调用 `revoke_points_by_credit_type`）
- 创建新的积分发放调度（根据新套餐配置）
- 发放新套餐的首次积分

#### 5.9.2 订阅降级

**规则**：降级下周期生效，不回收当前周期已发积分

**积分处理**：
- 当前周期继续享受原套餐积分
- 当前周期已发积分不回收
- 下周期按新套餐发放积分

**示例**：
- 用户从 pro（1000 积分）降级到 basic（500 积分）
- 当前周期继续使用 1000 积分
- 下周期发放 500 积分

**实现要求**：
- Webhook 事件：`subscription.update`
- 检查 `cancel_at_period_end` 标志
- 不执行积分回收操作

#### 5.9.3 订阅取消

**规则**：区分默认取消和立即取消

**默认取消（周期结束）**：
- 当前周期继续有效
- 当前已发积分保留
- 下周期不再发放会员积分
- 周期结束后，未使用的会员积分过期

**立即取消**：
- 仅回收未使用的会员积分
- 不回收充值积分
- 已使用会员积分不回收

**示例**：
- 用户有 1000 会员积分（未使用），500 充值积分
- 立即取消订阅
- 回收 1000 会员积分
- 保留 500 充值积分

**实现要求**：
- Webhook 事件：`subscription.canceled`
- 检查取消模式（立即取消 / 周期结束）
- 立即取消：调用积分回收接口
- 周期结束：设置会员积分的 `expires_at` 为周期结束时间

### 5.10 积分过期机制（P1）

会员积分需要在订阅周期结束后自动过期，充值积分默认长期有效。

#### 5.10.1 过期规则

**会员积分（subscription_credit）**：
- 发放时设置 `expires_at` 为当前周期结束时间
- 周期结束后自动过期
- 过期后 `status` 更新为 `expired`

**充值积分（topup_credit）**：
- 默认长期有效，`expires_at` 为 `null`
- 产品可以配置充值积分有效期（可选）

#### 5.10.2 过期处理

**定时任务**：
- 每小时扫描 `points_credit_ledger` 表
- 找到 `expires_at < now` 且 `status = active` 的记录
- 更新 `status = expired`
- 创建 `points_revocation_record` 记录

**用户体验**：
- 用户可查看即将过期的积分
- 过期前 7 天、3 天、1 天发送通知（如果通知系统已实现）

### 5.11 Stripe Webhook 事件处理（P0 核心）

系统必须处理以下 Stripe Webhook 事件，确保积分发放和回收的准确性。

#### 5.11.1 必须处理的事件

| 事件名称 | 触发时机 | 积分操作 |
|---------|---------|---------|
| `subscription.paid` | 订阅支付成功 | 发放首次订阅积分 |
| `subscription.paid` | 续费支付成功 | 发放续费积分 |
| `subscription.update` | 订阅变更 | 处理升级/降级积分 |
| `subscription.canceled` | 订阅取消 | 处理取消积分回收 |
| `refund.created` | 退款发生 | 回收未使用积分 |

#### 5.11.2 事件处理规则

**subscription.paid（首次订阅）**：
- 判断是否为首次订阅
- 根据 `plan_id` 查询积分配置
- 发放 `subscription_credit`（首次订阅积分）
- 幂等键：`subscription.id + paid_timestamp`

**subscription.paid（续费）**：
- 判断是否为续费
- 根据 `plan_id` 查询积分配置
- 发放 `subscription_credit`（续费积分）
- 幂等键：`subscription.id + paid_timestamp`

**subscription.update**：
- 判断是升级还是降级
- 升级：计算差额并发放 `subscription_credit`
- 降级：记录事件，不立即处理
- 幂等键：`subscription.id + update_timestamp`

**subscription.canceled**：
- 判断是立即取消还是周期结束
- 立即取消：回收未使用的 `subscription_credit`
- 周期结束：设置 `expires_at` 为周期结束时间
- 幂等键：`subscription.id + canceled_timestamp`

**refund.created**：
- 根据退款金额计算应回收积分
- 按未使用比例回收 `topup_credit` 或 `subscription_credit`
- 创建积分回收记录
- 幂等键：`refund.id + created_timestamp`

#### 5.11.3 幂等性设计

**幂等键生成**：
- 使用 Stripe 事件的唯一 ID + 时间戳
- 存储在 `points_transactions.external_ref_id` 字段

**幂等性检查**：
- 处理事件前检查 `external_ref_id` 是否已存在
- 如果存在，跳过处理

---

## 6. API 相关约束

**状态**: 必填

- 仅说明计费、套餐、积分、支付配置、订阅变更或 webhook 处理的能力边界，不在 PRD 中列出端点、schema 或状态码细节。
- 必须遵守 realm 隔离、管理员权限、金额与积分变更可追溯、回调幂等和失败补偿要求。
- 与支付平台、积分账本、订阅系统的详细契约应下沉到技术设计、接口说明或实现代码。

---

## 7. 前端/交互约束

**状态**: 必填

- 仅保留管理入口、关键操作路径、筛选/查看/变更的交互约束和状态反馈，不写组件实现、数据层封装或代码结构。
- 计费与积分场景必须突出金额/积分变化、变更影响范围、不可逆风险提示和回调同步中的状态说明。

---

## 8. 技术设计承接

**状态**: 必填

- 接口细节、数据库结构、迁移策略、类型定义、调度方案、SDK 设计和实现步骤，应在 `docs/design/`、`.ai/design/`、接口说明或代码中承接。
- 如历史实现已经存在，应以现有设计文档、OpenAPI、迁移文件和代码为依据补充，不回写到 PRD 正文。

---

## 9. 相关文件索引

### 9.1 后端文件

| 文件路径 | 状态 | 说明 |
|---------|------|------|
| `backend/core/src/domain/points/` | ❌ | 积分领域模型 |
| `backend/core/src/services/points_service.rs` | ❌ | 积分业务逻辑 |
| `backend/sdk/src/points.rs` | ❌ | 积分 SDK 模块 |

### 9.2 前端文件

| 文件路径 | 状态 | 说明 |
|---------|------|------|
| `frontend/src/routes/points/index.tsx` | ❌ | 积分管理页面 |
| `frontend/src/routes/points/recharge.tsx` | ❌ | 积分充值页面 |
| `frontend/src/routes/user/points.tsx` | ❌ | 用户积分页面 |
| `frontend/src/components/points/` | ❌ | 积分相关组件 |

---

## 10. 参考资料

- **PRD**: `docs/prd/billing.md` - Billing 订阅计费产品需求文档（依赖）
- **PRD**: `docs/prd/billing/points-free-user.md` - 免费用户积分系统产品需求文档（新增）
- **用户故事**: `docs/user-stories/points-admin-manage.md` - Tenant Admin 积分管理用户故事
- **用户故事**: `docs/user-stories/points-user-view.md` - Tenant User 积分查询用户故事
- **用户故事**: `docs/user-stories/points-free-user.md` - 免费用户积分用户故事（新增）
- **实施计划**: `.ai/future/credits_plan_split.md` - 积分系统解耦实施计划
- **规范**: `spec/backend/development.md` - 后端开发规范
- **规范**: `spec/frontend/development.md` - 前端开发规范
