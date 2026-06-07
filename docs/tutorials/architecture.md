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
├── api-billing/         # 计费相关 handler（Entitlement 映射、订阅、支付、发票、webhook）
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
docker/                  # Dockerfile
```

## 技术栈

后端：Rust 2024 edition + Axum 0.8 + SeaORM 1.1 + sqlx 0.8 + PostgreSQL 16+ + Redis + Tokio。
前端：React 19 + TypeScript + TanStack Router/Query/Form + Tailwind CSS v4 + Vite 7。

选型理由不是凑热门，而是每项都有具体约束：

- **Axum** 而不是 actix-web：tokio 原生，和 tower 中间件生态无缝衔接，trait-based handler 写起来更干净。
- **SeaORM + sqlx 并存**：SeaORM 处理常规 CRUD（entity crate 自动生成），sqlx 处理复杂查询和迁移。两套连接池指向同一个 PostgreSQL，SeaORM 底层用的就是 sqlx。
- **Redis**：session 存储、权限缓存、限流（Redis Functions）、幂等键。用自建的 `RedisConnectionManager` 包装，测试时自动切到 DB 1 隔离数据。
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

Handler 不直接写 SQL。它调 Domain Service 的方法，Service 通过 trait（port）抽象数据访问，infra 层提供具体实现。这是六边形架构的核心约束：`domain/` crate 的 `Cargo.toml` 里引入了 sea-orm、sqlx 等依赖，但仅用于错误类型转换（`From` impl），不直接操作数据库或发起 HTTP 请求。

权限检查走的另一条线：Handler → `RedisPermissionChecker` → Redis 缓存 → PostgreSQL（缓存 miss 时）。权限模型使用 `resource:action` 对（如 `product:read`），通过 `realm_id` 和 `client_id` 确定作用域。action 有层级关系：`manage` 覆盖 `view`、`create` 和 `manage` 本身；`create` 和 `view` 只覆盖自身。

## 模块说明

### entity — 数据库表映射

SeaORM 自动生成的实体定义。覆盖用户、角色、权限、订阅、积分、支付、发票等。每个 `.rs` 文件对应一张表，包含列定义、关系、默认值。

这个 crate 没有业务逻辑，纯粹的 ORM 映射层。改表结构时先写迁移 SQL（`backend/app/migrations/`），然后重新生成 entity。

### domain — 领域层

纯业务逻辑。Cargo.toml 依赖以基础库为主（serde、uuid、chrono、bcrypt 等），但也引入了 sea-orm、sqlx、redis、reqwest、axum——这些是为了实现 `From<ExternalError>` 错误转换，domain 层本身不直接操作数据库或发起 HTTP 请求。

关键子模块：

| 模块 | 职责 |
|------|------|
| `authentication` | 登录、注册、session 管理 |
| `authorization` | RBAC 权限模型（角色、策略、权限定义） |
| `audit` | 审计事件模型、事件采集（用户管理、RBAC 变更、认证事件） |
| `billing` | Entitlement 映射、订阅投影、支付 webhook 处理 |
| `points` | 积分账户、充值、消费、过期、幂等 |
| `payment_attempt` | 统一支付尝试（抽象不同支付渠道） |
| `purchase` | 购买履约（一次性充值 or 订阅开通） |
| `realm` | 租户管理 |
| `realm_config` | 租户配置（支付渠道密钥等） |
| `client` | 第三方应用管理 |
| `client_app` | Client App 实体与逻辑 |
| `client_api_keys` | API Key 管理 |
| `oauth` | OAuth 提供商配置 |
| `user` | 用户实体与查询 |
| `user_totp` | TOTP 二次认证 |
| `totp_key_management` | TOTP 密钥管理 |
| `rbac_init` | Realm 初始化时创建默认角色和权限 |
| `dashboard` | 仪表盘数据聚合 |
| `common` | 共享领域工具类型 |
| `security_constants` | 安全相关常量定义 |

每个子模块内部有 `ports/`（trait 定义）、`entities/`（领域实体）、`service.rs`（业务逻辑）。Ports 是 Repository trait，定义了 `find_by_id`、`save`、`update` 这类接口，但不关心底层是 PostgreSQL 还是内存。

### infra — 基础设施实现

domain 层 trait 的具体实现。`PostgresXxxRepository` 命名，一个 trait 对应一个实现。

除了数据库仓库，还有：
- `redis/` — `RedisConnectionManager`，连接池 + 测试隔离
- `authorization/` — `RedisPermissionChecker`，权限缓存
- `billing/` — 发票 PDF 生成（IronPress）、加密密钥管理

支付渠道客户端是独立的 crate（`infra-creem`、`infra-stripe`、`infra-wechat`、`infra-shopify`），不放在主 `infra` 里。原因是避免引入不需要的支付 SDK 依赖——如果你只用 Stripe，不会把微信支付的 SDK 也编译进去。

### core — 组装层

做两件事：

1. **ApplicationService Builder**：把 domain services 和 infra repositories 组装到一起。Builder 模式，依次注入 database、redis、permission_checker，最后 `.build()` 产出 `ApplicationService`。
2. **re-export**：把 `herald_domain` 重导出为 `domain`，`herald_entity` 为 `entity`，`herald_infra` 为 `infrastructure`。方便其他 crate 直接 `use herald_core::domain::xxx`。

### api 系列 — HTTP 接口

8 个 crate 组成 API 层：

| Crate | 职责 | 认证方式 |
|-------|------|---------|
| `api` | 主入口：路由注册、中间件编排、Swagger UI、OpenAPI spec 合并、审计日志查询 | — |
| `api-base` | `AppState` 定义（共享给所有 api 子 crate） | — |
| `api-auth` | 注册、登录、密码重置、邮箱验证 | session |
| `api-admin` | 用户 CRUD、角色管理、权限定义管理 | session + inject_identity |
| `api-billing` | Entitlement 映射、订阅投影、支付 webhook（Stripe/Creem/微信/Shopify）、发票、一次性购买 | 混合 |
| `api-oauth` | OAuth 登录（GitHub/Google/微信）、Device Code Grant（RFC 8628）、OAuth 配置管理 | 混合 |
| `api-ext` | 第三方 API：权限检查、订阅查询、积分余额和消费，按 API Key 绑定的 Client App 隔离 | API Key |
| `api-points` | 积分余额、交易历史、消费、充值 | session 或 API Key |

`api` crate 的 `create_api_routes()` 是路由注册的唯一入口。它把子 crate 的路由 `nest` 到统一前缀下，挂上 `inject_identity` 中间件。每个子 crate 独立定义自己的 `ApiDoc`（utoipa OpenApi spec），最后在 `build_openapi_spec()` 里 merge 成一份完整的 OpenAPI 文档。

拆成多个 crate 是为了编译速度。`api-billing` 最重（webhook 处理、类型定义），改一个支付渠道的 handler 不应该触发 `api-auth` 重编译。

### worker — 后台任务

定时执行的循环任务，和 API server 跑在同一个进程里：

- **积分过期**：每小时扫描过期积分，批量标记为已过期
- **发票逾期标记**：每小时扫描未支付发票，标记逾期状态

`WorkerConfig` 接受泛型 `R: InvoiceRepository`，方便测试时注入 mock。实际生产用 `PostgresInvoiceRepository`。

### app — 入口

`main.rs` 的启动流程，顺序固定：

1. 解析命令行参数。`--export-openapi <path>` 可以导出 OpenAPI JSON 然后退出（CI 里前端构建前用到）
2. 加载配置（`HERALD_CONFIG` 环境变量或 `config.toml`）
3. 初始化 tracing 日志
4. 连接 PostgreSQL（SeaORM 连接池，参数从配置读取）
5. 执行数据库迁移（sqlx migrate）
6. 连接 Redis 并做 health check
7. 初始化积分过期服务（points repository + expiration service）
8. 启动 API server（`herald_api::run_with_config`，内部再初始化所有 service 并绑定 Axum 路由）
9. 启动 Worker（定时任务循环）

8 和 9 并发运行，`tokio::select!` 等待任意一个结束或收到 shutdown 信号（Ctrl+C 或 SIGTERM）。

### sdk — Rust SDK

发布给第三方应用的 Rust crate。封装了 `/api/ext/` 下的所有接口：

- `check_permission` — 权限检查（带 moka 本地缓存，自动失效）
- `get_subscription` — 订阅状态查询，返回 `entitlement_key`
- `get_balance` / `consume_points` — 积分查询和消费

构造函数接收 `base_url` 和 `api_key`，所有请求自动带上 `X-API-Key` header。权限检查有本地缓存（moka），5 分钟 TTL，token 过期时批量清除关联缓存。

API Key 绑定到一个 Client App。默认绑定 `admin-api-client` 的 Key 是 realm 级 Key，可以访问同一 realm 下所有 Client App 的外部 API 资源；绑定普通 Client App 的 Key 只能访问该 Client App 的权限检查、订阅和积分资源。`api-ext` 在处理订阅、积分和权限检查时都会校验这个范围；API Key 认证即使命中缓存，也会重新检查绑定 Client App 是否仍启用，因此禁用 Client App 会立即阻断它的 API Key。

### frontend — React 管理后台

基于 TanStack Router 的文件路由，路由结构直接看 `frontend/src/routes/` 目录：

```
$realmId/
├── auth/          # 登录、注册、邮箱验证
├── user/          # 用户个人中心（资料、安全、积分、订阅、发票）
├── manage/        # 管理后台（用户、角色、权限、Entitlement 映射、计费、积分、审计日志、Client App、设置）
├── points/        # 积分余额和交易历史
├── subscription/  # 用户订阅状态
└── device/        # Device Code 授权页面
```

API 调用全部走 `frontend/src/lib/api-generated/` 里自动生成的 client。后端接口变更后，跑 `npm run generate-api`（先导出 OpenAPI JSON，再跑 openapi-ts 生成 TypeScript 类型）重新生成。

TanStack Query 管理服务端状态，TanStack Form + Zod 处理表单验证，Radix UI 提供无障碍的底层组件（Dialog、Select、Tabs 等），Tailwind CSS v4 做样式。

### 数据库迁移

按时间戳命名，放在 `backend/app/migrations/`。启动时自动执行（`sqlx::migrate!`），不需要手动跑脚本。
