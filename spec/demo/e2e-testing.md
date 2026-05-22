# Demo E2E 测试指南

用于 Demo 开发与功能验证的主入口。本文档只保留高频规则、标准命令和文档地图，不再承载维护案例、调试 recipe 或大段示例代码。

## 1. 适用范围

适用于：
- 新增或修改 `demo/e2e/**/*.e2e.ts`
- 修复 Demo 测试代码问题
- 运行单文件或最小回归验证

本页不展开：
- 故障案例与修复 recipe：见 [测试维护指南](test-maintenance.md)
- POM 设计细则：见 [POM 指南](pom-guide.md)
- 选择器细则：见 [选择器策略](/spec/agents/demo/selector-strategy.md)
- 日志分诊与深度排障：见 [调试排障指南](/spec/agents/shared/demo-debugging.md)
- `t-demo-run` 修复编排：见 [`.claude/commands/t-demo-run.md`](/.claude/commands/t-demo-run.md)

## 2. 核心规则

1. 完整用户故事主路径优先使用综合 Demo 文件，推荐后缀为 `-comprehensive-demo.e2e.ts`；独立配置流、专项回归或跨角色辅助场景可保留普通 `*-demo.e2e.ts`。
2. 业务数据只通过 UI 创建，禁止调用业务 API 或脚本绕过用户路径。
3. 一个完整故事的主演示优先在一次浏览器会话中完成，优先使用单个 `test()` + 多个 `test.step()`；非综合型回归文件不强制合并为单个测试。
4. 优先使用 `demo/e2e/fixtures/demo-page.fixtures.ts`，统一接入 `demoLogger`、`cleanupTestData`、`testStartTime`；底层 helper 如 `cleanupDemoTestData()` 不作为主文档推荐入口。
5. 每个测试都要做到“前验证 + 后清理”：`beforeEach` 验证环境，`afterEach` 清理本次创建数据。
6. 等待与断言优先使用 Playwright 自动等待；不要把固定 sleep 当作主策略。
7. 选择器优先语义定位；复杂或易波动元素使用 `data-testid`，不要散落脆弱 CSS 路径。
8. 验证运行使用项目脚本，失败后先判断 Demo 代码问题，再区分前后端实现问题。
9. 关键展示场景依赖的基础数据必须可重复初始化，不能把 happy-path 建立在空库或临时 mock 上。

## 3. 当前目录事实

```text
demo/e2e/
├── fixtures/
├── helpers/
├── pages/
├── billing-admin/
├── realm-admin/
├── regular-user/
├── super-admin/
├── selectors.ts
└── *.e2e.ts
```

角色目录和文件组织以 `demo/e2e/` 当前结构为准，不使用参考仓库中的 `workspace-admin` 或 `third-party-app` 作为默认示例。

## 4. 标准命令

```powershell
# 运行单个测试
uv run scripts/demo-test-runner.py demo/e2e/[test-file].ts

# 批量运行全部 Demo 测试
uv run scripts/demo-run-all.py

# 调试单个测试
uv run scripts/debug-test.py demo/e2e/[test-file].ts

# 按需停止 Demo 环境
uv run scripts/demo-stop.py
```

注意：
- Demo 环境与开发环境端口冲突，不能并行。
- 验证运行不要依赖 `page.pause()`、Inspector 或有界面选择器工具。

## 5. 验证运行与调试运行

| 项目 | 验证运行（`t-demo-run` / CI） | 调试运行（人工） |
| --- | --- | --- |
| 项目模式 | `demo-fast`（headless） | 可按需使用 headed |
| Playwright Inspector | 禁止 | 允许 |
| `page.pause()` | 禁止 | 允许 |
| 修复后补测 | 先跑相关 backend/frontend 最小测试集，再跑 demo | 本地定位时可临时跳过 |

`t-demo-run` 的修复停止规则、补测顺序和风险标记，以 [`.claude/commands/t-demo-run.md`](/.claude/commands/t-demo-run.md) 为准。

推荐批量修复节奏：
1. 先运行 `uv run scripts/demo-run-all.py` 获取失败文件面。
2. 再对失败文件逐个调用 `demo-diagnose` 做结构化分类，而不是天然按文件顺序逐个修。
3. 每一批先修共享层（selectors / pages / helpers / backend contract），再做最小补测，最后回归受影响 Demo 文件。
4. 所有批次完成后，再补跑一次 `uv run scripts/demo-run-all.py` 做最终验收。

## 6. 提交前自检

- [ ] 文件位置和命名符合当前角色目录与 `*.e2e.ts`
- [ ] 综合测试覆盖完整用户故事主路径，专项文件的边界清晰可解释
- [ ] 测试数据通过 UI 创建
- [ ] 使用 `demo-page.fixtures` 与 `demoLogger`
- [ ] `beforeEach` 包含环境验证
- [ ] `afterEach` 通过 `cleanupTestData()` 或等价封装包含本次数据清理
- [ ] 使用 `testStartTime` 或等价唯一标记
- [ ] 断言关注业务结果，不依赖易波动提示文案
- [ ] 无固定 sleep，等待策略可解释
- [ ] 已用项目脚本完成最小验证

## 8. 测试数据构造规范

### 8.1 职责分层

#### 环境初始化层（允许数据库/API操作）

