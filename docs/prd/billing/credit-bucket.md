# Credit Bucket 产品需求文档 (PRD)

**创建时间**: 2026-06-17
**优先级**: P0

---

## 1. 相关用户故事

> 详细故事与验收标准请查看 `docs/user-stories/` 中对应文档。

### 1.1 本特性新增故事

`docs/user-stories/billing/credit-bucket.md`

| US-ID | 标题 | 角色 | 优先级 |
|-------|------|------|--------|
| US-CB-001 | 管理 Credit Bucket 目录 | Realm Admin | P0 |
| US-CB-002 | 为 Bucket 绑定 Client App 覆盖集 | Realm Admin | P0 |
| US-CB-003 | 将套餐/积分包归属到 Bucket | Realm Admin | P0 |
| US-CB-004 | 购买 Bucket 套餐/积分包 | Regular User | P0 |
| US-CB-005 | 查看按 Bucket 分组的积分余额 | Regular User | P0 |
| US-CB-006 | 查看 Bucket 维度的交易历史 | Regular User | P1 |
| US-CB-007 | SDK 按 Client App 跨 Bucket 消费 | Third-Party App | P0 |
| US-CB-008 | 订阅生命周期按 Bucket 池发放与回收 | Herald 系统 | P0 |

### 1.2 既有相关故事（行为变更，随本特性调整）

- `docs/user-stories/billing/points-admin.md` — US-PO-001 / US-PO-006 / US-PO-008：积分配置与发放变为 Bucket 维度；主动发放需选择目标 Bucket（必选，无默认值）
- `docs/user-stories/billing/points-user.md` — US-PU-001 ~ US-PU-004：余额与交易历史增加 Bucket 维度
- `docs/user-stories/billing/points-free-user.md` — US-FU-001 ~ US-FU-003：注册/免费积分进入被指定为注册接收池的 Bucket（Bucket 级配置，非默认 Bucket）
- `docs/user-stories/billing/points-package-purchase.md` — US-PU-006 ~ US-PU-008：积分包购买按归属 Bucket 入账
- `docs/user-stories/integration/sdk.md` — US-TP-017：SDK 发放积分进入指定 Bucket 池
- `docs/user-stories/billing/entitlement-mapping.md` — US-EM-001 ~ US-EM-006：Entitlement Mapping 增加归属 Bucket
- `docs/user-stories/billing/payment-attempt.md` — US-PA-003：支付履约按 Bucket 路由入账

### 1.3 优先级汇总

| 优先级 | 数量（新增） | 关键故事 |
|--------|------|----------|
| P0 | 7 | US-CB-001 ~ US-CB-005, US-CB-007, US-CB-008 |
| P1 | 1 | US-CB-006 |

---

## 2. 范围界定

### 2.1 包含功能

- Realm 级 Credit Bucket 目录：创建、编辑、启用/禁用、展示顺序、注册积分接收池标记
- Bucket ↔ Client App 覆盖集：声明每个 Bucket 覆盖的 Client App，作为消费授权范围
- 套餐/积分包（Entitlement Mapping）归属 Bucket：购买对象与积分策略仍以 Mapping 为基座，增加强制的 Bucket 归属
- 每用户每 Bucket 独立积分池：替换单一钱包，一个用户可同时持有多 Bucket 独立池
- 按 Client App 的跨池消费：消费时按应用反查覆盖该应用的 Bucket 池，跨池按过期时间优先扣减
- 购买/订阅履约 Bucket 路由：支付成功后积分进入 `(user, bucket)` 池；订阅记录绑定 Bucket 作为续费/取消/退款回收的真源
- 注册积分接收池：Realm 内可指定一个 Bucket 为注册/免费定期系统发放积分的接收池（Bucket 自身配置，**非默认 Bucket**，不自动覆盖全部应用）；SDK/管理员主动发放需显式指定目标 Bucket，无隐式池解析
- 订阅生命周期（升级/降级/取消/退款）按订阅绑定 Bucket 池发放与回收
- 按 Bucket 维度的余额与交易历史展示（用户与管理员视角）
- 管理后台 Bucket 目录管理（Bucket、覆盖应用、归属套餐、积分策略、启用状态）

