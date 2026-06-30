# API 参考

Base URL、认证方式与全局错误码沿用 Herald 既有约定。本章只列出 points-grant-redesign 相关端点和字段。

## 消费积分

### POST `/api/ext/points/{realmId}/consume`

SDK/第三方应用通过 API Key 消费积分。请求/响应契约不变，内部按"窗口额度优先、超额转池子"协调。

**认证**：`X-API-Key` header，ThirdParty 身份，且 API Key 需要对 `client_app_id` 有作用域。

**限流**：realm 级别 100 次/分钟，user 级别 20 次/分钟。

**请求体**（`backend/api-ext/src/points.rs`）：

| 字段 | 类型 | 必填 | 说明 |
|---|---|---|---|
| `userId` | string | 是 | 目标用户 ID |
| `clientAppId` | string | 是 | 消费归属的 Client App |
| `amount` | i64 | 是 | 消费数量，必须 > 0 |
| `description` | string | 否 | 消费说明 |
| `idempotencyKey` | string | 否 | 幂等键，重放时返回原结果 |

```json
{
  "userId": "550e8400-e29b-41d4-a716-446655440000",
  "clientAppId": "a1b2c3d4-e5f6-7890-abcd-ef1234567890",
  "amount": 300,
  "description": "AI API call"
}
```

**响应**（成功 200）：

| 字段 | 类型 | 说明 |
|---|---|---|
| `userId` | string | 用户 ID |
| `amount` | i64 | 消费数量 |
| `correlationId` | string | 跨 bucket 消费分组键 |
| `transactions` | array | 每个 bucket 产生一条交易 |
| `allocations` | array | 实际扣减的 ledger/类型分配 |

**错误码**：

- `400`：参数非法、账户冻结/关闭
- `401/403`：API Key 无效、跨 Realm
- `409`：额度不足（`code=insufficient_points`，响应含 `have`/`need`）

## 查询钱包余额

### GET `/api/points/{realmId}/wallets`

列出 Realm 内钱包，按 `(user, bucket)` 分组返回。普通用户只看到自己；管理员（`points.manage`）可查看全租户。

**查询参数**（`backend/api-points/src/wallets.rs`）：

| 参数 | 类型 | 说明 |
|---|---|---|
| `status` | string | 按钱包状态筛选 |
| `search` | string | 按用户 ID 搜索（管理员有效） |
| `bucketId` | string | 按 Credit Bucket 筛选 |
| `page` | u64 | 页码，0 起，默认 0 |
| `pageSize` | u64 | 默认 20，最大 100 |

**响应**（`ListWalletsByBucketResponse`）：

| 字段 | 类型 | 说明 |
|---|---|---|
| `items` | `WalletByBucketResponse[]` | 按 bucket 分组的钱包列表 |
| `crossBucketTotal` | i64 | 所有 bucket 的 `bucketTotal` 之和 |

`WalletByBucketResponse` 字段：

| 字段 | 类型 | 说明 |
|---|---|---|
| `bucketId` | UUID \| null | Credit Bucket ID；聚合行可能为 null |
| `name` | string \| null | Bucket 显示名 |
| `enabled` | boolean \| null | Bucket 是否启用 |
| `userId` | UUID | 钱包归属用户 |
| `balancesByType` | `BalancesByType` | 五类积分余额 |
| `bucketTotal` | i64 | 当前可消费总额 = 窗口可用 + 池子余额 |
| `quotaWindows` | `QuotaWindowViewDto[]` \| null | 窗口视图，纯池子 bucket 为 null |
| `spendableFromQuota` | i64 \| null | 各窗口剩余最小值 |
| `spendableFromPool` | i64 \| null | topup + registration + granted 余额 |

`BalancesByType`：

```json
{
  "topup": 1000,
  "subscription": 0,
  "registration": 1000,
  "freePeriodic": 0,
  "granted": 0
}
```

`QuotaWindowViewDto`：

| 字段 | 类型 | 说明 |
|---|---|---|
| `key` | string | 稳定展示 key，如 `5h`/`week`/`month` |
| `limit` | i64 | 窗口上限 |
| `used` | i64 | 窗口内已用量 |
| `remaining` | i64 | 剩余额度 |
| `windowSeconds` | i64 | 窗口长度（秒） |
| `resetsAt` | ISO8601 \| null | 近似恢复时点 |
| `isTightest` | boolean | 是否是最小剩余窗口 |
| `exhausted` | boolean | 是否已耗尽 |

## Realm 默认配置

### GET `/api/points/{realmId}/default-config`

返回 Realm 的默认积分配置，包括注册初始积分和免费周期多窗口配额。

**权限**：`settings.view`

