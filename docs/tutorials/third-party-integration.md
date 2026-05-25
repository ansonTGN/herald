# 第三方后端服务对接 Herald

Herald 是独立的认证和计费服务。你的后端不用自己管用户、登录、权限、计费，全部交给 Herald 处理，自己只写业务逻辑。

```
用户浏览器 → 你的前端 → 你的后端 → Herald SDK → Herald 服务
                                    (校验 token、查权限、扣积分)
```

你的后端不直连 Herald 的数据库，所有交互走 HTTP API，SDK 内部带缓存。

## 1. 前置准备

在 Herald 管理后台完成以下操作，不需要写代码：

1. 创建一个 realm（租户），记下 `realm_id`
2. 在 realm 下创建一个 client app（代表你的服务），记下 `client_id`
3. 为这个 realm 生成一个 API Key，保存密钥值——只显示一次，丢了得重新生成。生成时选择对应的 Client App；不选择时默认绑定 `admin-api-client`
4. 定义权限点。权限格式是 `resource:action`，比如 `product:read`、`device:manage`
5. 创建角色，把权限点分配给角色
6. 创建一个管理员用户，把角色分配给这个用户

第 4 步的权限点怎么设计，下面"设计权限模型"一节会讲。

## 2. 后端集成

### 安装 SDK

在 `Cargo.toml` 中添加：

```toml
[dependencies]
herald-sdk = "0.1"
```

### 初始化客户端

启动时创建 SDK 客户端。两个必填参数：Herald 服务地址和 API Key。

```rust
use herald_sdk::Client;
use std::sync::Arc;
use std::time::Duration;

let herald_client = Arc::new(Client::new(
    "http://127.0.0.1:3000".to_string(),  // Herald 地址
    "your-api-key".to_string(),            // API Key
    Some(Duration::from_secs(300)),        // 缓存时间
));
```

API Key 是你的后端跟 Herald 之间的身份凭证，存在环境变量或配置文件里，不要硬编码。缓存时间 5 分钟够用，SDK 会在 token 过期时自动失效缓存条目，不需要手动管理。

API Key 同时带有 Client App 范围。绑定 `admin-api-client` 的 Key 是 realm 级管理 Key，可以访问该 realm 下的所有 Client App 资源；绑定普通 Client App 的 Key 只能访问该 Client App 的权限检查、订阅和积分资源。禁用 Client App 会立即让绑定到它的 API Key 失效。

### 写认证中间件

中间件做三件事：从 Cookie 提取 token、把请求路径映射成权限规则、调 Herald 校验。Herald 不可用时返回 503，不要放行——宁可服务暂时不可用，也不能让未认证的请求进来。

```rust
use axum::{
    extract::{Request, State},
    middleware::Next,
    response::Response,
};
use herald_sdk::{Client, PermissionCheckRequest, Rule};

#[derive(Clone)]
pub struct AuthState {
    pub herald_sdk: Arc<Client>,
    pub client_id: String,
}

pub async fn auth_middleware(
    State(state): State<AuthState>,
    mut request: Request,
    next: Next,
) -> Response {
    // 1. 从 Cookie 中提取 token
    let token = extract_token(&request);
    let Some(token) = token else {
        return (axum::http::StatusCode::UNAUTHORIZED, "missing token").into_response();
    };

    // 2. 根据请求路径生成权限规则
    let rule = match extract_rule(request.uri().path(), request.method()) {
        Some(r) => r,
        None => return (axum::http::StatusCode::FORBIDDEN, "unrecognized path").into_response(),
    };

    // 3. 调 Herald 校验
    let result = state.herald_sdk
        .check_permission(PermissionCheckRequest {
            token,
            rules: Some(vec![rule]),
            client_id: state.client_id.clone(),
        })
        .await;

    match result {
        Ok(resp) if resp.allowed => {
            // 把 user_id 注入请求，后续 handler 可以用
            if let Some(user_id) = resp.user_id {
                request.extensions_mut().insert(CurrentUser { user_id });
            }
            next.run(request).await
        }
        Ok(_) => (axum::http::StatusCode::FORBIDDEN, "permission denied").into_response(),
        Err(_) => (axum::http::StatusCode::SERVICE_UNAVAILABLE, "auth unavailable").into_response(),
    }
}
```

