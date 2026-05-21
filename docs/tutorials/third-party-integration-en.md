# Integrating a Third-Party Backend with Herald

Herald is a standalone authentication and billing service. Your backend doesn't need to handle user registration, login, permissions, or billing — delegate all of that to Herald and focus on business logic.

```
Browser → Your Frontend → Your Backend → Herald SDK → Herald Service
                                         (verify token, check perms, deduct points)
```

Your backend never connects to Herald's database. All interaction goes through HTTP API, with SDK-side caching.

## Prerequisites

Before writing code, set up these in the Herald admin console:

1. Create a realm (tenant), note the `realm_id`
2. Create a client app under that realm (represents your service), note the `client_id`
3. Generate an API Key for the realm, save the secret (shown only once)
4. Define your permission points (covered below)
5. Create roles and assign permissions
6. Create an admin user and assign the role

## Backend Integration

### Install the SDK

Add to `Cargo.toml`:

```toml
[dependencies]
herald-sdk = "0.1"
```

### Initialize the Client

Create the SDK client at startup with Herald's address and your API Key:

```rust
use herald_sdk::Client;
use std::sync::Arc;
use std::time::Duration;

let herald_client = Arc::new(Client::new(
    "http://127.0.0.1:3000".to_string(),  // Herald address
    "your-api-key".to_string(),            // API Key
    Some(Duration::from_secs(300)),        // Cache TTL, default 5 min
));
```

The cache duration is fine as-is. The SDK invalidates cached entries when tokens expire.

### Write an Auth Middleware

Add middleware to protected routes. Extract the `X-Auth` token from cookies, then call Herald to verify permissions:

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
    // 1. Extract token from Cookie
    let token = extract_token(&request);
    let Some(token) = token else {
        return (axum::http::StatusCode::UNAUTHORIZED, "missing token").into_response();
    };

    // 2. Map request path to a permission rule
    let rule = match extract_rule(request.uri().path(), request.method()) {
        Some(r) => r,
        None => return (axum::http::StatusCode::FORBIDDEN, "unrecognized path").into_response(),
    };

    // 3. Call Herald
    let result = state.herald_sdk
        .check_permission(PermissionCheckRequest {
            token,
            rules: Some(vec![rule]),
            client_id: state.client_id.clone(),
        })
        .await;

    match result {
        Ok(resp) if resp.allowed => {
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

When Herald is unavailable, return 503. Don't let requests through without authentication.

### Design Your Permission Model

Permissions have two dimensions: **resource** and **action**. You define them; Herald stores and enforces them.

Example for an IoT platform:

| Path prefix | Resource | HTTP method | Action |
|-------------|----------|-------------|--------|
| `/admin/product*` | `product` | GET | `read` |
| `/admin/product*` | `product` | POST, PUT, DELETE | `write` |
| `/admin/device*` | `device` | GET | `read` |
| `/admin/device*` | `device` | POST, PUT, DELETE | `write` |

Path-to-permission mapping happens in your middleware, not in Herald. Herald only answers "can this user do `product:read`?".

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

After defining these, configure them as permission points in the Herald admin console, create roles, and assign roles to users.

### Mount on Routes

Only apply the middleware to routes that need authentication:

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

## Frontend SSO

When a user opens your admin panel, check for an active Herald session. If none, redirect to Herald's login page.

### Login Redirect

Check in a route `beforeLoad` hook:

```typescript
beforeLoad: async () => {
    const { data } = await apiClient.get('/api/auth/config')
    if (!data.enabled) return  // Herald not configured, skip auth

    const authed = await checkSession()
    if (!authed) {
        window.location.href = data.login_url  // Redirect to Herald login
    }
}
```

The `login_url` from the backend follows the format `{base_url}/{realm_id}/auth/login`. The frontend doesn't construct URLs.

### Cookie Sharing

After login, Herald sets an `X-Auth` cookie in the browser. As long as Herald and your service share a host or root domain, the browser sends this cookie to your backend automatically. The user just navigates back to your page after logging in — no callback URL needed.

### Session Expiry

Add a 401 interceptor to your API client:

```typescript
apiClient.interceptors.response.use(
    response => response,
    error => {
        if (error.response?.status === 401) {
            window.location.href = '/api/auth/config'  // Re-trigger login flow
        }
        return Promise.reject(error)
    }
)
```

Use an `isRedirecting` flag to prevent multiple concurrent 401s from triggering duplicate redirects.

## Points System

If your service needs usage-based billing (e.g., AI API call counts), use Herald's points system.

### Check Balance

```rust
let balance = herald_client.get_balance("my-realm", &user_id).await?;
println!("Balance: {} {}", balance.balance, balance.unit);
```

### Deduct Points

```rust
let result = herald_client.consume_points(
    "my-realm",
    &user_id,
    "my-client-app",   // your client_app_id
    100,                // deduct 100 points
    Some("AI API call".to_string()),
    Some("unique-request-id".to_string()),  // idempotency key
).await?;
println!("Balance after: {}", result.balance_after);
```

Always pass an `idempotency_key`. On network timeout retries, the same key won't double-charge.

## Subscription System

Query a client app's subscription status to gate features:

```rust
let sub = herald_client.get_subscription("my-realm", "my-client-app").await?;
if sub.status == "active" {
    // user has a paid subscription
}
```

## Deployment

### Configuration

Your service needs four settings:

| Setting | Example | Description |
|---------|---------|-------------|
| `base_url` | `http://herald:3000` | Herald address, use container name in Docker |
| `api_key` | `sk_xxxx...` | API Key from Herald admin console |
| `realm_id` | `my-app` | Your service's realm |
| `client_id` | `my-app-admin` | Client identifier |

### Cookie Sharing Requirement

Herald and your service must be on the same host or root domain for browser cookies to work. Common patterns:

- Same host, different ports (`127.0.0.1:3000` and `127.0.0.1:8080`)
- Reverse proxy with unified entry (Caddy/Nginx routes `/auth` to Herald, `/` to your service)
- Same root domain subdomains (`auth.example.com` and `app.example.com`)

### Running Without Herald

For local development or isolated intranets, don't initialize the SDK client. Your admin API will run without authentication. Other auth mechanisms (device HMAC, etc.) are unaffected.
