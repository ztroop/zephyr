use clap::Parser;
use std::path::PathBuf;
use tracing::{error, info, Level};
use tracing_subscriber::FmtSubscriber;

mod config;
mod core;
mod service;
mod state;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(short, long, default_value = "~/.config/zephyr/scheduler.toml")]
    config: PathBuf,

    #[arg(short = 'i', long)]
    install_service: bool,

    #[arg(short = 'u', long)]
    uninstall_service: bool,

    #[arg(short = 'S', long)]
    start_service: bool,

    #[arg(short = 'X', long)]
    stop_service: bool,

    #[arg(short = 's', long)]
    state_path: Option<PathBuf>,

    #[arg(short = 'r', long)]
    reset_state: bool,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    info!("Starting Zephyr with config: {:?}", args.config);

    FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .with_target(false)
        .with_thread_ids(false)
        .with_file(false)
        .with_line_number(false)
        .with_thread_names(false)
        .with_ansi(true)
        .init();

    info!("Loading configuration from {:?}", args.config);
    let config = match config::Config::load(&args.config) {
        Ok(c) => {
            info!(
                "Successfully loaded configuration with {} commands",
                c.commands.len()
            );
            c
        }
        Err(e) => {
            if !args.config.exists() {
                error!(
                    "Configuration file not found at {:?}\n\n\
                    To get started with Zephyr:\n\
                    1. Create a configuration file at {:?}\n\
                    2. Add your scheduled commands to the file\n\
                    3. Run Zephyr again\n\n\
                    Example configuration:\n\
                    ```toml\n\
                    [general]\n\
                    log_level = \"info\"\n\
                    min_interval_seconds = 30\n\
                    state_path = \"~/.local/state/zephyr/state.db\"\n\
                    max_immediate_executions = 10\n\n\
                    [[commands]]\n\
                    name = \"backup\"\n\
                    command = \"backup.sh\"\n\
                    interval_minutes = 60.0\n\
                    max_runtime_minutes = 30\n\
                    enabled = true\n\
                    immediate = true\n\
                    ```\n\n\
                    For more examples, see the README at https://github.com/ztroop/zephyr",
                    args.config, args.config
                );
            } else {
                error!("Failed to load configuration: {}", e);
                if let Some(io_error) = e.downcast_ref::<std::io::Error>() {
                    error!("IO error: {}", io_error);
                } else {
                    error!("Configuration error: {}", e);
                }
            }
            return Err(e);
        }
    };

    if args.install_service {
        info!("Installing service...");
        service::install_service()?;
        return Ok(());
    }

    if args.uninstall_service {
        info!("Uninstalling service...");
        service::uninstall_service()?;
        return Ok(());
    }

    if args.start_service {
        info!("Starting service...");
        service::start_service()?;
        return Ok(());
    }

    if args.stop_service {
        info!("Stopping service...");
        service::stop_service()?;
        return Ok(());
    }

    let state_path = args.state_path.unwrap_or(config.general.state_path);

    if args.reset_state {
        info!("Resetting state database at {:?}", state_path);
        let state_manager = state::StateManager::new(&state_path)?;
        state_manager.reset_state()?;
        info!("State database reset successfully");
        return Ok(());
    }

    info!(
        "Initializing scheduler with {} commands (min_interval_seconds: {}, max_immediate_executions: {})",
        config.commands.len(),
        config.general.min_interval_seconds,
        config.general.max_immediate_executions
    );
    let mut scheduler = core::scheduler::Scheduler::new_with_config(
        config.commands,
        state_path,
        config.general.max_immediate_executions,
        config.general.min_interval_seconds,
    );

    info!("Starting Zephyr task scheduler");

    scheduler.run().await;

    Ok(())
}