`CurrentUser` 是你自己定义的结构体，从 request extensions 中提取就行。中间件的 `Err(_)` 分支没有区分网络错误和其他错误，统一返回 503 是对的——你的后端不应该在认证服务挂掉时继续处理请求。

### 设计权限模型

权限分两个维度：**资源**（resource）和**操作**（action）。你自己定义，Herald 只管存储和校验。

以一个 IoT 平台为例，路径到权限的映射在中间件里做：

```rust
fn extract_rule(path: &str, method: &Method) -> Option<Rule> {
    let resource = if path.starts_with("/admin/product") {
        "product"
    } else if path.starts_with("/admin/device") {
        "device"
    } else {
        return None;
    };

    let action = match *method {
        Method::GET => "read",
        _ => "write",
    };

    Some(Rule {
        resource: resource.to_string(),
        action: action.to_string(),
    })
}
```

这段代码和 Herald 没有关系。Herald 只回答一个问题："这个用户能不能做 `product:read`？"。至于哪个路径对应哪个权限，是你的中间件决定的。

路径映射设计好之后，去 Herald 管理后台把 `product:read`、`product:write`、`device:read`、`device:write` 配成权限点，创建角色，给用户分配角色。

**操作的层级关系**：Herald 内置了 action 层级。`manage` 覆盖 `view`、`create` 和 `manage` 本身。`create` 只覆盖 `create`。`view` 只覆盖 `view`。自定义 action（比如 `admin`）只匹配自身，不参与层级。

所以如果你给用户分配了 `product:manage`，中间件请求 `product:read` 时也会通过。

### 挂到路由上

只对需要认证的路由加中间件。设备回调、健康检查这些公开接口不加：

```rust
let protected_routes = axum::Router::new()
    .route("/admin/product", get(list_products))
    .route("/admin/device", get(list_devices))
    .layer(axum::middleware::from_fn_with_state(
        AuthState {
            herald_sdk: herald_client,
            client_id: "my-service-admin".to_string(),
        },
        auth_middleware,
    ));

let public_routes = axum::Router::new()
    .route("/health", get(health_check))
    .route("/webhook/device", post(device_webhook));

let app = axum::Router::new()
    .merge(protected_routes)
    .merge(public_routes);
```

`client_id` 传你在前置准备中创建的那个。Herald 用它来区分不同的客户端应用。

## 3. 通过 SDK 管理资源

API Key 在 Herald 中也是一种 Principal（主体），跟用户一样可以分配角色和权限。你给 API Key 分配了什么权限，通过这个 Key 发出的 SDK 调用就能做什么。权限仍然受 Client App 范围限制：普通 Client App Key 即使有角色，也不能越权访问其他 Client App 的订阅、积分或权限检查。

### 管理 Realm

```rust
// 创建 Realm（需要 admin realm 权限）
let realm = herald_client.create_realm(CreateRealmSdkRequest {
    name: "my-app".to_string(),
    description: Some("我的应用".to_string()),
    admin_user: AdminUserSdkInput {
        email: "admin@example.com".to_string(),
        password: "secure-password".to_string(),
    },
}).await?;

// 列出可访问的 Realm
let realms = herald_client.list_realms().await?;

// 查询 Realm 详情
let realm = herald_client.get_realm("my-realm").await?;
```

创建 Realm 需要 API Key 属于 admin realm 且有 `realm:create` 权限。列出和查询 Realm 只能访问 API Key 所属 realm 有权看到的范围。

### 管理用户

```rust
// 创建用户
let user = herald_client.create_user("my-realm", CreateUserSdkRequest {
    email: "user@example.com".to_string(),
    password: "secure-password".to_string(),
    nickname: Some("johndoe".to_string()),
}).await?;

// 列出用户
let users = herald_client.list_users("my-realm").await?;

// 查询用户详情
let user = herald_client.get_user("my-realm", &user_id).await?;
```

### 管理 Client App

```rust
// 创建 Client App
let app = herald_client.create_client_app("my-realm", CreateClientAppSdkRequest {
    name: "Mobile App".to_string(),
    description: Some("iOS and Android app".to_string()),
    redirect_uris: vec!["https://app.example.com/callback".to_string()],
}).await?;

// 列出 Client App
let apps = herald_client.list_client_apps("my-realm").await?;

// 查询 Client App 详情
let app = herald_client.get_client_app("my-realm", "my-mobile-app").await?;
```

