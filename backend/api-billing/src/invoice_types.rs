use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;
use validator::Validate;

use herald_core::domain::billing::credit_note::CreditNote;
use herald_core::domain::billing::invoice::{
    AdjustmentMode, InvoiceDetail, InvoiceHistory, InvoiceLineItem, InvoiceListFilters,
    InvoiceProvider, InvoiceSellerConfig, InvoiceSource, InvoiceStatus,
};

// ---------------------------------------------------------------------------
// Seller Config
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize, ToSchema, Validate)]
#[serde(rename_all = "camelCase")]
pub struct SellerConfigRequest {
    #[validate(length(min = 1, max = 200))]
    pub seller_name: String,
    #[validate(length(min = 1))]
    pub seller_address: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub seller_email: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub seller_phone: Option<String>,
    #[validate(length(min = 1, max = 100))]
    pub seller_tax_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_payment_terms: Option<String>,
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct SellerConfigResponse {
    pub seller_name: String,
    pub seller_address: String,
    pub seller_email: Option<String>,
    pub seller_phone: Option<String>,
    pub seller_tax_id: String,
    pub default_payment_terms: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

impl From<InvoiceSellerConfig> for SellerConfigResponse {
    fn from(c: InvoiceSellerConfig) -> Self {
        Self {
            seller_name: c.seller_name,
            seller_address: c.seller_address,
            seller_email: c.seller_email,
            seller_phone: c.seller_phone,
            seller_tax_id: c.seller_tax_id,
            default_payment_terms: c.default_payment_terms,
            created_at: c.created_at.to_rfc3339(),
            updated_at: c.updated_at.to_rfc3339(),
        }
    }
}

// ---------------------------------------------------------------------------
// Line Item
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize, Serialize, ToSchema, Validate)]
#[serde(rename_all = "camelCase")]
pub struct LineItemRequest {
    #[validate(length(min = 1, max = 200))]
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Decimal string, e.g. "1.5"
    #[validate(length(min = 1))]
    pub quantity: String,
    /// Price in smallest currency unit (e.g. cents)
    #[validate(range(min = 0))]
    pub unit_price: i64,
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct InvoiceLineItemResponse {
    pub id: Uuid,
    pub invoice_id: Uuid,
    pub sort_order: i32,
    pub name: String,
    pub description: Option<String>,
    pub quantity: String,
    pub unit_price: i64,
    pub subtotal: i64,
}

impl From<InvoiceLineItem> for InvoiceLineItemResponse {
    fn from(item: InvoiceLineItem) -> Self {
        Self {
            id: item.id,
            invoice_id: item.invoice_id,
            sort_order: item.sort_order,
            name: item.name,
            description: item.description,
            quantity: item.quantity,
            unit_price: item.unit_price,
            subtotal: item.subtotal,
        }
    }
}

// ---------------------------------------------------------------------------
// Invoice History
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct InvoiceHistoryResponse {
    pub id: Uuid,
    pub invoice_id: Uuid,
    pub event_type: String,
    pub actor_user_id: Option<Uuid>,
    pub actor_type: String,
    pub changes: serde_json::Value,
    pub created_at: String,
}

impl From<InvoiceHistory> for InvoiceHistoryResponse {
    fn from(h: InvoiceHistory) -> Self {
        Self {
            id: h.id,
            invoice_id: h.invoice_id,
            event_type: h.event_type.as_str().to_string(),
            actor_user_id: h.actor_user_id,
            actor_type: h.actor_type.as_str().to_string(),
            changes: h.changes,
            created_at: h.created_at.to_rfc3339(),
        }
    }
}

// ---------------------------------------------------------------------------
// Apply Invoice (user-facing)
// ---------------------------------------------------------------------------

/// Per-resource invoice apply-eligibility (read-only, context-level).
///
/// Returned by `GET /api/bill/{realmId}/my/invoices/apply-eligibility` so the
/// frontend can gate the Apply Invoice button BEFORE submit (Phase B of P0-2,
/// see `.ai/future/invoice_ux.md`). Users consume this verdict; they do NOT
/// read admin config/policy APIs directly.
#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct InvoiceApplyEligibilityResponse {
    /// Echoes the queried reference type: "payment_attempt" | "subscription".
    pub reference_type: String,
    /// Echoes the queried reference id.
    pub reference_id: Uuid,
    /// Whether the user can apply for a manual Herald invoice on this resource.
    pub can_apply: bool,
    /// "external_provider" | "manual_fallback" | "disabled".
    pub route: String,
    /// Resolved payment_provider ("stripe"|"shopify"|"wechat"|"creem"|None).
    pub provider: Option<String>,
    /// Human-readable reason when apply is not available, else `None`.
    pub reason: Option<String>,
}

