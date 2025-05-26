use clap::Parser;
use core::util::expand_tilde;
use std::path::PathBuf;
use tracing::{error, info, warn, Level};
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

    #[arg(short = 's', long, default_value = "~/.local/state/zephyr/state.db")]
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

    if args.reset_state {
        let state_path = if let Some(ref cli_path) = args.state_path {
            cli_path.clone()
        } else if args.config.exists() {
            match config::Config::load(&args.config) {
                Ok(config) => config.general.state_path,
                Err(e) => {
                    error!("Failed to load config for state path: {}", e);
                    return Err(e);
                }
            }
        } else {
            PathBuf::from("~/.local/state/zephyr/state.db")
        };

        info!("Resetting state database at {:?}", state_path);
        let state_path = expand_tilde(&state_path);
        let state_manager = state::StateManager::new(&state_path)?;
        state_manager.reset_state()?;
        info!("State database reset successfully");
        return Ok(());
    }

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
                warn!("Configuration file not found at {:?}", args.config);
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

    let state_path = args.state_path.unwrap_or(config.general.state_path);
    let state_path = expand_tilde(&state_path);

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