### 2.2 不包含功能 (Out of Scope)

- Bucket 池之间的合并与转移（v1 各池相互独立，不支持）
- 用户可同时持有 Bucket 数量的硬上限（v1 不设上限）
- Bucket 内维护价格、Provider 商品目录或积分策略（积分策略仍归属 Entitlement Mapping，不在 Bucket 上）
- 按 Client App 差异化的积分规则/优先级覆盖（沿用既有"过期时间优先"统一规则）
- 跨 Bucket 池的存量历史数据兼容（承接"项目未上线"前提，采用破坏性重建，不做正式历史回填）
- 积分转账/赠送、提现、积分等级/商城等既有 Out of Scope 项继续不包含

### 2.3 依赖项

- **积分系统核心**：复用积分账本、过期时间优先消费分摊、交易记录；将单钱包泛化为多池（见 `docs/prd/billing/points.md`）
- **订阅与 Entitlement Mapping（`docs/prd/billing/subscription.md`）**：购买对象仍为 Mapping；Mapping 强制归属 Bucket，订阅记录绑定 Bucket 作为生命周期回收真源
- **支付与 Webhook 履约**：复用 checkout、webhook 履约与幂等框架，补充 Bucket 路由
- **Client App 与 API Key 作用域（`docs/prd/integration/client-app.md`）**：消费授权沿用 `client_app_scope` 第一层授权，Bucket 覆盖集作为第二层池过滤
- **用户注册系统**：注册初始积分入注册积分接收池 Bucket

---

## 3. 需求概述

### 3.1 功能描述

当前积分模型采用**按 Bucket 多池**组织（替代原单钱包模型）：

- 一个 Realm 下可定义多个 Credit Bucket（Realm 级目录）。
- 每个 Bucket 对应一个独立积分池，并声明它覆盖的 Client App 集合作为消费授权范围。
- 用户购买该 Realm 内任意 Bucket 对应的套餐/积分包后，获得该 Bucket 的积分池；同一用户可同时持有多个 Bucket 独立池并存。
- SDK 按 Client App 消费时，自动从覆盖该应用的全部 Bucket 池跨池扣减。
- 所有积分都归属到 Bucket 池；注册/免费定期系统发放积分归入被指定为注册接收池的 Bucket，SDK/管理员主动发放按显式指定的目标 Bucket 入账。

本特性是对原单钱包模型的**替换**（重构取代），而非与单钱包并存；承接"项目未上线"前提，不考虑兼容性，采用破坏性重建。

### 3.2 关键特性

- **Bucket 目录与消费隔离**：Bucket 是积分池目录单位与按应用消费隔离的边界
- **多池并存**：同一用户可同时持有多个 Bucket 独立积分池
- **按应用消费隔离**：能否消费某池由该池所属 Bucket 是否覆盖当前 Client App 决定
- **统一池模型**：一切积分皆为池；注册/免费定期积分入注册接收池 Bucket，主动发放按显式 Bucket 入账，无隐式池解析
- **购买/订阅按池路由**：履约与生命周期回收始终回到订阅/购买对象绑定的 Bucket 池
- **Bucket 与套餐解耦**：Bucket 不维护价格或积分策略，只通过 Mapping 归属路由

---

## 4. 业务规则与状态

### 4.1 业务规则

**Bucket 目录**：
- Realm 维护一个 Bucket 目录，可定义多个 Bucket；**无"默认 Bucket"概念**
- 每个 Bucket 有名称、可选展示顺序、启用状态、是否为注册积分接收池
- 同一 Realm 同时只有一个注册积分接收池
- Bucket 不维护价格、Provider 商品或积分策略，仅作为池目录与消费隔离边界

