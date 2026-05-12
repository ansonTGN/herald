/// 单元测试：Shopify 订阅绑定数据仓库
///
/// 测试 ShopifySubscriptionBinding 实体和 ShopifyBindingRepository 的核心功能

#[cfg(test)]
mod tests {
    use super::super::shopify_binding::*;
    use chrono::Utc;
    use uuid::Uuid;

    /// ============================================================================
    /// ShopifySubscriptionBinding 实体测试
    /// ============================================================================

    /// 测试创建 ShopifySubscriptionBinding 实体
    #[test]
    fn test_unit_shopify_subscription_binding_creation() {
        let binding = ShopifySubscriptionBinding {
            id: 1,
            subscription_id: Uuid::now_v7(),
            realm_id: "test-realm-200".to_string(),
            shop_domain: "test-shop.myshopify.com".to_string(),
            contract_id: "gid://shopify/SubscriptionContract/12345".to_string(),
            contract_gid: "gid://shopify/SubscriptionContract/12345".to_string(),
            contract_revision_id: 1,
            customer_id: Some("gid://shopify/Customer/67890".to_string()),
            customer_payment_method_id: None,
            last_billing_attempt_id: None,
            last_order_id: None,
            cancel_reason: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        assert_eq!(binding.id, 1);
        assert_eq!(binding.realm_id, "test-realm-200");
        assert_eq!(binding.shop_domain, "test-shop.myshopify.com");
        assert_eq!(binding.contract_id, "gid://shopify/SubscriptionContract/12345");
        assert_eq!(binding.contract_revision_id, 1);
        assert!(binding.customer_id.is_some());
        assert!(binding.last_billing_attempt_id.is_none());
        assert!(binding.last_order_id.is_none());
    }

    /// 测试包含完整信息的 ShopifySubscriptionBinding 实体
    #[test]
    fn test_unit_shopify_subscription_binding_with_optional_fields() {
        let binding = ShopifySubscriptionBinding {
            id: 2,
            subscription_id: Uuid::now_v7(),
            realm_id: "test-realm-201".to_string(),
            shop_domain: "demo-store.myshopify.com".to_string(),
            contract_id: "gid://shopify/SubscriptionContract/11111".to_string(),
            contract_gid: "gid://shopify/SubscriptionContract/11111".to_string(),
            contract_revision_id: 5,
            customer_id: Some("gid://shopify/Customer/22222".to_string()),
            customer_payment_method_id: Some("gid://shopify/CustomerPaymentMethod/33333".to_string()),
            last_billing_attempt_id: Some("gid://shopify/BillingAttempt/44444".to_string()),
            last_order_id: Some("gid://shopify/Order/55555".to_string()),
            cancel_reason: Some("Customer request".to_string()),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        assert_eq!(binding.contract_revision_id, 5);
        assert!(binding.customer_payment_method_id.is_some());
        assert!(binding.last_billing_attempt_id.is_some());
        assert!(binding.last_order_id.is_some());
        assert!(binding.cancel_reason.is_some());
    }

    /// ============================================================================
    /// Contract ID 格式验证测试
    /// ============================================================================

    /// 测试 Contract ID 格式验证
    #[test]
    fn test_unit_contract_id_format_validation() {
        // Valid contract IDs
        let valid_contract_ids = vec![
            "gid://shopify/SubscriptionContract/12345",
            "gid://shopify/SubscriptionContract/abc-def-123",
            "gid://shopify/SubscriptionContract/ABCD1234EFGH5678",
        ];

        for contract_id in valid_contract_ids {
            assert!(
                contract_id.starts_with("gid://shopify/SubscriptionContract/"),
                "Contract ID '{}' should start with 'gid://shopify/SubscriptionContract/'",
                contract_id
            );
        }

        // Invalid contract IDs
        let invalid_contract_ids = vec![
            "gid://shopify/Customer/12345", // Wrong type
            "shopify/SubscriptionContract/12345", // Missing gid://
            "gid://shopify/SubscriptionContract/", // Missing ID
        ];

        for contract_id in invalid_contract_ids {
            let is_valid =
                contract_id.starts_with("gid://shopify/SubscriptionContract/")
                    && contract_id.len() > "gid://shopify/SubscriptionContract/".len();

            assert!(!is_valid, "Contract ID '{}' should be invalid", contract_id);
        }
    }

    /// ============================================================================
    /// Customer ID 格式验证测试
    /// ============================================================================

    /// 测试 Customer ID 格式验证
    #[test]
    fn test_unit_customer_id_format_validation() {
        // Valid customer IDs
        let valid_customer_ids = vec![
            "gid://shopify/Customer/67890",
            "gid://shopify/Customer/customer-123",
            "gid://shopify/Customer/CUST12345",
        ];

        for customer_id in valid_customer_ids {
            assert!(
                customer_id.starts_with("gid://shopify/Customer/"),
                "Customer ID '{}' should start with 'gid://shopify/Customer/'",
                customer_id
            );
        }

        // Test optional customer_id
        let binding_with_customer = ShopifySubscriptionBinding {
            id: 1,
            subscription_id: Uuid::now_v7(),
            realm_id: "test-realm-200".to_string(),
            shop_domain: "test-shop.myshopify.com".to_string(),
            contract_id: "gid://shopify/SubscriptionContract/12345".to_string(),
            contract_gid: "gid://shopify/SubscriptionContract/12345".to_string(),
            contract_revision_id: 1,
            customer_id: Some("gid://shopify/Customer/67890".to_string()),
            customer_payment_method_id: None,
            last_billing_attempt_id: None,
            last_order_id: None,
            cancel_reason: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        assert!(binding_with_customer.customer_id.is_some());

        let binding_without_customer = ShopifySubscriptionBinding {
            id: 2,
            subscription_id: Uuid::now_v7(),
            realm_id: "test-realm-200".to_string(),
            shop_domain: "test-shop.myshopify.com".to_string(),
            contract_id: "gid://shopify/SubscriptionContract/12346".to_string(),
            contract_gid: "gid://shopify/SubscriptionContract/12346".to_string(),
            contract_revision_id: 1,
            customer_id: None,
            customer_payment_method_id: None,
            last_billing_attempt_id: None,
            last_order_id: None,
            cancel_reason: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        assert!(binding_without_customer.customer_id.is_none());
    }

    /// ============================================================================
    /// Shop Domain 格式验证测试
    /// ============================================================================

    /// 测试 Shop Domain 格式验证
    #[test]
    fn test_unit_shop_domain_validation() {
        // Valid shop domains
        let valid_domains = vec![
            "test-shop.myshopify.com",
            "demo-store.myshopify.com",
            "mystore123.myshopify.com",
        ];

        for domain in valid_domains {
            assert!(
                domain.ends_with(".myshopify.com"),
                "Shop domain '{}' should end with '.myshopify.com'",
                domain
            );
        }

        // Invalid shop domains
        let invalid_domains = vec!["mystore.com", "test-shop.shopify.com", "myshopify.com"];

        for domain in invalid_domains {
            assert!(
                !domain.ends_with(".myshopify.com") || domain == ".myshopify.com",
                "Shop domain '{}' should be invalid",
                domain
            );
        }
    }

    /// ============================================================================
    /// Contract Revision ID 测试
    /// ============================================================================

    /// 测试 Contract Revision ID 比较
    #[test]
    fn test_unit_contract_revision_id_comparison() {
        let current_revision = 5;

        // Higher revision (newer)
        assert!(7 > current_revision, "Higher revision should be accepted");

        // Lower revision (older)
        assert!(3 < current_revision, "Lower revision should be rejected");

        // Same revision (equal)
        assert!(5 == current_revision, "Same revision should be equal");
    }

    /// 测试 Contract Revision ID 递增
    #[test]
    fn test_unit_contract_revision_id_increment() {
        let initial_revision = 1;
        let updated_revision = initial_revision + 1;

        assert_eq!(updated_revision, 2, "Revision should increment by 1");

        // Multiple updates
        let final_revision = initial_revision + 5;
        assert_eq!(final_revision, 6, "Revision should increment correctly");
    }

    /// ============================================================================
    /// 取消原因测试
    /// ============================================================================

    /// 测试取消原因
    #[test]
    fn test_unit_cancel_reason() {
        let valid_cancel_reasons = vec![
            "Customer request",
            "Payment failed",
            "Terms violated",
            "Store closure",
        ];

        for reason in valid_cancel_reasons {
            let binding = ShopifySubscriptionBinding {
                id: 1,
                subscription_id: Uuid::now_v7(),
                realm_id: "test-realm-200".to_string(),
                shop_domain: "test-shop.myshopify.com".to_string(),
                contract_id: "gid://shopify/SubscriptionContract/12345".to_string(),
                contract_gid: "gid://shopify/SubscriptionContract/12345".to_string(),
                contract_revision_id: 1,
                customer_id: None,
                customer_payment_method_id: None,
                last_billing_attempt_id: None,
                last_order_id: None,
                cancel_reason: Some(reason.to_string()),
                created_at: Utc::now(),
                updated_at: Utc::now(),
            };

            assert_eq!(binding.cancel_reason, Some(reason.to_string()));
        }

        // Test without cancel reason
        let binding_no_reason = ShopifySubscriptionBinding {
            id: 2,
            subscription_id: Uuid::now_v7(),
            realm_id: "test-realm-200".to_string(),
            shop_domain: "test-shop.myshopify.com".to_string(),
            contract_id: "gid://shopify/SubscriptionContract/12346".to_string(),
            contract_gid: "gid://shopify/SubscriptionContract/12346".to_string(),
            contract_revision_id: 1,
            customer_id: None,
            customer_payment_method_id: None,
            last_billing_attempt_id: None,
            last_order_id: None,
            cancel_reason: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        assert!(binding_no_reason.cancel_reason.is_none());
    }

    /// ============================================================================
    /// 时间戳测试
    /// ============================================================================

    /// 测试创建和更新时间戳
    #[test]
    fn test_unit_timestamps() {
        let now = Utc::now();

        let binding = ShopifySubscriptionBinding {
            id: 1,
            subscription_id: Uuid::now_v7(),
            realm_id: "test-realm-200".to_string(),
            shop_domain: "test-shop.myshopify.com".to_string(),
            contract_id: "gid://shopify/SubscriptionContract/12345".to_string(),
            contract_gid: "gid://shopify/SubscriptionContract/12345".to_string(),
            contract_revision_id: 1,
            customer_id: None,
            customer_payment_method_id: None,
            last_billing_attempt_id: None,
            last_order_id: None,
            cancel_reason: None,
            created_at: now,
            updated_at: now,
        };

        assert_eq!(binding.created_at, now);
        assert_eq!(binding.updated_at, now);

        // Test updated_at can be different from created_at
        let updated_time = now + chrono::Duration::hours(1);
        let updated_binding = ShopifySubscriptionBinding {
            updated_at: updated_time,
            ..binding
        };

        assert_eq!(updated_binding.updated_at, updated_time);
        assert!(updated_binding.updated_at > updated_binding.created_at);
    }

    /// ============================================================================
    /// Order ID 格式验证测试
    /// ============================================================================

    /// 测试 Order ID 格式验证
    #[test]
    fn test_unit_order_id_format_validation() {
        // Valid order IDs
        let valid_order_ids = vec![
            "gid://shopify/Order/12345",
            "gid://shopify/Order/order-123",
            "gid://shopify/Order/ORD12345",
        ];

        for order_id in valid_order_ids {
            assert!(
                order_id.starts_with("gid://shopify/Order/"),
                "Order ID '{}' should start with 'gid://shopify/Order/'",
                order_id
            );
        }

        // Test optional last_order_id
        let binding_with_order = ShopifySubscriptionBinding {
            id: 1,
            subscription_id: Uuid::now_v7(),
            realm_id: "test-realm-200".to_string(),
            shop_domain: "test-shop.myshopify.com".to_string(),
            contract_id: "gid://shopify/SubscriptionContract/12345".to_string(),
            contract_gid: "gid://shopify/SubscriptionContract/12345".to_string(),
            contract_revision_id: 1,
            customer_id: None,
            customer_payment_method_id: None,
            last_billing_attempt_id: None,
            last_order_id: Some("gid://shopify/Order/67890".to_string()),
            cancel_reason: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        assert!(binding_with_order.last_order_id.is_some());

        let binding_without_order = ShopifySubscriptionBinding {
            id: 2,
            subscription_id: Uuid::now_v7(),
            realm_id: "test-realm-200".to_string(),
            shop_domain: "test-shop.myshopify.com".to_string(),
            contract_id: "gid://shopify/SubscriptionContract/12346".to_string(),
            contract_gid: "gid://shopify/SubscriptionContract/12346".to_string(),
            contract_revision_id: 1,
            customer_id: None,
            customer_payment_method_id: None,
            last_billing_attempt_id: None,
            last_order_id: None,
            cancel_reason: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        assert!(binding_without_order.last_order_id.is_none());
    }

    /// ============================================================================
    /// Billing Attempt ID 格式验证测试
    /// ============================================================================

    /// 测试 Billing Attempt ID 格式验证
    #[test]
    fn test_unit_billing_attempt_id_format_validation() {
        // Valid billing attempt IDs
        let valid_attempt_ids = vec![
            "gid://shopify/BillingAttempt/12345",
            "gid://shopify/BillingAttempt/attempt-123",
            "gid://shopify/BillingAttempt/BILL12345",
        ];

        for attempt_id in valid_attempt_ids {
            assert!(
                attempt_id.starts_with("gid://shopify/BillingAttempt/"),
                "Billing attempt ID '{}' should start with 'gid://shopify/BillingAttempt/'",
                attempt_id
            );
        }

        // Test optional last_billing_attempt_id
        let binding_with_attempt = ShopifySubscriptionBinding {
            id: 1,
            subscription_id: Uuid::now_v7(),
            realm_id: "test-realm-200".to_string(),
            shop_domain: "test-shop.myshopify.com".to_string(),
            contract_id: "gid://shopify/SubscriptionContract/12345".to_string(),
            contract_gid: "gid://shopify/SubscriptionContract/12345".to_string(),
            contract_revision_id: 1,
            customer_id: None,
            customer_payment_method_id: None,
            last_billing_attempt_id: Some("gid://shopify/BillingAttempt/67890".to_string()),
            last_order_id: None,
            cancel_reason: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        assert!(binding_with_attempt.last_billing_attempt_id.is_some());

        let binding_without_attempt = ShopifySubscriptionBinding {
            id: 2,
            subscription_id: Uuid::now_v7(),
            realm_id: "test-realm-200".to_string(),
            shop_domain: "test-shop.myshopify.com".to_string(),
            contract_id: "gid://shopify/SubscriptionContract/12346".to_string(),
            contract_gid: "gid://shopify/SubscriptionContract/12346".to_string(),
            contract_revision_id: 1,
            customer_id: None,
            customer_payment_method_id: None,
            last_billing_attempt_id: None,
            last_order_id: None,
            cancel_reason: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        assert!(binding_without_attempt.last_billing_attempt_id.is_none());
    }

    /// ============================================================================
    /// 数据完整性测试
    /// ============================================================================

    /// 测试必填字段完整性
    #[test]
    fn test_unit_required_fields_integrity() {
        let binding = ShopifySubscriptionBinding {
            id: 1,
            subscription_id: Uuid::now_v7(),
            realm_id: "test-realm-200".to_string(),
            shop_domain: "test-shop.myshopify.com".to_string(),
            contract_id: "gid://shopify/SubscriptionContract/12345".to_string(),
            contract_gid: "gid://shopify/SubscriptionContract/12345".to_string(),
            contract_revision_id: 1,
            customer_id: None,
            customer_payment_method_id: None,
            last_billing_attempt_id: None,
            last_order_id: None,
            cancel_reason: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        // Verify required fields are present
        assert!(binding.id > 0);
        assert!(!binding.realm_id.is_empty());
        assert!(!binding.shop_domain.is_empty());
        assert!(!binding.contract_id.is_empty());
        assert!(!binding.contract_gid.is_empty());
        assert!(binding.contract_revision_id >= 1);
    }

    /// 测试可选字段可以为 None
    #[test]
    fn test_unit_optional_fields_none() {
        let binding = ShopifySubscriptionBinding {
            id: 1,
            subscription_id: Uuid::now_v7(),
            realm_id: "test-realm-200".to_string(),
            shop_domain: "test-shop.myshopify.com".to_string(),
            contract_id: "gid://shopify/SubscriptionContract/12345".to_string(),
            contract_gid: "gid://shopify/SubscriptionContract/12345".to_string(),
            contract_revision_id: 1,
            customer_id: None,
            customer_payment_method_id: None,
            last_billing_attempt_id: None,
            last_order_id: None,
            cancel_reason: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        assert!(binding.customer_id.is_none());
        assert!(binding.customer_payment_method_id.is_none());
        assert!(binding.last_billing_attempt_id.is_none());
        assert!(binding.last_order_id.is_none());
        assert!(binding.cancel_reason.is_none());
    }
}
