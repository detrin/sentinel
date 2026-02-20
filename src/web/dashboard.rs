use crate::{db, AppState};
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{Html, IntoResponse, Response},
};
use chrono::Utc;

/// Render dashboard page
pub async fn dashboard(State(state): State<AppState>) -> Response {
    // Get all switches
    let switches = match db::list_switches(&state.pool).await {
        Ok(switches) => switches,
        Err(e) => {
            tracing::error!("Failed to list switches: {}", e);
            return (StatusCode::INTERNAL_SERVER_ERROR, "Failed to load switches").into_response();
        }
    };

    let now = Utc::now().timestamp();

    // Build HTML
    let mut html = String::from(include_str!("../../templates/dashboard_header.html"));

    for switch in switches {
        let time_since_checkin = now - switch.last_checkin;
        let time_until_deadline = switch.timeout_seconds - time_since_checkin;

        let status_class = if switch.status != "active" {
            "expired"
        } else if time_until_deadline < 0 {
            "expired"
        } else if time_until_deadline < (switch.timeout_seconds / 4) {
            "warning"
        } else {
            "alive"
        };

        let last_checkin_text = format_relative_time(switch.last_checkin);
        let deadline_text = if time_until_deadline > 0 {
            format!("in {}", format_duration(time_until_deadline))
        } else {
            format!("{} ago", format_duration(-time_until_deadline))
        };

        html.push_str(&format!(
            r#"
            <div class="switch-card status-{}">
                <div class="switch-header">
                    <h2>{}</h2>
                    <span class="status-badge">{}</span>
                </div>
                <p class="description">{}</p>
                <div class="switch-info">
                    <div class="info-item">
                        <span class="label">Last check-in:</span>
                        <span class="value">{}</span>
                    </div>
                    <div class="info-item">
                        <span class="label">Deadline:</span>
                        <span class="value">{}</span>
                    </div>
                </div>
                <div class="switch-actions">
                    <button
                        class="btn-checkin"
                        hx-post="/api/checkin/{}"
                        hx-headers='{{"Authorization": "Bearer {}"}}'
                        hx-swap="none"
                        hx-on::after-request="location.reload()"
                    >
                        Check In
                    </button>
                    <a href="/switches/{}" class="btn-secondary">View Details</a>
                </div>
            </div>
            "#,
            status_class,
            escape_html(&switch.name),
            switch.status.to_uppercase(),
            escape_html(&switch.description.unwrap_or_else(|| "No description".to_string())),
            last_checkin_text,
            deadline_text,
            switch.id,
            switch.api_token,
            switch.id
        ));
    }

    html.push_str(include_str!("../../templates/dashboard_footer.html"));

    Html(html).into_response()
}

