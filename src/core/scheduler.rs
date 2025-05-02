use crate::config::CommandConfig;
use crate::core::executor::{CommandExecutor, DefaultExecutor};
use crate::state::StateManager;
use chrono::{DateTime, Duration, Utc};
use cron::Schedule;
use std::cmp::Ordering;
use std::collections::BinaryHeap;
use std::path::PathBuf;
use std::str::FromStr;
use std::time::Duration as StdDuration;
use tokio::time::{sleep, timeout};
use tracing::{error, info, warn};

/// Represents a command that is scheduled to run at a specific time
///
/// This struct combines a command configuration with its next scheduled execution time.
/// It implements ordering traits to allow commands to be sorted by their next run time.
#[derive(Debug)]
struct ScheduledCommand {
    command: CommandConfig,
    next_run: DateTime<Utc>,
}

impl PartialEq for ScheduledCommand {
    fn eq(&self, other: &Self) -> bool {
        self.next_run == other.next_run
    }
}

impl Eq for ScheduledCommand {}

impl PartialOrd for ScheduledCommand {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for ScheduledCommand {
    fn cmp(&self, other: &Self) -> Ordering {
        other.next_run.cmp(&self.next_run)
    }
}

/// Manages the scheduling and execution of commands
///
/// The scheduler maintains a priority queue of commands sorted by their next execution time.
/// It handles immediate execution of commands, enforces minimum intervals between executions,
/// and manages system sleep events to ensure commands are executed as expected.
pub struct Scheduler {
    commands: BinaryHeap<ScheduledCommand>,
    executor: Box<dyn CommandExecutor + Send + Sync>,
    min_interval_seconds: u64,
    last_execution_time: Option<DateTime<Utc>>,
    last_wake_time: Option<DateTime<Utc>>,
    state_manager: StateManager,
}

impl Scheduler {
    /// Creates a new scheduler with the given commands
    ///
    /// Initializes the scheduler with a set of commands, setting up their initial schedules.
    /// Commands marked as immediate will be executed right away, while others will be
    /// scheduled for their first run based on their interval.
    ///
    /// # Arguments
    ///
    /// * `commands` - A vector of command configurations to be scheduled
    pub fn new(commands: Vec<CommandConfig>, state_path: PathBuf) -> Self {
        let state_path_for_manager = state_path.clone();

        let state_manager =
            StateManager::new(state_path_for_manager).expect("Failed to initialize state manager");

        let existing_states = state_manager.load_command_states().unwrap_or_default();
        let mut state_map = existing_states
            .into_iter()
            .map(|state| (state.name.clone(), state))
            .collect::<std::collections::HashMap<_, _>>();

        let mut scheduler = Scheduler {
            commands: BinaryHeap::new(),
            executor: Box::new(DefaultExecutor),
            min_interval_seconds: 30,
            last_execution_time: None,
            last_wake_time: Some(Utc::now()),
            state_manager,
        };

        info!("Scheduling {} commands", commands.len());
        for command in commands {
            if command.enabled {
                info!("Scheduling command: {}", command.name);
                command.validate().expect("Invalid command configuration");
                let next_run = if let Some(state) = state_map.remove(&command.name) {
                    info!("Found existing state for command '{}'", command.name);
                    state.next_scheduled
                } else {
                    if command.immediate {
                        info!("Command '{}' will run immediately", command.name);
                        let state_path_clone = state_path.clone();
                        let command_clone = command.clone();
                        tokio::spawn(async move {
                            let mut temp_scheduler =
                                Scheduler::new(vec![command_clone.clone()], state_path_clone);
                            temp_scheduler.execute_command(command_clone).await;
                        });
                    }
                    Self::calculate_next_run(&command)
                };

                scheduler
                    .commands
                    .push(ScheduledCommand { command, next_run });
            }
        }

        scheduler
    }

    /// Calculates the next run time for a command based on its schedule type
    fn calculate_next_run(command: &CommandConfig) -> DateTime<Utc> {
        let now = Utc::now();
        if let Some(interval) = command.interval_minutes {
            now + Duration::minutes(interval as i64)
        } else if let Some(cron) = &command.cron {
            let schedule = Schedule::from_str(cron).expect("Invalid cron expression");
            schedule
                .upcoming(Utc)
                .next()
                .expect("Failed to calculate next cron run")
        } else {
            panic!("Command has no schedule type");
        }
    }

