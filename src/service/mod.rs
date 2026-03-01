use anyhow::{Context, Result};
use std::fs;
use std::process::{Command, ExitStatus};
use users::get_current_username;

fn check_status(status: std::io::Result<ExitStatus>, operation: &'static str) -> Result<()> {
    let status = status.context(operation)?;
    if !status.success() {
        anyhow::bail!("{} failed with exit code: {:?}", operation, status.code());
    }
    Ok(())
}

#[cfg(target_os = "linux")]
pub fn install_service() -> Result<()> {
    let username = get_current_username()
        .context("Failed to get current username")?
        .to_string_lossy()
        .to_string();

    let service_content = format!(
        "[Unit]
Description=Zephyr Task Scheduler
After=network.target

[Service]
Type=simple
User={}
ExecStart=/usr/local/bin/zephyr
Restart=always
RestartSec=60

[Install]
WantedBy=multi-user.target",
        username
    );

    let service_path = "/etc/systemd/system/zephyr.service";
    fs::write(service_path, service_content).context("Failed to write systemd service file")?;

    check_status(
        Command::new("systemctl").args(["daemon-reload"]).status(),
        "Failed to reload systemd daemon",
    )?;

    check_status(
        Command::new("systemctl")
            .args(["enable", "zephyr.service"])
            .status(),
        "Failed to enable zephyr service",
    )?;

    Ok(())
}

#[cfg(target_os = "macos")]
pub fn install_service() -> Result<()> {
    let username = get_current_username()
        .context("Failed to get current username")?
        .to_string_lossy()
        .to_string();

    let plist_content = format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>
<!DOCTYPE plist PUBLIC \"-//Apple//DTD PLIST 1.0//EN\" \"http://www.apple.com/DTDs/PropertyList-1.0.dtd\">
<plist version=\"1.0\">
<dict>
    <key>Label</key>
    <string>com.zephyr.scheduler</string>
    <key>ProgramArguments</key>
    <array>
        <string>/usr/local/bin/zephyr</string>
    </array>
    <key>RunAtLoad</key>
    <true/>
    <key>KeepAlive</key>
    <true/>
    <key>StandardErrorPath</key>
    <string>/Users/{}/Library/Logs/zephyr.log</string>
    <key>StandardOutPath</key>
    <string>/Users/{}/Library/Logs/zephyr.log</string>
</dict>
</plist>",
        username, username
    );

    let plist_path = format!(
        "/Users/{}/Library/LaunchAgents/com.zephyr.scheduler.plist",
        username
    );

    fs::write(&plist_path, plist_content).context("Failed to write launchd plist file")?;

    check_status(
        Command::new("launchctl")
            .args(["load", &plist_path])
            .status(),
        "Failed to load launchd service",
    )?;

    Ok(())
}

#[cfg(not(any(target_os = "linux", target_os = "macos")))]
pub fn install_service() -> Result<()> {
    anyhow::bail!("Service installation is not supported on this platform (only Linux and macOS are supported)");
}

#[cfg(not(any(target_os = "linux", target_os = "macos")))]
pub fn uninstall_service() -> Result<()> {
    anyhow::bail!("Service uninstallation is not supported on this platform (only Linux and macOS are supported)");
}

#[cfg(target_os = "linux")]
pub fn uninstall_service() -> Result<()> {
    check_status(
        Command::new("systemctl")
            .args(["stop", "zephyr.service"])
            .status(),
        "Failed to stop zephyr service",
    )?;

    check_status(
        Command::new("systemctl")
            .args(["disable", "zephyr.service"])
            .status(),
        "Failed to disable zephyr service",
    )?;

    fs::remove_file("/etc/systemd/system/zephyr.service")
        .context("Failed to remove systemd service file")?;

    check_status(
        Command::new("systemctl").args(["daemon-reload"]).status(),
        "Failed to reload systemd daemon",
    )?;

    Ok(())
}

#[cfg(target_os = "macos")]
pub fn uninstall_service() -> Result<()> {
    let username = get_current_username()
        .context("Failed to get current username")?
        .to_string_lossy()
        .to_string();

    let plist_path = format!(
        "/Users/{}/Library/LaunchAgents/com.zephyr.scheduler.plist",
        username
    );

    check_status(
        Command::new("launchctl")
            .args(["unload", &plist_path])
            .status(),
        "Failed to unload launchd service",
    )?;

    fs::remove_file(&plist_path).context("Failed to remove launchd plist file")?;

    Ok(())
}

pub fn start_service() -> Result<()> {
    #[cfg(target_os = "linux")]
    {
        check_status(
            Command::new("systemctl")
                .args(["start", "zephyr.service"])
                .status(),
            "Failed to start zephyr service",
        )?;
        Ok(())
    }

    #[cfg(target_os = "macos")]
    {
        check_status(
            Command::new("launchctl")
                .args(["start", "com.zephyr.scheduler"])
                .status(),
            "Failed to start zephyr service",
        )?;
        Ok(())
    }

    #[cfg(not(any(target_os = "linux", target_os = "macos")))]
    {
        anyhow::bail!("Service management is not supported on this platform (only Linux and macOS are supported)");
    }
}

pub fn stop_service() -> Result<()> {
    #[cfg(target_os = "linux")]
    {
        check_status(
            Command::new("systemctl")
                .args(["stop", "zephyr.service"])
                .status(),
            "Failed to stop zephyr service",
        )?;
        Ok(())
    }

    #[cfg(target_os = "macos")]
    {
        check_status(
            Command::new("launchctl")
                .args(["stop", "com.zephyr.scheduler"])
                .status(),
            "Failed to stop zephyr service",
        )?;
        Ok(())
    }

    #[cfg(not(any(target_os = "linux", target_os = "macos")))]
    {
        anyhow::bail!("Service management is not supported on this platform (only Linux and macOS are supported)");
    }
}