**Bucket ↔ Client App 覆盖集**：
- 每个 Bucket 声明它覆盖的 Client App 集合
- 一个 Bucket 必须至少关联一个 Client App 才能被消费；无覆盖应用的 Bucket，其池内积分不可被任何应用消费
- 一个 Client App 可被多个 Bucket 覆盖；一个 Bucket 可覆盖多个 Client App（多对多）
- 调整某 Bucket 的覆盖集仅影响后续消费资格，不回收已持有池的余额

**套餐/积分包归属 Bucket**：
- 可购买套餐/积分包（Entitlement Mapping）**必须归属唯一 Bucket**，创建与编辑时均需显式绑定（必填）；不存在"未归属"态，因此不存在"未归属但存在"的套餐/积分包
- 一个 Bucket 可归属多个套餐/积分包
- 积分策略（每次发放量、发放周期、有效期、是否订阅即发等）仍归属 Mapping，不在 Bucket 上
- `entitlement_key` 仍是权益业务标识，不作为 Bucket 标识

**每用户多 Bucket 独立池**：
- 每个用户对每个持有的 Bucket 拥有一个独立积分池（`user × bucket`）
- 同一用户可同时持有多个 Bucket 池并存，余额互相独立
- 各 Bucket 池之间不可合并或转移（v1 不支持）
- 持有 Bucket 数量无硬上限（v1）

**积分类型与池归属**：
- 充值积分（topup_credit）、会员积分（subscription_credit）：购买/订阅获得，进入对应归属 Bucket 池
- 注册初始积分（registration_credit）、免费定期积分（free_periodic_credit）：进入该 Realm 注册积分接收池 Bucket
- 发放积分（granted_credit）：SDK/管理员主动发放按显式指定 Bucket 入账（无默认 Bucket，无隐式解析）
- 各积分类型的过期、消费优先级、退款回收规则沿用既有积分规则（见 `docs/prd/billing/points.md`），仅"池归属"维度变为 Bucket

**按应用跨池消费**：
- SDK 按 Client App 消费时，系统反查该用户名下"覆盖该应用"的全部 Bucket 池，按过期时间升序（永久有效排最后）跨池分摊扣减
- 跨多池消费必须原子完成，不允许部分池扣减成功导致总额不一致或超扣
- 无任何覆盖该应用的可用池、或覆盖池合计余额不足时，拒绝消费并返回明确提示
- 应用越权（Bucket 未覆盖当前应用）时拒绝消费，相关池不被扣减

**购买/订阅履约与生命周期**：
- 购买创建时即从归属 Mapping 解析目标 Bucket 并固化为购买/订阅的 Bucket 归属；支付成功后，积分进入该 Bucket 池，而非无差别用户钱包
- 订阅记录绑定 Bucket，作为续费、升级、降级、取消、退款的回收真源
- 续费：新一期积分进入同一 Bucket 池
- 升级：在该 Bucket 池内回收旧套餐会员积分并发新套餐积分；注册初始积分保留
- 降级：当前周期已发积分保留，下周期按新套餐发放到同一 Bucket 池
- 取消：仅在该 Bucket 池内回收未使用会员积分，不影响其他 Bucket 池
- 退款：按未使用比例在该 Bucket 池内回收对应类型积分
- 路由解析异常时 fail loud 拒绝入账，不能默认到错误池（解析在购买创建时已固化，fail loud 作为防御性兜底）

**余额与历史查询**：
- 用户余额按 Bucket 分组展示各池余额（按积分类型分桶），并提供跨全部 Bucket 的合计
- 交易历史每条记录归属其 Bucket，支持按 Bucket 筛选
- 管理员可查看全租户按 Bucket 维度的余额与交易；用户仅能查看自己的数据
- Realm 隔离：所有查询与操作严格遵守 Realm 数据边界

### 4.2 关键状态与异常