**响应**（`RealmDefaultConfigResponse`）：

| 字段 | 类型 | 说明 |
|---|---|---|
| `realmId` | string | Realm ID |
| `registrationBonusPoints` | i64 | 注册初始积分数 |
| `freePeriodicPointsAmount` | i64 | 免费周期积分数量（旧字段，兼容） |
| `freePeriodicGrantPeriodType` | string | 周期类型：`once`/`daily`/`weekly`/`monthly` |
| `freePeriodicValidityDays` | i64 | 免费周期积分有效期（天） |
| `freePeriodicQuotaWindows` | `QuotaWindowInputDto[]` \| null | 多窗口配额定义 |
| `createdAt` | ISO8601 | 创建时间 |
| `updatedAt` | ISO8601 | 更新时间 |

### PUT `/api/points/{realmId}/default-config`

更新 Realm 默认配置。

**权限**：`settings.manage`

**请求体**（`UpdateRealmConfigRequest`）：

```json
{
  "registrationBonusPoints": 1500,
  "freePeriodicPointsAmount": 100,
  "freePeriodicGrantPeriodType": "daily",
  "freePeriodicValidityDays": 1,
  "freePeriodicQuotaWindows": [
    { "windowSeconds": 86400, "limit": 100 },
    { "windowSeconds": 604800, "limit": 400 }
  ]
}
```

`freePeriodicQuotaWindows` 语义：

- `null`：不修改已存储值
- `[]`：清空窗口配置
- `[{windowSeconds, limit}]`：替换

每行校验：`windowSeconds > 0`、`limit >= 0`；最多 8 行。

## Entitlement Mapping 批量更新

### PUT `/api/bill/{realmId}/entitlement-mappings/batch`

批量保存某个产品的价格/权益映射，包括订阅积分策略和多窗口配额。

**权限**：

1. `billing.manage`
2. 如果任何一行写了积分策略字段（`pointsPerPeriod`/`grantPeriodType`/`validityDays`/`grantOnSubscribe`/`maxPeriods`/`quotaWindows`），还需要 `points.manage`

**请求体**（`BatchUpdateEntitlementMappingsRequest`）：

```json
{
  "paymentProvider": "stripe",
  "externalProductId": "prod_xxx",
  "updates": [
    {
      "mappingId": "550e8400-e29b-41d4-a716-446655440000",
      "entitlementKey": "pro-plan",
      "billingType": "recurring",
      "billingPeriod": "month",
      "pointsPerPeriod": 1000,
      "grantPeriodType": "monthly",
      "validityDays": 30,
      "grantOnSubscribe": true,
      "maxPeriods": 10,
      "enabled": true,
      "quotaWindows": [
        { "windowSeconds": 18000, "limit": 500 },
        { "windowSeconds": 604800, "limit": 5000 },
        { "windowSeconds": 2592000, "limit": 20000 }
      ]
    }
  ]
}
```

新增字段：

| 字段 | 类型 | 说明 |
|---|---|---|
| `quotaWindows` | `QuotaWindowInputDto[]` \| null | 多窗口配额定义 |

`QuotaWindowInputDto`：

| 字段 | 类型 | 约束 | 说明 |
|---|---|---|---|
| `windowSeconds` | i64 | > 0 | 窗口长度（秒） |
| `limit` | i64 | >= 0 | 配额上限 |

**响应**（201）：

```json
{
  "saved": 1,
  "prices": [
    {
      "mappingId": "550e8400-e29b-41d4-a716-446655440000",
      "entitlementKey": "pro-plan",
      "billingType": "recurring",
      "billingPeriod": "month",
      "pointsPerPeriod": 1000,
      "grantPeriodType": "monthly",
      "validityDays": 30,
      "grantOnSubscribe": true,
      "maxPeriods": 10,
      "enabled": true,
      "quotaWindows": [
        { "windowSeconds": 18000, "limit": 500 },
        { "windowSeconds": 604800, "limit": 5000 },
        { "windowSeconds": 2592000, "limit": 20000 }
      ]
    }
  ]
}
```

**错误码**：

- `400`：entitlement key 非法、跨产品 shared-key 重命名、窗口配置非法
- `403`：缺少 `billing.manage` 或 `points.manage`
- `409`：活跃订阅保护（`MappingActiveSubscriptionLockErrorBody`）

## 常用窗口长度速查

| 窗口 | windowSeconds |
|---|---|
| 5 小时 | 18000 |
| 1 天 | 86400 |
| 1 周 | 604800 |
| 30 天（月窗近似） | 2592000 |

后端通过 `derive_window_key` 把这些长度映射为稳定 key：`5h`、`day`、`week`、`month`。