**执行者**：`scripts/lib/demo_seed.py`
**时机**：Demo 环境启动时（`demo-start.py` 调用）
**目的**：创建可重用的基础测试数据
**允许操作**：
- ✅ 通过 HTTP API 创建 Realm、User
- ✅ 直接数据库操作创建复杂业务数据（Points、Subscription History）
**不受 Demo 代码规则限制**：这是基础设施层，不是测试代码

**创建的数据**：
- Realm: `realm-001`
- Admin User: `admin@realm-001.com`
- Test User: `user@realm-001.com`
- Points System 数据（账户、积分、事务）
- Subscription History 数据（测试用户订阅历史）

#### Demo 测试层（禁止数据库/API操作）

**执行者**：demo-dev 生成的测试代码
**职责**：验证用户流程和产品功能
**强制约束**：
- ✅ 只通过 UI 操作进行验证和测试
- ✅ 可以使用 Demo Seed 创建的基础 Realm 和 User
- ❌ 禁止直接调用业务 API（`request.post/get/put/delete`）
- ❌ 禁止直接数据库操作（`sqlExec`, `sqlScalar`）
- ❌ 禁止管理端操作（创建 Client App、Billing Plan 等）

**测试数据来源**：
- Demo Seed 创建的预设数据（如 realm-001 的订阅历史）
- UI 用户操作触发的数据变更（如用户升级订阅）
- 不应该在测试中通过 UI 或 API 创建管理端数据

### 8.2 禁止使用的文件

**Demo 测试层禁止使用**：

```typescript
// ❌ 禁止：API 数据构造助手
import { createTestDataViaAPI } from '../helpers/api-test-data.helpers'
import { createClientAppViaAPI } from '../helpers/api-test-data.helpers'

// ❌ 禁止：数据库数据构造助手
import { createTestDataViaDB } from '../helpers/db-test-data.helpers'
import { sqlExec } from '../helpers/db-test-data.helpers'

// ❌ 禁止：订阅创建助手（包含管理端操作）
import { createClientApp } from '../helpers/subscription-creation.helpers'
import { createBillingPlan } from '../helpers/subscription-creation.helpers'
import { createTestSubscription } from '../helpers/subscription-creation.helpers'
```

### 8.3 允许使用的文件

**Demo 测试层允许使用**：

```typescript
// ✅ 允许：环境验证和清理
import { verifyTestEnvironment } from '../helpers/environment-setup'
import { cleanupTestData } from '../helpers/environment-setup'

// ✅ 允许：认证
import { loginWithCredentials } from '../helpers/auth'

// ✅ 允许：订阅历史相关辅助（仅用于 UI 操作和验证）
import { navigateToSubscriptionDetailHistory } from '../helpers/subscription-history.helpers'
import { isTimelineEmpty, waitForTimelineToLoad } from '../helpers/subscription-history.helpers'
```

### 8.4 测试数据使用指南

#### 订阅历史测试示例

**正确方式**：
```typescript
test('should view subscription history', async ({ page, demoLogger }) => {
  await test.step('Given: 使用 Demo Seed 创建的订阅历史', async () => {
    // ✅ 正确：依赖 Demo Seed 创建的数据
    // realm-001 已经有了测试用户的订阅历史（created, upgraded, renewed）
    // 不需要创建新数据，直接使用即可
  })

  await test.step('When: 用户登录并访问订阅历史页面', async () => {
    await loginWithCredentials(page, {
      email: 'user@realm-001.com', // Demo Seed 创建的用户
      password: 'password',
      realmId: 'realm-001',
    })
    await navigateToSubscriptionDetailHistory(page, 'realm-001')
  })

  await test.step('Then: 验证订阅历史显示', async () => {
    // 验证 Demo Seed 创建的订阅历史事件
    await expect(page.getByText('Created')).toBeVisible()
    await expect(page.getByText('Upgraded')).toBeVisible()
    await expect(page.getByText('Renewed')).toBeVisible()
  })
})
```

**错误方式**：
```typescript
test('should view subscription history', async ({ page, demoLogger }) => {
  // ❌ 错误：通过 API 创建数据
  const testData = await createTestDataViaAPI(adminToken, realmId, testStartTime)

  // ❌ 错误：通过 UI 创建管理端数据
  await createClientApp(page, realmId, config)
  await createBillingPlan(page, realmId, config)
  await createTestSubscription(page, realmId, testStartTime)
})
```

### 8.5 测试数据设计原则

1. **依赖 Demo Seed**：测试应该依赖 Demo Seed 创建的基础数据
2. **用户端操作**：测试应该验证用户端的操作，不是管理端
3. **真实流程**：测试应该模拟真实用户行为，不是快速创建数据
4. **数据完整性**：复杂业务数据（如订阅历史）应该在 Demo Seed 中创建

## 9. 文档地图

- 主维护入口：[`test-maintenance.md`](test-maintenance.md)
- POM 规则：[`pom-guide.md`](pom-guide.md)
- 选择器规则：[`../agents/demo/selector-strategy.md`](/spec/agents/demo/selector-strategy.md)
- 日志与分诊：[`../agents/shared/demo-debugging.md`](/spec/agents/shared/demo-debugging.md)
- 诊断说明：[`diagnose-guide.md`](diagnose-guide.md)
- 命令流程：[`../../.claude/commands/t-demo-run.md`](/.claude/commands/t-demo-run.md)
- 全局执行约束：[`../../AGENTS.md`](/AGENTS.md)