- **积分不足/无可用池**：消费时无覆盖该应用的池或合计余额不足，返回明确错误，不部分扣减
- **应用越权消费**：Bucket 未覆盖当前 Client App，拒绝消费
- **路由解析失败**：购买/订阅履约解析不到目标 Bucket，拒绝入账并明确报错（fail loud）
- **重复履约**：Webhook/履约基于幂等键去重，重复事件回到同一 Bucket 池结果
- **禁用 Bucket**：禁用后对新用户不可见可购，已持有其池的用户仍可消费剩余积分
- **并发消费**：跨多池并发消费须防超扣，沿用乐观锁与原子事务

---

## 5. 功能需求

### 5.1 核心需求

- 支持在 Realm 内管理 Bucket 目录：创建、编辑名称与展示顺序、启用/禁用、标记注册积分接收池
- 支持为 Bucket 配置覆盖的 Client App 集合（至少一个；注册接收池也需显式配置覆盖集，不自动覆盖全部应用）
- 支持将套餐/积分包归属到 Bucket；归属为必填，创建/编辑时均需显式绑定
- 支持每用户每 Bucket 独立积分池，替换单一钱包，允许同一用户持有多池并存
- 支持按 Client App 跨池原子消费，按过期时间优先跨池分摊扣减
- 支持购买/订阅履约按绑定 Bucket 路由入账（购买创建时固化归属），解析异常 fail loud
- 支持订阅续费/升级/降级/取消/退款按订阅绑定 Bucket 池发放与回收
- 支持注册/免费定期系统发放积分进入注册积分接收池 Bucket；SDK/管理员主动发放显式指定目标 Bucket
- 支持用户按 Bucket 分组查看余额（含合计）与按 Bucket 筛选的交易历史
- 支持管理员管理 Bucket 目录、查看按 Bucket 维度的余额与交易
- 修改 Bucket 配置或覆盖集仅影响后续行为，不回溯回收已持有积分

### 5.2 验收目标

- Realm 内可定义多个 Bucket，可配置覆盖应用、归属套餐、注册积分接收池标记
- 套餐/积分包归属唯一 Bucket（必填）；用户购买并支付成功后，积分进入该 Bucket 池，且仅能被覆盖应用消费
- 同一用户可同时持有多个 Bucket 独立池，余额互不影响
- SDK 按 Client App 消费时，跨覆盖该应用的全部 Bucket 池按过期优先原子扣减
- 应用越权、无可用池、余额不足时消费被明确拒绝，无超扣或串池
- 订阅续费/取消/退款始终回到订阅绑定的 Bucket 池，不影响其他池
- 注册/免费定期积分进入注册积分接收池 Bucket；主动发放按显式 Bucket 入账；注册接收池覆盖集显式配置，不自动覆盖全部应用
- 用户余额页按 Bucket 分组展示并提供合计；交易历史显示 Bucket 并可按 Bucket 筛选
- 管理员可管理 Bucket 目录并查看按 Bucket 维度的租户积分数据；用户仅能查看自身数据
- 路由解析失败时 fail loud 拒绝入账，不串池

---

## 6. API 相关约束

**适用性**: 适用

- 接口能力范围：Bucket 目录管理类（管理员）、覆盖集与 Mapping 归属管理类、积分消费类（SDK，按应用跨池）、余额/历史查询类（按 Bucket 维度）、购买/订阅履约与生命周期回调类
- 访问控制：Bucket 目录与归属管理需 Realm Admin（`points.manage`）；SDK 消费需 API Key 授权并校验 `client_app_scope`；用户查询类仅允许查询本人按 Bucket 的数据
- 消费授权双层校验：API Key 的 `client_app_scope` 为第一层授权，Bucket 覆盖集为第二层池过滤；两者皆须通过
- 发放显式路由：SDK/管理员主动发放类接口必须显式指定目标 `bucketId`（无默认值）；履约/订阅发放分别由 Mapping/订阅绑定的 Bucket 路由；解析不到目标 Bucket 时 fail loud 拒绝入账
- Realm 数据边界：所有接口严格遵守 Realm 隔离，防止跨 Realm 操作或消费
- 履约幂等与一致性：Webhook/履约须幂等，重复事件回到同一 Bucket 池结果；路由解析失败须 fail loud
- 积分变更可追溯：所有发放、消费、回收按 Bucket 维度生成交易记录
- 接口明细（端点、请求响应结构、状态码、数据结构演进）不在 PRD 范围，下沉到 `/t-design`

