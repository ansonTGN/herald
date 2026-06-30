# 快速上手

本章介绍如何为订阅套餐和免费用户配置多时间窗滚动配额。

## 配置订阅配额（Entitlement Mapping）

订阅积分策略挂在 `provider_entitlement_mappings` 上，通过管理后台的 Entitlement Mappings 页面或 `PUT /api/bill/{realmId}/entitlement-mappings/batch` 配置。

### 管理后台操作

1. 以 Realm Admin 登录后进入 `/{realmId}/manage/billing/entitlement-mappings`
2. 从左侧产品列表选择目标产品
3. 在价格行的高级配置里找到多窗口配额编辑器
4. 添加窗口行并保存

### API 请求示例

`backend/api-billing/src/entitlement_mapping_handlers.rs` 的批量更新接口接受如下请求体：

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

字段说明：

| 字段 | 约束 | 说明 |
|---|---|---|
| `windowSeconds` | 必须 > 0 | 滑动窗口长度，单位为秒 |
| `limit` | 必须 >= 0 | 该窗口的配额上限 |
| `quotaWindows` | 最多 8 行 | 超出会被 400 拒绝 |

配置仅影响后续授予的配额权益，已激活的权益不受影响（权益授予时会把 `quota_windows` 快照写入 `points_quota_entitlements`）。

### 活跃订阅保护

如果批量更新会把某个仍有活跃订阅的 mapping 从 `enabled=true` 改为 `enabled=false`，后端会回滚整个事务并返回 409，响应体包含受影响的活跃订阅数量。前端会弹出 `ProtectedPriceConfirmDialog` 提示。

## 配置 Realm 默认免费周期配额

免费用户注册时获得的周期额度由 Realm 默认配置管理，路径为 `/{realmId}/manage/points/default-config`，对应 `backend/api-points/src/realm_configs.rs` 的 `GET/PUT /api/points/{realmId}/default-config`。

### 管理后台操作

1. 以 Realm Admin 登录后进入 `/{realmId}/manage/points/default-config`
2. 找到"免费周期多窗口配额"编辑器
3. 添加窗口行并保存

### API 请求示例

```json
PUT /api/points/{realmId}/default-config
{
  "registrationBonusPoints": 1000,
  "freePeriodicPointsAmount": 50,
  "freePeriodicGrantPeriodType": "daily",
  "freePeriodicValidityDays": 1,
  "freePeriodicQuotaWindows": [
    { "windowSeconds": 86400, "limit": 50 },
    { "windowSeconds": 604800, "limit": 200 }
  ]
}
```

`freePeriodicQuotaWindows` 语义：

- `null`：不修改已存储值（PUT 部分更新）
- `[]`：清空窗口配置
- `[{windowSeconds, limit}]`：替换为新的窗口定义

校验规则与 Entitlement Mapping 一致：`windowSeconds > 0`、`limit >= 0`、最多 8 行。

### 注册初始积分

`registrationBonusPoints` 控制新用户注册时一次性获得的 `registration_credit`，永久有效。该字段与免费周期配额是同一配置页的两个独立区域。

## 验证配置是否生效

1. 新用户注册或新订阅支付完成后，访问 `/{realmId}/user/points`
2. 页面上的 `PointsUsageDashboard` 应显示各窗口行的剩余/上限/已用/恢复时点
3. 如果没有窗口行，检查对应 mapping 或 realm default 是否配置了 `quotaWindows`，以及用户是否已获得配额权益
