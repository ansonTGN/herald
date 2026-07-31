// =============================================================================
// IAP Provider CHECK Constraint Migration Regression Scenario Tests
// =============================================================================
//
// Verifies the post-migration state of migration `0010_iap_provider_check.sql`
// (design support-iap §4.3.3 / §6.1 DB migration regression).
//
// The test schema is cloned from a template that has `0010` applied, so this
// suite validates the **post-migration** state only. Rollback state is NOT
// covered: per design §6.1 / §7 and the 2026-07-27 user decision, this
// feature is not yet shipped, the repo maintains unidirectional sqlx
// migrations (no down SQL), so no rollback / replay case is written here.
//
// User Story: n/a (DB regression)
// Covers: design support-iap §4.3.3 (CHECK extension),
//         §6.1 (DB migration regression — post-migration state).
//
// =============================================================================

#[cfg(test)]
mod tests {
    use crate::tests::schema_test_context::SchemaTestContext;
    use sqlx::PgPool;
    use test_context::test_context;
    use uuid::Uuid;

    use SchemaTestContext as MigrationContext;

    /// Insert a `payment_attempts` row with the given provider and return its
    /// id. Panics (via `expect`) if the insert fails — the CHECK constraint
    /// violation surfaces here.
    async fn insert_payment_attempt(pool: &PgPool, realm_id: &str, provider: &str) -> Uuid {
        let id = Uuid::now_v7();
        sqlx::query(
            "INSERT INTO payment_attempts
                (id, realm_id, user_id, payment_provider, target_type, target_id,
                 amount, currency, status, expires_at)
             VALUES ($1, $2, $3, $4, 'entitlement_mapping', $5, 100, 'usd', 'Pending', NOW())",
        )
        .bind(id)
        .bind(realm_id)
        .bind(Uuid::now_v7())
        .bind(provider)
        .bind(Uuid::now_v7())
        .execute(pool)
        .await
        .expect("payment_attempts insert should succeed under extended CHECK");
        id
    }

    /// Insert a `provider_entitlement_mappings` row with the given provider
    /// and return its id.
    async fn insert_mapping_row(
        pool: &PgPool,
        realm_id: &str,
        provider: &str,
        external_product_id: &str,
    ) -> Uuid {
        let id = Uuid::now_v7();
        sqlx::query(
            "INSERT INTO provider_entitlement_mappings
                (id, realm_id, payment_provider, external_product_id, entitlement_key,
                 billing_type, enabled, created_at, updated_at)
             VALUES ($1, $2, $3, $4, 'pro', 'recurring', true, NOW(), NOW())",
        )
        .bind(id)
        .bind(realm_id)
        .bind(provider)
        .bind(external_product_id)
        .execute(pool)
        .await
        .expect("provider_entitlement_mappings insert should succeed under extended CHECK");
        id
    }

    /// Count rows for a provider in a table.
    async fn count_rows(pool: &PgPool, table: &str, provider: &str) -> i64 {
        let sql = format!("SELECT COUNT(*) FROM {table} WHERE payment_provider = $1");
        sqlx::query_scalar(&sql)
            .bind(provider)
            .fetch_one(pool)
            .await
            .unwrap()
    }

    /// User Story: n/a (DB migration regression)
    /// Covers: design §4.3.3, §6.1 (post-migration state)
    ///
    /// After migration `0010`, `payment_provider='apple'` / `'google'` rows
    /// must insert successfully into both `payment_attempts` and
    /// `provider_entitlement_mappings`, and the existing `'stripe'` /
    /// `'creem'` rows must continue to satisfy the (now widened) CHECK
    /// constraint. This is the post-migration regression anchor; rollback
    /// state is intentionally NOT covered (feature not yet shipped, sqlx
    /// unidirectional migrations — design §6.1 / §7).
    #[test_context(MigrationContext)]
    #[tokio::test]
    async fn test_iap_migration_post_apple_google_writes_succeed(ctx: &mut MigrationContext) {
        let realm_id = ctx._realm_id.clone();
        let pool = &ctx.app_state.pool;

        for provider in ["stripe", "creem", "apple", "google"] {
            insert_payment_attempt(pool, &realm_id, provider).await;
        }
        for provider in ["stripe", "creem", "apple", "google"] {
            assert_eq!(
                count_rows(pool, "payment_attempts", provider).await,
                1,
                "payment_attempts row for {provider} must persist"
            );
        }

        for provider in ["stripe", "creem", "apple", "google"] {
            insert_mapping_row(
                pool,
                &realm_id,
                provider,
                &format!("prod_{provider}_migration"),
            )
            .await;
        }
        for provider in ["stripe", "creem", "apple", "google"] {
            assert_eq!(
                count_rows(pool, "provider_entitlement_mappings", provider).await,
                1,
                "provider_entitlement_mappings row for {provider} must persist"
            );
        }

        // ---- backward compatibility: the widened CHECK still accepts the
        //      legacy stripe/creem values (regression anchor). Verified by
        //      the inserts above succeeding; additionally assert that no
        //      other provider string is accepted by attempting an invalid
        //      one and expecting failure. ----
        let invalid = sqlx::query(
            "INSERT INTO payment_attempts
                (id, realm_id, user_id, payment_provider, target_type, target_id,
                 amount, currency, status, expires_at)
             VALUES ($1, $2, $3, 'paypal', 'entitlement_mapping', $4, 100, 'usd', 'Pending', NOW())",
        )
        .bind(Uuid::now_v7())
        .bind(&realm_id)
        .bind(Uuid::now_v7())
        .bind(Uuid::now_v7())
        .execute(pool)
        .await;
        assert!(
            invalid.is_err(),
            "payment_attempts insert with non-whitelisted provider must fail (CHECK enforced)"
        );
    }
}