// Query params for GET /my/invoices/apply-eligibility.
#[derive(Debug, Deserialize, ToSchema, IntoParams)]
#[serde(rename_all = "camelCase")]
pub struct InvoiceApplyEligibilityQuery {
    /// "payment_attempt" | "subscription".
    pub reference_type: String,
    pub reference_id: Uuid,
}

#[derive(Debug, Deserialize, ToSchema, Validate)]
#[serde(rename_all = "camelCase")]
pub struct ApplyInvoiceRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub payment_attempt_id: Option<Uuid>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subscription_id: Option<Uuid>,
    #[validate(length(min = 3, max = 3))]
    pub currency: String,
    #[validate(length(min = 1, max = 200))]
    pub billing_name: String,
    #[validate(length(min = 1))]
    pub billing_address: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub billing_email: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub billing_phone: Option<String>,
    #[validate(length(min = 1, max = 100))]
    pub billing_tax_id: String,
    pub due_date: NaiveDate,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
}

// ---------------------------------------------------------------------------
// Create Invoice (admin)
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize, ToSchema, Validate)]
#[serde(rename_all = "camelCase")]
pub struct CreateInvoiceRequest {
    pub account_id: Uuid,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub applicant_user_id: Option<Uuid>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subscription_id: Option<Uuid>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub payment_attempt_id: Option<Uuid>,
    #[validate(length(min = 3, max = 3))]
    pub currency: String,

    #[validate(length(min = 1))]
    pub line_items: Vec<LineItemRequest>,

    // Buyer
    #[validate(length(min = 1, max = 200))]
    pub billing_name: String,
    #[validate(length(min = 1))]
    pub billing_address: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub billing_email: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub billing_phone: Option<String>,
    #[validate(length(min = 1, max = 100))]
    pub billing_tax_id: String,

    // Seller snapshot
    #[validate(length(min = 1, max = 200))]
    pub seller_name: String,
    #[validate(length(min = 1))]
    pub seller_address: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub seller_email: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub seller_phone: Option<String>,
    #[validate(length(min = 1, max = 100))]
    pub seller_tax_id: String,

    // Adjustments
    #[serde(skip_serializing_if = "Option::is_none")]
    pub discount_mode: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub discount_value: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tax_mode: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tax_value: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub shipping_mode: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub shipping_value: Option<String>,

    pub due_date: NaiveDate,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub payment_terms: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
}

// ---------------------------------------------------------------------------
// Update Invoice (draft only)
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize, ToSchema, Validate)]
#[serde(rename_all = "camelCase")]
pub struct UpdateInvoiceRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub billing_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub billing_address: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub billing_email: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub billing_phone: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub billing_tax_id: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub seller_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub seller_address: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub seller_email: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub seller_phone: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub seller_tax_id: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub line_items: Option<Vec<LineItemRequest>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub discount_mode: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub discount_value: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tax_mode: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tax_value: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub shipping_mode: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub shipping_value: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub due_date: Option<NaiveDate>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub payment_terms: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
}

// ---------------------------------------------------------------------------
// Status action requests
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct IssueInvoiceRequest {
    /// Optional override for issue date (defaults to today)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub issue_date: Option<NaiveDate>,
}

#[derive(Debug, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct VoidInvoiceRequest {
    pub void_reason: Option<String>,
}

#[derive(Debug, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct MarkPaidRequest {
    /// Optional payment timestamp (defaults to now)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub paid_at: Option<String>,
}

// ---------------------------------------------------------------------------
// List query params
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize, ToSchema, IntoParams)]
#[serde(rename_all = "camelCase")]
pub struct InvoiceListQuery {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub search: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub date_from: Option<NaiveDate>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub date_to: Option<NaiveDate>,
    #[serde(default = "default_page")]
    pub page: Option<u64>,
    #[serde(default = "default_page_size")]
    pub page_size: Option<u64>,
}

fn default_page() -> Option<u64> {
    Some(1)
}

fn default_page_size() -> Option<u64> {
    Some(20)
}

impl InvoiceListQuery {
    pub fn to_filters(&self) -> InvoiceListFilters {
        InvoiceListFilters {
            status: self
                .status
                .as_deref()
                .and_then(|s| s.parse::<InvoiceStatus>().ok()),
            source: self
                .source
                .as_deref()
                .and_then(|s| s.parse::<InvoiceSource>().ok()),
            provider: self
                .provider
                .as_deref()
                .and_then(|s| s.parse::<InvoiceProvider>().ok()),
            search: self.search.clone(),
            date_from: self.date_from,
            date_to: self.date_to,
            page: self.page,
            page_size: self.page_size,
        }
    }
}

