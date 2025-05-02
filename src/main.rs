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
    #[arg(short, long, default_value = "config/scheduler.toml")]
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
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    info!("Starting Zephyr with config: {:?}", args.config);

    FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .with_target(false)
        .with_thread_ids(true)
        .with_file(true)
        .with_line_number(true)
        .with_thread_names(true)
        .pretty()
        .init();

    let state_path = args.state_path.unwrap_or_else(|| {
        let mut path = dirs::home_dir().expect("Could not find home directory");
        path.push(".local/state/zephyr");
        std::fs::create_dir_all(&path).expect("Failed to create state directory");
        path.push("state.db");
        path
    });

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
            error!("Failed to load configuration: {}", e);
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

    info!(
        "Initializing scheduler with {} commands",
        config.commands.len()
    );
    let mut scheduler = core::scheduler::Scheduler::new(config.commands, state_path);

    info!("Starting Zephyr task scheduler");

    scheduler.run().await;

    Ok(())
}