所有操作都受 API Key 权限范围约束。SDK 调用不带 realm 范围外的数据。如果你的服务只服务一个 Client App，建议给后端使用绑定该 Client App 的 API Key；只有需要跨 Client App 管理时才使用默认的 `admin-api-client` Key。

## 4. 前端 OAuth 登录

### OAuth 2.1 + PKCE 流程（推荐用于 SPA）

如果你的前端是单页应用（SPA），用 OAuth 2.1 Authorization Code + PKCE 流程。

**第一步**：前端生成 `code_verifier`（随机字符串）和 `code_challenge`（SHA256 哈希后 Base64url 编码）。

**第二步**：把用户重定向到 Herald 的授权端点：

```
GET /api/oauth/{realmId}/authorize?client_id=your-client-id&redirect_uri=https://app.example.com/callback&state=random-state&code_challenge=BASE64URL(SHA256(code_verifier))&code_challenge_method=S256&response_type=code
```

用户在 Herald 登录页完成认证后，Herald 把用户重定向回你的 `redirect_uri`，带上 `code` 和 `state` 参数。

**第三步**：你的后端用授权码换 token：

```
POST /api/oauth/{realmId}/token
Content-Type: application/json

{
  "grant_type": "authorization_code",
  "code": "收到的授权码",
  "redirect_uri": "https://app.example.com/callback",
  "client_id": "your-client-id",
  "code_verifier": "第一步生成的原始字符串"
}
```

返回：

```json
{
  "access_token": "...",
  "token_type": "Bearer",
  "expires_in": 1800
}
```

**第四步**：拿到 `access_token` 后，设成 `X-Auth` Cookie（`httpOnly`、`secure`、`sameSite=Lax`），后续请求浏览器会自动带上。

`state` 参数防止 CSRF 攻击。`code_challenge`（PKCE）防止授权码被截获。授权码是一次性的——Herald 用 `GETDEL` 原子操作读取并删除，同一个 code 换第二次会报错。

### Device Code 流程（适用于 IoT / CLI）

设备没有浏览器？用 Device Code 流程。

**请求设备码**：

```
POST /api/device/{realmId}/authorize
Content-Type: application/x-www-form-urlencoded

client_id=your-client-id
```

返回：

```json
{
  "device_code": "...",
  "user_code": "BCDF-GHJK",
  "verification_uri": "https://herald/{realmId}/device",
  "verification_uri_complete": "https://herald/{realmId}/device/BCDF-GHJK",
  "expires_in": 900,
  "interval": 5
}
```

`user_code` 显示给用户（在另一个有浏览器的设备上打开 `verification_uri` 输入），设备端每隔 5 秒轮询一次 token 端点：

```
POST /api/device/{realmId}/token
Content-Type: application/x-www-form-urlencoded

grant_type=urn:ietf:params:oauth:grant-type:device_code&device_code=收到的device_code
```

用户完成授权后，轮询返回 access token。轮询太频繁会收到 `slow_down` 错误，interval 自动加 5 秒。设备码有效期 15 分钟。

使用 Device Code 流程需要在 Client App 配置中启用 `device_code_grant_enabled`。

### 会话管理

Cookie 名称：`X-Auth`，属性：`httpOnly`、`secure`（生产环境）、`sameSite=Lax`。

默认会话 TTL 1800 秒（30 分钟），可在 Client App 配置中修改 `session_ttl_seconds`。OAuth 流程返回的 token TTL 同样读取 Client App 的 `session_ttl_seconds` 配置，默认也是 1800 秒。

**滑动续期**：如果 Client App 配置了 `session_renewal_ttl_seconds`，Herald 的 identity middleware 在每次请求时检查剩余 TTL。当剩余 TTL <= `renewal_ttl_seconds / 2` 时，自动把 session TTL 续期到 `renewal_ttl_seconds`，同时在响应头中更新 Cookie。

三种续期策略：

| 策略 | 配置 | 效果 |
|------|------|------|
| 严格 | `session_ttl=300, renewal_ttl=null` | 5 分钟硬过期，不续期 |
| 宽松 | `session_ttl=28800, renewal_ttl=28800` | 8 小时，活跃用户永不过期 |
| 渐进 | `session_ttl=300, renewal_ttl=7200` | 初始 5 分钟，首次续期后延长到 2 小时 |