// ---------------------------------------------------------------------------
// Invoice Response (summary for list views)
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct InvoiceResponse {
    pub id: Uuid,
    pub invoice_number: String,
    pub source: String,
    pub account_id: Option<Uuid>,
    pub status: String,
    pub currency: String,
    pub total: i64,
    pub billing_name: Option<String>,
    pub due_date: Option<NaiveDate>,
    pub provider: String,
    pub payment_provider: Option<String>,
    pub external_hosted_url: Option<String>,
    pub external_pdf_url: Option<String>,
    pub amount_refunded: i64,
    pub created_at: String,
}

// ---------------------------------------------------------------------------
// Invoice Detail Response
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct InvoiceDetailResponse {
    pub id: Uuid,
    pub realm_id: String,
    pub invoice_number: String,
    pub source: String,
    pub account_id: Option<Uuid>,
    pub applicant_user_id: Option<Uuid>,
    pub subscription_id: Option<Uuid>,
    pub payment_attempt_id: Option<Uuid>,
    pub status: String,
    pub currency: String,

    pub provider: String,
    pub payment_provider: Option<String>,
    pub external_invoice_id: Option<String>,
    pub external_hosted_url: Option<String>,
    pub external_pdf_url: Option<String>,
    pub tax_details: Option<serde_json::Value>,

    pub issue_date: Option<NaiveDate>,
    pub due_date: Option<NaiveDate>,
    pub issued_at: Option<String>,
    pub paid_at: Option<String>,
    pub voided_at: Option<String>,

    pub subtotal: i64,
    pub discount_amount: i64,
    pub tax_amount: i64,
    pub shipping_amount: i64,
    pub total: i64,
    pub amount_refunded: i64,
    pub amount_remaining: i64,

    pub discount_mode: Option<String>,
    pub discount_value: Option<String>,
    pub tax_mode: Option<String>,
    pub tax_value: Option<String>,
    pub shipping_mode: Option<String>,
    pub shipping_value: Option<String>,

    pub billing_name: Option<String>,
    pub billing_address: Option<String>,
    pub billing_email: Option<String>,
    pub billing_phone: Option<String>,
    pub billing_tax_id: Option<String>,

    pub seller_name: Option<String>,
    pub seller_address: Option<String>,
    pub seller_email: Option<String>,
    pub seller_phone: Option<String>,
    pub seller_tax_id: Option<String>,

    pub notes: Option<String>,
    pub payment_terms: Option<String>,
    pub void_reason: Option<String>,

    pub line_items: Vec<InvoiceLineItemResponse>,
    pub history: Vec<InvoiceHistoryResponse>,
    pub credit_notes: Vec<CreditNoteResponse>,

    pub created_at: String,
    pub updated_at: String,
}

// ---------------------------------------------------------------------------
// Invoice List Response
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct InvoiceListResponse {
    pub total: u64,
    pub page: u64,
    pub page_size: u64,
    pub data: Vec<InvoiceResponse>,
}

// ---------------------------------------------------------------------------
// Conversion helpers
// ---------------------------------------------------------------------------

/// Parse optional adjustment mode string.
pub fn parse_adjustment_mode(s: Option<&str>) -> Option<AdjustmentMode> {
    s.and_then(AdjustmentMode::from_str_opt)
}

/// Convert domain InvoiceSummary to API InvoiceResponse.
pub fn summary_to_response(
    s: herald_core::domain::billing::invoice::InvoiceSummary,
) -> InvoiceResponse {
    InvoiceResponse {
        id: s.id,
        invoice_number: s.invoice_number,
        source: s.source.as_str().to_string(),
        account_id: s.account_id,
        status: s.status.as_str().to_string(),
        currency: s.currency,
        total: s.total,
        billing_name: s.billing_name,
        due_date: s.due_date,
        provider: s.provider.as_str().to_string(),
        payment_provider: s.payment_provider,
        external_hosted_url: s.external_hosted_url,
        external_pdf_url: s.external_pdf_url,
        amount_refunded: s.amount_refunded,
        created_at: s.created_at.to_rfc3339(),
    }
}

