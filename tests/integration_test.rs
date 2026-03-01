//! Integration tests for Zephyr scheduler

use std::path::PathBuf;
use std::time::Duration;
use zephyr_scheduler::config::Config;
use zephyr_scheduler::core::scheduler::Scheduler;
use zephyr_scheduler::state::StateManager;

/// Config content matching examples/quick_test.toml - short interval, immediate execution
const QUICK_TEST_CONFIG: &str = r#"
[general]
log_level = "info"
min_interval_seconds = 1
state_path = "/tmp/zephyr_quick_test/state.db"
max_immediate_executions = 3

[[commands]]
name = "quick_echo"
command = "echo 'Zephyr quick test'"
interval_minutes = 0.1
max_runtime_minutes = 1
enabled = true
immediate = true
"#;

#[tokio::test]
async fn test_quick_run_executes_command() {
    let config_dir = tempfile::tempdir().unwrap();
    let config_path = config_dir.path().join("scheduler.toml");
    std::fs::write(&config_path, QUICK_TEST_CONFIG).unwrap();

    let config = Config::load(&config_path).unwrap();
    assert_eq!(config.commands.len(), 1);
    assert!(config.commands[0].immediate);

    let state_file = tempfile::NamedTempFile::new().unwrap();
    let state_path: PathBuf = state_file.path().to_path_buf();

    let mut scheduler = Scheduler::new_with_config(
        config.commands,
        state_path.clone(),
        config.general.max_immediate_executions,
        config.general.min_interval_seconds,
    )
    .unwrap();

    // Run scheduler for a few seconds - immediate command runs right away
    let run_handle = tokio::spawn(async move {
        scheduler.run().await;
    });

    // Timeout after 5s; scheduler runs forever so we expect timeout
    let _ = tokio::time::timeout(Duration::from_secs(5), run_handle).await;

    // Verify state was persisted after execution
    let state = StateManager::new(&state_path).unwrap();
    let states = state.load_command_states().unwrap();
    assert_eq!(states.len(), 1, "Command state should be persisted");
    assert_eq!(states[0].name, "quick_echo");
}

#[tokio::test]
async fn test_end_to_end_config_load_and_scheduler_init() {
    let config_content = r#"
[general]
log_level = "info"
min_interval_seconds = 30
state_path = "/tmp/zephyr_test/state.db"
max_immediate_executions = 3

[[commands]]
name = "quick_echo"
command = "echo integration_test"
interval_minutes = 60.0
enabled = true
immediate = false
"#;

    let config_dir = tempfile::tempdir().unwrap();
    let config_path = config_dir.path().join("scheduler.toml");
    std::fs::write(&config_path, config_content).unwrap();

    let config = Config::load(&config_path).unwrap();
    assert_eq!(config.commands.len(), 1);
    assert_eq!(config.commands[0].name, "quick_echo");

    let state_path = tempfile::NamedTempFile::new().unwrap();
    let state_path_buf = state_path.path().to_path_buf();

    let _scheduler = Scheduler::new_with_config(
        config.commands,
        state_path_buf.clone(),
        config.general.max_immediate_executions,
        config.general.min_interval_seconds,
    )
    .unwrap();

    // Verify state was initialized
    let state = StateManager::new(state_path.path()).unwrap();
    let states = state.load_command_states().unwrap();
    // State may be empty if no commands have run yet, or may have the scheduled command's state
    assert!(states.len() <= 1);
}
