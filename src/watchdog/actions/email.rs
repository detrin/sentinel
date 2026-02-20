use crate::{config::SmtpConfig, models::EmailActionConfig, watchdog::executor::ActionResult};
use lettre::{
    message::header::ContentType,
    transport::smtp::{authentication::Credentials, client::Tls, client::TlsParameters},
    Message, SmtpTransport, Transport,
};
use tokio::time::{timeout, Duration};

/// Execute an email action
pub async fn execute(config_json: &str, smtp_config: &SmtpConfig) -> ActionResult {
    // Parse email configuration
    let email_config: EmailActionConfig = serde_json::from_str(config_json)
        .map_err(|e| format!("Failed to parse email config: {}", e))?;

    // Build email message
    let mut builder = Message::builder()
        .from(
            smtp_config
                .from
                .parse()
                .map_err(|e| format!("Invalid 'from' address: {}", e))?,
        )
        .subject(&email_config.subject)
        .header(ContentType::TEXT_PLAIN);

    for recipient in &email_config.bcc {
        builder = builder.bcc(recipient.parse().map_err(|e| format!("Invalid BCC address '{}': {}", recipient, e))?);
    }

    let email = builder
        .body(email_config.body.clone())
        .map_err(|e| format!("Failed to build email: {}", e))?;

    // Create SMTP transport
    let creds = Credentials::new(smtp_config.username.clone(), smtp_config.password.clone());

    // Create TLS parameters with the SMTP host domain
    let tls_params = TlsParameters::new(smtp_config.host.clone())
        .map_err(|e| format!("Failed to create TLS parameters: {}", e))?;

    // Determine TLS mode based on port
    let tls = if smtp_config.port == 465 {
        Tls::Wrapper(tls_params)
    } else {
        Tls::Required(tls_params)
    };

    let mailer = SmtpTransport::relay(&smtp_config.host)
        .map_err(|e| format!("Failed to create SMTP transport: {}", e))?
        .credentials(creds)
        .port(smtp_config.port)
        .tls(tls)
        .build();

    // Send email with timeout
    let mailer_clone = mailer;
    let email_clone = email;

    let result = timeout(Duration::from_secs(30), async move {
        tokio::task::spawn_blocking(move || mailer_clone.send(&email_clone)).await
    })
    .await;

    match result {
        Ok(Ok(Ok(_response))) => {
            Ok((0, format!("Email sent to {} BCC recipients", email_config.bcc.len()), String::new()))
        }
        Ok(Ok(Err(e))) => Err(format!("Failed to send email: {}", e)),
        Ok(Err(e)) => Err(format!("Task join error: {}", e)),
        Err(_) => Err("Email send timeout (30s)".to_string()),
    }
}
