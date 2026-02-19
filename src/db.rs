use crate::models::*;
use anyhow::Result;
use sqlx::{sqlite::SqlitePoolOptions, SqlitePool};

pub async fn init_pool(database_url: &str) -> Result<SqlitePool> {
    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect(database_url)
        .await?;

    // Run migrations
    sqlx::migrate!("./migrations").run(&pool).await?;

    Ok(pool)
}

// Switch operations
pub async fn create_switch(pool: &SqlitePool, switch: &Switch) -> Result<()> {
    sqlx::query!(
        r#"
        INSERT INTO switches (id, name, description, api_token, timeout_seconds, last_checkin, last_trigger, status, created_at, trigger_count_max, trigger_interval_seconds, trigger_count_executed)
        VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        "#,
        switch.id,
        switch.name,
        switch.description,
        switch.api_token,
        switch.timeout_seconds,
        switch.last_checkin,
        switch.last_trigger,
        switch.status,
        switch.created_at,
        switch.trigger_count_max,
        switch.trigger_interval_seconds,
        switch.trigger_count_executed
    )
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn get_switch_by_id(pool: &SqlitePool, id: &str) -> Result<Option<Switch>> {
    let switch = sqlx::query_as!(
        Switch,
        r#"SELECT id as "id!", name as "name!", description, api_token as "api_token!", timeout_seconds, last_checkin, last_trigger, status as "status!", created_at, trigger_count_max, trigger_interval_seconds, trigger_count_executed FROM switches WHERE id = ?"#,
        id
    )
    .fetch_optional(pool)
    .await?;
    Ok(switch)
}

pub async fn list_switches(pool: &SqlitePool) -> Result<Vec<Switch>> {
    let switches = sqlx::query_as!(
        Switch,
        r#"SELECT id as "id!", name as "name!", description, api_token as "api_token!", timeout_seconds, last_checkin, last_trigger, status as "status!", created_at, trigger_count_max, trigger_interval_seconds, trigger_count_executed FROM switches ORDER BY created_at DESC"#
    )
    .fetch_all(pool)
    .await?;
    Ok(switches)
}

pub async fn update_checkin(pool: &SqlitePool, id: &str, timestamp: i64) -> Result<()> {
    sqlx::query!(
        r#"UPDATE switches SET last_checkin = ? WHERE id = ?"#,
        timestamp,
        id
    )
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn mark_switch_triggered(pool: &SqlitePool, id: &str, timestamp: i64) -> Result<()> {
    sqlx::query!(
        r#"UPDATE switches SET status = 'triggered', last_trigger = ?, trigger_count_executed = 1 WHERE id = ?"#,
        timestamp,
        id
    )
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn update_trigger_execution(
    pool: &SqlitePool,
    switch_id: &str,
    trigger_time: i64,
) -> Result<()> {
    sqlx::query!(
        r#"UPDATE switches SET last_trigger = ?, trigger_count_executed = trigger_count_executed + 1 WHERE id = ?"#,
        trigger_time,
        switch_id
    )
    .execute(pool)
    .await?;

    Ok(())
}

// Warning stage operations
pub async fn create_warning_stage(pool: &SqlitePool, stage: &WarningStage) -> Result<()> {
    sqlx::query!(
        r#"INSERT INTO warning_stages (switch_id, seconds_before_deadline) VALUES (?, ?)"#,
        stage.switch_id,
        stage.seconds_before_deadline
    )
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn get_warning_stages_for_switch(
    pool: &SqlitePool,
    switch_id: &str,
) -> Result<Vec<WarningStage>> {
    let stages = sqlx::query_as!(
        WarningStage,
        r#"SELECT id as "id!", switch_id as "switch_id!", seconds_before_deadline as "seconds_before_deadline!" FROM warning_stages WHERE switch_id = ? ORDER BY seconds_before_deadline ASC"#,
        switch_id
    )
    .fetch_all(pool)
    .await?;
    Ok(stages)
}

// Warning execution tracking
pub async fn record_warning_execution(
    pool: &SqlitePool,
    switch_id: &str,
    stage_seconds: i64,
    timestamp: i64,
) -> Result<()> {
    sqlx::query!(
        r#"INSERT INTO warning_executions (switch_id, stage_seconds, executed_at) VALUES (?, ?, ?)"#,
        switch_id,
        stage_seconds,
        timestamp
    )
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn was_warning_sent(
    pool: &SqlitePool,
    switch_id: &str,
    stage_seconds: i64,
) -> Result<bool> {
    let result = sqlx::query!(
        r#"SELECT COUNT(*) as count FROM warning_executions WHERE switch_id = ? AND stage_seconds = ?"#,
        switch_id,
        stage_seconds
    )
    .fetch_one(pool)
    .await?;
    Ok(result.count > 0)
}

// Action operations
pub async fn create_action(pool: &SqlitePool, action: &Action) -> Result<i64> {
    let result: sqlx::sqlite::SqliteQueryResult = sqlx::query!(
        r#"INSERT INTO actions (switch_id, action_order, action_type, is_warning, config) VALUES (?, ?, ?, ?, ?)"#,
        action.switch_id,
        action.action_order,
        action.action_type,
        action.is_warning,
        action.config
    )
    .execute(pool)
    .await?;
    Ok(result.last_insert_rowid())
}

pub async fn get_actions_for_switch(
    pool: &SqlitePool,
    switch_id: &str,
    is_warning: bool,
) -> Result<Vec<Action>> {
    let actions = sqlx::query_as!(
        Action,
        r#"SELECT id as "id!", switch_id as "switch_id!", action_order as "action_order!", action_type as "action_type!", is_warning as "is_warning!", config as "config!" FROM actions WHERE switch_id = ? AND is_warning = ? ORDER BY action_order ASC"#,
        switch_id,
        is_warning
    )
    .fetch_all(pool)
    .await?;
    Ok(actions)
}

// Action execution tracking
pub async fn create_action_execution(
    pool: &SqlitePool,
    switch_id: &str,
    action_id: i64,
    execution_type: &str,
    timestamp: i64,
) -> Result<i64> {
    let result: sqlx::sqlite::SqliteQueryResult = sqlx::query!(
        r#"INSERT INTO action_executions (switch_id, action_id, execution_type, started_at, status) VALUES (?, ?, ?, ?, 'running')"#,
        switch_id,
        action_id,
        execution_type,
        timestamp
    )
    .execute(pool)
    .await?;
    Ok(result.last_insert_rowid())
}

pub async fn complete_action_execution(
    pool: &SqlitePool,
    execution_id: i64,
    completed_at: i64,
    exit_code: Option<i64>,
    stdout: Option<String>,
    stderr: Option<String>,
    error_message: Option<String>,
) -> Result<()> {
    let status = if error_message.is_some() || (exit_code.is_some() && exit_code.unwrap() != 0) {
        "failed"
    } else {
        "completed"
    };

    sqlx::query!(
        r#"UPDATE action_executions SET completed_at = ?, status = ?, exit_code = ?, stdout = ?, stderr = ?, error_message = ? WHERE id = ?"#,
        completed_at,
        status,
        exit_code,
        stdout,
        stderr,
        error_message,
        execution_id
    )
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn mark_orphaned_executions_failed(pool: &SqlitePool) -> Result<u64> {
    let result: sqlx::sqlite::SqliteQueryResult = sqlx::query!(
        r#"UPDATE action_executions SET status = 'failed', error_message = 'Process crashed during execution' WHERE status = 'running'"#
    )
    .execute(pool)
    .await?;
    Ok(result.rows_affected())
}

pub async fn get_execution_history(
    pool: &SqlitePool,
    switch_id: &str,
) -> Result<Vec<ActionExecution>> {
    let executions = sqlx::query_as!(
        ActionExecution,
        r#"SELECT id as "id!", switch_id as "switch_id!", action_id as "action_id!", execution_type as "execution_type!", started_at as "started_at!", completed_at, status as "status!", exit_code, stdout, stderr, error_message FROM action_executions WHERE switch_id = ? ORDER BY started_at DESC LIMIT 100"#,
        switch_id
    )
    .fetch_all(pool)
    .await?;
    Ok(executions)
}

// Watchdog queries
pub async fn get_active_switches(pool: &SqlitePool) -> Result<Vec<Switch>> {
    let switches = sqlx::query_as!(
        Switch,
        r#"SELECT id as "id!", name as "name!", description, api_token as "api_token!", timeout_seconds, last_checkin, last_trigger, status as "status!", created_at, trigger_count_max, trigger_interval_seconds, trigger_count_executed FROM switches WHERE status = 'active'"#
    )
    .fetch_all(pool)
    .await?;
    Ok(switches)
}

pub async fn get_triggered_switches(pool: &SqlitePool) -> Result<Vec<Switch>> {
    let switches = sqlx::query_as!(
        Switch,
        r#"SELECT id as "id!", name as "name!", description, api_token as "api_token!", timeout_seconds, last_checkin, last_trigger, status as "status!", created_at, trigger_count_max, trigger_interval_seconds, trigger_count_executed FROM switches WHERE status = 'triggered'"#
    )
    .fetch_all(pool)
    .await?;

    Ok(switches)
}
