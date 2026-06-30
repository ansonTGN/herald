# 架构

points-grant-redesign 把订阅/免费周期积分从"预发 ledger 行"改为"配额权益 + 滚动窗口用量"。充值/注册/发放积分仍走原有池子路径。

## 后端组件

### 配额权益（Quota Entitlement）

`backend/domain/src/points/entities.rs` 定义了核心实体：

```rust
pub struct QuotaWindow {
    pub window_seconds: i64,
    pub limit: i64,
    pub key: String,
}

pub struct PointsQuotaEntitlement {
    pub id: Uuid,
    pub user_id: Uuid,
    pub realm_id: String,
    pub bucket_id: Uuid,
    pub credit_type: CreditType,        // subscription_credit / free_periodic_credit
    pub source_type: QuotaSourceType,   // subscription_initial / subscription_renewal / subscription_upgrade / free_periodic_grant
    pub source_id: String,
    pub quota_windows: Vec<QuotaWindow>, // 快照
    pub effective_from: DateTime<Utc>,
    pub effective_until: Option<DateTime<Utc>>,
    pub status: QuotaEntitlementStatus, // active / revoked / expired
    pub idempotency_key: String,
}
```

存储表为 `points_quota_entitlements`（见 `.ai/design/points-grant-redesign.md` §4.3.2）。唯一约束为 `(realm_id, user_id, bucket_id, credit_type, idempotency_key)`，保证同一订阅周期/注册来源不会重复授予。

### 生命周期管理

`backend/domain/src/points/subscription_service.rs` 负责权益授予与撤销：

- `grant_quota_entitlement`：订阅 paid/续费、升级、免费注册时调用
- `revoke_quota_entitlement`：订阅取消/退款、免费升级付费时调用

撤销时只设置 `status = revoked` 和 `effective_until`，不反向调整已消费的流水。已用量随窗口滚动自然释放。

### 窗口聚合

`backend/domain/src/points/service.rs` 提供：

- `compute_window_available_for_credit_type`：计算某 `(user, bucket, credit_type)` 的窗口可用额度
- `compute_quota_windows_view`：返回前端仪表盘使用的窗口视图

窗口用量来自 `points_transactions` 的覆盖索引 `idx_points_transactions_window_agg`：

```sql
CREATE INDEX idx_points_transactions_window_agg
  ON points_transactions (user_id, bucket_id, credit_type, created_at DESC)
  INCLUDE (amount)
  WHERE type = 'consume';
```

聚合规则（`aggregate_quota_windows`）：

1. 按 `key` 分组汇总同一 `(user, bucket)` 下所有活跃权益
2. 同一 `key` 的窗口：limit 求和，used 取最大值（同一长度的窗口共享滑动区间）
3. `remaining = max(0, limit - used)`
4. 最终可用额度 = 所有 `key` 的 `remaining` 的最小值

### 混合消费

`backend/infra/src/points/postgres_repository.rs` 在 `consume_points_atomic` 的事务内完成：

```text
BEGIN
锁定 points_wallets 行（FOR UPDATE）
window_avail = 该 bucket 的窗口可用额度
pool_avail   = 池子类型可用额度（topup/registration/granted）
if amount > window_avail + pool_avail: ROLLBACK, 返回 InsufficientBalance
window_part = min(amount, window_avail)
pool_part   = amount - window_part
if window_part > 0: 写入 points_transactions(type='consume', credit_type=subscription/free_periodic)
if pool_part > 0:  按 expires_at 升序扣减 points_credit_ledger，并写入对应 consume 流水
更新钱包统计
COMMIT
```

该流程保证单次消费要么全部成功，要么完全不扣。

### 懒发放的正确性边界

`backend/domain/src/points/service.rs` 的 `reconcile_due_for_user` 已改为纯读路径，仅确认当前是否有活跃配额权益覆盖当前时刻，不执行任何写入。因此：

- 后台 job 未运行不影响余额和消费正确性
- 周期中途新订阅用户首访/首消费即得窗口额度
- 不活跃用户没有任何后台预发或回收开销

## 前端组件

### 用户余额页

路由：`/{realmId}/user/points`，组件 `frontend/src/components/points/UserPointsPage.tsx`。

页面结构：

1. 跨 bucket 合计区（当用户持有 >=2 个 bucket 时显示）
2. 每个 bucket 的 `PointsUsageDashboard`（窗口模型）
3. 每个 bucket 的 `PointsBalanceCard`（池子模型）
4. 交易历史

`PointsUsageDashboard` 展示：

- 当前可消费总额 `bucketTotal`
- 公式提示："各窗口剩余最小值 + 充值余额"
- 每窗口一行：key、limit、used、remaining、进度条、恢复时点、最严约束/耗尽标记

`PointsBalanceCard` 只展示池子类型（topup/registration/granted）余额，订阅/免费周期余额由仪表盘负责。

### 视图派生

`frontend/src/components/points/user-points-view.ts` 的 `deriveUserPointsView` 把 `listWallets` 返回的 realm-wide 数据过滤为当前用户视图，输出：

```typescript
interface DerivedBucketCard {
  bucketId: string
  name: string | null
  enabled: boolean | null
  bucketTotal: number
  balancesByType: BalancesByType
  quotaWindows: QuotaWindowViewDto[] | null
  spendableFromQuota: number | null
  spendableFromPool: number | null
}
```

### 多窗口配额编辑器

`frontend/src/components/billing/MultiWindowQuotaEditor.tsx` 是一个受控组件，同时用于：

- `frontend/src/components/billing/entitlement-mappings-page.tsx`（订阅配额）
- `frontend/src/routes/$realmId/manage/points/default-config.tsx`（免费周期配额）

编辑器输入输出为 `QuotaWindowInputDto[]`，每行包含 `windowSeconds` 和 `limit`。展示层提供单位切换（seconds/minutes/hours/days/weeks/months-30d），但 wire 上只传 `windowSeconds`。

### 数据查询

`frontend/src/data/query-options.ts` 定义：

- `walletsByBucketQueryOptions(realmId)` → `GET /api/points/{realmId}/wallets`
- `pointsDefaultConfigQueryOptions(realmId)` → `GET /api/points/{realmId}/default-config`
- `entitlementMappingsQueryOptions(realmId, filters)` → 获取 Entitlement Mapping 列表

`listWallets` 对 `points.view`-only 用户硬隔离为只返回自己的钱包行，管理员可查看全租户。
