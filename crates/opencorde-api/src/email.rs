//! # Email Service
//! Handles sending transactional emails (password resets, etc.).
//!
//! ## Features
//! - Development mode: logs reset links instead of sending email
//! - Production mode: logs SMTP details (ready for integration with external SMTP)
//! - Configurable sender email and base URL
//!
//! ## Configuration
//! - SMTP_HOST: SMTP server hostname (optional, enables production mode)
//! - SMTP_PORT: SMTP port (default 587)
//! - SMTP_USERNAME: SMTP username (optional)
//! - SMTP_PASSWORD: SMTP password (optional)
//! - SMTP_FROM: Sender email (default noreply@localhost)
//! - BASE_URL: Base URL for reset links (default http://localhost:5173)
//!
//! ## Depends On
//! - lettre (async SMTP transport)
//! - tracing (structured logging)
//! - anyhow (error handling)

use lettre::{
    message::header::ContentType,
    transport::smtp::authentication::Credentials,
    AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor,
};
use tracing::instrument;

/// Email service for sending transactional emails.
///
/// If SMTP is not configured, emails are logged to tracing::info() instead.
#[derive(Clone, Debug)]
pub struct EmailService {
    /// Whether SMTP is configured
    smtp_configured: bool,
    /// SMTP hostname
    smtp_host: Option<String>,
    /// SMTP port
    smtp_port: u16,
    /// SMTP username
    smtp_username: Option<String>,
    /// SMTP password
    smtp_password: Option<String>,
    /// Sender email address
    smtp_from: String,
    /// Base URL for generating reset links
    base_url: String,
}

impl EmailService {
    /// Create a new email service from configuration values.
    ///
    /// # Arguments
    /// - `smtp_host`: SMTP hostname (None = dev mode, log only)
    /// - `smtp_port`: SMTP port (default 587 if host provided)
    /// - `smtp_username`: SMTP username (optional)
    /// - `smtp_password`: SMTP password (optional)
    /// - `smtp_from`: Sender email (e.g., "noreply@example.com")
    /// - `base_url`: Base URL for generating links (e.g., "https://example.com")
    pub fn new(
        smtp_host: Option<String>,
        smtp_port: u16,
        smtp_username: Option<String>,
        smtp_password: Option<String>,
        smtp_from: String,
        base_url: String,
    ) -> Self {
        let smtp_configured = smtp_host.is_some();
        Self {
            smtp_configured,
            smtp_host,
            smtp_port,
            smtp_username,
            smtp_password,
            smtp_from,
            base_url,
        }
    }

    /// Check if SMTP is configured.
    pub fn is_configured(&self) -> bool {
        self.smtp_configured
    }

    /// Send an email verification email.
    ///
    /// In development mode, logs the link. In production, sends via SMTP.
    ///
    /// # Arguments
    /// - `to`: Recipient email address
    /// - `token`: Email verification token
    #[instrument(skip(self), fields(to, token = %token.chars().take(8).collect::<String>()))]
    pub async fn send_verification_email(
        &self,
        to: &str,
        token: &str,
    ) -> anyhow::Result<()> {
        let verify_link = format!("{}/verify-email?token={}", self.base_url, token);

        if !self.smtp_configured {
            tracing::info!(
                to = %to,
                verify_link = %verify_link,
                "verification email (development mode - not sent)"
            );
            return Ok(());
        }

        self.send_smtp_email(to, "Verify your OpenCorde email", &self.build_verification_html(&verify_link))
            .await
    }

    /// Send a password reset email.
    ///
    /// In development mode (no SMTP configured), logs the reset link instead.
    /// In production, sends an actual email via SMTP.
    ///
    /// # Arguments
    /// - `to`: Recipient email address
    /// - `token`: Password reset token
    ///
    /// # Returns
    /// Ok(()) on success, Err if email sending fails.
    #[instrument(skip(self), fields(to, token = %token.chars().take(8).collect::<String>()))]
    pub async fn send_password_reset(
        &self,
        to: &str,
        token: &str,
    ) -> anyhow::Result<()> {
        let reset_link = format!("{}/reset-password?token={}", self.base_url, token);

        if !self.smtp_configured {
            // Development mode: log the reset link instead of sending
            tracing::info!(
                to = %to,
                reset_link = %reset_link,
                "password reset email (development mode - not sent)"
            );
            return Ok(());
        }

        // Production mode: send email via SMTP
        self.send_smtp_email(to, "Password Reset", &self.build_reset_html(&reset_link))
            .await
    }

    /// Send email via SMTP using lettre async transport.
    async fn send_smtp_email(
        &self,
        to: &str,
        subject: &str,
        html_body: &str,
    ) -> anyhow::Result<()> {
        let host = self
            .smtp_host
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("SMTP not configured"))?;