/// Render switch detail page
pub async fn switch_detail(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Response {
    // Get switch
    let switch = match db::get_switch_by_id(&state.pool, &id).await {
        Ok(Some(s)) => s,
        Ok(None) => return (StatusCode::NOT_FOUND, "Switch not found").into_response(),
        Err(e) => {
            tracing::error!("Failed to get switch: {}", e);
            return (StatusCode::INTERNAL_SERVER_ERROR, "Internal error").into_response();
        }
    };

    // Get warning stages
    let warning_stages = db::get_warning_stages_for_switch(&state.pool, &id)
        .await
        .unwrap_or_default();

    // Get actions
    let warning_actions = db::get_actions_for_switch(&state.pool, &id, true)
        .await
        .unwrap_or_default();
    let final_actions = db::get_actions_for_switch(&state.pool, &id, false)
        .await
        .unwrap_or_default();

    // Get execution history
    let executions = db::get_execution_history(&state.pool, &id)
        .await
        .unwrap_or_default();

    let html = format!(
        r#"<!DOCTYPE html>
<html>
<head>
    <title>{} - Sentinel</title>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <script src="https://unpkg.com/htmx.org@1.9.10"></script>
    <style>
        * {{
            margin: 0;
            padding: 0;
            box-sizing: border-box;
        }}

        body {{
            font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif;
            background: #fafafa;
            padding: 40px 20px;
            color: #333;
        }}

        .container {{
            max-width: 1000px;
            margin: 0 auto;
        }}

        header {{
            display: flex;
            justify-content: space-between;
            align-items: center;
            margin-bottom: 40px;
        }}

        h1 {{
            font-size: 32px;
            font-weight: 300;
            color: #000;
            letter-spacing: -0.5px;
        }}

        .back-btn {{
            padding: 8px 20px;
            background: #000;
            color: white;
            border: none;
            border-radius: 4px;
            cursor: pointer;
            text-decoration: none;
            font-size: 14px;
            font-weight: 500;
            transition: opacity 0.2s;
        }}

        .back-btn:hover {{
            opacity: 0.8;
        }}

        .detail-card {{
            background: white;
            border-radius: 8px;
            padding: 32px;
            border: 1px solid #e0e0e0;
            margin-bottom: 24px;
        }}

        .detail-header {{
            display: flex;
            justify-content: space-between;
            align-items: center;
            margin-bottom: 24px;
            padding-bottom: 16px;
            border-bottom: 1px solid #e0e0e0;
        }}

        .detail-header h2 {{
            font-size: 24px;
            font-weight: 300;
            color: #000;
        }}

        .actions-bar {{
            display: flex;
            gap: 12px;
        }}

        .btn {{
            padding: 8px 16px;
            border: none;
            border-radius: 4px;
            cursor: pointer;
            font-size: 14px;
            font-weight: 500;
            transition: opacity 0.2s;
        }}

        .btn-delete {{
            background: #ff3b30;
            color: white;
        }}

        .btn:hover {{
            opacity: 0.8;
        }}

        .info-grid {{
            display: grid;
            grid-template-columns: repeat(auto-fit, minmax(250px, 1fr));
            gap: 20px;
            margin-bottom: 24px;
        }}

        .info-item {{
            padding: 16px;
            background: #fafafa;
            border-radius: 4px;
            border: 1px solid #e0e0e0;
        }}

        .info-label {{
            font-size: 12px;
            text-transform: uppercase;
            letter-spacing: 0.5px;
            color: #666;
            margin-bottom: 8px;
            font-weight: 600;
        }}

        .info-value {{
            font-size: 16px;
            color: #000;
            font-weight: 500;
        }}

        .info-value code {{
            background: #f0f0f0;
            padding: 4px 8px;
            border-radius: 4px;
            font-size: 14px;
            word-break: break-all;
        }}

        .section {{
            margin-bottom: 32px;
        }}

        .section-title {{
            font-size: 18px;
            font-weight: 500;
            color: #000;
            margin-bottom: 16px;
            padding-bottom: 8px;
            border-bottom: 1px solid #e0e0e0;
        }}

        .list-item {{
            padding: 12px 16px;
            background: #fafafa;
            border-radius: 4px;
            border: 1px solid #e0e0e0;
            margin-bottom: 8px;
            font-size: 14px;
        }}

        .badge {{
            display: inline-block;
            padding: 4px 10px;
            border-radius: 3px;
            font-size: 11px;
            font-weight: 600;
            text-transform: uppercase;
            letter-spacing: 0.5px;
            margin-right: 8px;
        }}

        .badge-email {{
            background: #e3f2fd;
            color: #1565c0;
        }}

        .badge-webhook {{
            background: #f3e5f5;
            color: #6a1b9a;
        }}

        .badge-script {{
            background: #e8f5e9;
            color: #2e7d32;
        }}

        .badge-success {{
            background: #e8f5e9;
            color: #2e7d32;
        }}

        .badge-failed {{
            background: #ffebee;
            color: #c62828;
        }}

        .badge-running {{
            background: #fff3e0;
            color: #e65100;
        }}

        .action-card {{
            padding: 16px !important;
        }}

        .action-header {{
            display: flex;
            align-items: center;
            gap: 8px;
            margin-bottom: 12px;
        }}

        .action-detail {{
            margin: 8px 0 8px 32px;
            color: #333;
            line-height: 1.6;
        }}

        .action-detail strong {{
            color: #000;
            font-weight: 600;
            margin-right: 8px;
        }}

        .action-detail pre {{
            display: inline-block;
            background: #f5f5f5;
            padding: 4px 8px;
            border-radius: 3px;
            font-size: 12px;
            margin: 0;
        }}
    </style>
</head>
<body>
    <div class="container">
        <header>
            <h1>Switch Details</h1>
            <a href="/dashboard" class="back-btn">← Back to Dashboard</a>
        </header>

        <div class="detail-card">
            <div class="detail-header">
                <h2>{}</h2>
                <div class="actions-bar">
                    <button class="btn btn-delete" onclick="deleteSwitch()">Delete</button>
                </div>
            </div>

            <div class="info-grid">
                <div class="info-item">
                    <div class="info-label">Status</div>
                    <div class="info-value">{}</div>
                </div>
                <div class="info-item">
                    <div class="info-label">Timeout</div>
                    <div class="info-value">{} seconds</div>
                </div>
                <div class="info-item">
                    <div class="info-label">Last Check-in</div>
                    <div class="info-value">{}</div>
                </div>
                <div class="info-item">
                    <div class="info-label">Trigger Configuration</div>
                    <div class="info-value">{} times every {} seconds</div>
                </div>
                <div class="info-item">
                    <div class="info-label">Triggers Executed</div>
                    <div class="info-value">{} / {}</div>
                </div>
            </div>

            <div class="info-item" style="margin-bottom: 24px;">
                <div class="info-label">API Token</div>
                <div class="info-value"><code>{}</code></div>
            </div>

            <div class="info-item">
                <div class="info-label">Description</div>
                <div class="info-value">{}</div>
            </div>
        </div>

        <div class="detail-card">
            <div class="section">
                <div class="section-title">Warning Stages</div>
                {}
            </div>

            <div class="section">
                <div class="section-title">Warning Actions ({})</div>
                {}
            </div>

            <div class="section">
                <div class="section-title">Final Actions ({})</div>
                {}
            </div>

            <div class="section">
                <div class="section-title">Execution History (Last 10)</div>
                {}
            </div>
        </div>
    </div>

    <script>
        async function deleteSwitch() {{
            if (!confirm('Are you sure you want to delete this switch? This action cannot be undone.')) {{
                return;
            }}

            try {{
                const response = await fetch('/api/switches/{}', {{
                    method: 'DELETE'
                }});

                if (response.ok) {{
                    alert('Switch deleted successfully!');
                    window.location.href = '/dashboard';
                }} else {{
                    const error = await response.text();
                    alert('Error deleting switch: ' + error);
                }}
            }} catch (error) {{
                alert('Error: ' + error.message);
            }}
        }}
    </script>
</body>
</html>"#,
        escape_html(&switch.name),
        escape_html(&switch.name),
        switch.status.to_uppercase(),
        switch.timeout_seconds,
        format_relative_time(switch.last_checkin),
        if switch.trigger_count_max == 0 { "infinite".to_string() } else { switch.trigger_count_max.to_string() },
        switch.trigger_interval_seconds,
        switch.trigger_count_executed,
        if switch.trigger_count_max == 0 { "∞".to_string() } else { switch.trigger_count_max.to_string() },
        switch.api_token,
        escape_html(&switch.description.clone().unwrap_or_else(|| "No description".to_string())),
        if warning_stages.is_empty() {
            "<div class=\"list-item\">No warning stages configured</div>".to_string()
        } else {
            warning_stages
                .iter()
                .map(|s| format!("<div class=\"list-item\">{} seconds before deadline</div>", s.seconds_before_deadline))
                .collect::<Vec<_>>()
                .join("\n")
        },
        warning_actions.len(),
        if warning_actions.is_empty() {
            "<div class=\"list-item\">No warning actions configured</div>".to_string()
        } else {
            warning_actions
                .iter()
                .enumerate()
                .map(|(i, a)| {
                    let config_display = match a.action_type.as_str() {
                        "email" => {
                            if let Ok(config) = serde_json::from_str::<serde_json::Value>(&a.config) {
                                let bcc_display = if let Some(bcc_arr) = config.get("bcc").and_then(|v| v.as_array()) {
                                    bcc_arr.iter().filter_map(|v| v.as_str()).collect::<Vec<_>>().join(", ")
                                } else if let Some(to) = config.get("to").and_then(|v| v.as_str()) {
                                    to.to_string()
                                } else {
                                    "N/A".to_string()
                                };
                                format!(
                                    "<div class=\"action-detail\"><strong>Bcc:</strong> {}</div>\
                                     <div class=\"action-detail\"><strong>Subject:</strong> {}</div>\
                                     <div class=\"action-detail\"><strong>Body:</strong> {}</div>",
                                    escape_html(&bcc_display),
                                    escape_html(config.get("subject").and_then(|v| v.as_str()).unwrap_or("N/A")),
                                    escape_html(config.get("body").and_then(|v| v.as_str()).unwrap_or("N/A"))
                                )
                            } else {
                                "Invalid config".to_string()
                            }
                        }
                        "webhook" => {
                            if let Ok(config) = serde_json::from_str::<serde_json::Value>(&a.config) {
                                let headers = config.get("headers")
                                    .and_then(|h| serde_json::to_string_pretty(h).ok())
                                    .unwrap_or_else(|| "{}".to_string());
                                let body = config.get("body").and_then(|v| v.as_str()).unwrap_or("");
                                format!(
                                    "<div class=\"action-detail\"><strong>Method:</strong> {}</div>\
                                     <div class=\"action-detail\"><strong>URL:</strong> {}</div>\
                                     <div class=\"action-detail\"><strong>Headers:</strong> <pre>{}</pre></div>\
                                     <div class=\"action-detail\"><strong>Body:</strong> {}</div>",
                                    escape_html(config.get("method").and_then(|v| v.as_str()).unwrap_or("POST")),
                                    escape_html(config.get("url").and_then(|v| v.as_str()).unwrap_or("N/A")),
                                    escape_html(&headers),
                                    escape_html(body)
                                )
                            } else {
                                "Invalid config".to_string()
                            }
                        }
                        "script" => {
                            if let Ok(config) = serde_json::from_str::<serde_json::Value>(&a.config) {
                                let args = config.get("args")
                                    .and_then(|a| a.as_array())
                                    .map(|arr| arr.iter().filter_map(|v| v.as_str()).collect::<Vec<_>>().join(" "))
                                    .unwrap_or_default();
                                format!(
                                    "<div class=\"action-detail\"><strong>Path:</strong> {}</div>\
                                     <div class=\"action-detail\"><strong>Arguments:</strong> {}</div>",
                                    escape_html(config.get("script_path").and_then(|v| v.as_str()).unwrap_or("N/A")),
                                    escape_html(&args)
                                )
                            } else {
                                "Invalid config".to_string()
                            }
                        }
                        _ => "Unknown action type".to_string()
                    };
                    format!(
                        "<div class=\"list-item action-card\">\
                            <div class=\"action-header\">\
                                <span class=\"badge badge-{}\">→</span>\
                                <strong>{} - Warning Action #{}</strong>\
                            </div>\
                            {}\
                         </div>",
                        a.action_type,
                        a.action_type.to_uppercase(),
                        i + 1,
                        config_display
                    )
                })
                .collect::<Vec<_>>()
                .join("\n")
        },
        final_actions.len(),
        if final_actions.is_empty() {
            "<div class=\"list-item\">No final actions configured</div>".to_string()
        } else {
            final_actions
                .iter()
                .enumerate()
                .map(|(i, a)| {
                    let config_display = match a.action_type.as_str() {
                        "email" => {
                            if let Ok(config) = serde_json::from_str::<serde_json::Value>(&a.config) {
                                let bcc_display = if let Some(bcc_arr) = config.get("bcc").and_then(|v| v.as_array()) {
                                    bcc_arr.iter().filter_map(|v| v.as_str()).collect::<Vec<_>>().join(", ")
                                } else if let Some(to) = config.get("to").and_then(|v| v.as_str()) {
                                    to.to_string()
                                } else {
                                    "N/A".to_string()
                                };
                                format!(
                                    "<div class=\"action-detail\"><strong>Bcc:</strong> {}</div>\
                                     <div class=\"action-detail\"><strong>Subject:</strong> {}</div>\
                                     <div class=\"action-detail\"><strong>Body:</strong> {}</div>",
                                    escape_html(&bcc_display),
                                    escape_html(config.get("subject").and_then(|v| v.as_str()).unwrap_or("N/A")),
                                    escape_html(config.get("body").and_then(|v| v.as_str()).unwrap_or("N/A"))
                                )
                            } else {
                                "Invalid config".to_string()
                            }
                        }
                        "webhook" => {
                            if let Ok(config) = serde_json::from_str::<serde_json::Value>(&a.config) {
                                let headers = config.get("headers")
                                    .and_then(|h| serde_json::to_string_pretty(h).ok())
                                    .unwrap_or_else(|| "{}".to_string());
                                let body = config.get("body").and_then(|v| v.as_str()).unwrap_or("");
                                format!(
                                    "<div class=\"action-detail\"><strong>Method:</strong> {}</div>\
                                     <div class=\"action-detail\"><strong>URL:</strong> {}</div>\
                                     <div class=\"action-detail\"><strong>Headers:</strong> <pre>{}</pre></div>\
                                     <div class=\"action-detail\"><strong>Body:</strong> {}</div>",
                                    escape_html(config.get("method").and_then(|v| v.as_str()).unwrap_or("POST")),
                                    escape_html(config.get("url").and_then(|v| v.as_str()).unwrap_or("N/A")),
                                    escape_html(&headers),
                                    escape_html(body)
                                )
                            } else {
                                "Invalid config".to_string()
                            }
                        }
                        "script" => {
                            if let Ok(config) = serde_json::from_str::<serde_json::Value>(&a.config) {
                                let args = config.get("args")
                                    .and_then(|a| a.as_array())
                                    .map(|arr| arr.iter().filter_map(|v| v.as_str()).collect::<Vec<_>>().join(" "))
                                    .unwrap_or_default();
                                format!(
                                    "<div class=\"action-detail\"><strong>Path:</strong> {}</div>\
                                     <div class=\"action-detail\"><strong>Arguments:</strong> {}</div>",
                                    escape_html(config.get("script_path").and_then(|v| v.as_str()).unwrap_or("N/A")),
                                    escape_html(&args)
                                )
                            } else {
                                "Invalid config".to_string()
                            }
                        }
                        _ => "Unknown action type".to_string()
                    };
                    format!(
                        "<div class=\"list-item action-card\">\
                            <div class=\"action-header\">\
                                <span class=\"badge badge-{}\">→</span>\
                                <strong>{} - Final Action #{}</strong>\
                            </div>\
                            {}\
                         </div>",
                        a.action_type,
                        a.action_type.to_uppercase(),
                        i + 1,
                        config_display
                    )
                })
                .collect::<Vec<_>>()
                .join("\n")
        },
        if executions.is_empty() {
            "<div class=\"list-item\">No executions yet</div>".to_string()
        } else {
            executions
                .iter()
                .take(10)
                .map(|e| {
                    format!(
                        "<div class=\"list-item\"><span class=\"badge badge-{}\">→</span>{} - {}</div>",
                        e.status.to_lowercase(),
                        e.execution_type.to_uppercase(),
                        format_relative_time(e.started_at)
                    )
                })
                .collect::<Vec<_>>()
                .join("\n")
        },
        id
    );

    Html(html).into_response()
}

fn escape_html(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

fn format_relative_time(timestamp: i64) -> String {
    let now = Utc::now().timestamp();
    let diff = now - timestamp;

    format_duration(diff.abs()) + if diff < 0 { " from now" } else { " ago" }
}

fn format_duration(seconds: i64) -> String {
    if seconds < 60 {
        format!("{} seconds", seconds)
    } else if seconds < 3600 {
        format!("{} minutes", seconds / 60)
    } else if seconds < 86400 {
        format!("{} hours", seconds / 3600)
    } else {
        format!("{} days", seconds / 86400)
    }
}
