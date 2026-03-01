use clap::Parser;
use std::path::PathBuf;
use tracing::{error, info, warn, Level};
use tracing_subscriber::FmtSubscriber;
use zephyr::util::{expand_tilde, log_level_from_str};

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

fn init_tracing(level: Level) {
    FmtSubscriber::builder()
        .with_max_level(level)
        .with_target(false)
        .with_thread_ids(false)
        .with_file(false)
        .with_line_number(false)
        .with_thread_names(false)
        .with_ansi(true)
        .init();
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    let config_path = expand_tilde(&args.config);

    if args.reset_state {
        init_tracing(Level::INFO);
        let state_path = if let Some(ref cli_path) = args.state_path {
            cli_path.clone()
        } else if config_path.exists() {
            match zephyr::config::Config::load(&config_path) {
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
        let state_manager = zephyr::state::StateManager::new(&state_path)?;
        state_manager.reset_state()?;
        info!("State database reset successfully");
        return Ok(());
    }

    if args.install_service {
        init_tracing(Level::INFO);
        info!("Installing service...");
        zephyr::service::install_service()?;
        return Ok(());
    }

    if args.uninstall_service {
        init_tracing(Level::INFO);
        info!("Uninstalling service...");
        zephyr::service::uninstall_service()?;
        return Ok(());
    }

    if args.start_service {
        init_tracing(Level::INFO);
        info!("Starting service...");
        zephyr::service::start_service()?;
        return Ok(());
    }

    if args.stop_service {
        init_tracing(Level::INFO);
        info!("Stopping service...");
        zephyr::service::stop_service()?;
        return Ok(());
    }

    let config = match zephyr::config::Config::load(&config_path) {
        Ok(c) => c,
        Err(e) => {
            init_tracing(Level::INFO);
            if !config_path.exists() {
                warn!("Configuration file not found at {:?}", config_path);
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

    init_tracing(log_level_from_str(&config.general.log_level));
    info!("Starting Zephyr with config: {:?}", config_path);
    info!("Loading configuration from {:?}", config_path);
    info!(
        "Successfully loaded configuration with {} commands",
        config.commands.len()
    );

    let state_path = args.state_path.unwrap_or(config.general.state_path);
    let state_path = expand_tilde(&state_path);

    info!(
        "Initializing scheduler with {} commands (min_interval_seconds: {}, max_immediate_executions: {})",
        config.commands.len(),
        config.general.min_interval_seconds,
        config.general.max_immediate_executions
    );
    let mut scheduler = zephyr::core::scheduler::Scheduler::new_with_config(
        config.commands,
        state_path,
        config.general.max_immediate_executions,
        config.general.min_interval_seconds,
    )?;

    info!("Starting Zephyr task scheduler");

    scheduler.run().await;

    Ok(())
}
