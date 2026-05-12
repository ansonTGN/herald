# WeChat Payment Sandbox Setup Guide

## Overview

This guide provides instructions for setting up the WeChat Pay sandbox environment to enable real integration testing for WeChat payment functionality.

## Current Status

**WeChat payment tests are currently IGNORED** pending sandbox environment setup. All WeChat payment scenario tests have been marked with `#[ignore]` and will not run during regular test execution.

## Prerequisites

### 1. WeChat Pay Sandbox Account

- Apply for WeChat Pay merchant account in sandbox mode
- Obtain test merchant ID (mchid)
- Generate API keys for sandbox environment
- Complete merchant verification in sandbox

### 2. Required Credentials

You will need the following information from WeChat Pay:

- **Merchant ID (mchid)**: Test merchant identifier
- **API v3 Key (APIv3)**: 32-character string for API authentication
- **Merchant Certificate Serial Number**: Certificate serial number
- **Merchant Private Key**: PEM format private key file
- **WeChat Pay Public Key**: For webhook signature verification

### 3. Environment Variables

Set the following environment variables for testing:

```bash
# WeChat Pay Sandbox Configuration
WECHAT_PAY_MCHID=<test_merchant_id>
WECHAT_PAY_APIV3_KEY=<test_api_v3_key>
WECHAT_PAY_CERT_SERIAL_NO=<test_cert_serial_number>
WECHAT_PAY_PRIVATE_KEY_PATH=<path_to_private_key.pem>
WECHAT_PAY_PUBLIC_KEY_PATH=<path_to_public_key.pem>
WECHAT_PAY_SANDBOX=true
```

### 4. Sandbox API Endpoints

WeChat Pay sandbox uses different endpoints:

```
Base URL: https://api.mch.weixin.qq.com/sandboxnew
```

Key endpoints:
- Native Pay: `/v3/pay/transactions/native`
- Order Query: `/v3/pay/transactions/out-trade-no/{out_trade_no}`
- Close Order: `/v3/pay/transactions/out-trade-no/{out_trade_no}/close`

## Test Data Requirements

### Test Products

Configure test products in your database:
- Set product prices to small amounts (0.01-0.10 yuan)
- Enable products for test purchases
- Configure proper point rewards

### Test Users

Create test users with:
- Sufficient balance for testing
- Various subscription tiers
- Different permission levels for authorization testing

## Re-enabling Tests

### Step 1: Update Test Configuration

Modify `backend/api/src/tests/helpers/wechat_helpers.rs`:

```rust
// Remove the TODO and implement real sandbox configuration
pub async fn setup_wechat_config_with_keys(
    ctx: &mut TestContext,
    merchant_id: &str,
    api_v3_key: &str,
    cert_serial_no: &str,
    private_key_pem: &str,
) -> WechatConfig {
    // Use real sandbox credentials from environment variables
    let config = CreateWechatConfigRequest {
        merchant_id: merchant_id.to_string(),
        api_v3_key: api_v3_key.to_string(),
        cert_serial_no: cert_serial_no.to_string(),
        private_key_pem: private_key_pem.to_string(),
        // ... other fields
    };
    
    // Create config in database
    // Return config for test use
}
```

### Step 2: Remove #[ignore] Attributes

Remove the `#[ignore]` attribute from all WeChat payment tests:

**Files to modify:**
- `backend/api/src/tests/scenarios/billing/wechat_config_scenarios.rs`
- `backend/api/src/tests/scenarios/billing/wechat_order_scenarios.rs`
- `backend/api/src/tests/scenarios/billing/wechat_webhook_scenarios.rs`

**Pattern to remove:**
```rust
// Remove this line:
#[ignore]

// Keep this line as documentation:
/// TODO: Re-enable when WeChat payment sandbox environment is ready
```

### Step 3: Update Test Fixtures

Replace test fixture files with real sandbox credentials:

1. Delete old test files (if they exist):
   - `backend/api/src/fixtures/rsa_test_key.pem`
   - `backend/api/src/fixtures/rsa_test_pubkey.pem`

2. Add real sandbox credentials (secure storage recommended):
   - Use environment variables
   - Or use encrypted test fixtures
   - Never commit real credentials to git

### Step 4: Run Tests

Execute WeChat payment tests:

```bash
# Run all WeChat payment tests
cargo test --package cas-api wechat

# Run specific test file
cargo test --package cas-api wechat_config_scenarios
cargo test --package cas-api wechat_order_scenarios
cargo test --package cas-api wechat_webhook_scenarios
```

## Test Coverage

When re-enabled, the WeChat payment tests cover:

### Configuration Tests (9 tests)
- Create configuration with validation
- Duplicate configuration handling
- Read configuration with data masking
- Update configuration and key rotation
- Delete configuration with subscription checks

### Order Lifecycle Tests (7 tests)
- Native QR code order creation
- Order status querying and caching
- Order closing with ownership validation
- Order expiration and cleanup
- Authorization and permission checks

### Webhook Processing Tests (6 tests)
- SHA256-RSA signature verification
- AEAD_AES_256_GCM payload decryption
- Payment success and points granting
- Duplicate callback idempotency
- Amount validation and mismatch rejection
- Error handling (401, 404, 400)

## Troubleshooting

### Common Issues

**1. Authentication Failures**
- Verify API v3 key matches sandbox credentials
- Check merchant ID is for sandbox environment
- Ensure private key matches the uploaded certificate

**2. Certificate Errors**
- Verify certificate serial number is correct
- Check private key PEM format is valid
- Ensure certificate hasn't expired

**3. Webhook Verification Failures**
- Verify WeChat Pay public key is current
- Check payload decryption implementation
- Ensure signature verification matches WeChat Pay specification

**4. Order Creation Failures**
- Verify test product exists in database
- Check product is enabled for testing
- Ensure user has sufficient balance

### Debug Mode

Enable debug logging for WeChat Pay SDK:

```rust
// In test setup
env_logger::builder()
    .filter_level(log::LevelFilter::Debug)
    .init();
```

## Security Considerations

### Never Commit Real Credentials
- Use environment variables for sensitive data
- Add `.env` files to `.gitignore`
- Consider using encrypted secrets management
- Rotate sandbox credentials regularly

### Test Data Isolation
- Use separate test merchant account
- Don't use production credentials
- Clean up test data after runs
- Use test user accounts only

### Webhook Security
- Test signature verification thoroughly
- Validate all webhook payloads
- Implement replay attack protection
- Log all webhook events for debugging

## References

- [WeChat Pay Official Documentation](https://pay.weixin.qq.com/wiki/doc/apiv3/index.shtml)
- [WeChat Pay SDK for Rust](https://github.com/wechat-pay-rs/wechat-pay-rust-sdk)
- [WeChat Pay Sandbox Guide](https://pay.weixin.qq.com/wiki/doc/apiv3/open/pay/chapter2_8_1.shtml)

## Maintenance

### Regular Tasks

1. **Credential Rotation**: Update sandbox credentials periodically
2. **Certificate Updates**: Renew test certificates before expiration
3. **API Version Updates**: Test new WeChat Pay API versions in sandbox first
4. **Test Data Maintenance**: Clean up stale test orders and configurations

### Monitoring

Track test success rates and failures:
- Authentication failures
- API response time
- Webhook processing delays
- Test data consistency issues

---

**Last Updated**: 2026-04-05
**Status**: Awaiting Sandbox Environment Setup
**Next Action**: Set up WeChat Pay sandbox account and configure environment variables
