# 第三方后端服务对接 Herald

Herald 是独立的统一认证和计费服务。你的后端服务不需要自己管用户注册、登录、权限、计费，把这些全交给 Herald，自己只做业务逻辑。

两个服务的关系：

```
用户浏览器 → 你的前端 → 你的后端 → Herald SDK → Herald 服务
                                    (校验 token、查权限、扣积分)
```

你的后端不直接连 Herald 的数据库，所有交互走 HTTP API，SDK 内部有缓存。

## 前置准备

对接之前，在 Herald 管理端完成这些：

1. 创建一个 realm（租户），记住 `realm_id`
2. 在 realm 下创建一个 client app（代表你的服务），记住 `client_id`
3. 为这个 realm 生成一个 API Key，记住密钥值（只显示一次）
4. 定义你的权限点（后面会讲怎么设计）
5. 创建角色并分配权限
6. 创建一个管理员用户，分配角色

这些步骤在 Herald 的管理后台操作，不需要写代码。

## 后端集成

### 安装 SDK

在 `Cargo.toml` 中添加依赖：

```toml
[dependencies]
herald-sdk = "0.1"
```

### 初始化客户端

启动时创建 SDK 客户端。需要 Herald 服务地址和 API Key：

```rust
use herald_sdk::Client;
use std::sync::Arc;
use std::time::Duration;

let herald_client = Arc::new(Client::new(
    "http://127.0.0.1:3000".to_string(),  // Herald 地址
    "your-api-key".to_string(),            // API Key
    Some(Duration::from_secs(300)),        // 缓存时间，默认 5 分钟
));
```

缓存时间不用改。SDK 会在 token 过期时自动失效缓存。

### 写认证中间件

在需要保护的 API 路由上加中间件，从请求的 Cookie 中提取 `X-Auth` token，调用 Herald 校验权限：

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

Herald 不可用时返回 503，不要放行。宁可服务暂时不可用，也不能让未认证的请求进来。

### 设计权限模型

权限分两个维度：**资源**（resource）和**操作**（action）。你自己定义，Herald 只管存储和校验。

比如一个 IoT 平台可能这样定义：

| 路径前缀 | 资源 | HTTP 方法 | 操作 |
|----------|------|-----------|------|
| `/admin/product*` | `product` | GET | `read` |
| `/admin/product*` | `product` | POST, PUT, DELETE | `write` |
| `/admin/device*` | `device` | GET | `read` |
| `/admin/device*` | `device` | POST, PUT, DELETE | `write` |

路径到权限的映射在中间件里做，跟 Herald 无关。Herald 只回答"这个用户能不能 `product:read`"。

提取权限规则的函数大概长这样：

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

定义好之后，在 Herald 管理端把这些资源/操作配成权限点，创建角色，给用户分配角色。

### 挂到路由上

只对需要认证的路由加中间件。设备回调、健康检查这些不需要：

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

// 这些不需要认证
let public_routes = axum::Router::new()
    .route("/health", get(health_check))
    .route("/webhook/device", post(device_webhook));

let app = axum::Router::new()
    .merge(protected_routes)
    .merge(public_routes);
```

## 前端 SSO

用户打开你的管理后台时，检查有没有 Herald 的 session。没有就跳到 Herald 登录页。

### 登录跳转

在路由的 `beforeLoad` 钩子里检查：

```typescript
beforeLoad: async () => {
    // 先问后端 Herald 是否开启
    const { data } = await apiClient.get('/api/auth/config')
    if (!data.enabled) return  // 没开 Herald，不需要认证

    // 发一个探测请求，看 session 是否有效
    const authed = await checkSession()
    if (!authed) {
        window.location.href = data.login_url  // 跳到 Herald 登录
    }
}
```

后端返回的 `login_url` 格式是 `{base_url}/{realm_id}/auth/login`，前端不用拼。

### Cookie 共享

Herald 登录成功后会在浏览器设置 `X-Auth` Cookie。只要 Herald 和你的服务在同一个主机或同根域下，浏览器会自动把这个 Cookie 带给你的后端请求。用户登录后手动回到你的页面就行，不需要回调中转。

### 会话过期

API 客户端加个 401 拦截器，过期了自动跳登录页：

```typescript
apiClient.interceptors.response.use(
    response => response,
    error => {
        if (error.response?.status === 401) {
            window.location.href = '/api/auth/config'  // 重新走登录流程
        }
        return Promise.reject(error)
    }
)
```

用 `isRedirecting` 标记防止多个并发 401 触发多次跳转。

## 积分系统

如果你的服务需要按量计费（比如 AI API 调用次数），可以用 Herald 的积分系统。

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
    Some("unique-request-id".to_string()),  // 幂等键，防重复扣
).await?;
println!("扣费后余额: {}", result.balance_after);
```

`idempotency_key` 建议每次请求都传。网络超时重试时，相同的 key 不会重复扣费。

## 订阅系统

查询某个 client app 的订阅状态，判断用户能用到什么功能：

```rust
let sub = herald_client.get_subscription("my-realm", "my-client-app").await?;
if sub.status == "active" {
    // 用户有付费订阅
}
```

也可以查可用的套餐计划，展示给用户选择。

## 部署

### 配置

你的服务需要四个配置项：

| 配置项 | 示例 | 说明 |
|--------|------|------|
| `base_url` | `http://herald:3000` | Herald 服务地址，Docker 网络内用容器名 |
| `api_key` | `sk_xxxx...` | API Key，在 Herald 管理端生成 |
| `realm_id` | `my-app` | 你的服务所属的 realm |
| `client_id` | `my-app-admin` | 客户端标识 |

### Cookie 共享要求

Herald 和你的服务必须部署在同一主机或同根域下，否则浏览器 Cookie 不共享。三种常见部署方式：

- 同主机不同端口（`127.0.0.1:3000` 和 `127.0.0.1:8080`）
- 反向代理统一入口（Caddy/Nginx 把 `/auth` 转发到 Herald，`/` 转发到你的服务）
- 同根域子域名（`auth.example.com` 和 `app.example.com`）

### 不用 Herald 的场景

本地开发或内网隔离环境，不初始化 SDK 客户端就行。你的管理端 API 不做认证。设备端或其他独立认证体系不受影响。
