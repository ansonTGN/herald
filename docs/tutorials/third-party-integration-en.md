# Integrating a Third-Party Backend with Herald

Herald is a standalone authentication and billing service. Your backend delegates user registration, login, permissions, and billing to Herald and focuses on business logic.

```
Browser -> Your Frontend -> Your Backend -> Herald SDK -> Herald Service
                                            (verify token, check perms, deduct points)
```

Your backend never connects to Herald's database. All interaction goes through HTTP API, with SDK-side caching.

## Prerequisites

Complete these steps in the Herald admin console before writing code.

**Create a realm.** A realm is a tenant boundary. All users, permissions, and billing live within a realm. Note the `realm_id` -- you'll pass it to every SDK call.

**Create a client app** under that realm. A client app represents your service (web admin panel, mobile app, CLI tool). Note the `client_id`. If you need different login behaviors (session duration, redirect URIs), create separate client apps for each.

**Generate an API Key** for the realm. Choose the client app this backend serves. If you leave it empty, Herald binds the key to `admin-api-client`. The secret is shown once. Store it somewhere your backend can read at startup -- environment variable or secret manager, not hardcoded.

**Define permission points.** Permissions use a `resource:action` format. You define what resources and actions exist based on your domain. Herald stores and enforces them. The action hierarchy matters: `manage` covers `view`, `create`, and `manage` itself. `create` only covers `create`. `view` only covers `view`. Hierarchy only applies within the same resource.

**Create roles and assign permissions.** Bundle permission points into roles. A "product admin" role might have `product.manage`, which implicitly grants `product.view` and `product.create`.

**Create an admin user and assign the role.** This user will be the first person who can log into your admin panel.

## Backend Integration

### Install the SDK

```toml
[dependencies]
herald-sdk = "0.1"
```

### Initialize the Client

Create the SDK client at application startup with Herald's address and your API key:

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

The API key authenticates your backend to Herald. The SDK sends it as the `X-API-Key` header on every request.

API keys also carry a client app scope. A key bound to `admin-api-client` is realm-wide and can access resources for any client app in that realm. A key bound to an ordinary client app can only access permission checks, subscriptions, and points for that client app. Disabling a client app immediately disables the API keys bound to it.

The cache duration of 5 minutes is fine for most cases. The SDK tracks when it last saw a token and automatically invalidates cached permission checks when that token reaches the 5-minute threshold. You don't need to manage cache invalidation yourself.

### Write an Auth Middleware

The middleware extracts the `X-Auth` token from cookies, maps the request path to a permission rule (resource + action), and asks Herald whether the user is allowed through.

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
    let token = extract_token(&request);
    let Some(token) = token else {
        return (axum::http::StatusCode::UNAUTHORIZED, "missing token").into_response();
    };

    let rule = match extract_rule(request.uri().path(), request.method()) {
        Some(r) => r,
        None => return (axum::http::StatusCode::FORBIDDEN, "unrecognized path").into_response(),
    };

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

When Herald is down, the middleware returns 503. This is intentional. Never let unauthenticated requests through because the auth service is unavailable -- a temporary outage is better than a security hole.

### Design Your Permission Model

Permissions have two dimensions: resource and action. You define them; Herald stores and enforces.

Example for an IoT platform:

| Path prefix           | Resource | HTTP method         | Action  |
|-----------------------|----------|---------------------|---------|
| `/admin/product*`     | `product`| GET                 | `read`  |
| `/admin/product*`     | `product`| POST, PUT, DELETE   | `write` |
| `/admin/device*`      | `device` | GET                 | `read`  |
| `/admin/device*`      | `device` | POST, PUT, DELETE   | `write` |

Path-to-permission mapping happens in your middleware. Herald only answers one question: can this user do `product:read`?

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

The action hierarchy in Herald works like this: if a role has `product:manage`, the user passes permission checks for `product:view`, `product:create`, and `product:manage`. If the role only has `product:create`, it only passes `product:create` -- not `product:view`. Granting `manage` is usually simpler than granting individual actions.

After defining your permission model, configure these resource/action pairs as permission points in the Herald admin console. Create roles that bundle those permissions, then assign roles to users.

### Mount on Routes

Only apply the auth middleware to routes that need it. Health checks and webhook receivers typically don't.

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

The `client_id` in `AuthState` tells Herald which client app this permission check is for. Use the `client_id` you created in the prerequisites.

## Resource Management via SDK

API keys act as principals in Herald's RBAC system. Assign roles and permissions to an API key the same way you would for a user, and the key can perform the corresponding operations through the SDK. Roles do not bypass client app scope: an ordinary client app key cannot access another client app's permission checks, subscriptions, or points.

### Manage Realms

Realm creation requires the API key to belong to the admin realm and have the `realm:create` permission.