    /// Schedules the next run of a command based on its schedule type
    fn schedule_next_run(&mut self, command: CommandConfig) -> DateTime<Utc> {
        let next_run = Self::calculate_next_run(&command);

        let interval_display = if let Some(interval) = command.interval_minutes {
            if interval < 1.0 {
                format!("{:.1} seconds", interval * 60.0)
            } else if interval < 60.0 {
                format!("{:.1} minutes", interval)
            } else {
                format!("{:.1} hours", interval / 60.0)
            }
        } else if let Some(cron) = &command.cron {
            format!("cron: {}", cron)
        } else {
            "unknown".to_string()
        };

        info!(
            "Command '{}' next scheduled for {} (in {})",
            command.name, next_run, interval_display
        );

        self.commands.push(ScheduledCommand { command, next_run });
        next_run
    }

    /// Detects and handles system sleep events
    ///
    /// This method checks if the system has been asleep for an extended period (more than 5 minutes)
    /// and executes any commands that were scheduled to run during that time. It maintains the
    /// regular schedule for future executions.
    ///
    /// # Examples
    ///
    /// ```
    /// let mut scheduler = Scheduler::new(commands);
    /// scheduler.handle_sleep_resume().await;
    /// ```
    pub async fn handle_sleep_resume(&mut self) {
        let now = Utc::now();

        if let Some(last_wake) = self.last_wake_time {
            let time_since_last_wake = now.signed_duration_since(last_wake);

            let was_sleeping = time_since_last_wake.num_minutes() > 5
                && (self.last_execution_time.is_none()
                    || now
                        .signed_duration_since(self.last_execution_time.unwrap())
                        .num_minutes()
                        > 5);

            if was_sleeping {
                info!(
                    "Detected system sleep of {} minutes",
                    time_since_last_wake.num_minutes()
                );

                let current_commands = std::mem::take(&mut self.commands);
                let command_list: Vec<_> = current_commands.into_iter().collect();

                let (missed_commands, future_commands): (Vec<_>, Vec<_>) = command_list
                    .into_iter()
                    .partition(|scheduled| scheduled.next_run < now);

                for scheduled in future_commands {
                    self.commands.push(scheduled);
                }

                let missed_count = missed_commands.len();
                if missed_count > 0 {
                    info!(
                        "Found {} commands that should have run during sleep",
                        missed_count
                    );

                    let max_immediate_executions = 10; // Configurable limit
                    let (immediate_executions, reschedule_rest) =
                        if missed_commands.len() > max_immediate_executions {
                            missed_commands.split_at(max_immediate_executions)
                        } else {
                            (missed_commands.as_slice(), &[][..])
                        };

                    for scheduled in immediate_executions {
                        info!(
                            "Executing missed command: {} (originally scheduled for {})",
                            scheduled.command.name, scheduled.next_run
                        );
                        self.execute_command(scheduled.command.clone()).await;
                    }

                    for scheduled in reschedule_rest {
                        info!(
                            "Rescheduling missed command without execution: {} (was scheduled for {})",
                            scheduled.command.name, scheduled.next_run
                        );
                        self.schedule_next_run(scheduled.command.clone());
                    }
                }
            }
        }

        // Update the last wake time
        self.last_wake_time = Some(now);
    }

