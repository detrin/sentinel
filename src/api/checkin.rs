use crate::{db, models::CheckinResponse, AppState};
use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use chrono::Utc;
use subtle::ConstantTimeEq;

/// Check-in endpoint with Bearer token authentication
/// POST /api/checkin/:id
pub async fn checkin(
    State(state): State<AppState>,
    Path(switch_id): Path<String>,
    headers: HeaderMap,
) -> Response {
    // Extract Bearer token from Authorization header
    let token = match extract_bearer_token(&headers) {
        Some(token) => token,
        None => {
            return (
                StatusCode::UNAUTHORIZED,
                Json(serde_json::json!({
                    "error": "Missing or invalid Authorization header"
                })),
            )
                .into_response();
        }
    };

    // Get switch by ID
    let switch = match db::get_switch_by_id(&state.pool, &switch_id).await {
        Ok(Some(switch)) => switch,
        Ok(None) => {
            // Return same error as invalid token to prevent switch ID enumeration
            tracing::warn!("Authentication failed for switch {}", switch_id);
            return (
                StatusCode::UNAUTHORIZED,
                Json(serde_json::json!({
                    "error": "Authentication failed"
                })),
            )
                .into_response();
        }
        Err(e) => {
            tracing::error!("Database error during checkin");
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({
                    "error": "Internal server error"
                })),
            )
                .into_response();
        }
    };

    // Verify token matches switch's API token using constant-time comparison
    let token_valid = switch.api_token.as_bytes().ct_eq(token.as_bytes()).into();

    if !token_valid {
        // Log without exposing actual token values
        tracing::warn!("Authentication failed for switch {}", switch_id);
        return (
            StatusCode::UNAUTHORIZED,
            Json(serde_json::json!({
                "error": "Authentication failed"
            })),
        )
            .into_response();
    }

    // Update last checkin timestamp
    let now = Utc::now().timestamp();
    if let Err(e) = db::update_checkin(&state.pool, &switch_id, now).await {
        tracing::error!("Failed to update checkin: {}", e);
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({
                "error": "Failed to update checkin"
            })),
        )
            .into_response();
    }

    tracing::info!("Check-in successful for switch: {}", switch.name);

    let next_deadline = now + switch.timeout_seconds;

    (
        StatusCode::OK,
        Json(CheckinResponse {
            success: true,
            last_checkin: now,
            next_deadline,
        }),
    )
        .into_response()
}

/// Extract Bearer token from Authorization header
fn extract_bearer_token(headers: &HeaderMap) -> Option<String> {
    let auth_header = headers.get("authorization")?.to_str().ok()?;

    if auth_header.starts_with("Bearer ") {
        Some(auth_header[7..].to_string())
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_bearer_token() {
        let mut headers = HeaderMap::new();
        headers.insert(
            "authorization",
            "Bearer abc123def456".parse().unwrap(),
        );

        let token = extract_bearer_token(&headers);
        assert_eq!(token, Some("abc123def456".to_string()));
    }

    #[test]
    fn test_extract_bearer_token_missing() {
        let headers = HeaderMap::new();
        let token = extract_bearer_token(&headers);
        assert_eq!(token, None);
    }

    #[test]
    fn test_extract_bearer_token_invalid_format() {
        let mut headers = HeaderMap::new();
        headers.insert("authorization", "Basic abc123".parse().unwrap());

        let token = extract_bearer_token(&headers);
        assert_eq!(token, None);
    }
}
