#![allow(dead_code)]

use crate::config::CommandConfig;
use anyhow::Result;
use chrono::{DateTime, Utc};
use rusqlite::{params, Connection, OptionalExtension};
use std::path::Path;

/// Represents the last execution time and next scheduled time for a command
#[derive(Debug)]
pub struct CommandState {
    pub name: String,
    pub last_execution: Option<DateTime<Utc>>,
    pub next_scheduled: DateTime<Utc>,
}

/// Manages persistent state for the scheduler
pub struct StateManager {
    conn: Connection,
}

impl StateManager {
    /// Creates a new state manager, initializing the database if needed
    pub fn new<P: AsRef<Path>>(path: P) -> Result<Self> {
        let conn = Connection::open(path)?;
        Self::init_db(&conn)?;
        Ok(Self { conn })
    }

    /// Initializes the database schema
    fn init_db(conn: &Connection) -> Result<()> {
        conn.execute(
            "CREATE TABLE IF NOT EXISTS commands (
                name TEXT PRIMARY KEY,
                last_execution TEXT,
                next_scheduled TEXT NOT NULL,
                schedule_type TEXT NOT NULL,
                schedule_data TEXT NOT NULL
            )",
            [],
        )?;
        Ok(())
    }

    /// Loads the state for all commands
    pub fn load_command_states(&self) -> Result<Vec<CommandState>> {
        let mut stmt = self
            .conn
            .prepare("SELECT name, last_execution, next_scheduled FROM commands")?;
        let states = stmt
            .query_map([], |row| {
                Ok(CommandState {
                    name: row.get(0)?,
                    last_execution: row.get::<_, Option<String>>(1)?.map(|s| s.parse().unwrap()),
                    next_scheduled: row.get::<_, String>(2)?.parse().unwrap(),
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(states)
    }

    /// Saves the state for a command
    pub fn save_command_state(
        &self,
        command: &CommandConfig,
        last_execution: Option<DateTime<Utc>>,
        next_scheduled: DateTime<Utc>,
    ) -> Result<()> {
        let (schedule_type, schedule_data) = if let Some(interval) = command.interval_minutes {
            ("interval", interval.to_string())
        } else if let Some(cron) = &command.cron {
            ("cron", cron.clone())
        } else {
            return Err(anyhow::anyhow!(
                "Command '{}' has no schedule type",
                command.name
            ));
        };

        self.conn.execute(
            "INSERT OR REPLACE INTO commands
            (name, last_execution, next_scheduled, schedule_type, schedule_data)
            VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                command.name,
                last_execution.map(|dt| dt.to_rfc3339()),
                next_scheduled.to_rfc3339(),
                schedule_type,
                schedule_data,
            ],
        )?;
        Ok(())
    }

    /// Gets the state for a specific command
    pub fn get_command_state(&self, name: &str) -> Result<Option<CommandState>> {
        self.conn
            .query_row(
                "SELECT name, last_execution, next_scheduled FROM commands WHERE name = ?1",
                [name],
                |row| {
                    Ok(CommandState {
                        name: row.get(0)?,
                        last_execution: row
                            .get::<_, Option<String>>(1)?
                            .map(|s| s.parse().unwrap()),
                        next_scheduled: row.get::<_, String>(2)?.parse().unwrap(),
                    })
                },
            )
            .optional()
            .map_err(Into::into)
    }

    /// Deletes the state for a specific command
    pub fn delete_command_state(&self, name: &str) -> Result<()> {
        self.conn
            .execute("DELETE FROM commands WHERE name = ?1", [name])?;
        Ok(())
    }

    /// Resets the entire state database by dropping and recreating the table
    pub fn reset_state(&self) -> Result<()> {
        self.conn.execute("DROP TABLE IF EXISTS commands", [])?;
        Self::init_db(&self.conn)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;

    fn create_test_command(name: &str, interval: f64) -> CommandConfig {
        CommandConfig {
            name: name.to_string(),
            command: "echo test".to_string(),
            interval_minutes: Some(interval),
            cron: None,
            max_runtime_minutes: Some(5),
            enabled: true,
            working_dir: None,
            environment: None,
            immediate: false,
        }
    }

    #[test]
    fn test_state_persistence() -> Result<()> {
        let temp_file = NamedTempFile::new()?;
        let state = StateManager::new(temp_file.path())?;

        let command = create_test_command("test", 5.0);
        let now = Utc::now();
        let next_run = now + chrono::Duration::minutes(5);

        // Save state
        state.save_command_state(&command, Some(now), next_run)?;

        // Load state
        let loaded = state.get_command_state("test")?.unwrap();
        assert_eq!(loaded.name, "test");
        assert!(loaded.last_execution.unwrap().timestamp() - now.timestamp() < 1);
        assert!(loaded.next_scheduled.timestamp() - next_run.timestamp() < 1);

        // Delete state
        state.delete_command_state("test")?;
        assert!(state.get_command_state("test")?.is_none());

        Ok(())
    }
}
