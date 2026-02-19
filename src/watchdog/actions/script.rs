use crate::{config::SecurityConfig, models::ScriptActionConfig, watchdog::executor::ActionResult};
use std::process::Stdio;
use tokio::{process::Command, time::{timeout, Duration}};

/// Execute a script action with sandboxing
pub async fn execute(
    config_json: &str,
    security_config: &SecurityConfig,
    switch_id: &str,
    execution_type: &str,
) -> ActionResult {
    // Parse script configuration
    let script_config: ScriptActionConfig = serde_json::from_str(config_json)
        .map_err(|e| format!("Failed to parse script config: {}", e))?;

    // Build full script path
    let script_path = std::path::Path::new(&security_config.scripts_dir)
        .join(&script_config.script_path);

    // Verify script exists
    if !script_path.exists() {
        return Err(format!("Script not found: {:?}", script_path));
    }

    // Build command with timeout wrapper and sandboxing
    let mut cmd = Command::new("timeout");
    cmd.arg("--signal=KILL")
        .arg(format!("{}", security_config.script_timeout_seconds))
        .arg(&script_path);

    // Add arguments
    for arg in &script_config.args {
        cmd.arg(arg);
    }

    // Sandbox: clear environment and set only safe variables
    cmd.env_clear()
        .env("SWITCH_ID", switch_id)
        .env("EXECUTION_TYPE", execution_type)
        .env("PATH", "/usr/local/bin:/usr/bin:/bin"); // Minimal PATH

    // Set working directory to scripts dir
    cmd.current_dir(&security_config.scripts_dir);

    // Configure I/O
    cmd.stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    // Execute with timeout
    let timeout_duration = Duration::from_secs(security_config.script_timeout_seconds + 5); // +5s buffer
    let result = timeout(timeout_duration, cmd.output()).await;

    match result {
        Ok(Ok(output)) => {
            let exit_code = output.status.code().unwrap_or(-1) as i64;
            let stdout = String::from_utf8_lossy(&output.stdout).to_string();
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();

            Ok((exit_code, stdout, stderr))
        }
        Ok(Err(e)) => Err(format!("Failed to execute script: {}", e)),
        Err(_) => Err(format!(
            "Script timeout ({}s + 5s buffer)",
            security_config.script_timeout_seconds
        )),
    }
}