---

## 7. 前端/交互约束

**适用性**: 适用

- 管理入口：Realm Admin 可在管理后台访问 Credit Bucket 管理页（Bucket 列表、覆盖应用、归属套餐/积分包、积分策略、启用状态、注册积分接收池标记）
- 用户余额入口：用户可在个人积分页查看按 Bucket 分组的余额（按积分类型分桶）与跨全部 Bucket 的合计
- 用户交易历史：每条记录显示所属 Bucket，支持按 Bucket 筛选
- 购买入口：购买页按 Bucket 选择套餐/积分包；积分到账后展示于对应 Bucket 池
- 发放入口：管理员/SDK 主动发放需选择目标 Bucket（必选，无默认值）
- 状态反馈：积分发放、跨池消费、过期、按池回收时提供明确的状态与归属 Bucket 提示
- 风险提示：积分变更（尤其跨池消费、订阅按池回收）须突出变化量、影响范围与不可逆性
- 页面具体路径与组件实现不在 PRD 范围

---

## 8. 已确认决策

- 引入 Credit Bucket 作为积分池组织单位，是对原单钱包模型的**替换**（重构取代），非并存
- Bucket 归属单个用户持有；Realm 维护 Bucket 目录
- 多 Client App 的含义是"按应用限定消费范围"（Bucket 覆盖集），即只有 Bucket 覆盖的 Client App 才能消费该池
- 同一用户可同时持有多个 Bucket，各自独立积分池并存
- 所有积分都归属到 Bucket 池；**无"默认 Bucket"概念**
- 注册/免费定期积分归入该 Realm 的注册积分接收池 Bucket（每 Realm 一个，Bucket 级配置）；主动发放按显式 Bucket 入账
- 每笔积分发放显式指定目标 Bucket（履约→Mapping、订阅→订阅绑定 Bucket、SDK/管理员→请求 `bucketId`、注册/免费→注册接收池），无隐式池解析
- 不考虑兼容性，采用破坏性重建（移除单钱包唯一约束，建立池维度），不做正式历史数据回填
- 套餐/积分包归属 Bucket 为**强制必填**（单一 Bucket，创建与编辑均需绑定），不存在未归属态
- 购买/订阅的 Bucket 归属在购买创建时即解析固化（绑定到购买与订阅记录），作为履约与生命周期回收的路由真源；解析异常以 fail loud 兜底，不串池
- Bucket 通过 Entitlement Mapping 归属（Mapping 强制带 Bucket 归属），积分策略仍保留在 Mapping 上；`entitlement_key` 不作为 Bucket 标识
- 消费沿用"过期时间优先"规则，泛化为跨覆盖该应用的全部 Bucket 池分摊
- Bucket 必须至少关联一个 Client App 才能被消费
- 覆盖集变更仅影响后续消费资格，不回溯回收已持有积分
- Bucket 池间不支持合并/转移；持有 Bucket 数量无硬上限（v1 产品边界）

---

## 9. 参考资料

- 用户故事：`docs/user-stories/billing/credit-bucket.md`
- 相关用户故事：`docs/user-stories/billing/points-admin.md`、`points-user.md`、`points-free-user.md`、`points-package-purchase.md`、`entitlement-mapping.md`、`payment-attempt.md`、`integration/sdk.md`
- 相关 PRD：`docs/prd/billing/points.md`、`docs/prd/billing/subscription.md`、`docs/prd/integration/client-app.md`
- 角色定义：`docs/user-stories/_roles.md`