```rust
use herald_sdk::{CreateRealmSdkRequest, AdminUserSdkInput};

// Create a realm
let realm = herald_client.create_realm(CreateRealmSdkRequest {
    name: "my-app".to_string(),
    description: Some("My application".to_string()),
    admin_user: AdminUserSdkInput {
        email: "admin@example.com".to_string(),
        password: "secure-password".to_string(),
    },
}).await?;

// List accessible realms
let realms = herald_client.list_realms().await?;

// Get realm details
let realm = herald_client.get_realm("my-realm").await?;
```

Every realm needs an admin user at creation time. The `admin_user` field is required, not optional.

### Manage Users

```rust
use herald_sdk::CreateUserSdkRequest;

// Create a user
let user = herald_client.create_user("my-realm", CreateUserSdkRequest {
    email: "user@example.com".to_string(),
    password: "secure-password".to_string(),
    nickname: Some("johndoe".to_string()),
}).await?;

// List users
let users = herald_client.list_users("my-realm").await?;

// Get user details
let user = herald_client.get_user("my-realm", &user.id).await?;
```

### Manage Client Apps

```rust
use herald_sdk::CreateClientAppSdkRequest;

// Create a client app
let app = herald_client.create_client_app("my-realm", CreateClientAppSdkRequest {
    name: "Mobile App".to_string(),
    description: Some("iOS and Android app".to_string()),
    redirect_uris: vec!["https://app.example.com/callback".to_string()],
}).await?;

// List client apps
let apps = herald_client.list_client_apps("my-realm").await?;

// Get client app details
let app = herald_client.get_client_app("my-realm", "app-001").await?;
```

The server generates the `client_id` when you create a client app. Check the `client_id` field in the response.

All SDK operations are scoped to the API key's realm. The API key determines what you can access -- there is no way to escape the realm boundary through the SDK. If your backend serves one client app, prefer an API key bound to that client app. Use the default `admin-api-client` key only for realm-level management across client apps.

## Frontend OAuth Login

### OAuth 2.1 + PKCE Flow (for SPAs)

Standard three-legged OAuth with PKCE. Your SPA never handles passwords directly.

Step 1: Your SPA generates a `code_verifier` (random string) and computes `code_challenge` (SHA256 of the verifier, base64url-encoded without padding).

Step 2: Redirect the user's browser to:

```
GET /api/oauth/{realmId}/authorize?client_id=your-client-id&redirect_uri=https://app.example.com/callback&state=random-state&code_challenge=E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM&code_challenge_method=S256&response_type=code
```

Step 3: The user logs in at Herald's login page. After authentication, Herald redirects back to your `redirect_uri` with `?code=authorization-code&state=random-state`.

Step 4: Your backend exchanges the authorization code for a token:

```
POST /api/oauth/{realmId}/token
Content-Type: application/json

{
    "grant_type": "authorization_code",
    "code": "authorization-code",
    "redirect_uri": "https://app.example.com/callback",
    "client_id": "your-client-id",
    "code_verifier": "original-plaintext-verifier"
}
```

Step 5: The response:

```json
{
    "access_token": "0192a3b4-c5d6-7e8f-9a0b-1c2d3e4f5a6b",
    "token_type": "Bearer",
    "expires_in": 600
}
```

Step 6: Set the `access_token` as an `X-Auth` cookie (`httpOnly`, `secure`, `sameSite=Lax`) for subsequent requests. Your frontend never reads this token directly -- the browser sends it automatically.

The `state` parameter prevents CSRF attacks. The `code_challenge` (PKCE) prevents code interception attacks even if someone captures the authorization code from the redirect URL. Authorization codes are single-use -- Herald atomically deletes the code when exchanging it for a token.

### Device Code Flow (for IoT / CLI)

For devices without a browser -- CLI tools, embedded hardware, IoT devices.

Step 1: Request a device code:

```
POST /api/device/{realmId}/authorize
Content-Type: application/x-www-form-urlencoded

client_id=your-client-id
```

Note: the device code grant must be enabled on the client app (`device_code_grant_enabled`). It's disabled by default.

Step 2: The response:

```json
{
    "device_code": "0192a3b4-c5d6-...",
    "user_code": "BCDF-GHJK",
    "verification_uri": "https://herald.example.com/my-realm/device",
    "verification_uri_complete": "https://herald.example.com/my-realm/device/BCDF-GHJK",
    "expires_in": 900,
    "interval": 5
}
```

Step 3: Display the `user_code` and `verification_uri` to the user on whatever output is available. The user opens the URL on another device with a browser, enters the code, and authorizes.

Step 4: Your device polls the token endpoint every `interval` seconds (5 by default):

