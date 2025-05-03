# zephyr

[![Build](https://github.com/ztroop/zephyr/actions/workflows/build.yml/badge.svg)](https://github.com/ztroop/zephyr/actions/workflows/build.yml)

Zephyr is a modern, lightweight task scheduler for Unix-like systems that runs as a background service. It combines the flexibility of CRON with the simplicity of interval-based scheduling, while handling system sleep and restarts gracefully. Perfect for automation tasks, backups, and periodic maintenance jobs.

## Features

- **Flexible Scheduling**: Supports both interval-based and CRON scheduling
- **Immediate Execution**: Run commands immediately on startup
- **Sleep Handling**: Automatically detects and recovers from system sleep
- **State Persistence**: Saves command history and schedules between restarts
- **Command Management**:
  - Working directory and environment variable support
  - Command timeout handling
  - Minimum interval enforcement
- **Service Integration**: Install as a system service (systemd/launchd)
- **Cross-Platform**: Works on Linux and macOS
- **TOML Configuration**: Simple, readable configuration format
- **Detailed Logging**: Comprehensive execution and error logging

## Configuration

- `name`: Unique identifier for the command
- `command`: The command to execute
- `interval_minutes`: How often to run the command (in minutes)
- `cron`: CRON expression for scheduling (e.g., "0 0 \* \* \*" for daily at midnight)
- `max_runtime_minutes`: Optional timeout for command execution
- `enabled`: Whether the command is active
- `immediate`: Whether to run the command immediately on startup
- `working_dir`: Optional working directory for the command
- `environment`: Optional environment variables for the command. Values can be either direct strings or references to existing environment variables using `$VARIABLE_NAME` syntax.

Note: You must specify either `interval_minutes` or `cron`, but not both.

Here's an example configuration using both interval and CRON scheduling:

```toml
[[commands]]
name = "backup"
command = "backup.sh"
interval_minutes = 60.0
max_runtime_minutes = 30
enabled = true
immediate = true
working_dir = "/backups"
environment = [
    ["BACKUP_DIR", "/data/backups"],
    ["COMPRESSION", "gzip"],
    ["PATH", "$PATH"]
]

[[commands]]
name = "cleanup"
command = "cleanup.sh"
cron = "0 0 * * *"  # Run daily at midnight
enabled = true
```

## Installation

```sh
git clone git@github.com:ztroop/zephyr.git && cd ./zephyr
cargo install --path .
```

## Usage

```bash
# Run with custom config file
zephyr --config /path/to/config.toml

# Run with custom state file
zephyr --state-path /path/to/state.db

# Reset state database
zephyr --reset-state

# Service management
zephyr --install-service
zephyr --uninstall-service
zephyr --start-service
zephyr --stop-service

# Show help
zephyr --help
```

#### Options

- `-c, --config <PATH>`: Path to configuration file (default: ~/.config/zephyr/scheduler.toml)
- `-s, --state-path <PATH>`: Path to state database file (default: ~/.local/state/zephyr/state.db)
- `-r, --reset-state`: Reset the state database, clearing all command history
- `-i, --install-service`: Install Zephyr as a system service
- `-u, --uninstall-service`: Remove Zephyr service
- `-S, --start-service`: Start the Zephyr service
- `-X, --stop-service`: Stop the Zephyr service

### Example Usage

1. Create a configuration file:

   ```bash
   mkdir -p ~/.config/zephyr
   touch ~/.config/zephyr/scheduler.toml
   ```

2. Edit the configuration to add your commands:

   ```bash
   nano ~/.config/zephyr/scheduler.toml
   ```

3. Run Zephyr:

   ```bash
   zephyr --config ~/.config/zephyr/scheduler.toml
   ```

4. Or install as a service:
   ```bash
   zephyr --install-service
   zephyr --start-service
   ```
