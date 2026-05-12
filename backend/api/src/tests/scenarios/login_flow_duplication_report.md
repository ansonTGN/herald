# Login Flow Test Duplication Analysis Report

## Files Analyzed
- `login_flow_scenarios.rs` - Tests complete login flow
- `user_login_test.rs` - Tests user login endpoint

## Test Functions Analyzed

### login_flow_scenarios.rs
- `test_scenario_complete_login_flow` - Complete user journey test

### user_login_test.rs
- `test_scenario_user_login_success` - Login endpoint test

## Duplication Analysis

### Similarity Level: **30-40%** (Low-Medium)

### Areas of Overlap
Both files test the login endpoint (`POST /api/auth/{realmId}/login`) with:
- Same test data setup (creating user with email="newuser@cas.com", password="password123")
- Same login payload structure
- Similar response validation (status code, JSON parsing)
- Same session token extraction logic

### Key Differences

#### login_flow_scenarios.rs
**Purpose**: Integration testing of the complete user journey
**Test Scope**:
1. Status check before login (unauthenticated state)
2. Public config retrieval (no auth required)
3. Login execution
4. Status check after login (authenticated state)
5. User info and permissions validation

**User Story**: Complete login flow verification

#### user_login_test.rs
**Purpose**: Unit testing of the login API endpoint
**Test Scope**:
1. User creation with password hashing
2. Login execution
3. Response validation (status code, JSON body)
4. Session token extraction
5. Session storage verification in Redis

**User Story**: User login endpoint functionality

### Redundant Code Examples

**User Creation** (90% identical):
```rust
// Both files create users the same way
let user_uuid = uuid::Uuid::now_v7();
let password_hash = bcrypt::hash(password, bcrypt::DEFAULT_COST).expect("Failed to hash password");

sqlx::query(
    "INSERT INTO account (id, realm_id, email, password, status)
         VALUES ($1, $2, $3, $4, 1)",
)
.bind(user_uuid)
.bind(&ctx._realm_id)
.bind(email)
.bind(&password_hash)
.execute(&ctx._app_state.pool)
.await
.expect("Failed to create test user");
```

**Login Payload** (100% identical):
```rust
let login_payload = json!({
    "clientId": ctx._client_id,
    "email": email,
    "password": password,
    "turnstileToken": "dummy"
});
```

**Login Request** (95% identical):
```rust
let login_request = Request::builder()
    .method("POST")
    .uri(format!("/api/auth/{}/login", ctx._realm_id))
    .header("content-type", "application/json")
    .header("x-forwarded-for", "3.3.3.3")
    .body(Body::from(login_payload.to_string()))
    .unwrap();
```

## Assessment

### Not Pure Duplication
These tests serve **different purposes**:
- `login_flow_scenarios`: Integration test for complete user journey
- `user_login_test`: Unit test for login endpoint functionality

### Code Reuse Opportunity
However, there is **significant code duplication** in:
1. **User creation helper** - Can be extracted to shared function
2. **Login request builder** - Can be extracted to shared function
3. **Session token extraction** - Already exists in `mod.rs`
4. **Test data setup** - Can use shared fixtures

## Recommendations

### Priority 1: Extract Common Helper Functions
Create `backend/api/src/tests/helpers/login_test_helpers.rs` with:
- `create_test_user_with_password(email, password)` - Reusable user creation
- `make_login_request(email, password, ctx)` - Reusable login request
- `verify_login_response(response)` - Reusable response validation

### Priority 2: Keep Both Test Files Separate
Do **not** merge the test files because they test different aspects:
- Keep `login_flow_scenarios.rs` for integration testing
- Keep `user_login_test.rs` for endpoint testing

### Priority 3: Use Shared Helpers
Update both files to use the new helper functions to reduce code duplication by ~60%.

## Conclusion

**Duplication Level**: 30-40% (Low-Medium)
**Action Required**: Extract common helpers, keep test files separate
**Expected Reduction**: 40-50 lines of duplicate code
**Benefit**: Easier maintenance without losing test coverage