// ---------------------------------------------------------------------------
// Credit Note (Manual refund recording)
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize, ToSchema, Validate)]
#[serde(rename_all = "camelCase")]
pub struct CreateCreditNoteRequest {
    /// Refund amount in the smallest currency unit (must be a positive integer).
    #[validate(range(min = 1))]
    pub amount: i64,
    /// Admin-provided refund reason (free text, 1-500 chars).
    #[validate(length(min = 1, max = 500))]
    pub memo: String,
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CreditNoteResponse {
    pub id: Uuid,
    pub amount: i64,
    pub currency: String,
    pub source: String,
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub external_credit_note_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub memo: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_by_user_id: Option<Uuid>,
    pub created_at: String,
}

impl From<CreditNote> for CreditNoteResponse {
    fn from(cn: CreditNote) -> Self {
        Self {
            id: cn.id,
            amount: cn.amount,
            currency: cn.currency,
            source: cn.source.as_str().to_string(),
            status: cn.status.as_str().to_string(),
            external_credit_note_id: cn.external_credit_note_id,
            memo: cn.memo,
            created_by_user_id: cn.created_by_user_id,
            created_at: cn.created_at.to_rfc3339(),
        }
    }
}

/// Convert domain InvoiceDetail to API InvoiceDetailResponse.
pub fn invoice_to_detail_response(detail: InvoiceDetail) -> InvoiceDetailResponse {
    InvoiceDetailResponse {
        id: detail.invoice.id,
        realm_id: detail.invoice.realm_id,
        invoice_number: detail.invoice.invoice_number,
        source: detail.invoice.source.as_str().to_string(),
        account_id: detail.invoice.account_id,
        applicant_user_id: detail.invoice.applicant_user_id,
        subscription_id: detail.invoice.subscription_id,
        payment_attempt_id: detail.invoice.payment_attempt_id,
        status: detail.invoice.status.as_str().to_string(),
        currency: detail.invoice.currency,

        provider: detail.invoice.provider.as_str().to_string(),
        payment_provider: detail.invoice.payment_provider,
        external_invoice_id: detail.invoice.external_invoice_id,
        external_hosted_url: detail.invoice.external_hosted_url,
        external_pdf_url: detail.invoice.external_pdf_url,
        tax_details: detail.invoice.tax_details,

        issue_date: detail.invoice.issue_date,
        due_date: detail.invoice.due_date,
        issued_at: detail.invoice.issued_at.map(|dt| dt.to_rfc3339()),
        paid_at: detail.invoice.paid_at.map(|dt| dt.to_rfc3339()),
        voided_at: detail.invoice.voided_at.map(|dt| dt.to_rfc3339()),

        subtotal: detail.invoice.subtotal,
        discount_amount: detail.invoice.discount_amount,
        tax_amount: detail.invoice.tax_amount,
        shipping_amount: detail.invoice.shipping_amount,
        total: detail.invoice.total,
        amount_refunded: detail.invoice.amount_refunded,
        amount_remaining: detail.invoice.amount_remaining,

        discount_mode: detail.invoice.discount_mode.map(|m| m.as_str().to_string()),
        discount_value: detail.invoice.discount_value,
        tax_mode: detail.invoice.tax_mode.map(|m| m.as_str().to_string()),
        tax_value: detail.invoice.tax_value,
        shipping_mode: detail.invoice.shipping_mode.map(|m| m.as_str().to_string()),
        shipping_value: detail.invoice.shipping_value,

        billing_name: detail.invoice.billing_name,
        billing_address: detail.invoice.billing_address,
        billing_email: detail.invoice.billing_email,
        billing_phone: detail.invoice.billing_phone,
        billing_tax_id: detail.invoice.billing_tax_id,

        seller_name: detail.invoice.seller_name,
        seller_address: detail.invoice.seller_address,
        seller_email: detail.invoice.seller_email,
        seller_phone: detail.invoice.seller_phone,
        seller_tax_id: detail.invoice.seller_tax_id,

        notes: detail.invoice.notes,
        payment_terms: detail.invoice.payment_terms,
        void_reason: detail.invoice.void_reason,

        line_items: detail
            .line_items
            .into_iter()
            .map(InvoiceLineItemResponse::from)
            .collect(),
        history: detail
            .history
            .into_iter()
            .map(InvoiceHistoryResponse::from)
            .collect(),
        // No credit notes loaded in the base converter; populated by
        // `invoice_to_detail_response_with_credits` for handlers that need them.
        credit_notes: Vec::new(),

        created_at: detail.invoice.created_at.to_rfc3339(),
        updated_at: detail.invoice.updated_at.to_rfc3339(),
    }
}

/// Convert domain InvoiceDetail to API InvoiceDetailResponse, attaching the
/// provided `credit_notes` (already filtered/transformed for the audience).
/// Refund amounts come from the base converter (which reads them off the
/// invoice struct). Use this variant only when the response should also list
/// the credit notes themselves.
pub fn invoice_to_detail_response_with_credits(
    detail: InvoiceDetail,
    credit_notes: Vec<CreditNote>,
) -> InvoiceDetailResponse {
    let mut response = invoice_to_detail_response(detail);
    response.credit_notes = credit_notes
        .into_iter()
        .map(CreditNoteResponse::from)
        .collect();
    response
}
