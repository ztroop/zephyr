# zephyr

Zephyr is a modern task scheduler for Unix-like systems (Linux and macOS) that runs as a background service. It tracks when commands last ran and executes them based on their specific time intervals, handling computer sleep periods gracefully.

## Features

- **Dynamic Scheduling**: Commands can be scheduled with configurable intervals
- **Immediate Execution**: Commands can be configured to run immediately on startup
- **Sleep Detection**: Automatically detects system sleep and handles missed commands
- **Persistent State**: Command execution history and schedules are saved between restarts
- **Command Management**:
  - Configurable working directories and environment variables
  - Command timeout handling
  - Minimum interval enforcement between executions
- **Service Integration**: Can be installed as a system service
- Cross-platform support (Linux and macOS)
- TOML-based configuration
- Service installation for both systemd and launchd
- Command execution with environment variables and working directories
- Timeout handling for long-running commands
- Detailed logging

## Configuration

Configuration is done via a TOML file. Here's an example:

```toml
[[commands]]
name = "backup"
command = "backup.sh"
interval_minutes = 60.0
max_runtime_minutes = 30
enabled = true
immediate = true  # Run immediately on startup
working_dir = "/backups"
environment = [
    ["BACKUP_DIR", "/data/backups"],
    ["COMPRESSION", "gzip"]
]

[[commands]]
name = "cleanup"
command = "cleanup.sh"
interval_minutes = 1440.0  # 24 hours
enabled = true
```

### Command Configuration Options

- `name`: Unique identifier for the command
- `command`: The command to execute
- `interval_minutes`: How often to run the command (in minutes)
- `cron`: CRON expression for scheduling (e.g., "0 0 \* \* \*" for daily at midnight)
- `max_runtime_minutes`: Optional timeout for command execution
- `enabled`: Whether the command is active
- `immediate`: Whether to run the command immediately on startup
- `working_dir`: Optional working directory for the command
- `environment`: Optional environment variables for the command

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
    ["COMPRESSION", "gzip"]
]

[[commands]]
name = "cleanup"
command = "cleanup.sh"
cron = "0 0 * * *"  # Run daily at midnight
enabled = true
```

## Installation

```sh
git clone git@github.com:ztroop/dead-ringer.git && cd ./dead-ringer
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
zephyr --install-service    # Install as system service
zephyr --uninstall-service  # Remove system service
zephyr --start-service      # Start the service
zephyr --stop-service       # Stop the service

# Show help
zephyr --help
```

#### Options

- `-c, --config <PATH>`: Path to configuration file (default: config/scheduler.toml)
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
   cp config/scheduler.toml ~/.config/zephyr/
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
