use crate::{models::WebhookActionConfig, watchdog::executor::ActionResult};
use reqwest::Client;
use tokio::time::{timeout, Duration};

/// Execute a webhook action
pub async fn execute(config_json: &str) -> ActionResult {
    // Parse webhook configuration
    let webhook_config: WebhookActionConfig = serde_json::from_str(config_json)
        .map_err(|e| format!("Failed to parse webhook config: {}", e))?;

    // Create HTTP client
    let client = Client::builder()
        .timeout(Duration::from_secs(30))
        .build()
        .map_err(|e| format!("Failed to create HTTP client: {}", e))?;

    // Build request
    let mut request = match webhook_config.method.to_uppercase().as_str() {
        "GET" => client.get(&webhook_config.url),
        "POST" => client.post(&webhook_config.url),
        _ => return Err(format!("Unsupported HTTP method: {}", webhook_config.method)),
    };

    // Add headers if provided
    if let Some(headers) = &webhook_config.headers {
        for (key, value) in headers {
            request = request.header(key, value);
        }
    }

    // Add body if provided (for POST requests)
    if let Some(body) = &webhook_config.body {
        request = request.header("Content-Type", "application/json").body(body.clone());
    }

    // Execute request with timeout
    let result = timeout(Duration::from_secs(30), request.send()).await;

    match result {
        Ok(Ok(response)) => {
            let status = response.status();
            let status_code = status.as_u16() as i64;

            // Read response body
            let body = response
                .text()
                .await
                .unwrap_or_else(|_| "(failed to read body)".to_string());

            if status.is_success() {
                Ok((
                    0,
                    format!("Webhook executed successfully (HTTP {})", status_code),
                    body,
                ))
            } else {
                Err(format!(
                    "Webhook failed with HTTP {}: {}",
                    status_code, body
                ))
            }
        }
        Ok(Err(e)) => Err(format!("Failed to execute webhook: {}", e)),
        Err(_) => Err("Webhook timeout (30s)".to_string()),
    }
}
