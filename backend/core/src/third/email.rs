use serde::Serialize;

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

    /// Sends an HTML email
    ///
    /// # Arguments
    ///
    /// * `to` - Recipient email address
    /// * `subject` - Email subject line
    /// * `html` - Email body content in HTML format
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The recipient email address is invalid
    /// - The Resend API request fails
    /// - Network connectivity issues occur
    /// - The API key is invalid or expired
    pub async fn send_html(&self, to: &str, subject: &str, html: &str) -> anyhow::Result<()> {
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
