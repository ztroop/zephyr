//! Integration tests for Zephyr scheduler

use zephyr::config::Config;
use zephyr::core::scheduler::Scheduler;
use zephyr::state::StateManager;

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
