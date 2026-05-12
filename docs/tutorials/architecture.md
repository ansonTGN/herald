# 架构

Rust 后端（六边形架构）+ React 前端（TanStack 全家桶），单体部署，所有组件跑在一个进程里。

## 目录结构

```
backend/
├── entity/              # SeaORM 实体定义（数据库表映射）
├── domain/              # 领域层 — 纯业务逻辑，零外部依赖
├── infra/               # 基础设施层 — 数据库、Redis、第三方 API 的具体实现
├── infra-creem/         # Creem 支付集成
├── infra-stripe/        # Stripe 支付集成
├── infra-wechat/        # 微信支付集成
├── infra-shopify/       # Shopify 集成
├── core/                # 组装层 — 依赖注入、ApplicationService Builder
├── api/                 # 主 API crate（Axum 路由注册、中间件、AppState）
├── api-base/            # 共享 API 工具（AppState 定义、通用 HTTP 工具）
├── api-billing/         # 计费相关 handler（套餐、支付、发票、webhook）
├── api-admin/           # 管理后台 handler（用户管理、角色、权限定义）
├── api-auth/            # 认证 handler（注册、登录、密码重置）
├── api-ext/             # 外部 API handler（API Key 认证，供第三方调用）
├── api-oauth/           # OAuth handler（GitHub/Google/微信登录）
├── api-points/          # 积分 handler（余额查询、消费、充值）
├── worker/              # 后台任务（积分过期、发票逾期标记）
├── app/                 # 入口（main.rs、数据库迁移）
├── sdk/                 # 发布给第三方用的 Rust SDK crate
├── test-db/             # 测试数据库工具（testcontainers）
├── test-support/        # 测试辅助
└── integration-tests/   # 集成测试

frontend/                # React 管理前端
docker/                  # Dockerfile、docker-compose.yml
```

## 技术栈

后端：Rust 2024 edition + Axum 0.8 + SeaORM 1.1 + sqlx 0.8 + PostgreSQL 16+ + Redis + Tokio。
前端：React 19 + TypeScript + TanStack Router/Query/Form + Tailwind CSS v4 + Vite 7。

选型理由不是凑热门，而是每项都有具体约束：

- **Axum** 而不是 actix-web：tokio 原生，和 tower 中间件生态无缝衔接，trait-based handler 写起来更干净。
- **SeaORM + sqlx 并存**：SeaORM 处理常规 CRUD（entity crate 自动生成），sqlx 处理复杂查询和迁移。两套连接池指向同一个 PostgreSQL，SeaORM 底层用的就是 sqlx。
- **Redis**：session 存储、权限缓存（Casbin 策略缓存）、限流（Redis Functions）、幂等键。用自建的 `RedisConnectionManager` 包装，测试时自动切到 DB 1 隔离数据。
- **TanStack Router**：类型安全的文件路由，`$realmId` 这类路径参数直接推导类型。比 react-router 的运行时匹配靠谱。
- **API 类型生成**：后端用 utoipa 导出 OpenAPI JSON，前端用 `@hey-api/openapi-ts` 生成 TypeScript client 和类型。后端接口变了，`npm run generate-api` 一条命令同步。

## 核心流程

一个典型请求的路径：

```
HTTP Request
  → Axum Router（路由匹配）
  → inject_identity 中间件（从 session cookie 或 Bearer token 解析用户身份）
  → Handler（提取 AppState、解析请求体）
  → Domain Service（业务逻辑，通过 trait 调用 Repository）
  → Repository 实现（infra 层，操作 PostgreSQL 或 Redis）
  → SeaORM Entity / sqlx 查询
  → PostgreSQL
```

Handler 不直接写 SQL。它调 Domain Service 的方法，Service 通过 trait（port）抽象数据访问，infra 层提供具体实现。这是六边形架构的核心约束：`domain/` crate 的 `Cargo.toml` 里没有任何数据库或 HTTP 依赖，只有纯 Rust 类型。

权限检查走的另一条线：Handler → `RedisPermissionChecker` → Redis 缓存 → PostgreSQL（缓存 miss 时）。RBAC 模型定义在 `backend/api/config/rbac_model.conf`，四元组 `(domain, subject, object, action)` 匹配，domain 是 client_app ID 或通配符 `*`。

## 模块说明

### entity — 数据库表映射

SeaORM 自动生成的实体定义。39 张表，覆盖用户、角色、权限、订阅、积分、支付、发票等。每个 `.rs` 文件对应一张表，包含列定义、关系、默认值。

这个 crate 没有业务逻辑，纯粹的 ORM 映射层。改表结构时先写迁移 SQL（`backend/app/migrations/`），然后重新生成 entity。

### domain — 领域层

纯业务逻辑，Cargo.toml 零外部依赖（只有 serde、uuid、chrono 这类基础库）。

关键子模块：

| 模块 | 职责 |
|------|------|
| `authentication` | 登录、注册、session 管理 |
| `authorization` | RBAC 权限模型（角色、策略、权限定义） |
| `billing` | 套餐管理、订阅生命周期 |
| `points` | 积分账户、充值、消费、过期、幂等 |
| `points_package` | 积分包定义 |
| `payment_attempt` | 统一支付尝试（抽象不同支付渠道） |
| `purchase` | 购买履约（积分包充值 or 订阅开通） |
| `realm` | 租户管理 |
| `client` | 第三方应用管理 |
| `oauth` | OAuth 提供商配置 |

每个子模块内部有 `ports/`（trait 定义）、`entities/`（领域实体）、`service.rs`（业务逻辑）。Ports 是 Repository trait，定义了 `find_by_id`、`save`、`update` 这类接口，但不关心底层是 PostgreSQL 还是内存。

