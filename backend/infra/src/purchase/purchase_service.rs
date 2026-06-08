// Purchase orchestration service
// Moved from domain/purchase/services.rs to eliminate domain -> infrastructure dependency

use std::collections::HashMap;
use std::sync::Arc;

use sqlx::PgPool;
use uuid::Uuid;

use herald_domain::billing::BillingRepository;
use herald_domain::billing::entities::BillingType;
use herald_domain::common::entities::app_errors::CoreError;
use herald_domain::payment_attempt::entities::{PaymentAttempt, PaymentContext};
use herald_domain::payment_attempt::{
    CreatePaymentAttemptInput, PaymentAttemptRepository, PaymentAttemptService, PurchasableTarget,
};
use herald_domain::purchase::errors::PurchaseResult;
use herald_domain::purchase::ports::{FulfillmentResult, FulfillmentService};
use herald_domain::purchase::services::{
    CompletePaymentAttemptInput, CreatedPaymentAttempt, PaymentCompletionSource,
    PreparePaymentAttemptInput, PreparedPaymentAttempt, PurchaseTargetSnapshot, metadata_keys,
};
/// Build common herald metadata map for payment providers.
fn build_herald_metadata(
    realm_id: &str,
    user_id: Uuid,
    target_type: &str,
    target_id: Uuid,
    attempt_id: Uuid,
) -> HashMap<String, String> {
    let mut metadata = HashMap::new();
    metadata.insert(
        metadata_keys::HERALD_REALM_ID.to_string(),
        realm_id.to_string(),
    );
    metadata.insert(
        metadata_keys::HERALD_USER_ID.to_string(),
        user_id.to_string(),
    );
    metadata.insert(
        metadata_keys::TARGET_TYPE.to_string(),
        target_type.to_string(),
    );
    metadata.insert(metadata_keys::TARGET_ID.to_string(), target_id.to_string());
    metadata.insert(
        metadata_keys::ATTEMPT_ID.to_string(),
        attempt_id.to_string(),
    );
    metadata
}

use herald_infra_creem::{CreateCheckoutRequest as CreemCreateCheckoutRequest, CreemClient};
use herald_infra_stripe::{CreateCheckoutRequest as StripeCreateCheckoutRequest, StripeClient};
use herald_infra_wechat::client::WechatPayClient;
use herald_infra_wechat::models::{CreateOrderParams, WechatOrderStatus, WechatPaymentOrder};
use herald_infra_wechat::repository::WechatOrderRepository;

/// Purchase service for unified purchase orchestration and fulfillment
pub struct PurchaseService<B, PA, F>
where
    B: BillingRepository,
    PA: PaymentAttemptRepository,
    F: FulfillmentService,
{
    pool: PgPool,
    public_base_url: String,
    billing_repository: Arc<B>,
    payment_attempt_service: Arc<PaymentAttemptService<PA>>,
    fulfillment_service: Arc<F>,
}

