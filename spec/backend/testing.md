# 后端测试指南

**适用项目**：Herald 项目的 Rust 后端（`api/`、`core/`、`sdk/`）

**相关文档**:
- [测试策略总览](/spec/core/environment-and-testing-guide.md#测试策略决策) - 测试选择决策和职责划分
- [环境选择决策](/spec/core/environment-and-testing-guide.md#环境选择决策) - 环境使用入口

## 测试类型

| 类型 | 命名前缀 | 测试对象 | 依赖 | 位置 |
|-----|---------|---------|------|------|
| 单元测试 | `test_unit_*` | 单个函数 | Mock | `<module>_test.rs` |
| 场景测试 | `test_scenario_*` | 跨 API 业务流程 | 真实 DB + Redis | `tests/scenarios/*.rs` |

**设计理念**：
- 单元测试只验证高价值局部行为：业务规则、边界条件、状态转换、权限判断、错误映射、数据规范化、核心算法，以及场景测试难稳定覆盖但有回归风险的分支
- 不为只做字段赋值的 struct `new()`/builder/getter/setter、DTO/derive-only 类型、常量、简单 enum、机械字段映射或第三方库保证行为编写单元测试
- 允许不新增单元测试；若改动只是 DTO、路由注册、字段透传或 OpenAPI 注解，优先用编译、定向场景测试或 OpenAPI 生成验证覆盖
- 场景测试基于 User Story，同时作为 API 使用示例和开发者文档
- 避免编写中间层的单个 API 测试

---

## 环境启动

**推荐路径**：优先直接使用 `uv run scripts/backend-test.py`（内部使用 `cargo nextest` 运行测试）。

```powershell
# 可选：先显式启动测试环境（需要复用环境时）
uv run scripts/test-start.py

# 运行测试
cd backend
uv run scripts/backend-test.py

# 重跑上一次失败测试（基于 nextest recording）
uv run scripts/backend-test.py -- -R latest

# 可选：结束后手动停止测试环境
uv run scripts/test-stop.py
```

**测试环境配置**:
- PgDog: `localhost:16432`（后端测试默认数据库入口，代理到测试 PostgreSQL）
- PostgreSQL: `localhost:15433`（供 PgDog 代理使用，必要时用于专项排障）
- Redis: `localhost:6380`
- 配置文件: `backend/config.toml`（测试配置）
- nextest recording: `backend/.config/nextest.toml` 已启用 `[experimental].record = true` 和 `[record].enabled = true`
- nextest 版本要求: `-R latest` / `--rerun latest` 需要 `cargo-nextest >= 0.9.123`

**详见**: [环境与测试入口](/spec/core/environment-and-testing-guide.md#详细指南入口)

---

## 必须使用 uv run scripts/backend-test.py

```bash
# ✅ 正确
uv run scripts/backend-test.py

```

运行入口会先执行后端测试 DDL 守卫：
- 测试代码中禁止出现 `CREATE TABLE`、`ALTER TABLE`、`DROP TABLE`
- 真实数据库测试的表结构唯一来源是 `backend/app/migrations/*.sql`
- 允许 `CREATE SCHEMA`、`DROP SCHEMA`、`SET search_path` 这类 schema 隔离 SQL
- 如果测试需要新增表/列，先修改 migration，再修改测试数据或断言

---

## Binary Crate 测试

`api/` 是 Binary Crate（有 `src/main.rs`，无 `src/lib.rs`），测试代码在各模块的 `#[cfg(test)]` 中：

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_login_success() {
        // 测试代码
    }
}
```

---

## 测试上下文选择指南

项目提供三种测试上下文，根据测试需求选择合适的上下文可以显著提升测试性能。

### 三种测试上下文对比

| 上下文类型 | 提供能力 | 适用场景 | 初始化时间 |
|-----------|---------|---------|----------|
| **BareSchemaTestContext** | 数据库隔离（无 admin 用户） | 数据库约束、Repository 层测试 | ~2-3s |
| **AuthSchemaTestContext** | + admin 用户、Redis、权限检查 | 登录/注册、会话管理、权限测试 | ~5-7s |
| **SchemaTestContext** | + 完整 AppState（含 billing/points） | E2E API 测试、复杂业务流程 | ~10-15s |

### 选择原则

1. **能轻则轻**：优先用 BareSchemaTestContext
2. **按需升级**：需要认证用户才用 AuthSchemaTestContext
3. **完整集成**：E2E 测试才用 SchemaTestContext
4. **纯逻辑**：不涉及 I/O 的用普通单元测试

### 使用示例

```rust
// BareSchemaTestContext - 只需数据库
use cas_test_support::BareSchemaTestContext;

#[test_context(BareSchemaTestContext)]
#[tokio::test]
async fn test_repository_constraint(ctx: &BareSchemaTestContext) {
    sqlx::query("INSERT INTO users (id, email) VALUES ($1, $2)")
        .bind("test-user")
        .bind("test@example.com")
        .execute(&*ctx.pool)
        .await
        .unwrap();
}

// AuthSchemaTestContext - 需要认证
use cas_test_support::AuthSchemaTestContext;

#[test_context(AuthSchemaTestContext)]
#[tokio::test]
async fn test_user_login(ctx: &AuthSchemaTestContext) {
    let result = ctx.application_service
        .auth_service()
        .login(email, password)
        .await;
    assert!(result.is_ok());
}

// SchemaTestContext - E2E 测试
use crate::tests::schema_test_context::SchemaTestContext as TestContext;

#[test_context(TestContext)]
#[tokio::test]
async fn test_example(ctx: &TestContext) {
    let app = ctx.create_unified_test_router();
    // ctx._realm_id: 预创建的测试 realm ID
    // ctx._client_id: 预创建的测试 client ID
}
```

### SchemaTestContext 详细说明

项目使用 Schema 隔离实现完全并行测试，每个测试使用独立的 PostgreSQL Schema（如 `test_abc123`）。

**特性**：
- Redis 隔离通过 DB 1 实现（测试专用）
- 完全隔离，可并行执行
- 测试结束根据环境变量决定是否清理（默认保留，由 `test-start.py` 统一清理）

### Redis 隔离方案

**统一使用 ConnectionManager**：

```rust
// 生产环境和测试环境统一使用 ConnectionManager
pub struct AppState {
    pub redis: redis::aio::ConnectionManager,  // 自动重连 + 连接池
    // ...
}
```

**测试隔离通过 DB 1 实现**：

测试环境使用 Redis DB 1，与生产环境（DB 0）完全隔离，无需 UUID key prefix。



---

## 单元测试

```rust
#[test]
fn test_unit_hash_password() {
    let password = "test123";
    let hash = hash_password(password);
    assert!(verify_password(password, &hash));
}

#[tokio::test]
async fn test_unit_create_user_success() {
    let mut mock = MockUserRepository::new();
    mock.expect_create_user()
        .with(eq(CreateUserRequest { ... }))
        .returning(|_| Ok(User::new(...)));

    let service = UserServiceImpl::new(Arc::new(mock));
    let result = service.create_user(identity, input).await;
    assert!(result.is_ok());
}
```

---

## 场景测试

基于 User Story 的完整业务流程测试，同时作为 API 使用示例：

```rust
/// User Story: RBAC 完整工作流
#[test_context(TestContext)]
#[tokio::test]
async fn test_scenario_complete_rbac_workflow(ctx: &mut TestContext) {
    let app = ctx.create_test_router();

    // Step 1: 创建权限定义
    let perm = create_permission(
        &app, ctx, &ctx._realm_id, &token, "users.read", "Read users"
    ).await;

    // Step 2: 创建角色
    let role = create_role(
        &app, ctx, &ctx._realm_id, &token, "moderator", "Can moderate users"
    ).await;

    // Step 3: 给角色分配权限
    assign_permission_to_role(
        &app, &ctx._realm_id, &token, &role_id, &perm
    ).await;

    // Step 4: 创建用户
    // Step 5: 给用户分配角色
    // Step 6: 用户登录
    // Step 7: 验证用户权限
    // Step 8: 验证无权限资源被拒绝
}
```

---

## 认证测试统一模式

```rust
#[test_context(TestContext)]
#[tokio::test]
async fn test_admin_endpoint(ctx: &mut TestContext) {
    // 1. 授予超级管理员权限
    {
        let mut enforcer = ctx._app_state.enforcer.write().await;
        enforcer.add_policy(vec![
            ctx._client_id.clone(),
            "test-user".to_string(),
            "All".to_string(),
            "allow".to_string(),
        ]).await.unwrap();
    }

    // 2. 创建会话
    let session_state = SessionData {
        realm_id: ctx._realm_id.clone(),
        client_id: ctx._client_id.clone(),
        user_id: "test-user".to_string(),
    };
    let token = "test-session-token";
    store_session(&ctx._app_state, token, &session_state, 30).await.unwrap();

    // 3. 请求中添加认证头
    let req = Request::builder()
        .uri("/api/admin/users")
        .header("cookie", format!("X-Auth={}", token))
        .body(Body::empty())
        .unwrap();
}
```

---

## Enforcer 锁管理

**关键规则**：中间件中获取 enforcer 锁后，必须在调用 `next.run()` 前释放！

```rust
// ❌ 错误：死锁
pub async fn require_super_admin(/* ... */) -> Result<impl IntoResponse, AuthError> {
    let enforcer = state.enforcer.read().await;
    let is_super_admin = enforcer.enforce(...).unwrap_or(false);
    Ok(next.run(req).await)  // 锁仍未释放
}

// ✅ 正确：使用块作用域确保释放
pub async fn require_super_admin(/* ... */) -> Result<impl IntoResponse, AuthError> {
    let is_super_admin = {
        let enforcer = state.enforcer.read().await;
        enforcer.enforce(...).unwrap_or(false)
    }; // 锁在这里释放

    Ok(next.run(req).await)
}
```

---

## 路由路径测试一致性

```rust
// ✅ 好：使用 format! 宏动态构建路径
.uri(&format!("/api/{}/auth/login", ctx._realm_id))

// ❌ 坏：硬编码路径
.uri("/api/auth/login")
```

---

## 测试 Router 创建规则

**所有场景测试必须使用统一的测试路由函数**：

```rust
let app = ctx.create_unified_test_router();
```

### 包含的路由

`create_unified_test_router()` 包含以下所有 API 路由：

- **Admin API**: 用户管理、权限管理、角色管理、客户端应用管理
- **OAuth Provider API**: Provider 列表、配置 CRUD
- **Realm Config API**: 配置管理、批量操作
- **Realm API**: Realm CRUD

### 优势

- 统一管理所有测试路由
- 避免路由遗漏导致的 404 错误
- 简化测试代码
- 便于维护和更新

---

## 测试容器管理（故障排查/兜底）

```bash
# 主路径：使用脚本运行测试
uv run scripts/backend-test.py -- test_scenario

# 可选：显式管理测试环境
uv run scripts/test-start.py
uv run scripts/test-stop.py
```

**容器信息**：
- PostgreSQL: `localhost:5433` (用户/密码: `postgres`, 数据库: `postgres`)
- Redis: `localhost:6380`

---

## 常见错误

### 404 Not Found

**原因**：路由路径不匹配

**检查**：
1. router() 中的路径定义
2. 嵌套时的完整路径
3. 测试中的 URI 格式

### 401 Unauthorized

**原因**：缺少认证设置

**解决**：按照"认证测试统一模式"添加权限和 session

### 类型推断失败

```rust
error[E0282]: type annotations needed
```

**解决**：
```rust
use cas_core::infrastructure::authorization::role_policy_repository::RolePolicyRepository;
// 使用 RolePolicyRepository 创建策略
```

---

## 运行测试的常用命令

### 代码质量检查（后端最终收口必做）

在 backend `accept` 通过后的最终收口阶段，必须按以下顺序执行：

```bash
/simplify
cargo clippy --fix --allow-dirty --allow-staged --all-targets --all-features
cargo fmt --all
uv run scripts/backend-test.py
```

规则：
- 这组命令属于 `/t-backend-finalize [feature]` 的固定流程。
- `backend-test` 默认仍先做变更分析和定向测试，不直接跳到全量测试。
- `backend-accept` 也默认沿用定向测试验证，不直接运行全量 `uv run scripts/backend-test.py`。
- 若最终全量测试失败，先修复，再重新执行 `cargo clippy --fix -> cargo fmt --all -> uv run scripts/backend-test.py`。

### 测试执行命令

```bash
# 运行所有测试（日志化，节省 token）
uv run scripts/backend-test.py

# 重跑上一次失败测试
uv run scripts/backend-test.py -- -R latest

# 等价长参数写法
uv run scripts/backend-test.py -- --rerun latest

# 运行特定模块
uv run scripts/backend-test.py -- auth::tests

# 运行特定测试
uv run scripts/backend-test.py -- test_login_success

# 只运行单元测试
uv run scripts/backend-test.py -- test_unit

# 只运行场景测试
uv run scripts/backend-test.py -- test_scenario

# 显示测试输出
uv run scripts/backend-test.py -- --success-output=immediate
```

### 测试输出日志化（节省 Token）

为了减少 token 消耗，测试执行输出应重定向到日志文件：

```bash
# 运行测试并保存到日志文件
cd backend
uv run scripts/backend-test.py

# 需要完整上下文时再查看日志
Get-Content backend-test-output.log -Tail 200
```

**使用场景**：
- AI agent 运行测试时使用日志化方式
- 失败时脚本会直接输出精简摘要，日志用于补充完整上下文
- 完整日志保存在 `backend-test-output.log` 供详细分析时使用

---

## 开发工具和辅助

### Context7 实时文档查询

AI 会自动使用 Context7 获取测试库最新文档。

**常用库**:
- uv run scripts/backend-test.py: 查询特定用法
- testcontainers: `/testcontainers/testcontainers-rs`
- wiremock: `/lukemathwalker/cargo-wiremock`

**注意**：项目特定的测试隔离和清理策略仍需遵循本文档。