### infra — 基础设施实现

domain 层 trait 的具体实现。`PostgresXxxRepository` 命名，一个 trait 对应一个实现。

除了数据库仓库，还有：
- `redis/` — `RedisConnectionManager`，连接池 + 测试隔离
- `authorization/` — `RedisPermissionChecker`，Casbin 风格的权限缓存
- `billing/` — 发票 PDF 生成（IronPress）、加密密钥管理
- `creem/`、`stripe/`、`wechat/`、`shopify/` — 支付渠道客户端

`infra-creem`、`infra-stripe`、`infra-wechat`、`infra-shopify` 是独立 crate，避免主 `infra` 引入不需要的支付 SDK 依赖。

### core — 组装层

做两件事：

1. **ApplicationService Builder**：把 domain services 和 infra repositories 组装到一起。Builder 模式，依次注入 database、redis、permission_checker，最后 `.build()` 产出 `ApplicationService`。
2. **re-export**：把 `herald_domain` 重导出为 `domain`，`herald_entity` 为 `entity`，`herald_infra` 为 `infrastructure`。方便其他 crate 直接 `use herald_core::domain::xxx`。

### api 系列 — HTTP 接口

7 个 crate 组成 API 层：

| Crate | 职责 | 认证方式 |
|-------|------|---------|
| `api` | 主入口：路由注册、中间件编排、Swagger UI、OpenAPI spec 合并 | — |
| `api-base` | `AppState` 定义（共享给所有 api 子 crate） | — |
| `api-auth` | 注册、登录、密码重置、邮箱验证 | session |
| `api-admin` | 用户 CRUD、角色管理、权限定义管理 | session + inject_identity |
| `api-billing` | 套餐、订阅、支付 webhook（Stripe/Creem/微信/Shopify）、发票、积分包购买 | 混合 |
| `api-oauth` | OAuth 登录（GitHub/Google/微信）、OAuth 配置管理 | 混合 |
| `api-ext` | 第三方 API：权限检查、订阅查询、积分余额和消费 | API Key |
| `api-points` | 积分余额、交易历史、消费、充值 | session 或 API Key |

`api` crate 的 `create_api_routes()` 是路由注册的唯一入口。它把子 crate 的路由 `nest` 到统一前缀下，挂上 `inject_identity` 中间件。每个子 crate 独立定义自己的 `ApiDoc`（utoipa OpenApi spec），最后在 `build_openapi_spec()` 里 merge 成一份完整的 OpenAPI 文档。

拆成多个 crate 是为了编译速度。`api-billing` 最重（webhook 处理、类型定义），改一个支付渠道的 handler 不应该触发 `api-auth` 重编译。

### worker — 后台任务

定时执行的循环任务，和 API server 跑在同一个进程里：

- **积分过期**：每小时扫描过期积分，批量标记为已过期
- **发票逾期标记**：每小时扫描未支付发票，标记逾期状态

`WorkerConfig` 接受泛型 `R: InvoiceRepository`，方便测试时注入 mock。实际生产用 `PostgresInvoiceRepository`。

### app — 入口

`main.rs` 做六件事，顺序固定：

1. 加载配置（`HERALD_CONFIG` 环境变量或 `config.toml`）
2. 连接 PostgreSQL（SeaORM 连接池，参数从配置读取）
3. 执行数据库迁移（sqlx migrate）
4. 连接 Redis
5. 启动 API server（`herald_api::run_with_config`，内部再初始化所有 service 并绑定 Axum 路由）
6. 启动 Worker（定时任务循环）

5 和 6 并发运行，`tokio::select!` 等待任意一个结束或收到 shutdown 信号。

### sdk — Rust SDK

发布给第三方应用的 Rust crate。封装了 `/api/ext/` 下的所有接口：

- `check_permission` — 权限检查（带 moka 本地缓存，自动失效）
- `get_subscription` / `list_plans` / `list_plan_assignments` — 订阅查询
- `get_balance` / `consume_points` — 积分查询和消费

构造函数接收 `base_url` 和 `api_key`，所有请求自动带上 `X-API-Key` header。权限检查有本地缓存（moka），5 分钟 TTL，token 过期时批量清除关联缓存。

### frontend — React 管理后台

基于 TanStack Router 的文件路由，路由结构直接看 `frontend/src/routes/` 目录：

```
$realmId/
├── auth/          # 登录、注册、邮箱验证
├── user/          # 用户个人中心（资料、安全、积分、订阅、发票）
└── manage/        # 管理后台（用户、角色、权限、套餐、计费、积分、Client App、设置）
```

API 调用全部走 `frontend/src/lib/api-generated/` 里自动生成的 client。后端接口变更后，跑 `npm run generate-api`（先导出 OpenAPI JSON，再跑 openapi-ts 生成 TypeScript 类型）重新生成。

TanStack Query 管理服务端状态，TanStack Form + Zod 处理表单验证，Radix UI 提供无障碍的底层组件（Dialog、Select、Tabs 等），Tailwind CSS v4 做样式。

### 数据库迁移

按时间戳命名，`backend/app/migrations/` 下 12 个 SQL 文件，对应核心模块的建表顺序：

1. `20260209_core_init` — realm、account、profile
2. `20260210_auth` — session、email_verification、TOTP
3. `20260211_billing` — plan、subscription、product
4. `20260212_payment` — payment_attempt、payment_event
5. `20260401_shopify` — Shopify 绑定表
6. `20260408_unified_purchase` — 统一购买（积分包购买记录）
7. `20260508_invoice` — 发票表

启动时自动执行迁移（`sqlx::migrate!`），不需要手动跑脚本。
