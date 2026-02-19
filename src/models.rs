use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Switch {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub api_token: String,
    pub timeout_seconds: i64,
    pub last_checkin: i64,
    pub last_trigger: Option<i64>,
    pub status: String, // active/triggered/paused
    pub created_at: i64,
    pub trigger_count_max: i64,
    pub trigger_interval_seconds: i64,
    pub trigger_count_executed: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct WarningStage {
    pub id: i64,
    pub switch_id: String,
    pub seconds_before_deadline: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
#[allow(dead_code)]
pub struct WarningExecution {
    pub id: i64,
    pub switch_id: String,
    pub stage_seconds: i64,
    pub executed_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Action {
    pub id: i64,
    pub switch_id: String,
    pub action_order: i64,
    pub action_type: String, // email/webhook/script
    pub is_warning: bool,
    pub config: String, // JSON config
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ActionExecution {
    pub id: i64,
    pub switch_id: String,
    pub action_id: i64,
    pub execution_type: String, // warning/final
    pub started_at: i64,
    pub completed_at: Option<i64>,
    pub status: String, // running/completed/failed
    pub exit_code: Option<i64>,
    pub stdout: Option<String>,
    pub stderr: Option<String>,
    pub error_message: Option<String>,
}

// Action configuration types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmailActionConfig {
    pub to: String,
    pub subject: String,
    pub body: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebhookActionConfig {
    pub url: String,
    pub method: String, // GET/POST
    pub headers: Option<std::collections::HashMap<String, String>>,
    pub body: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScriptActionConfig {
    pub script_path: String,
    pub args: Vec<String>,
}

// Helper structs for API requests
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateSwitchRequest {
    pub name: String,
    pub description: Option<String>,
    pub timeout_seconds: i64,
    pub trigger_count_max: i64,
    pub trigger_interval_seconds: i64,
    pub warning_stages: Vec<i64>, // seconds before deadline
    pub warning_actions: Vec<CreateActionRequest>,
    pub final_actions: Vec<CreateActionRequest>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateActionRequest {
    pub action_type: String,
    pub config: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckinResponse {
    pub success: bool,
    pub last_checkin: i64,
    pub next_deadline: i64,
}