```
POST /api/device/{realmId}/token
Content-Type: application/x-www-form-urlencoded

grant_type=urn:ietf:params:oauth:grant-type:device_code&device_code=0192a3b4-c5d6-...
```

While the user hasn't authorized yet, you get `authorization_pending`. If you poll too fast, you get `slow_down` and the required interval increases by 5 seconds. Once the user approves, the endpoint returns the access token.

The device code expires after 900 seconds (15 minutes). If it expires before the user authorizes, start over with a new device code request.

### Session Management

The cookie name is `X-Auth`. Herald sets it with `httpOnly`, `secure` (in production), and `sameSite=Lax` attributes.

Default session TTL is 1800 seconds (30 minutes), configurable per client app via `session_ttl_seconds`. The OAuth token endpoint uses a separate default of 600 seconds.

**Sliding renewal** extends sessions for active users. When `session_renewal_ttl_seconds` is set on a client app, Herald's identity middleware checks the remaining TTL on each request. If the remaining TTL drops below `renewal_ttl / 2`, the middleware extends the session to the full renewal TTL and refreshes the cookie.

Three common strategies:

- **Strict timeout**: `session_ttl=300, renewal_ttl=null` (or omitted) -- 5-minute hard timeout, no renewal. Good for sensitive admin panels.
- **Relaxed (infinite while active)**: `session_ttl=28800, renewal_ttl=28800` -- 8-hour session that renews on every request. The user stays logged in as long as they're active. Good for internal tools.
- **Progressive**: `session_ttl=300, renewal_ttl=7200` -- starts at 5 minutes, extends to 2 hours after the first renewal. Balances security and convenience.

Handle session expiry in your frontend with a 401 interceptor:

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

The `/api/auth/config` endpoint on your backend returns the Herald login URL. Use an `isRedirecting` flag to prevent multiple concurrent 401 responses from triggering duplicate redirects.

## Points System

For usage-based billing -- API call counts, token consumption, credits.

### Check Balance

```rust
let balance = herald_client.get_balance("my-realm", &user_id).await?;
println!("Balance: {} {}", balance.balance, balance.unit);
```

### Consume Points

```rust
let result = herald_client.consume_points(
    "my-realm",
    &user_id,
    "my-client-app",    // your client_app_id
    100,                 // deduct 100 points
    Some("AI API call".to_string()),
    Some("unique-request-id".to_string()),  // idempotency key
).await?;
println!("Balance after: {}", result.balance_after);
```

Always pass an `idempotency_key`. Generate a unique one per logical operation (UUID v7 works well). On network timeout retries, the same key prevents double-charging. The response includes `balance_after` so you can check remaining balance without a separate query.

When the SDK uses an ordinary client app API key, the `client_app_id` here must be the key's bound client app. An `admin-api-client` key can consume points for any client app in the same realm.

## Subscription System

Query a client app's subscription status to gate features:

```rust
let sub = herald_client.get_subscription("my-realm", "my-client-app").await?;
if sub.status == "active" {
    // user has a paid subscription
}
```

An ordinary client app API key can only query the subscription for its bound client app. An `admin-api-client` key can query any client app in the same realm.

List available plans to show upgrade options:

```rust
let plans = herald_client.list_plans("my-realm").await?;
for plan in &plans {
    println!("{}: {} ({} {})", plan.name, plan.title, plan.price, plan.currency);
}
```

## Deployment

### Configuration

Your service needs four settings, typically from environment variables or a config file:

| Setting     | Example                | Description                                              |
|-------------|------------------------|----------------------------------------------------------|
| `base_url`  | `http://herald:3000`   | Herald address. Use container name inside Docker networks. |
| `api_key`   | `sk_xxxx...`           | API key generated in Herald admin console.                |
| `realm_id`  | `my-app`               | Your service's realm.                                     |
| `client_id` | `my-app-admin`         | Client identifier for your admin panel.                   |

### Cookie Sharing Requirements

Herald and your service must share a host or root domain for browser cookies to work. The `X-Auth` cookie is set by Herald and needs to reach your backend. Common deployment patterns:

**Same host, different ports.** `127.0.0.1:3000` (Herald) and `127.0.0.1:8080` (your service). Works in development. Herald sets `Domain=localhost` in dev mode.

**Reverse proxy with unified entry.** Caddy or Nginx routes `/auth` to Herald and `/` to your service. Both appear on the same domain. This is the recommended production setup.

**Same root domain subdomains.** `auth.example.com` (Herald) and `app.example.com` (your service). Requires configuring the cookie domain to `.example.com`.

### Running Without Herald

For local development or isolated intranets where Herald isn't deployed, skip SDK client initialization. Your admin API runs without authentication. Other auth mechanisms (device HMAC, API key auth, etc.) are unaffected since they don't depend on Herald sessions.
