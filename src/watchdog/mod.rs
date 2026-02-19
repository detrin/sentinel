pub mod actions;
pub mod executor;

use crate::{config::Config, db};
use chrono::Utc;
use sqlx::SqlitePool;
use std::sync::Arc;
use tokio::time::{sleep, Duration};
use tracing::{error, info, warn};

/// Main watchdog loop that checks for warnings and expired switches
pub async fn run_watchdog(pool: SqlitePool, config: Arc<Config>) {
    info!("Watchdog starting...");

    // Crash recovery: Mark orphaned executions as failed
    match db::mark_orphaned_executions_failed(&pool).await {
        Ok(count) => {
            if count > 0 {
                warn!("Marked {} orphaned executions as failed (crash recovery)", count);
            } else {
                info!("No orphaned executions found");
            }
        }
        Err(e) => error!("Failed to mark orphaned executions: {}", e),
    }

    // Main loop
    loop {
        let now = Utc::now().timestamp();

        // Get all active switches
        let switches = match db::get_active_switches(&pool).await {
            Ok(switches) => switches,
            Err(e) => {
                error!("Failed to fetch active switches: {}", e);
                sleep(Duration::from_secs(10)).await;
                continue;
            }
        };

        for switch in switches {
            let time_since_checkin = now - switch.last_checkin;
            let _time_until_deadline = switch.timeout_seconds - time_since_checkin;

            // Check if switch has expired (past deadline)
            if time_since_checkin >= switch.timeout_seconds {
                info!(
                    "Switch '{}' has expired ({} seconds past deadline)",
                    switch.name,
                    time_since_checkin - switch.timeout_seconds
                );

                // Get final actions
                let final_actions = match db::get_actions_for_switch(&pool, &switch.id, false).await {
                    Ok(actions) => actions,
                    Err(e) => {
                        error!("Failed to get final actions for switch '{}': {}", switch.name, e);
                        continue;
                    }
                };

                // Execute final actions
                executor::execute_actions(&pool, config.clone(), &switch.id, final_actions, "final").await;

                // Mark switch as triggered
                if let Err(e) = db::mark_switch_triggered(&pool, &switch.id, now).await {
                    error!("Failed to mark switch as triggered: {}", e);
                }

                continue;
            }

            // Check for warning stages that need to be sent
            let warning_stages = match db::get_warning_stages_for_switch(&pool, &switch.id).await {
                Ok(stages) => stages,
                Err(e) => {
                    error!("Failed to get warning stages for switch '{}': {}", switch.name, e);
                    continue;
                }
            };

            for stage in warning_stages {
                // Calculate when this warning should be sent
                // stage.seconds_before_deadline is how many seconds before the deadline to send
                let warning_threshold = switch.timeout_seconds - stage.seconds_before_deadline;

                // Check if we've passed the warning threshold
                if time_since_checkin >= warning_threshold {
                    // Check if this warning was already sent
                    let already_sent = match db::was_warning_sent(
                        &pool,
                        &switch.id,
                        stage.seconds_before_deadline,
                    )
                    .await
                    {
                        Ok(sent) => sent,
                        Err(e) => {
                            error!("Failed to check warning status: {}", e);
                            continue;
                        }
                    };

                    if !already_sent {
                        info!(
                            "Sending warning for switch '{}' ({} seconds before deadline)",
                            switch.name, stage.seconds_before_deadline
                        );

                        // Get warning actions
                        let warning_actions = match db::get_actions_for_switch(&pool, &switch.id, true).await {
                            Ok(actions) => actions,
                            Err(e) => {
                                error!("Failed to get warning actions for switch '{}': {}", switch.name, e);
                                continue;
                            }
                        };

                        // Execute warning actions
                        executor::execute_actions(&pool, config.clone(), &switch.id, warning_actions, "warning").await;

                        // Record that warning was sent
                        if let Err(e) = db::record_warning_execution(
                            &pool,
                            &switch.id,
                            stage.seconds_before_deadline,
                            now,
                        )
                        .await
                        {
                            error!("Failed to record warning execution: {}", e);
                        }
                    }
                }
            }
        }

        // Check triggered switches for repeated triggers
        let triggered_switches = match db::get_triggered_switches(&pool).await {
            Ok(switches) => switches,
            Err(e) => {
                error!("Failed to fetch triggered switches: {}", e);
                Vec::new()
            }
        };

        for switch in triggered_switches {
            // Determine if we should trigger again
            let should_trigger = if switch.trigger_count_max == 0 {
                // Infinite mode - always check for next trigger
                true
            } else {
                // Finite mode - check if we haven't reached the max
                switch.trigger_count_executed < switch.trigger_count_max
            };

            if should_trigger {
                // Check if enough time has passed since last trigger
                if let Some(last_trigger_time) = switch.last_trigger {
                    let time_since_last_trigger = now - last_trigger_time;

                    if time_since_last_trigger >= switch.trigger_interval_seconds {
                        info!(
                            "Re-triggering switch '{}' (execution {} of {})",
                            switch.name,
                            switch.trigger_count_executed + 1,
                            if switch.trigger_count_max == 0 { "∞".to_string() } else { switch.trigger_count_max.to_string() }
                        );

                        // Get final actions
                        let final_actions = match db::get_actions_for_switch(&pool, &switch.id, false).await {
                            Ok(actions) => actions,
                            Err(e) => {
                                error!("Failed to get final actions for switch '{}': {}", switch.name, e);
                                continue;
                            }
                        };

                        // Execute final actions
                        executor::execute_actions(&pool, config.clone(), &switch.id, final_actions, "final").await;

                        // Update trigger execution count and timestamp
                        if let Err(e) = db::update_trigger_execution(&pool, &switch.id, now).await {
                            error!("Failed to update trigger execution: {}", e);
                        }
                    }
                }
            }
        }

        // Sleep for 10 seconds before next check
        sleep(Duration::from_secs(10)).await;
    }
}

