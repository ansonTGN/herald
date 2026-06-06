# Integrating a Third-Party Backend with Herald

Herald handles users, login, permissions, and billing. Your backend only writes business logic, calling Herald's authentication and billing interfaces through the SDK.

```
Browser -> Your Frontend -> Your Backend -> Herald SDK -> Herald Service
                |                       (verify token, check perms, deduct points)
              OAuth redirect -> Herald login page -> callback to your backend -> set cookie
```

Your backend never connects to Herald's database. All interaction goes through HTTP API, with SDK-side caching.

## 1. Prerequisites

Complete these steps in the Herald admin console before writing code:

1. Create a realm (tenant) and note the `realm_id`
2. Create a client app under that realm and note the `client_id`
3. Generate an API Key for the realm. The secret is shown once. Store it somewhere your backend can read at startup -- config file or secret manager, not hardcoded. Choose the client app this backend serves; if left empty, Herald binds the key to `admin-api-client`
4. Define permission points in `resource:action` format, e.g. `product:read`, `device:manage`
5. Create roles and assign permission points to them
6. Create an admin user and assign roles

See [Design Your Permission Model](#design-your-permission-model) for step 4.

## 2. Backend Integration

### Configuration

Add a `[herald]` section to your service config:

```toml
[herald]
base_url = "http://127.0.0.1:13000"
api_key = "sk-your-api-key"
realm_id = "my-app"
client_id = "admin-web-console"
```

| Field | Description |
|-------|-------------|
| `base_url` | Herald address. Use container name inside Docker networks |
| `api_key` | API key generated in Herald admin console |
| `realm_id` | Your service's realm |
| `client_id` | The client app identifier from prerequisites |

Wrap the config in `Option<HeraldConfig>`. When absent, the entire auth system is disabled:

```rust
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct HeraldConfig {
    pub base_url: String,
    pub api_key: String,
    pub realm_id: String,
    pub client_id: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct Config {
    // ... other fields
    #[serde(default)]
    pub herald: Option<HeraldConfig>,
}
```

### Install the SDK

```toml
[dependencies]
herald-sdk = "0.1"
```

### Initialization

Create the SDK client at startup based on config. When `Option<Arc<Client>>` is `None`, routes won't get the auth middleware:

```rust
let herald_client = config.herald.as_ref().map(|herald| {
    Arc::new(herald_sdk::Client::new(
        herald.base_url.clone(),
        herald.api_key.clone(),
        None,  // default 5-minute cache
    ))
});
```

The third argument is cache duration. `None` means 300 seconds. The SDK automatically invalidates cached entries when a token expires.

The API key authenticates your backend to Herald. Store it in config or environment variables, never hardcode it. API keys carry a client app scope: a key bound to `admin-api-client` can access all client apps in the realm; a key bound to an ordinary client app can only access that app's permission checks, subscriptions, and points.

### Auth Config Endpoint

Your frontend needs to know whether Herald is enabled. Provide a public endpoint:

```rust
#[derive(Serialize)]
pub struct AuthConfigResponse {
    pub enabled: bool,
    pub login_url: Option<String>,
    pub herald_login_url: Option<String>,
}

// GET /api/auth/config
pub async fn get_auth_config(State(state): State<Arc<AppState>>) -> Json<AuthConfigResponse> {
    let herald = &state.config.herald;
    Json(AuthConfigResponse {
        enabled: herald.is_some(),
        login_url: herald.as_ref().map(|_| "/api/auth/oauth/start".to_string()),
        herald_login_url: herald.as_ref().map(|h| {
            format!("{}/{}/auth/login", h.base_url.trim_end_matches('/'), h.realm_id)
        }),
    })
}
```

Frontend calls this once at startup. `enabled: false` means skip all auth logic.

### OAuth Backend Handlers

SPAs use OAuth 2.1 Authorization Code + PKCE. Your backend needs two handlers: `oauth_start` to initiate login, `oauth_callback` to receive the redirect.

#### oauth_start: Initiate Login

```rust
// GET /api/auth/oauth/start?redirect=/devices
pub async fn oauth_start(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Query(query): Query<OAuthStartQuery>,
) -> Result<impl IntoResponse, ApiError> {
    let herald = state.config.herald.as_ref()?;

    let (app_origin, return_to) = resolve_app_origin_and_return_to(&headers, query.redirect)?;
    let redirect_uri = format!("{app_origin}/api/auth/oauth/callback");
    let oauth_state = random_token(32);
    let code_verifier = random_token(64);
    let code_challenge = pkce_challenge(&code_verifier);

    let authorize_url = format!(
        "{}/api/oauth/{}/authorize?client_id={}&redirect_uri={}&state={}&response_type=code&code_challenge={}&code_challenge_method=S256",
        herald.base_url.trim_end_matches('/'),
        herald.realm_id,
        herald.client_id,
        urlencoding::encode(&redirect_uri),
        oauth_state,
        code_challenge,
    );

    // Store OAuth state in a cookie, valid for 5 minutes
    let oauth_cookie = encode_oauth_cookie(&OAuthCookie {
        state: oauth_state,
        code_verifier,
        return_to,
        redirect_uri,
    });

    Ok((
        StatusCode::FOUND,
        [
            (header::LOCATION, authorize_url),
            (header::SET_COOKIE, build_cookie("APP_OAUTH", &oauth_cookie, 300)),
        ],
    ))
}
```

Four things happen here:

1. Generate `code_verifier` (random string) and `code_challenge` (SHA256 hash, base64url-encoded)
2. Build Herald's authorize URL and redirect the user there
3. Store `{state, code_verifier, return_to, redirect_uri}` encoded in an `APP_OAUTH` cookie
4. `return_to` remembers the user's original page for the callback redirect

#### oauth_callback: Handle Redirect

```rust
// GET /api/auth/oauth/callback?code=xxx&state=yyy
pub async fn oauth_callback(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Query(query): Query<OAuthCallbackQuery>,
) -> Result<impl IntoResponse, ApiError> {
    let herald = state.config.herald.as_ref()?;

    // 1. Retrieve OAuth state from cookie, verify state to prevent CSRF
    let oauth_cookie_value = get_cookie(&headers, "APP_OAUTH").ok_or(ApiError::unauthorized)?;
    let oauth_cookie = decode_oauth_cookie(&oauth_cookie_value)?;
    if oauth_cookie.state != query.state {
        return Err(ApiError::unauthorized());
    }

    // 2. Exchange authorization code + code_verifier for token
    let token = exchange_oauth_code(
        herald.base_url.trim_end_matches('/'),
        &herald.realm_id,
        &herald.client_id,
        &query.code,
        &oauth_cookie.redirect_uri,
        &oauth_cookie.code_verifier,
    ).await?;

    // 3. Set X-Auth cookie, clear OAuth state cookie, redirect to original page
    let mut headers = HeaderMap::new();
    headers.insert(header::LOCATION, HeaderValue::from_str(&oauth_cookie.return_to)?);
    headers.append(header::SET_COOKIE, HeaderValue::from_str(&build_cookie("X-Auth", &token.access_token, token.expires_in))?);
    headers.append(header::SET_COOKIE, HeaderValue::from_str(&clear_cookie("APP_OAUTH"))?);

    Ok((StatusCode::FOUND, headers))
}
```

`exchange_oauth_code` sends a POST to Herald's token endpoint:

```rust
async fn exchange_oauth_code(
    herald_base_url: &str, realm_id: &str, client_id: &str,
    code: &str, redirect_uri: &str, code_verifier: &str,
) -> Result<TokenResponse, ApiError> {
    let url = format!("{herald_base_url}/api/oauth/{realm_id}/token");
    let response = reqwest::Client::new()
        .post(url)
        .json(&serde_json::json!({
            "grant_type": "authorization_code",
            "code": code,
            "redirect_uri": redirect_uri,
            "client_id": client_id,
            "code_verifier": code_verifier,
        }))
        .send().await?;
    // ...
}
```

PKCE's security comes from `code_verifier` only existing in your backend's cookie -- it never passes through the frontend URL. Authorization codes are single-use: Herald atomically reads and deletes them with `GETDEL`, so a second exchange attempt fails.

#### Cookie Helpers

```rust
fn build_cookie(name: &str, value: &str, max_age_seconds: i64) -> String {
    format!("{name}={value}; Path=/; Max-Age={max_age_seconds}; HttpOnly; SameSite=Lax")
}

fn clear_cookie(name: &str) -> String {
    format!("{name}=; Path=/; Max-Age=0; HttpOnly; SameSite=Lax")
}

fn get_cookie(headers: &HeaderMap, name: &str) -> Option<String> {
    let cookies = headers.get(header::COOKIE)?.to_str().ok()?;
    cookies.split(';').find_map(|cookie| {
        let (cookie_name, value) = cookie.trim().split_once('=')?;
        (cookie_name == name && !value.is_empty()).then(|| value.to_string())
    })
}

fn pkce_challenge(code_verifier: &str) -> String {
    let digest = sha2::Sha256::digest(code_verifier.as_bytes());
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(digest)
}
```

### Auth Middleware

The middleware extracts the token from cookies, maps the request path to a permission rule, and calls Herald to check.

```rust
use axum::extract::State;
use axum::http::{Method, Request, header};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use herald_sdk::{Client, Error, PermissionCheckRequest, Rule};
use std::sync::Arc;

#[derive(Clone)]
pub struct HeraldAuthState {
    pub herald_sdk: Arc<Client>,
    pub client_id: Arc<str>,
}

pub async fn herald_auth_middleware(
    State(auth_state): State<HeraldAuthState>,
    mut request: Request,
    next: Next,
) -> Response {
    // 1. Extract token from cookie
    let Some(token) = extract_auth_token(&request) else {
        return ApiError::unauthorized().into_response();
    };

    // 2. Map request path to permission rule
    let Some(rule) = extract_permission(request.uri().path(), request.method()) else {
        return ApiError::forbidden().into_response();
    };

    // 3. Check with Herald
    let response = auth_state.herald_sdk
        .check_permission(PermissionCheckRequest {
            token,
            rules: Some(vec![rule]),
            client_id: auth_state.client_id.to_string(),
        })
        .await;

    match response {
        // Allowed: inject user_id into request extensions
        Ok(permission) if permission.allowed => {
            let Some(user_id) = permission.user_id else {
                return ApiError::unauthorized().into_response();
            };
            request.extensions_mut().insert(CurrentUser { user_id });
            next.run(request).await
        }
        // Session not found or expired: allowed=false, user_id=None
        Ok(permission) if permission.user_id.is_none() => {
            ApiError::unauthorized().into_response()
        }
        // Authenticated but lacks permission: allowed=false, user_id=Some(...)
        Ok(_) => ApiError::forbidden().into_response(),
        // Herald error: distinguish 401/403/other
        Err(error) => classify_auth_error(&error).into_response(),
    }
}

fn classify_auth_error(error: &Error) -> ApiError {
    match error {
        Error::Unauthorized(_) => ApiError::unauthorized(),
        Error::Forbidden(_) => ApiError::forbidden(),
        _ => ApiError::service_unavailable("auth service unavailable"),
    }
}

fn extract_auth_token(request: &Request) -> Option<String> {
    let cookies = request.headers().get(header::COOKIE)?.to_str().ok()?;
    cookies.split(';').find_map(|cookie| {
        let (name, value) = cookie.trim().split_once('=')?;
        (name == "X-Auth" && !value.is_empty()).then(|| value.to_string())
    })
}
```

The three rejection scenarios mean different things:

| Condition | Herald returns | Your backend returns | Frontend behavior |
|-----------|---------------|---------------------|-------------------|
| Cookie missing or token invalid | `allowed=false, user_id=None` | 401 | Redirect to login |
| User has session but no permission | `allowed=false, user_id=Some(...)` | 403 | Show permission denied |
| Herald service unavailable | Network error or 500 | 503 | Show service unavailable |

503, not passthrough. If the auth service is down, processing requests means bypassing auth.

### Design Your Permission Model

Permissions have two dimensions: **resource** and **action**. You define them; Herald stores and enforces.

Path-to-permission mapping happens in your middleware. Herald only answers one question: can this user do `product:read`?

```rust
pub fn extract_permission(path: &str, method: &Method) -> Option<Rule> {
    // Strip /api prefix if present -- the middleware runs inside a nest("/api", ...)
    let path = path.strip_prefix("/api").unwrap_or(path);

    let resource = if path.starts_with("/admin/product")
        || path.starts_with("/admin/valid")
        || path.starts_with("/admin/file")
    {
        "product"
    } else if path.starts_with("/admin/device")
        || path.starts_with("/admin/property")
        || path.starts_with("/admin/event")
        || path.starts_with("/admin/alarm-rule")
        || path.starts_with("/admin/alarm")
    {
        "device"
    } else if path.starts_with("/admin/ca") || path.starts_with("/admin/ota") {
        "cert"
    } else {
        return None;
    };

    let action = match *method {
        Method::GET => "read",
        Method::POST | Method::PUT | Method::PATCH | Method::DELETE => "write",
        _ => return None,
    };

    Some(Rule {
        resource: resource.to_string(),
        action: action.to_string(),
    })
}
```

After defining your mapping, configure the resource/action pairs as permission points in the Herald admin console. Create roles, assign permissions, assign roles to users.

Herald has built-in action hierarchy: `manage` covers `view`, `create`, and `manage` itself. `create` only covers `create`. `view` only covers `view`. Custom actions (like `admin`) only match themselves. If a user has `product:manage`, middleware requests for `product:read` will pass.

### Route Wiring

Routes split into three groups: auth endpoints, public routes, and protected admin routes.

```rust
pub fn create_router(
    config: Arc<Config>,
    herald_client: Option<Arc<herald_sdk::Client>>,
) -> Router {
    // Auth endpoints: public, no Herald auth needed
    let auth_routes = Router::new()
        .route("/auth/config", get(get_auth_config))
        .route("/auth/oauth/start", get(oauth_start))
        .route("/auth/oauth/callback", get(oauth_callback));

    // Public routes: health checks, webhooks
    let public_routes = Router::new()
        .route("/health", get(health_check))
        .route("/webhook/device", post(device_webhook));

    // Admin routes: require Herald auth
    let admin_routes = Router::new()
        .route("/admin/product", get(list_products).post(create_product))
        .route("/admin/device", get(list_devices));

    // Conditional mounting: only add auth middleware when Herald is configured
    let admin_routes = match (config.herald.as_ref(), herald_client) {
        (Some(herald_config), Some(herald_sdk)) => {
            admin_routes.layer(axum::middleware::from_fn_with_state(
                HeraldAuthState {
                    herald_sdk,
                    client_id: herald_config.client_id.clone().into(),
                },
                herald_auth_middleware,
            ))
        }
        (_, _) => admin_routes,
    };

    Router::new()
        .merge(auth_routes)
        .merge(public_routes)
        .merge(admin_routes)
}
```

The `match` branch is the key. Without `[herald]` in config, `herald_client` is `None` and admin routes get no middleware -- all management endpoints are accessible without auth. This is useful for local development or isolated intranet environments.

## 3. Frontend Integration

The frontend handles three things: detect auth state, redirect to login, handle 401 responses.

### Detect Auth State

```typescript
interface AuthConfig {
  enabled: boolean
  login_url: string | null
  herald_login_url?: string | null
}

let cachedAuthConfig: AuthConfig | null = null

async function getAuthConfig(): Promise<AuthConfig> {
  if (cachedAuthConfig) return cachedAuthConfig
  const res = await fetch('/api/auth/config')
  cachedAuthConfig = await res.json()
  return cachedAuthConfig
}
```

Call `/api/auth/config` once at startup. If `enabled: false`, skip all auth logic.

### Check Login Status

```typescript
export async function checkAuth(): Promise<boolean> {
  const config = await getAuthConfig()
  if (!config.enabled) return true

  // Probe with a lightweight admin endpoint
  const response = await fetch('/api/admin/product?page=1&page_size=1', {
    credentials: 'include',
  })
  return response.status !== 401
}
```

No dedicated "check login" endpoint. A real admin endpoint does double duty as a login probe.

### Handle 401

```typescript
let isRedirecting = false

export function handle401(): void {
  if (isRedirecting) return
  isRedirecting = true

  const loginUrl = new URL(cachedAuthConfig?.login_url || '/', window.location.origin)
  loginUrl.searchParams.set('redirect', window.location.href)
  window.location.href = loginUrl.toString()
}
```

`isRedirecting` prevents multiple concurrent 401s from triggering duplicate redirects. The redirect URL includes the current page so OAuth callback returns the user here.

Wire it into an API interceptor:

```typescript
apiClient.interceptors.response.use(
  response => response,
  error => {
    if (error.response?.status === 401) {
      handle401()
    }
    return Promise.reject(error)
  }
)
```

## 4. Resource Management via SDK

API keys are principals in Herald's RBAC system. Assign roles and permissions to a key, and SDK calls through that key can perform the corresponding operations. Roles don't bypass client app scope.

### Manage Realms

```rust
let realm = herald_client.create_realm(CreateRealmSdkRequest {
    name: "my-app".to_string(),
    description: Some("My application".to_string()),
    admin_user: AdminUserSdkInput {
        email: "admin@example.com".to_string(),
        password: "secure-password".to_string(),
    },
}).await?;

let realms = herald_client.list_realms().await?;
let realm = herald_client.get_realm("my-realm").await?;
```

### Manage Users

```rust
let user = herald_client.create_user("my-realm", CreateUserSdkRequest {
    email: "user@example.com".to_string(),
    password: "secure-password".to_string(),
    nickname: Some("johndoe".to_string()),
}).await?;

let users = herald_client.list_users("my-realm").await?;
let user = herald_client.get_user("my-realm", &user_id).await?;
```

### Manage Client Apps

```rust
let app = herald_client.create_client_app("my-realm", CreateClientAppSdkRequest {
    name: "Mobile App".to_string(),
    description: Some("iOS and Android app".to_string()),
    redirect_uris: vec!["https://app.example.com/callback".to_string()],
}).await?;

let apps = herald_client.list_client_apps("my-realm").await?;
let app = herald_client.get_client_app("my-realm", "my-mobile-app").await?;
```

Prefer an API key bound to the client app your backend serves. Use the `admin-api-client` key only for cross-client-app management.

## 5. Points System

For usage-based billing (API call counts, token consumption, credits).

```rust
// Check balance
let balance = herald_client.get_balance("my-realm", &user_id).await?;
println!("Balance: {} {}", balance.balance, balance.unit);

// Consume points
let result = herald_client.consume_points(
    "my-realm",
    &user_id,
    "my-client-app",
    100,
    Some("AI API call".to_string()),
    Some("unique-request-id".to_string()),
).await?;
println!("Balance after: {}", result.balance_after);
```

Always pass an `idempotency_key`. On network timeout retries, the same key prevents double-charging. When using an ordinary client app API key, `client_app_id` must be that key's bound client app.

## 6. Subscription System

Query a client app's subscription status:

```rust
let sub = herald_client.get_subscription("my-realm", "my-client-app").await?;
if sub.status == "active" {
    // user has a paid subscription
}
```

Subscription entitlements are synced from payment-provider products into Herald `entitlement_key` values. Payment is handled between Herald and the payment provider; your backend only queries subscription status and the current entitlement via SDK.

## 7. Deployment

### Cookie Sharing Requirements

Herald sets the `X-Auth` cookie in the browser after login. Your backend middleware needs to read this cookie, so Herald and your service must share a host or root domain.

Three deployment patterns:

- **Same host, different ports** (`127.0.0.1:3000` and `127.0.0.1:8080`) -- easiest for development
- **Reverse proxy** (Caddy or Nginx routes `/auth` to Herald, `/` to your service) -- recommended for production
- **Same root domain subdomains** (`auth.example.com` and `app.example.com`) -- requires cookie domain configuration

### Session Management

Cookie name: `X-Auth`, attributes: `httpOnly`, `secure` (production), `sameSite=Lax`. Default session TTL is 1800 seconds (30 minutes), configurable per client app via `session_ttl_seconds`.

Sliding renewal: when `session_renewal_ttl_seconds` is set, Herald's identity middleware checks remaining TTL on each request. If remaining TTL <= `renewal_ttl / 2`, the session extends to the full renewal TTL.

Three renewal strategies:

| Strategy | Config | Effect |
|----------|--------|--------|
| Strict | `session_ttl=300, renewal_ttl=null` | 5-minute hard timeout, no renewal |
| Relaxed | `session_ttl=28800, renewal_ttl=28800` | 8 hours, active users never expire |
| Progressive | `session_ttl=300, renewal_ttl=7200` | 5 minutes initially, extends to 2 hours on first renewal |

Progressive works well for admin panels: short sessions for quick tasks, auto-renewal for extended use, expire when the browser closes.
