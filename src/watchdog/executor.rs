use crate::{config::Config, db, models::Action, watchdog::actions};
use chrono::Utc;
use sqlx::SqlitePool;
use std::sync::Arc;
use tracing::{error, info};

/// Execute all actions for a switch (either warning or final)
/// Executes sequentially with continue-on-error semantics
pub async fn execute_actions(
    pool: &SqlitePool,
    config: Arc<Config>,
    switch_id: &str,
    actions: Vec<Action>,
    execution_type: &str,
) {
    info!(
        "Executing {} actions for switch {} ({} actions)",
        execution_type,
        switch_id,
        actions.len()
    );

    for action in actions {
        execute_single_action(pool, config.clone(), switch_id, &action, execution_type).await;
        // Continue even if action fails (continue-on-error semantics)
    }

    info!(
        "Completed {} actions for switch {}",
        execution_type, switch_id
    );
}

/// Execute a single action with full tracking
async fn execute_single_action(
    pool: &SqlitePool,
    config: Arc<Config>,
    switch_id: &str,
    action: &Action,
    execution_type: &str,
) {
    let now = Utc::now().timestamp();

    // Create execution record
    let execution_id = match db::create_action_execution(
        pool,
        switch_id,
        action.id,
        execution_type,
        now,
    )
    .await
    {
        Ok(id) => id,
        Err(e) => {
            error!("Failed to create execution record: {}", e);
            return;
        }
    };

    info!(
        "Executing action {} (type: {}, execution_id: {})",
        action.id, action.action_type, execution_id
    );

    // Execute the action based on type
    let result = match action.action_type.as_str() {
        "email" => actions::email::execute(&action.config, &config.smtp).await,
        "webhook" => actions::webhook::execute(&action.config).await,
        "script" => {
            actions::script::execute(&action.config, &config.security, switch_id, execution_type)
                .await
        }
        _ => Err(format!("Unknown action type: {}", action.action_type)),
    };

    // Process result and complete execution record
    let completed_at = Utc::now().timestamp();

    match result {
        Ok((exit_code, stdout, stderr)) => {
            if let Err(e) = db::complete_action_execution(
                pool,
                execution_id,
                completed_at,
                Some(exit_code),
                Some(stdout.clone()),
                Some(stderr.clone()),
                None,
            )
            .await
            {
                error!("Failed to update execution record: {}", e);
            }

            if exit_code == 0 {
                info!(
                    "Action {} completed successfully (execution_id: {})",
                    action.id, execution_id
                );
            } else {
                error!(
                    "Action {} failed with exit code {} (execution_id: {})",
                    action.id, exit_code, execution_id
                );
            }
        }
        Err(error_message) => {
            error!(
                "Action {} failed: {} (execution_id: {})",
                action.id, error_message, execution_id
            );

            if let Err(e) = db::complete_action_execution(
                pool,
                execution_id,
                completed_at,
                None,
                None,
                None,
                Some(error_message),
            )
            .await
            {
                error!("Failed to update execution record: {}", e);
            }
        }
    }
}

/// Action execution result: (exit_code, stdout, stderr)
pub type ActionResult = Result<(i64, String, String), String>;