    /// Runs the scheduler loop, executing commands at their scheduled times
    pub async fn run(&mut self) {
        info!("Starting scheduler loop");
        loop {
            self.handle_sleep_resume().await;

            if self.commands.is_empty() {
                info!("No commands scheduled, sleeping for 60 seconds");
                sleep(StdDuration::from_secs(60)).await;
                continue;
            }

            let now = Utc::now();

            if let Some(last_time) = self.last_execution_time {
                let time_since_last = now.signed_duration_since(last_time);
                let min_interval_millis = (self.min_interval_seconds * 1000) as i64;

                if time_since_last.num_milliseconds() < min_interval_millis {
                    let wait_millis = min_interval_millis - time_since_last.num_milliseconds();
                    let wait_duration = StdDuration::from_millis(wait_millis as u64);
                    info!(
                        "Enforcing minimum interval: waiting for {} milliseconds before next execution",
                        wait_millis
                    );
                    sleep(wait_duration).await;
                    continue;
                }
            }

            if let Some(scheduled) = self.commands.peek() {
                let time_until_next = scheduled.next_run.signed_duration_since(now);

                if time_until_next.num_milliseconds() <= 0 {
                    if let Some(command_to_run) = self.commands.pop() {
                        let cmd_name = command_to_run.command.name.clone();
                        info!("Executing command: {}", cmd_name);
                        self.last_execution_time = Some(Utc::now());

                        let execution_timeout = StdDuration::from_secs(300);
                        match timeout(
                            execution_timeout,
                            self.execute_command(command_to_run.command.clone()),
                        )
                        .await
                        {
                            Ok(_) => {
                                info!("Command '{}' execution completed within timeout", cmd_name);
                            }
                            Err(_) => {
                                warn!(
                                    "Command '{}' execution timed out after {:?}",
                                    cmd_name, execution_timeout
                                );
                                self.schedule_next_run(command_to_run.command);
                            }
                        }
                    }
                } else {
                    let sleep_time_secs = std::cmp::max(time_until_next.num_seconds(), 1) as u64;
                    let sleep_time_secs = std::cmp::min(sleep_time_secs, 3600);
                    info!(
                        "Sleeping for {} seconds until next command",
                        sleep_time_secs
                    );
                    sleep(StdDuration::from_secs(sleep_time_secs)).await;
                }
            } else {
                warn!("Command queue unexpectedly empty, sleeping for 1 second");
                sleep(StdDuration::from_secs(1)).await;
            }
        }
    }

