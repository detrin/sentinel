-- Main switches table
CREATE TABLE switches (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    description TEXT,
    api_token TEXT NOT NULL UNIQUE,
    timeout_seconds INTEGER NOT NULL,
    last_checkin INTEGER NOT NULL,
    last_trigger INTEGER,
    status TEXT DEFAULT 'active',  -- active/triggered/paused
    created_at INTEGER NOT NULL,
    trigger_count_max INTEGER NOT NULL DEFAULT 1,
    trigger_interval_seconds INTEGER NOT NULL DEFAULT 300,
    trigger_count_executed INTEGER NOT NULL DEFAULT 0
);

-- Warning stages (e.g., 5 days, 6 days, 6.5 days before 7 day deadline)
CREATE TABLE warning_stages (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    switch_id TEXT NOT NULL,
    seconds_before_deadline INTEGER NOT NULL,
    FOREIGN KEY (switch_id) REFERENCES switches(id) ON DELETE CASCADE
);

-- Warning execution tracking
CREATE TABLE warning_executions (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    switch_id TEXT NOT NULL,
    stage_seconds INTEGER NOT NULL,
    executed_at INTEGER NOT NULL,
    FOREIGN KEY (switch_id) REFERENCES switches(id) ON DELETE CASCADE
);

-- Actions to execute (warnings or final)
CREATE TABLE actions (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    switch_id TEXT NOT NULL,
    action_order INTEGER NOT NULL,
    action_type TEXT NOT NULL,  -- email/webhook/script
    is_warning BOOLEAN NOT NULL DEFAULT 0,  -- warning action vs final action
    config TEXT NOT NULL,  -- JSON config specific to action type
    FOREIGN KEY (switch_id) REFERENCES switches(id) ON DELETE CASCADE
);

-- Execution history (complete audit trail)
CREATE TABLE action_executions (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    switch_id TEXT NOT NULL,
    action_id INTEGER NOT NULL,
    execution_type TEXT NOT NULL,  -- warning/final
    started_at INTEGER NOT NULL,
    completed_at INTEGER,
    status TEXT NOT NULL DEFAULT 'running',  -- running/completed/failed
    exit_code INTEGER,
    stdout TEXT,
    stderr TEXT,
    error_message TEXT,
    FOREIGN KEY (switch_id) REFERENCES switches(id) ON DELETE CASCADE,
    FOREIGN KEY (action_id) REFERENCES actions(id) ON DELETE CASCADE
);

-- Create indexes for performance
CREATE INDEX idx_switches_status ON switches(status);
CREATE INDEX idx_warning_stages_switch ON warning_stages(switch_id);
CREATE INDEX idx_warning_executions_switch ON warning_executions(switch_id);
CREATE INDEX idx_actions_switch ON actions(switch_id);
CREATE INDEX idx_action_executions_switch ON action_executions(switch_id);
CREATE INDEX idx_action_executions_status ON action_executions(status);
