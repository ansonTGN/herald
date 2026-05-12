use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;
use validator::{Validate, ValidationError};

#[derive(Debug, Clone, Deserialize, Serialize, ToSchema, Validate)]
#[serde(rename_all = "camelCase")]
#[validate(schema(function = "validate_shopify_claim_request"))]
pub struct ShopifyClaimRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    #[validate(length(min = 1))]
    pub shopify_customer_id: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    #[validate(length(min = 1))]
    pub contract_id: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    #[validate(length(min = 1))]
    pub order_id: Option<String>,
}

fn validate_shopify_claim_request(request: &ShopifyClaimRequest) -> Result<(), ValidationError> {
    let has_identifier = request
        .shopify_customer_id
        .as_ref()
        .map(|value| !value.trim().is_empty())
        .unwrap_or(false)
        || request
            .contract_id
            .as_ref()
            .map(|value| !value.trim().is_empty())
            .unwrap_or(false)
        || request
            .order_id
            .as_ref()
            .map(|value| !value.trim().is_empty())
            .unwrap_or(false);

    if has_identifier {
        Ok(())
    } else {
        Err(ValidationError::new("missing_identifier"))
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ShopifyClaimResponse {
    pub realm_id: String,
    pub user_id: Uuid,
    pub shop_domain: String,
    pub shopify_customer_id: String,
    pub claimed_subscription_ids: Vec<Uuid>,
    pub granted_subscription_ids: Vec<Uuid>,
    pub skipped_subscription_ids: Vec<Uuid>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_empty_claim_request() {
        let request = ShopifyClaimRequest {
            shopify_customer_id: None,
            contract_id: None,
            order_id: None,
        };

        assert!(request.validate().is_err());
    }

    #[test]
    fn accepts_customer_id_claim_request() {
        let request = ShopifyClaimRequest {
            shopify_customer_id: Some("gid://shopify/Customer/123".to_string()),
            contract_id: None,
            order_id: None,
        };

        assert!(request.validate().is_ok());
    }
}