    /// Executes a command and handles its output
    async fn execute_command(&mut self, command: CommandConfig) {
        let execution_start = Utc::now();

        match self.executor.execute(&command).await {
            Ok(output) => {
                info!("Command '{}' completed successfully", command.name);
                if !output.stdout.is_empty() {
                    info!("Output: {}", String::from_utf8_lossy(&output.stdout));
                }
                if !output.stderr.is_empty() {
                    error!("Error output: {}", String::from_utf8_lossy(&output.stderr));
                }
            }
            Err(e) => {
                error!("Failed to execute command '{}': {}", command.name, e);
            }
        }

        let execution_duration = Utc::now().signed_duration_since(execution_start);
        info!(
            "Command '{}' execution took {} milliseconds",
            command.name,
            execution_duration.num_milliseconds()
        );

        // Save state after execution
        let next_run = self.schedule_next_run(command.clone());
        if let Err(e) =
            self.state_manager
                .save_command_state(&command, Some(execution_start), next_run)
        {
            error!("Failed to save state for command '{}': {}", command.name, e);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use tempfile::NamedTempFile;

    fn create_test_command(name: &str, interval_minutes: f64) -> CommandConfig {
        CommandConfig {
            name: name.to_string(),
            command: "echo test".to_string(),
            interval_minutes: Some(interval_minutes),
            cron: None,
            max_runtime_minutes: Some(5),
            enabled: true,
            working_dir: None,
            environment: None,
            immediate: false,
        }
    }

    fn create_test_cron_command(name: &str, cron: &str) -> CommandConfig {
        CommandConfig {
            name: name.to_string(),
            command: "echo test".to_string(),
            interval_minutes: None,
            cron: Some(cron.to_string()),
            max_runtime_minutes: Some(5),
            enabled: true,
            working_dir: None,
            environment: None,
            immediate: false,
        }
    }

    fn create_temp_state_path() -> PathBuf {
        let temp_file = NamedTempFile::new().unwrap();
        temp_file.path().to_path_buf()
    }

    #[tokio::test]
    async fn test_scheduler_initialization() {
        let commands = vec![
            create_test_command("test1", 1.0),
            create_test_command("test2", 2.0),
        ];
        let scheduler = Scheduler::new(commands.clone(), create_temp_state_path());

        assert_eq!(scheduler.commands.len(), 2);
        assert!(scheduler.last_execution_time.is_none());
    }

    #[tokio::test]
    async fn test_cron_scheduler_initialization() {
        let commands = vec![
            create_test_cron_command("test1", "0 0 * * * *"), // Every hour
            create_test_cron_command("test2", "0 0 0 * * *"), // Daily at midnight
        ];
        let scheduler = Scheduler::new(commands.clone(), create_temp_state_path());

        assert_eq!(scheduler.commands.len(), 2);
        assert!(scheduler.last_execution_time.is_none());
    }

    #[tokio::test]
    async fn test_schedule_next_run() {
        let mut scheduler = Scheduler::new(vec![], create_temp_state_path());
        let command = create_test_command("test", 1.0);

        let _next_run = scheduler.schedule_next_run(command.clone());
        assert_eq!(scheduler.commands.len(), 1);

        let scheduled = scheduler.commands.peek().unwrap();
        assert_eq!(scheduled.command.name, "test");
        assert!(scheduled.next_run > Utc::now());
    }

    #[tokio::test]
    async fn test_cron_schedule_next_run() {
        let mut scheduler = Scheduler::new(vec![], create_temp_state_path());
        let command = create_test_cron_command("test", "0 0 * * * *"); // Every hour

        let _next_run = scheduler.schedule_next_run(command.clone());
        assert_eq!(scheduler.commands.len(), 1);

        let scheduled = scheduler.commands.peek().unwrap();
        assert_eq!(scheduled.command.name, "test");
        assert!(scheduled.next_run > Utc::now());

        let time_str = scheduled.next_run.format("%H:%M:%S").to_string();
        assert_eq!(time_str.split(':').nth(1).unwrap(), "00");
        assert_eq!(time_str.split(':').nth(2).unwrap(), "00");
    }

    #[tokio::test]
    async fn test_command_ordering() {
        let mut scheduler = Scheduler::new(vec![], create_temp_state_path());
        let command1 = create_test_command("test1", 1.0);
        let command2 = create_test_command("test2", 2.0);

        scheduler.schedule_next_run(command1);
        scheduler.schedule_next_run(command2);

        let first = scheduler.commands.pop().unwrap();
        let second = scheduler.commands.pop().unwrap();

        assert!(first.next_run < second.next_run);
    }

    #[tokio::test]
    async fn test_cron_command_ordering() {
        let mut scheduler = Scheduler::new(vec![], create_temp_state_path());
        let command1 = create_test_cron_command("test1", "0 0 * * * *"); // Every hour
        let command2 = create_test_cron_command("test2", "0 0 0 * * *"); // Daily at midnight

        scheduler.schedule_next_run(command1);
        scheduler.schedule_next_run(command2);

        let first = scheduler.commands.pop().unwrap();
        let second = scheduler.commands.pop().unwrap();

        assert!(first.next_run < second.next_run);
    }

    #[tokio::test]
    async fn test_mixed_command_ordering() {
        let mut scheduler = Scheduler::new(vec![], create_temp_state_path());
        let command1 = create_test_command("test1", 1.0);
        let command2 = create_test_cron_command("test2", "0 0 * * * *"); // Every hour

        scheduler.schedule_next_run(command1);
        scheduler.schedule_next_run(command2);

        let first = scheduler.commands.pop().unwrap();
        let second = scheduler.commands.pop().unwrap();

        assert!(first.next_run < second.next_run);
    }

    #[tokio::test]
    async fn test_disabled_commands() {
        let mut commands = vec![
            create_test_command("enabled", 1.0),
            create_test_command("disabled", 1.0),
        ];
        commands[1].enabled = false;

        let scheduler = Scheduler::new(commands, create_temp_state_path());
        assert_eq!(scheduler.commands.len(), 1);
        assert_eq!(scheduler.commands.peek().unwrap().command.name, "enabled");
    }

    #[tokio::test]
    async fn test_immediate_execution() {
        let mut commands = vec![
            create_test_command("normal", 1.0),
            create_test_command("immediate", 1.0),
        ];
        commands[1].immediate = true;

        let scheduler = Scheduler::new(commands, create_temp_state_path());
        assert_eq!(scheduler.commands.len(), 2);
    }
}