渐进模式最适合管理后台：短时间操作够了，长时间使用自动续期，关掉浏览器就过期。

前端 401 拦截器处理会话过期：

```typescript
apiClient.interceptors.response.use(
    response => response,
    error => {
        if (error.response?.status === 401) {
            window.location.href = '/api/auth/config'
        }
        return Promise.reject(error)
    }
)
```

用 `isRedirecting` 标记防止多个并发 401 触发多次跳转。

## 5. 积分系统

如果你的服务按量计费（比如 AI API 调用次数），用 Herald 的积分系统。

### 查余额

```rust
let balance = herald_client.get_balance("my-realm", &user_id).await?;
println!("余额: {} {}", balance.balance, balance.unit);
```

### 扣积分

```rust
let result = herald_client.consume_points(
    "my-realm",
    &user_id,
    "my-client-app",   // 你的 client_app_id
    100,                // 扣 100 积分
    Some("AI API 调用".to_string()),
    Some("unique-request-id".to_string()),  // 幂等键
).await?;
println!("扣费后余额: {}", result.balance_after);
```

`idempotency_key` 建议每次请求都传。网络超时重试时，相同的 key 不会重复扣费——这个 key 的唯一性由你的业务决定，用请求 ID 或者业务 ID 都行。

如果 SDK 使用的是普通 Client App API Key，这里的 `client_app_id` 必须是该 Key 绑定的 Client App。`admin-api-client` Key 可以为同一 realm 下任意 Client App 扣积分。

余额不足时 Herald 返回错误，你的业务代码需要处理这个情况（拒绝请求或提示用户充值）。

## 6. 订阅系统

查询 Client App 的订阅状态，判断用户能用到什么功能：

```rust
let sub = herald_client.get_subscription("my-realm", "my-client-app").await?;
if sub.status == "active" {
    // 用户有付费订阅
}
```

普通 Client App API Key 只能查询自身绑定 Client App 的订阅状态；`admin-api-client` Key 可查询同一 realm 下任意 Client App。

查询可用的套餐计划展示给用户选择：

```rust
let plans = herald_client.list_plans("my-realm").await?;
for plan in &plans {
    println!("{}: {} ({} 分)", plan.title, plan.name, plan.price);
}
```

套餐计划通过 Herald 管理后台配置，支持绑定支付平台（Creem 等）。用户在你的前端选择套餐后，Herald 创建支付会话，用户跳转到支付平台完成付款，支付平台通过 Webhook 通知 Herald 更新订阅状态。

你的后端只需要调 SDK 查状态和查计划，支付流程由 Herald 和支付平台处理。

## 7. 部署

### 配置项

你的服务需要四个 Herald 相关的配置项：

| 配置项 | 示例值 | 说明 |
|--------|--------|------|
| `herald_base_url` | `http://herald:3000` | Herald 服务地址，Docker 网络内用容器名 |
| `herald_api_key` | `sk_xxxx...` | API Key，在 Herald 管理端生成 |
| `herald_realm_id` | `my-app` | 你的服务所属的 realm |
| `herald_client_id` | `my-app-admin` | 客户端标识 |

### Cookie 共享要求

Herald 登录成功后在浏览器设置 `X-Auth` Cookie。你的后端中间件需要读到这个 Cookie，所以 Herald 和你的服务必须部署在同一主机或同根域下，否则浏览器不会把 Cookie 带给你的后端。

三种常见部署方式：

- 同主机不同端口（`127.0.0.1:3000` 和 `127.0.0.1:8080`）——开发环境最方便
- 反向代理统一入口（Caddy 或 Nginx 把 `/auth` 转发到 Herald，`/` 转发到你的服务）——生产环境推荐
- 同根域子域名（`auth.example.com` 和 `app.example.com`）——需要配置 Cookie Domain

### 不用 Herald 的场景

本地开发或内网隔离环境，不初始化 SDK 客户端就行。你的管理端 API 不做认证，设备端或其他独立认证体系不受影响。判断方式：配置文件中没有 `herald_base_url` 就跳过 Herald 初始化。
