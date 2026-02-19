use crate::{auth, db, models::*, AppState};
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use chrono::Utc;

pub async fn list_switches(State(state): State<AppState>) -> Response {
    match db::list_switches(&state.pool).await {
        Ok(switches) => (StatusCode::OK, Json(switches)).into_response(),
        Err(e) => {
            tracing::error!("Failed to list switches: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({
                    "error": "Failed to list switches"
                })),
            )
                .into_response()
        }
    }
}

pub async fn get_switch(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Response {
    let switch = match db::get_switch_by_id(&state.pool, &id).await {
        Ok(Some(switch)) => switch,
        Ok(None) => {
            return (
                StatusCode::NOT_FOUND,
                Json(serde_json::json!({
                    "error": "Switch not found"
                })),
            )
                .into_response();
        }
        Err(e) => {
            tracing::error!("Failed to get switch: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({
                    "error": "Failed to get switch"
                })),
            )
                .into_response();
        }
    };

    let warning_stages = match db::get_warning_stages_for_switch(&state.pool, &id).await {
        Ok(stages) => stages,
        Err(e) => {
            tracing::error!("Failed to get warning stages: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({
                    "error": "Failed to get warning stages"
                })),
            )
                .into_response();
        }
    };

    let warning_actions = match db::get_actions_for_switch(&state.pool, &id, true).await {
        Ok(actions) => actions,
        Err(e) => {
            tracing::error!("Failed to get warning actions: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({
                    "error": "Failed to get warning actions"
                })),
            )
                .into_response();
        }
    };

    let final_actions = match db::get_actions_for_switch(&state.pool, &id, false).await {
        Ok(actions) => actions,
        Err(e) => {
            tracing::error!("Failed to get final actions: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({
                    "error": "Failed to get final actions"
                })),
            )
                .into_response();
        }
    };

    let execution_history = match db::get_execution_history(&state.pool, &id).await {
        Ok(history) => history,
        Err(e) => {
            tracing::error!("Failed to get execution history: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({
                    "error": "Failed to get execution history"
                })),
            )
                .into_response();
        }
    };

    (
        StatusCode::OK,
        Json(serde_json::json!({
            "switch": switch,
            "warning_stages": warning_stages,
            "warning_actions": warning_actions,
            "final_actions": final_actions,
            "execution_history": execution_history
        })),
    )
        .into_response()
}

pub async fn create_switch(
    State(state): State<AppState>,
    Json(request): Json<CreateSwitchRequest>,
) -> Response {
    if request.trigger_count_max < 0 {
        return (
            StatusCode::BAD_REQUEST,
            "trigger_count_max must be >= 0",
        )
            .into_response();
    }

    if request.trigger_interval_seconds < 1 {
        return (
            StatusCode::BAD_REQUEST,
            "trigger_interval_seconds must be >= 1",
        )
            .into_response();
    }

    let switch_id = uuid::Uuid::new_v4().to_string();
    let api_token = auth::generate_api_token();
    let now = Utc::now().timestamp();

    let switch = Switch {
        id: switch_id.clone(),
        name: request.name,
        description: request.description,
        api_token: api_token.clone(),
        timeout_seconds: request.timeout_seconds,
        last_checkin: now,
        last_trigger: None,
        status: "active".to_string(),
        created_at: now,
        trigger_count_max: request.trigger_count_max,
        trigger_interval_seconds: request.trigger_interval_seconds,
        trigger_count_executed: 0,
    };

    if let Err(e) = db::create_switch(&state.pool, &switch).await {
        tracing::error!("Failed to create switch: {}", e);
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({
                "error": "Failed to create switch"
            })),
        )
            .into_response();
    }

    for seconds_before_deadline in request.warning_stages {
        let stage = WarningStage {
            id: 0,
            switch_id: switch_id.clone(),
            seconds_before_deadline,
        };
        if let Err(e) = db::create_warning_stage(&state.pool, &stage).await {
            tracing::error!("Failed to create warning stage: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({
                    "error": "Failed to create warning stage"
                })),
            )
                .into_response();
        }
    }

    for (order, action_req) in request.warning_actions.iter().enumerate() {
        let action = Action {
            id: 0,
            switch_id: switch_id.clone(),
            action_order: order as i64,
            action_type: action_req.action_type.clone(),
            is_warning: true,
            config: action_req.config.to_string(),
        };
        if let Err(e) = db::create_action(&state.pool, &action).await {
            tracing::error!("Failed to create warning action: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({
                    "error": "Failed to create warning action"
                })),
            )
                .into_response();
        }
    }

    for (order, action_req) in request.final_actions.iter().enumerate() {
        let action = Action {
            id: 0,
            switch_id: switch_id.clone(),
            action_order: order as i64,
            action_type: action_req.action_type.clone(),
            is_warning: false,
            config: action_req.config.to_string(),
        };
        if let Err(e) = db::create_action(&state.pool, &action).await {
            tracing::error!("Failed to create final action: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({
                    "error": "Failed to create final action"
                })),
            )
                .into_response();
        }
    }

    tracing::info!("Created new switch: {} ({})", switch.name, switch.id);

    (
        StatusCode::CREATED,
        Json(serde_json::json!({
            "success": true,
            "switch_id": switch_id,
            "api_token": api_token
        })),
    )
        .into_response()
}

pub async fn delete_switch(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Response {
    match db::get_switch_by_id(&state.pool, &id).await {
        Ok(Some(_)) => {}
        Ok(None) => {
            return (
                StatusCode::NOT_FOUND,
                Json(serde_json::json!({
                    "error": "Switch not found"
                })),
            )
                .into_response();
        }
        Err(e) => {
            tracing::error!("Failed to get switch: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({
                    "error": "Failed to delete switch"
                })),
            )
                .into_response();
        }
    }

    if let Err(e) = sqlx::query!("DELETE FROM switches WHERE id = ?", id)
        .execute(&state.pool)
        .await
    {
        tracing::error!("Failed to delete switch: {}", e);
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({
                "error": "Failed to delete switch"
            })),
        )
            .into_response();
    }

    tracing::info!("Deleted switch: {}", id);

    (
        StatusCode::OK,
        Json(serde_json::json!({
            "success": true
        })),
    )
        .into_response()
}

pub async fn list_scripts(State(state): State<AppState>) -> Response {
    let scripts_dir = &state.config.security.scripts_dir;

    match std::fs::read_dir(scripts_dir) {
        Ok(entries) => {
            let scripts: Vec<String> = entries
                .filter_map(|entry| entry.ok())
                .filter(|entry| {
                    entry.file_type().ok().map(|ft| ft.is_file()).unwrap_or(false)
                })
                .filter_map(|entry| {
                    entry.file_name().to_str().map(|s| s.to_string())
                })
                .filter(|name| !name.starts_with('.'))
                .collect();

            (StatusCode::OK, Json(scripts)).into_response()
        }
        Err(e) => {
            tracing::error!("Failed to list scripts: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(Vec::<String>::new())
            ).into_response()
        }
    }
}
