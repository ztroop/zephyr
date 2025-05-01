use anyhow::{Context, Result};
use std::fs;
use std::process::Command;
use users::get_current_username;

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

    Command::new("systemctl")
        .args(["daemon-reload"])
        .status()
        .context("Failed to reload systemd daemon")?;

    Command::new("systemctl")
        .args(["enable", "zephyr.service"])
        .status()
        .context("Failed to enable zephyr service")?;

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

    Command::new("launchctl")
        .args(["load", &plist_path])
        .status()
        .context("Failed to load launchd service")?;

    Ok(())
}

#[cfg(target_os = "linux")]
pub fn uninstall_service() -> Result<()> {
    Command::new("systemctl")
        .args(["stop", "zephyr.service"])
        .status()
        .context("Failed to stop zephyr service")?;

    Command::new("systemctl")
        .args(["disable", "zephyr.service"])
        .status()
        .context("Failed to disable zephyr service")?;

    fs::remove_file("/etc/systemd/system/zephyr.service")
        .context("Failed to remove systemd service file")?;

    Command::new("systemctl")
        .args(["daemon-reload"])
        .status()
        .context("Failed to reload systemd daemon")?;

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

    Command::new("launchctl")
        .args(["unload", &plist_path])
        .status()
        .context("Failed to unload launchd service")?;

    fs::remove_file(&plist_path).context("Failed to remove launchd plist file")?;

    Ok(())
}

pub fn start_service() -> Result<()> {
    #[cfg(target_os = "linux")]
    {
        Command::new("systemctl")
            .args(["start", "zephyr.service"])
            .status()
            .context("Failed to start zephyr service")?;
    }

    #[cfg(target_os = "macos")]
    {
        Command::new("launchctl")
            .args(["start", "com.zephyr.scheduler"])
            .status()
            .context("Failed to start zephyr service")?;
    }

    Ok(())
}

pub fn stop_service() -> Result<()> {
    #[cfg(target_os = "linux")]
    {
        Command::new("systemctl")
            .args(["stop", "zephyr.service"])
            .status()
            .context("Failed to stop zephyr service")?;
    }

    #[cfg(target_os = "macos")]
    {
        Command::new("launchctl")
            .args(["stop", "com.zephyr.scheduler"])
            .status()
            .context("Failed to stop zephyr service")?;
    }

    Ok(())
}