        let email = Message::builder()
            .from(self.smtp_from.parse()?)
            .to(to.parse()?)
            .subject(subject)
            .header(ContentType::TEXT_HTML)
            .body(html_body.to_string())?;

        let mut builder = AsyncSmtpTransport::<Tokio1Executor>::relay(host)?.port(self.smtp_port);

        if let (Some(username), Some(password)) = (&self.smtp_username, &self.smtp_password) {
            let creds = Credentials::new(username.clone(), password.clone());
            builder = builder.credentials(creds);
        }

        builder.build().send(email).await?;

        tracing::info!(to = %to, subject = %subject, "email sent via SMTP");
        Ok(())
    }

    /// Build HTML email body for email verification.
    fn build_verification_html(&self, verify_link: &str) -> String {
        format!(
            r#"<!DOCTYPE html>
<html>
<head>
    <meta charset="utf-8">
    <style>
        body {{ font-family: Arial, sans-serif; line-height: 1.6; color: #333; }}
        .container {{ max-width: 600px; margin: 0 auto; padding: 20px; }}
        .header {{ background-color: #4f46e5; color: white; padding: 20px; text-align: center; border-radius: 5px 5px 0 0; }}
        .content {{ background-color: #f9fafb; padding: 20px; border: 1px solid #e5e7eb; border-radius: 0 0 5px 5px; }}
        .button {{ display: inline-block; background-color: #4f46e5; color: white; padding: 12px 24px; text-decoration: none; border-radius: 5px; margin: 20px 0; }}
        .footer {{ margin-top: 20px; font-size: 12px; color: #666; text-align: center; }}
    </style>
</head>
<body>
    <div class="container">
        <div class="header">
            <h1>Verify Your Email</h1>
        </div>
        <div class="content">
            <p>Welcome to OpenCorde!</p>
            <p>Click the button below to verify your email address:</p>
            <a href="{}" class="button">Verify Email</a>
            <p>Or copy and paste this link in your browser:</p>
            <p><code>{}</code></p>
            <p>This link expires in 24 hours.</p>
            <p>If you didn't create an OpenCorde account, you can ignore this email.</p>
        </div>
        <div class="footer">
            <p>OpenCorde - Federated Chat System</p>
        </div>
    </div>
</body>
</html>"#,
            verify_link, verify_link
        )
    }

    /// Build HTML email body for password reset.
    fn build_reset_html(&self, reset_link: &str) -> String {
        format!(
            r#"<!DOCTYPE html>
<html>
<head>
    <meta charset="utf-8">
    <style>
        body {{ font-family: Arial, sans-serif; line-height: 1.6; color: #333; }}
        .container {{ max-width: 600px; margin: 0 auto; padding: 20px; }}
        .header {{ background-color: #4f46e5; color: white; padding: 20px; text-align: center; border-radius: 5px 5px 0 0; }}
        .content {{ background-color: #f9fafb; padding: 20px; border: 1px solid #e5e7eb; border-radius: 0 0 5px 5px; }}
        .button {{ display: inline-block; background-color: #4f46e5; color: white; padding: 12px 24px; text-decoration: none; border-radius: 5px; margin: 20px 0; }}
        .footer {{ margin-top: 20px; font-size: 12px; color: #666; text-align: center; }}
    </style>
</head>
<body>
    <div class="container">
        <div class="header">
            <h1>Password Reset</h1>
        </div>
        <div class="content">
            <p>Hello,</p>
            <p>We received a request to reset your OpenCorde password. Click the button below to set a new password:</p>
            <a href="{}" class="button">Reset Password</a>
            <p>Or copy and paste this link in your browser:</p>
            <p><code>{}</code></p>
            <p>This link expires in 1 hour.</p>
            <p>If you didn't request a password reset, you can ignore this email.</p>
        </div>
        <div class="footer">
            <p>OpenCorde - Federated Chat System</p>
        </div>
    </div>
</body>
</html>"#,
            reset_link, reset_link
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_email_service_dev_mode() {
        let service = EmailService::new(
            None,
            587,
            None,
            None,
            "noreply@localhost".to_string(),
            "http://localhost:5173".to_string(),
        );
        assert!(!service.smtp_configured);
    }

    #[test]
    fn test_email_service_prod_mode() {
        let service = EmailService::new(
            Some("smtp.example.com".to_string()),
            587,
            Some("user".to_string()),
            Some("pass".to_string()),
            "noreply@example.com".to_string(),
            "https://example.com".to_string(),
        );
        assert!(service.smtp_configured);
    }

    #[test]
    fn test_reset_html_generation() {
        let service = EmailService::new(
            None,
            587,
            None,
            None,
            "noreply@localhost".to_string(),
            "http://localhost:5173".to_string(),
        );
        let html = service.build_reset_html("http://localhost:5173/reset-password?token=abc123");
        assert!(html.contains("Password Reset"));
        assert!(html.contains("abc123"));
    }
}
