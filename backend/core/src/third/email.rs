use std::future::Future;

use serde::Serialize;
use sqlx::PgPool;

// ---------------------------------------------------------------------------
// EmailProvider trait
// ---------------------------------------------------------------------------

/// Trait abstracting email delivery backends.
///
/// Implementors must be `Send + Sync` so they can be stored in shared state.
pub trait EmailProvider: Send + Sync {
    fn send_html(
        &self,
        to: &str,
        subject: &str,
        html: &str,
    ) -> impl Future<Output = anyhow::Result<()>> + Send;
}

// ---------------------------------------------------------------------------
// ResendClient (HTTP-based, wraps existing Resend API logic)
// ---------------------------------------------------------------------------

#[derive(Clone)]
pub struct ResendClient {
    token: String,
    from: String,
    http: reqwest::Client,
}

#[derive(Serialize)]
struct SendEmailRequest<'a> {
    from: &'a str,
    to: Vec<&'a str>,
    subject: &'a str,
    html: &'a str,
}

impl ResendClient {
    pub fn new(token: String, from: String) -> Self {
        Self {
            token,
            from,
            http: reqwest::Client::new(),
        }
    }
}

/// Sends an HTML email via the Resend API.
///
/// # Errors
///
/// Returns an error if the API request fails or returns a non-success status.
impl EmailProvider for ResendClient {
    async fn send_html(&self, to: &str, subject: &str, html: &str) -> anyhow::Result<()> {
        let body = SendEmailRequest {
            from: &self.from,
            to: vec![to],
            subject,
            html,
        };

        let resp = self
            .http
            .post("https://api.resend.com/emails")
            .bearer_auth(&self.token)
            .json(&body)
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            anyhow::bail!("resend send failed: {status} {text}");
        }
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// SmtpEmailProvider (uses lettre crate)
// ---------------------------------------------------------------------------

/// TLS mode for SMTP connections.
pub enum SmtpEncryption {
    /// Upgrade the connection with STARTTLS after connecting (port 587)
    StartTls,
    /// Use implicit TLS from the start (port 465)
    Ssl,
}

pub struct SmtpEmailProvider {
    host: String,
    port: u16,
    username: String,
    password: String,
    encryption: SmtpEncryption,
    from_address: String,
}

impl SmtpEmailProvider {
    pub fn new(
        host: String,
        port: u16,
        username: String,
        password: String,
        encryption: SmtpEncryption,
        from_address: String,
    ) -> Self {
        Self {
            host,
            port,
            username,
            password,
            encryption,
            from_address,
        }
    }
}

/// Sends an HTML email via SMTP using the `lettre` crate.
///
/// # Errors
///
/// Returns an error if the message cannot be built or the SMTP delivery fails.
impl EmailProvider for SmtpEmailProvider {
    async fn send_html(&self, to: &str, subject: &str, html: &str) -> anyhow::Result<()> {
        use lettre::message::{Mailbox, header::ContentType};
        use lettre::{AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor};

        let from_mailbox: Mailbox = self.from_address.parse()?;
        let to_mailbox: Mailbox = to.parse()?;

        let email = Message::builder()
            .from(from_mailbox)
            .to(to_mailbox)
            .subject(subject)
            .header(ContentType::TEXT_HTML)
            .body(html.to_string())?;

        let creds = lettre::transport::smtp::authentication::Credentials::new(
            self.username.clone(),
            self.password.clone(),
        );

        let mailer: AsyncSmtpTransport<Tokio1Executor> = match self.encryption {
            SmtpEncryption::StartTls => {
                AsyncSmtpTransport::<Tokio1Executor>::starttls_relay(&self.host)?
                    .port(self.port)
                    .credentials(creds)
                    .build()
            }
            SmtpEncryption::Ssl => AsyncSmtpTransport::<Tokio1Executor>::relay(&self.host)?
                .port(self.port)
                .credentials(creds)
                .build(),
        };

        mailer.send(email).await?;
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// EmailProviderKind (enum dispatch to avoid dyn)
// ---------------------------------------------------------------------------

/// Enum-dispatched email provider, avoiding the need for `Box<dyn EmailProvider>`.
pub enum EmailProviderKind {
    Resend(ResendClient),
    Smtp(SmtpEmailProvider),
}

impl EmailProvider for EmailProviderKind {
    async fn send_html(&self, to: &str, subject: &str, html: &str) -> anyhow::Result<()> {
        match self {
            Self::Resend(p) => p.send_html(to, subject, html).await,
            Self::Smtp(p) => p.send_html(to, subject, html).await,
        }
    }
}

// ---------------------------------------------------------------------------
// EmailConfig — parsed realm email configuration
// ---------------------------------------------------------------------------

/// Parsed email configuration extracted from realm_config key-value entries.
pub struct EmailConfig {
    /// Provider type: "resend" or "smtp"
    pub provider: String,
    /// Default sender address
    pub from_address: String,
    /// Resend API key (only set when provider == "resend")
    pub resend_api_key: Option<String>,
    /// SMTP host (only set when provider == "smtp")
    pub smtp_host: Option<String>,
    /// SMTP port (only set when provider == "smtp")
    pub smtp_port: Option<u16>,
    /// SMTP username (only set when provider == "smtp")
    pub smtp_username: Option<String>,
    /// SMTP password (only set when provider == "smtp")
    pub smtp_password: Option<String>,
    /// SMTP encryption mode: "starttls" or "ssl" (only set when provider == "smtp")
    pub smtp_encryption: Option<String>,
}

impl EmailConfig {
    /// Constructs the appropriate `EmailProviderKind` from this configuration.
    ///
    /// # Errors
    ///
    /// Returns an error if required fields for the chosen provider are missing.
    pub fn build_provider(&self) -> anyhow::Result<EmailProviderKind> {
        match self.provider.as_str() {
            "resend" => {
                let api_key = self.resend_api_key.as_deref().ok_or_else(|| {
                    anyhow::anyhow!("resend_api_key is required for Resend provider")
                })?;
                Ok(EmailProviderKind::Resend(ResendClient::new(
                    api_key.to_string(),
                    self.from_address.clone(),
                )))
            }
            "smtp" => {
                let host = self
                    .smtp_host
                    .as_deref()
                    .ok_or_else(|| anyhow::anyhow!("smtp_host is required for SMTP provider"))?;
                let port = self
                    .smtp_port
                    .ok_or_else(|| anyhow::anyhow!("smtp_port is required for SMTP provider"))?;
                let username = self.smtp_username.as_deref().ok_or_else(|| {
                    anyhow::anyhow!("smtp_username is required for SMTP provider")
                })?;
                let password = self.smtp_password.as_deref().ok_or_else(|| {
                    anyhow::anyhow!("smtp_password is required for SMTP provider")
                })?;
                let encryption = match self.smtp_encryption.as_deref().unwrap_or("starttls") {
                    "ssl" => SmtpEncryption::Ssl,
                    _ => SmtpEncryption::StartTls,
                };
                Ok(EmailProviderKind::Smtp(SmtpEmailProvider::new(
                    host.to_string(),
                    port,
                    username.to_string(),
                    password.to_string(),
                    encryption,
                    self.from_address.clone(),
                )))
            }
            other => anyhow::bail!("unknown email provider: {other}"),
        }
    }
}

// ---------------------------------------------------------------------------
// EmailConfigStatus — result of checking email configuration completeness
// ---------------------------------------------------------------------------

/// Status of email configuration for a realm.
#[derive(Debug, Clone, Serialize)]
pub struct EmailConfigStatus {
    /// Whether the email configuration is complete and usable.
    pub configured: bool,
    /// Provider type if set (e.g., "resend" or "smtp").
    pub provider: Option<String>,
    /// Sender address if set.
    pub from_address: Option<String>,
    /// List of config keys that are missing or empty.
    pub missing_fields: Vec<String>,
}

// ---------------------------------------------------------------------------
// EmailService — reads per-realm email config from realm_config, sends email
// ---------------------------------------------------------------------------

/// Stateless service for reading email configuration from `realm_config` and
/// sending emails through the appropriate provider.
pub struct EmailService;

impl EmailService {
    /// Read email configuration key-value pairs from `realm_config` for a realm.
    ///
    /// Returns `Ok(None)` if no rows with `config_type = 'email'` exist.
    async fn read_email_config(
        pool: &PgPool,
        realm_id: &str,
    ) -> anyhow::Result<Option<EmailConfig>> {
        let rows: Vec<(String, String)> = sqlx::query_as(
            "SELECT config_key, config_value FROM realm_config
             WHERE realm_id = $1 AND config_type = 'email'",
        )
        .bind(realm_id)
        .fetch_all(pool)
        .await?;

        if rows.is_empty() {
            return Ok(None);
        }

        let mut cfg = EmailConfig {
            provider: String::new(),
            from_address: String::new(),
            resend_api_key: None,
            smtp_host: None,
            smtp_port: None,
            smtp_username: None,
            smtp_password: None,
            smtp_encryption: None,
        };

        for (key, value) in rows {
            match key.as_str() {
                "provider" => cfg.provider = value,
                "from_address" => cfg.from_address = value,
                "resend_api_key" => cfg.resend_api_key = Some(value),
                "smtp_host" => cfg.smtp_host = Some(value),
                "smtp_port" => {
                    cfg.smtp_port = value.parse::<u16>().ok();
                }
                "smtp_username" => cfg.smtp_username = Some(value),
                "smtp_password" => cfg.smtp_password = Some(value),
                "smtp_encryption" => cfg.smtp_encryption = Some(value),
                _ => {} // ignore unknown keys
            }
        }

        Ok(Some(cfg))
    }

    /// Check whether email is fully configured for a realm.
    ///
    /// Returns a detailed status including which required fields are missing.
    pub async fn is_email_configured(
        pool: &PgPool,
        realm_id: &str,
    ) -> anyhow::Result<EmailConfigStatus> {
        let cfg = Self::read_email_config(pool, realm_id).await?;

        let Some(cfg) = cfg else {
            return Ok(EmailConfigStatus {
                configured: false,
                provider: None,
                from_address: None,
                missing_fields: vec!["provider".to_string(), "from_address".to_string()],
            });
        };

        let provider_str = if cfg.provider.is_empty() {
            None
        } else {
            Some(cfg.provider.clone())
        };

        let from_address_str = if cfg.from_address.is_empty() {
            None
        } else {
            Some(cfg.from_address.clone())
        };

        let mut missing = Vec::new();

        // Common required fields
        if cfg.provider.is_empty() {
            missing.push("provider".to_string());
        }
        if cfg.from_address.is_empty() {
            missing.push("from_address".to_string());
        }

        // Provider-specific required fields
        match cfg.provider.as_str() {
            "resend" if cfg.resend_api_key.as_deref().unwrap_or("").is_empty() => {
                missing.push("resend_api_key".to_string());
            }
            "resend" => {}
            "smtp" => {
                if cfg.smtp_host.as_deref().unwrap_or("").is_empty() {
                    missing.push("smtp_host".to_string());
                }
                if cfg.smtp_port.is_none() {
                    missing.push("smtp_port".to_string());
                }
                if cfg.smtp_username.as_deref().unwrap_or("").is_empty() {
                    missing.push("smtp_username".to_string());
                }
                if cfg.smtp_password.as_deref().unwrap_or("").is_empty() {
                    missing.push("smtp_password".to_string());
                }
            }
            _ => {} // unknown provider — already flagged by missing "provider"
        }

        let configured = missing.is_empty();

        Ok(EmailConfigStatus {
            configured,
            provider: provider_str,
            from_address: from_address_str,
            missing_fields: missing,
        })
    }

    /// Send an HTML email for a realm.
    ///
    /// Reads the realm's email configuration, builds the appropriate provider,
    /// and sends the email. Returns `Ok(())` silently when email is not
    /// configured for the realm (callers can ignore the result).
    /// Returns `Err` on send failure so callers can decide propagation.
    pub async fn send_html_email(
        pool: &PgPool,
        realm_id: &str,
        to: &str,
        subject: &str,
        html: &str,
    ) -> anyhow::Result<()> {
        let cfg = Self::read_email_config(pool, realm_id).await?;

        let Some(cfg) = cfg else {
            // Not configured — silently skip.
            return Ok(());
        };

        // Basic sanity: need at least provider and from_address to attempt sending.
        if cfg.provider.is_empty() || cfg.from_address.is_empty() {
            return Ok(());
        }

        let provider = cfg.build_provider()?;
        provider.send_html(to, subject, html).await?;

        Ok(())
    }
}