impl<B, PA, F> PurchaseService<B, PA, F>
where
    B: BillingRepository,
    PA: PaymentAttemptRepository,
    F: FulfillmentService,
{
    pub fn new(
        pool: PgPool,
        public_base_url: String,
        billing_repository: Arc<B>,
        payment_attempt_service: Arc<PaymentAttemptService<PA>>,
        fulfillment_service: Arc<F>,
    ) -> Self {
        Self {
            pool,
            public_base_url,
            billing_repository,
            payment_attempt_service,
            fulfillment_service,
        }
    }

    pub async fn prepare_payment_attempt(
        &self,
        input: PreparePaymentAttemptInput,
    ) -> PurchaseResult<PreparedPaymentAttempt> {
        if input.payment_provider != "wechat" && input.user_email.is_none() {
            return Err(CoreError::BadRequest(
                "A formal user email is required for non-WeChat payment providers".to_string(),
            ));
        }

        let target = self
            .resolve_target(
                &input.realm_id,
                &input.target_type,
                input.target_id,
                &input.payment_provider,
            )
            .await?;

        let (attempt, _) = self
            .payment_attempt_service
            .create_payment_attempt(
                CreatePaymentAttemptInput {
                    realm_id: input.realm_id,
                    user_id: input.user_id,
                    payment_provider: input.payment_provider,
                    target_type: target.target_type.to_string(),
                    target_id: target.target_id,
                    amount: target.amount,
                    currency: target.currency.clone(),
                    provider_reference: None,
                    metadata: input.metadata,
                },
                PaymentContext {
                    wechat_code_url: None,
                    stripe_checkout_url: None,
                    creem_checkout_url: None,
                    client_secret: None,
                },
            )
            .await?;

        Ok(PreparedPaymentAttempt { attempt, target })
    }

    pub async fn create_payment_attempt(
        &self,
        input: PreparePaymentAttemptInput,
    ) -> PurchaseResult<CreatedPaymentAttempt> {
        let user_email = input.user_email.clone();
        let prepared = self.prepare_payment_attempt(input).await?;
        let (provider_reference, context) = self
            .build_payment_context(
                &prepared.attempt.realm_id,
                prepared.attempt.user_id,
                &prepared.attempt.target_type.to_string(),
                prepared.attempt.target_id,
                &prepared.attempt.payment_provider,
                &prepared.target,
                prepared.attempt.id,
                user_email.as_deref(),
            )
            .await?;

        let attempt = self
            .payment_attempt_service
            .update_provider_reference(
                &prepared.attempt.realm_id,
                prepared.attempt.id,
                provider_reference,
            )
            .await?;

        Ok(CreatedPaymentAttempt { attempt, context })
    }

    /// Fulfill a payment attempt based on billing type from entitlement mapping.
    /// When `billing_type_override` is provided, it takes precedence.
    /// When absent, resolves from the entitlement mapping — returns an error if
    /// the mapping is missing or has no billing_type set.
    pub async fn fulfill_payment_attempt(
        &self,
        attempt: PaymentAttempt,
        provider_transaction_id: String,
        _completed_at: chrono::DateTime<chrono::Utc>,
        billing_type_override: Option<BillingType>,
    ) -> Result<FulfillmentResult, CoreError> {
        let billing_type = if let Some(bt) = billing_type_override {
            bt
        } else {
            // Resolve billing_type from the entitlement mapping
            let mapping = self
                .billing_repository
                .find_entitlement_mapping_by_id(attempt.target_id)
                .await?
                .filter(|m| m.realm_id == attempt.realm_id)
                .ok_or_else(|| {
                    CoreError::BillingError(format!(
                        "Entitlement mapping '{}' not found for realm '{}'",
                        attempt.target_id, attempt.realm_id
                    ))
                })?;

            mapping.billing_type.ok_or_else(|| {
                CoreError::BillingError(format!(
                    "Entitlement mapping '{}' has no billing_type set",
                    attempt.target_id
                ))
            })?
        };

        match billing_type {
            BillingType::OneTime => {
                self.fulfillment_service
                    .fulfill_one_time_purchase(&attempt, provider_transaction_id)
                    .await
            }
            BillingType::Recurring => {
                self.fulfillment_service
                    .fulfill_subscription_purchase(&attempt, provider_transaction_id)
                    .await
            }
        }
    }

    pub async fn complete_succeeded_payment_attempt(
        &self,
        input: CompletePaymentAttemptInput,
    ) -> PurchaseResult<FulfillmentResult> {
        self.validate_completion_source(&input.source)?;

        let attempt_for_realm = self
            .payment_attempt_service
            .get_payment_attempt_by_id_only(input.attempt_id)
            .await?;

        let marked_attempt = self
            .payment_attempt_service
            .mark_payment_succeeded(
                &attempt_for_realm.realm_id,
                input.attempt_id,
                input.provider_status,
                input.provider_transaction_id.clone(),
                input.completed_at,
            )
            .await?;

        self.fulfill_payment_attempt(
            marked_attempt,
            input.provider_transaction_id,
            input.completed_at,
            input.billing_type_override,
        )
        .await
    }

    async fn resolve_target(
        &self,
        realm_id: &str,
        target_type: &str,
        target_id: Uuid,
        payment_provider: &str,
    ) -> PurchaseResult<PurchaseTargetSnapshot> {
        let parsed_target_type = target_type.parse::<PurchasableTarget>()?;

        // All purchasable targets are now EntitlementMapping
        let mapping = self
            .billing_repository
            .find_entitlement_mapping_by_id(target_id)
            .await?
            .filter(|m| m.realm_id == realm_id && m.payment_provider == payment_provider)
            .ok_or_else(|| {
                CoreError::Conflict(format!(
                    "No entitlement mapping found for provider '{payment_provider}' target '{}' in realm '{}'",
                    target_id, realm_id
                ))
            })?;

        if !mapping.enabled {
            return Err(CoreError::Conflict(format!(
                "Entitlement mapping for provider '{payment_provider}' product '{}' is disabled",
                target_id
            )));
        }

        // Extract price info from provider_product_info if available
        let (amount, currency, title) = mapping
            .provider_product_info
            .as_ref()
            .and_then(|info| {
                let price = info.get("price")?.as_i64()?;
                let curr = info.get("currency")?.as_str()?.to_string();
                let name = info
                    .get("name")
                    .and_then(|v| v.as_str())
                    .unwrap_or(&mapping.entitlement_key)
                    .to_string();
                Some((price, curr, name))
            })
            .unwrap_or_else(|| {
                (
                    0, // No price info available
                    "usd".to_string(),
                    mapping.entitlement_key.clone(),
                )
            });

        Ok(PurchaseTargetSnapshot {
            target_type: parsed_target_type,
            target_id,
            amount,
            currency,
            title,
            provider_external_product_id: Some(mapping.external_product_id.clone()),
            billing_period: mapping.billing_period.clone(),
            billing_type: mapping.billing_type,
        })
    }

    async fn build_payment_context(
        &self,
        realm_id: &str,
        user_id: Uuid,
        target_type: &str,
        target_id: Uuid,
        payment_provider: &str,
        target: &PurchaseTargetSnapshot,
        attempt_id: Uuid,
        user_email: Option<&str>,
    ) -> PurchaseResult<(Option<String>, PaymentContext)> {
        match payment_provider {
            "wechat" => {
                self.build_wechat_payment_context(realm_id, user_id, target_type, target_id, target)
                    .await
            }
            "creem" => {
                self.build_creem_payment_context(
                    realm_id,
                    user_id,
                    target_type,
                    target_id,
                    target,
                    attempt_id,
                    user_email,
                )
                .await
            }
            "stripe" => {
                self.build_stripe_payment_context(
                    realm_id,
                    user_id,
                    target_type,
                    target_id,
                    target,
                    attempt_id,
                    user_email,
                )
                .await
            }
            _ => Err(CoreError::BadRequest(
                "Unsupported payment provider".to_string(),
            )),
        }
    }

    async fn build_wechat_payment_context(
        &self,
        realm_id: &str,
        user_id: Uuid,
        target_type: &str,
        target_id: Uuid,
        target: &PurchaseTargetSnapshot,
    ) -> PurchaseResult<(Option<String>, PaymentContext)> {
        let repo = WechatOrderRepository::new(self.pool.clone());
        let config_row = repo
            .get_wechat_config(realm_id)
            .await?
            .ok_or(CoreError::NotFound)?;

        let app_id = config_row
            .app_id
            .ok_or_else(|| CoreError::InternalServerError("WeChat config missing app_id".into()))?;
        let mch_id = config_row
            .mch_id
            .ok_or_else(|| CoreError::InternalServerError("WeChat config missing mch_id".into()))?;
        let private_key = config_row.private_key.ok_or_else(|| {
            CoreError::InternalServerError("WeChat config missing private_key".into())
        })?;
        let serial_no = config_row.serial_no.ok_or_else(|| {
            CoreError::InternalServerError("WeChat config missing serial_no".into())
        })?;
        let v3_key = config_row
            .v3_key
            .ok_or_else(|| CoreError::InternalServerError("WeChat config missing v3_key".into()))?;
        let notify_url = config_row.notify_url.ok_or_else(|| {
            CoreError::InternalServerError("WeChat config missing notify_url".into())
        })?;

        let client = WechatPayClient::new_async(
            app_id,
            mch_id,
            private_key,
            serial_no,
            v3_key,
            notify_url.clone(),
            config_row.mock_base_url,
        )
        .await
        .map_err(|e| {
            CoreError::InternalServerError(format!("Failed to create WeChat client: {e}"))
        })?;

        let create_params = CreateOrderParams {
            realm_id: realm_id.to_string(),
            user_id,
            plan_id: target_id,
            client_app_id: None,
            amount: i32::try_from(target.amount).map_err(|_| {
                CoreError::BadRequest("Amount exceeds WeChat supported range".into())
            })?,
            currency: target.currency.clone(),
            description: format!("{target_type}: {}", target.title),
            notify_url,
        };

        let result =
            tokio::task::spawn_blocking(move || client.create_native_order(&create_params))
                .await
                .map_err(|e| {
                    CoreError::InternalServerError(format!("WeChat order task failed: {e}"))
                })?
                .map_err(|e| {
                    CoreError::InternalServerError(format!("WeChat order creation failed: {e}"))
                })?;

        let order = WechatPaymentOrder {
            id: result.order_id,
            realm_id: realm_id.to_string(),
            user_id,
            plan_id: target_id,
            client_app_id: None,
            out_trade_no: result.out_trade_no.clone(),
            transaction_id: None,
            amount: i32::try_from(target.amount).map_err(|_| {
                CoreError::BadRequest("Amount exceeds WeChat supported range".into())
            })?,
            currency: target.currency.clone(),
            code_url: result.code_url.clone(),
            status: WechatOrderStatus::Pending,
            description: Some(format!("{target_type}: {}", target.title)),
            paid_at: None,
            closed_at: None,
            expires_at: result.expires_at,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };

        repo.create_order(&order).await?;

        Ok((
            Some(result.out_trade_no),
            PaymentContext {
                wechat_code_url: Some(result.code_url),
                stripe_checkout_url: None,
                creem_checkout_url: None,
                client_secret: None,
            },
        ))
    }

    async fn build_creem_payment_context(
        &self,
        realm_id: &str,
        user_id: Uuid,
        target_type: &str,
        target_id: Uuid,
        target: &PurchaseTargetSnapshot,
        attempt_id: Uuid,
        user_email: Option<&str>,
    ) -> PurchaseResult<(Option<String>, PaymentContext)> {
        let product_id = target.provider_external_product_id.clone().ok_or_else(|| {
            CoreError::Conflict("Creem product mapping missing external_product_id".into())
        })?;
        let client = self.get_creem_client_for_realm(realm_id).await?;
        let metadata = build_herald_metadata(realm_id, user_id, target_type, target_id, attempt_id);

        let session = client
            .create_checkout_session(&CreemCreateCheckoutRequest {
                product_id,
                success_url: Some(format!("{}/billing/success", self.public_base_url)),
                customer: herald_infra_creem::CreemCheckoutCustomer {
                    email: Some(
                        user_email
                            .expect("validated in prepare_payment_attempt")
                            .to_owned(),
                    ),
                },
                metadata: Some(metadata),
            })
            .await
            .map_err(|e| {
                CoreError::InternalServerError(format!(
                    "Failed to create Creem checkout session: {e}"
                ))
            })?;

        Ok((
            Some(session.id),
            PaymentContext {
                wechat_code_url: None,
                stripe_checkout_url: None,
                creem_checkout_url: Some(session.checkout_url),
                client_secret: None,
            },
        ))
    }

    async fn build_stripe_payment_context(
        &self,
        realm_id: &str,
        user_id: Uuid,
        target_type: &str,
        target_id: Uuid,
        target: &PurchaseTargetSnapshot,
        attempt_id: Uuid,
        user_email: Option<&str>,
    ) -> PurchaseResult<(Option<String>, PaymentContext)> {
        let client = self.get_stripe_client_for_realm(realm_id).await?;

        let metadata = build_herald_metadata(realm_id, user_id, target_type, target_id, attempt_id);

        let mode = match target.billing_type {
            Some(BillingType::OneTime) => Some("payment".to_string()),
            _ => None, // defaults to "subscription" in the client
        };

        let session = client
            .create_checkout_session(&StripeCreateCheckoutRequest {
                client_app_id: target_id,
                mapping_id: target_id,
                user_id: Some(user_id),
                customer_email: Some(
                    user_email
                        .expect("validated in prepare_payment_attempt")
                        .to_owned(),
                ),
                success_url: format!("{}/billing/success", self.public_base_url),
                cancel_url: format!("{}/billing/cancel", self.public_base_url),
                billing_period: target
                    .billing_period
                    .clone()
                    .unwrap_or_else(|| "monthly".to_string()),
                trial_days: None,
                price_amount: target.amount,
                currency: target.currency.clone(),
                plan_name: target.title.clone(),
                realm_id: realm_id.to_string(),
                webhook_url: Some(format!(
                    "{}/api/third/pay/{}/stripe/webhooks",
                    self.public_base_url, realm_id
                )),
                metadata: Some(metadata),
                mode,
            })
            .await
            .map_err(|e| {
                CoreError::InternalServerError(format!(
                    "Failed to create Stripe checkout session: {e}"
                ))
            })?;

        let client_secret = session.payment_intent.or_else(|| Some(session.id.clone()));

        Ok((
            Some(session.id),
            PaymentContext {
                wechat_code_url: None,
                stripe_checkout_url: Some(session.url),
                creem_checkout_url: None,
                client_secret,
            },
        ))
    }

    async fn get_creem_client_for_realm(&self, realm_id: &str) -> PurchaseResult<CreemClient> {
        let api_key = sqlx::query_scalar::<_, String>(
            "SELECT config_value
             FROM realm_config
             WHERE realm_id = $1 AND config_type = 'creem' AND config_key = 'api_key' AND enabled = true
             LIMIT 1",
        )
        .bind(realm_id)
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(|| {
            CoreError::InternalServerError(format!("Creem not configured for realm: {realm_id}"))
        })?;

        let timeout = sqlx::query_scalar::<_, String>(
            "SELECT config_value
             FROM realm_config
             WHERE realm_id = $1 AND config_type = 'creem' AND config_key = 'timeout' AND enabled = true
             LIMIT 1",
        )
        .bind(realm_id)
        .fetch_optional(&self.pool)
        .await?
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(30);

        let mock_base_url = sqlx::query_scalar::<_, String>(
            "SELECT config_value
             FROM realm_config
             WHERE realm_id = $1 AND config_type = 'creem' AND config_key = 'mock_base_url' AND enabled = true
             LIMIT 1",
        )
        .bind(realm_id)
        .fetch_optional(&self.pool)
        .await?;

        if let Some(base_url) = mock_base_url {
            CreemClient::with_base_url(api_key, base_url, timeout)
        } else {
            CreemClient::new(api_key, timeout)
        }
    }

    async fn get_stripe_client_for_realm(&self, realm_id: &str) -> PurchaseResult<StripeClient> {
        let api_key = sqlx::query_scalar::<_, String>(
            "SELECT config_value
             FROM realm_config
             WHERE realm_id = $1 AND config_type = 'stripe' AND config_key = 'api_key' AND enabled = true
             LIMIT 1",
        )
        .bind(realm_id)
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(|| {
            CoreError::InternalServerError(format!("Stripe not configured for realm: {realm_id}"))
        })?;

        let timeout = sqlx::query_scalar::<_, String>(
            "SELECT config_value
             FROM realm_config
             WHERE realm_id = $1 AND config_type = 'stripe' AND config_key = 'timeout' AND enabled = true
             LIMIT 1",
        )
        .bind(realm_id)
        .fetch_optional(&self.pool)
        .await?
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(30);

        let mock_base_url = sqlx::query_scalar::<_, String>(
            "SELECT config_value
             FROM realm_config
             WHERE realm_id = $1 AND config_type = 'stripe' AND config_key = 'mock_base_url' AND enabled = true
             LIMIT 1",
        )
        .bind(realm_id)
        .fetch_optional(&self.pool)
        .await?;

        if let Some(base_url) = mock_base_url {
            StripeClient::with_base_url(api_key, base_url, timeout)
        } else {
            StripeClient::new(api_key, timeout)
        }
    }

    fn validate_completion_source(&self, source: &PaymentCompletionSource) -> PurchaseResult<()> {
        match source {
            PaymentCompletionSource::InternalApi => Ok(()),
            PaymentCompletionSource::ProviderWebhook { provider }
                if matches!(provider.as_str(), "wechat" | "stripe" | "creem" | "shopify") =>
            {
                Ok(())
            }
            PaymentCompletionSource::ProviderWebhook { provider } => Err(CoreError::BadRequest(
                format!("Unsupported payment completion source provider: {provider}"),
            )),
        }
    }
}
