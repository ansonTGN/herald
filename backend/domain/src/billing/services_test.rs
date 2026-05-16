#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::sync::{Arc, Mutex};

    use crate::authentication::Identity;
    use crate::billing::{
        AllowAllBillingPolicy, BillingRepository, ClientAppPlan, CreatePlanInput, PaymentEvent,
        Plan, PlanPaymentProvider, PlanService, Product, Subscription,
        SubscriptionHistoryEvent, SubscriptionHistoryQuery, UpdatePlanInput, test_helpers::*,
    };
    use crate::common::entities::app_errors::CoreError;
    use crate::user::entities::{User, UserStatus};
    use chrono::Utc;
    use uuid::Uuid;

    // These tests use the builder pattern from test_helpers
    // See entities_test.rs for serialization/deserialization tests

    #[derive(Default)]
    struct StubBillingRepository {
        products: Mutex<HashMap<Uuid, Product>>,
        plans: Mutex<HashMap<Uuid, Plan>>,
    }

    impl StubBillingRepository {
        fn with_product(mut self, product: Product) -> Self {
            self.products
                .get_mut()
                .expect("products mutex poisoned")
                .insert(product.id, product);
            self
        }

        fn with_plan(mut self, plan: Plan) -> Self {
            self.plans
                .get_mut()
                .expect("plans mutex poisoned")
                .insert(plan.id, plan);
            self
        }
    }

    impl BillingRepository for StubBillingRepository {
        async fn create_subscription(&self, _sub: Subscription) -> Result<Subscription, CoreError> {
            unimplemented!()
        }

        async fn find_by_realm_id(
            &self,
            _realm_id: &str,
        ) -> Result<Option<Subscription>, CoreError> {
            unimplemented!()
        }

        async fn find_by_external_subscription_id(
            &self,
            _external_sub_id: &str,
            _provider: &str,
        ) -> Result<Option<Subscription>, CoreError> {
            unimplemented!()
        }

        async fn find_subscription_by_id(
            &self,
            _subscription_id: Uuid,
        ) -> Result<Option<Subscription>, CoreError> {
            unimplemented!()
        }

        async fn update_subscription(&self, _sub: Subscription) -> Result<Subscription, CoreError> {
            unimplemented!()
        }

        async fn create_payment_event(
            &self,
            _event: PaymentEvent,
        ) -> Result<PaymentEvent, CoreError> {
            unimplemented!()
        }

        async fn find_payment_event_by_external_id(
            &self,
            _external_event_id: &str,
            _payment_provider: &str,
        ) -> Result<Option<PaymentEvent>, CoreError> {
            unimplemented!()
        }

        async fn mark_payment_event_processed(&self, _id: Uuid) -> Result<(), CoreError> {
            unimplemented!()
        }

        async fn create_plan(&self, plan: Plan) -> Result<Plan, CoreError> {
            self.plans
                .lock()
                .expect("plans mutex poisoned")
                .insert(plan.id, plan.clone());
            Ok(plan)
        }

        async fn find_plan_by_id(&self, plan_id: Uuid) -> Result<Option<Plan>, CoreError> {
            let plan = self
                .plans
                .lock()
                .expect("plans mutex poisoned")
                .get(&plan_id)
                .cloned();
            Ok(plan)
        }

        async fn find_public_plan_by_id(
            &self,
            _realm_id: &str,
            plan_id: Uuid,
        ) -> Result<Option<Plan>, CoreError> {
            let plan = self
                .plans
                .lock()
                .expect("plans mutex poisoned")
                .get(&plan_id)
                .cloned();
            Ok(plan)
        }

        async fn list_plans_by_realm(&self, realm_id: &str) -> Result<Vec<Plan>, CoreError> {
            let plans = self
                .plans
                .lock()
                .expect("plans mutex poisoned")
                .values()
                .filter(|plan| plan.realm_id == realm_id)
                .cloned()
                .collect::<Vec<_>>();
            Ok(plans)
        }

        async fn list_public_plans_by_realm(&self, realm_id: &str) -> Result<Vec<Plan>, CoreError> {
            let plans = self
                .plans
                .lock()
                .expect("plans mutex poisoned")
                .values()
                .filter(|plan| plan.realm_id == realm_id)
                .cloned()
                .collect::<Vec<_>>();
            Ok(plans)
        }

        async fn update_plan(&self, plan: Plan) -> Result<Plan, CoreError> {
            self.plans
                .lock()
                .expect("plans mutex poisoned")
                .insert(plan.id, plan.clone());
            Ok(plan)
        }

        async fn delete_plan(&self, _plan_id: Uuid) -> Result<(), CoreError> {
            unimplemented!()
        }

        async fn assign_plan_to_client_app(
            &self,
            _client_app_id: Uuid,
            _plan_id: Uuid,
        ) -> Result<ClientAppPlan, CoreError> {
            unimplemented!()
        }

        async fn remove_plan_from_client_app(&self, _assignment_id: Uuid) -> Result<(), CoreError> {
            unimplemented!()
        }

        async fn list_plans_for_client_app(
            &self,
            _client_app_id: Uuid,
        ) -> Result<Vec<ClientAppPlan>, CoreError> {
            unimplemented!()
        }

        async fn find_plan_assignment(
            &self,
            _client_app_id: Uuid,
            _plan_id: Uuid,
        ) -> Result<Option<ClientAppPlan>, CoreError> {
            unimplemented!()
        }

        async fn toggle_plan_assignment(
            &self,
            _assignment_id: Uuid,
            _enabled: bool,
        ) -> Result<ClientAppPlan, CoreError> {
            unimplemented!()
        }

        async fn find_subscription_by_client_app_id(
            &self,
            _client_app_id: Uuid,
        ) -> Result<Option<Subscription>, CoreError> {
            unimplemented!()
        }

        async fn cancel_subscription(
            &self,
            _subscription_id: Uuid,
            _cancel_at_period_end: bool,
        ) -> Result<Subscription, CoreError> {
            unimplemented!()
        }

        async fn count_active_subscriptions_for_plan(
            &self,
            _plan_id: Uuid,
        ) -> Result<i64, CoreError> {
            Ok(0)
        }

        async fn save_history_event(
            &self,
            _event: SubscriptionHistoryEvent,
        ) -> Result<SubscriptionHistoryEvent, CoreError> {
            unimplemented!()
        }

        async fn get_subscription_history(
            &self,
            _realm_id: &str,
            _subscription_id: &Uuid,
        ) -> Result<Vec<SubscriptionHistoryEvent>, CoreError> {
            unimplemented!()
        }

        async fn list_subscription_history(
            &self,
            _realm_id: &str,
            _query: SubscriptionHistoryQuery,
        ) -> Result<(Vec<SubscriptionHistoryEvent>, u64), CoreError> {
            unimplemented!()
        }

        async fn create_product(&self, product: Product) -> Result<Product, CoreError> {
            self.products
                .lock()
                .expect("products mutex poisoned")
                .insert(product.id, product.clone());
            Ok(product)
        }

        async fn find_product_by_id(
            &self,
            realm_id: &str,
            product_id: Uuid,
        ) -> Result<Option<Product>, CoreError> {
            let product = self
                .products
                .lock()
                .expect("products mutex poisoned")
                .get(&product_id)
                .filter(|product| product.realm_id == realm_id)
                .cloned();
            Ok(product)
        }

        async fn product_code_exists(
            &self,
            _realm_id: &str,
            _code: &str,
        ) -> Result<bool, CoreError> {
            Ok(false)
        }

        async fn list_products(
            &self,
            _realm_id: &str,
            _enabled_only: Option<bool>,
        ) -> Result<Vec<Product>, CoreError> {
            unimplemented!()
        }

        async fn update_product(
            &self,
            _realm_id: &str,
            _product_id: Uuid,
            _title: Option<String>,
            _description: Option<String>,
            _enabled: Option<bool>,
        ) -> Result<Product, CoreError> {
            unimplemented!()
        }

        async fn delete_product(
            &self,
            _realm_id: &str,
            _product_id: Uuid,
        ) -> Result<(), CoreError> {
            unimplemented!()
        }

        async fn count_plans_by_product(&self, _product_id: Uuid) -> Result<i64, CoreError> {
            Ok(0)
        }

        async fn find_plans_by_product(
            &self,
            _realm_id: &str,
            _product_id: Uuid,
        ) -> Result<Vec<Plan>, CoreError> {
            unimplemented!()
        }

        async fn create_plan_payment_provider(
            &self,
            mapping: PlanPaymentProvider,
        ) -> Result<PlanPaymentProvider, CoreError> {
            Ok(mapping)
        }

        async fn find_plan_payment_provider_by_id(
            &self,
            _mapping_id: Uuid,
        ) -> Result<Option<PlanPaymentProvider>, CoreError> {
            Ok(None)
        }

        async fn find_plan_payment_provider_by_plan_and_provider(
            &self,
            _plan_id: Uuid,
            _payment_provider: &str,
        ) -> Result<Option<PlanPaymentProvider>, CoreError> {
            Ok(None)
        }

        async fn list_plan_payment_providers(
            &self,
            _plan_id: Uuid,
        ) -> Result<Vec<PlanPaymentProvider>, CoreError> {
            Ok(Vec::new())
        }

        async fn update_plan_payment_provider(
            &self,
            mapping: PlanPaymentProvider,
        ) -> Result<PlanPaymentProvider, CoreError> {
            Ok(mapping)
        }

        async fn delete_plan_payment_provider(&self, _mapping_id: Uuid) -> Result<(), CoreError> {
            Ok(())
        }

        async fn delete_plan_payment_providers_by_plan(
            &self,
            _plan_id: Uuid,
        ) -> Result<(), CoreError> {
            Ok(())
        }

        async fn list_plan_payment_providers_batch(
            &self,
            _plan_ids: &[Uuid],
        ) -> Result<Vec<PlanPaymentProvider>, CoreError> {
            Ok(Vec::new())
        }
    }

    fn test_identity(realm_id: &str) -> Identity {
        Identity::User(User {
            id: Uuid::now_v7(),
            realm_id: realm_id.to_string(),
            email: format!("{}@example.com", realm_id),
            nickname: None,
            password_hash: None,
            provider_ids: vec![],
            status: UserStatus::Normal,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        })
    }

    fn test_product(realm_id: &str, product_id: Uuid) -> Product {
        Product {
            id: product_id,
            realm_id: realm_id.to_string(),
            code: format!("product-{}", realm_id),
            title: format!("Product {}", realm_id),
            description: None,
            enabled: true,
            plans_count: 0,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    #[tokio::test]
    async fn test_create_plan_rejects_product_from_another_realm() {
        let realm_id = "realm-a";
        let foreign_product_id = Uuid::now_v7();
        let repository = Arc::new(
            StubBillingRepository::default()
                .with_product(test_product("realm-b", foreign_product_id)),
        );
        let service = PlanService::new(repository, Arc::new(AllowAllBillingPolicy));

        let input: CreatePlanInput = CreatePlanInputBuilder::new()
            .with_realm_id(realm_id)
            .with_name("starter")
            .with_product_id(foreign_product_id)
            .build();

        let error = service
            .create_plan(test_identity(realm_id), realm_id, input)
            .await
            .expect_err("cross-realm product should be rejected");

        match error {
            CoreError::ProductNotFound {
                realm_id: err_realm,
                product_id,
            } => {
                assert_eq!(err_realm, realm_id);
                assert_eq!(product_id, foreign_product_id.to_string());
            }
            other => panic!("expected ProductNotFound, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn test_update_plan_rejects_move_to_product_from_another_realm() {
        let realm_id = "realm-a";
        let current_product_id = Uuid::now_v7();
        let foreign_product_id = Uuid::now_v7();
        let existing_plan = test_plan(realm_id, "starter");
        let existing_plan_id = existing_plan.id;
        let repository = Arc::new(
            StubBillingRepository::default()
                .with_product(test_product(realm_id, current_product_id))
                .with_product(test_product("realm-b", foreign_product_id))
                .with_plan(Plan {
                    product_id: current_product_id,
                    ..existing_plan
                }),
        );
        let service = PlanService::new(repository.clone(), Arc::new(AllowAllBillingPolicy));

        let input: UpdatePlanInput = UpdatePlanInputBuilder::new()
            .with_product_id(foreign_product_id)
            .build();

        let error = service
            .update_plan(test_identity(realm_id), realm_id, existing_plan_id, input)
            .await
            .expect_err("cross-realm plan move should be rejected");

        match error {
            CoreError::ProductNotFound {
                realm_id: err_realm,
                product_id,
            } => {
                assert_eq!(err_realm, realm_id);
                assert_eq!(product_id, foreign_product_id.to_string());
            }
            other => panic!("expected ProductNotFound, got {other:?}"),
        }

        let saved_plan = repository
            .find_plan_by_id(existing_plan_id)
            .await
            .expect("plan lookup should succeed")
            .expect("existing plan should remain stored");
        assert_eq!(saved_plan.product_id, current_product_id);
    }
}
