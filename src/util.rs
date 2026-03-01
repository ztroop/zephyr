use std::path::{Path, PathBuf};

use tracing::Level;

/// Maps a log level string (e.g. "info", "debug") to tracing::Level.
/// Returns Level::INFO for unknown values.
pub fn log_level_from_str(s: &str) -> Level {
    match s.to_lowercase().as_str() {
        "trace" => Level::TRACE,
        "debug" => Level::DEBUG,
        "info" => Level::INFO,
        "warn" | "warning" => Level::WARN,
        "error" => Level::ERROR,
        _ => Level::INFO,
    }
}

pub fn expand_tilde(path: &Path) -> PathBuf {
    let s = path.to_string_lossy();
    if s.starts_with('~') {
        let home = std::env::var_os("HOME")
            .map(|p| p.to_string_lossy().to_string())
            .or_else(|| dirs::home_dir().map(|p| p.to_string_lossy().to_string()));
        if let Some(home) = home {
            return PathBuf::from(s.replacen("~", &home, 1));
        }
    }
    path.to_path_buf()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_expand_tilde_with_home() {
        let home = std::env::var_os("HOME").expect("HOME must be set in test environment");
        let home_str = home.to_string_lossy().to_string();

        let path = PathBuf::from("~");
        let expanded = expand_tilde(&path);
        assert_eq!(expanded.to_string_lossy(), home_str);

        let path = PathBuf::from("~/foo/bar");
        let expanded = expand_tilde(&path);
        assert_eq!(expanded.to_string_lossy(), format!("{}/foo/bar", home_str));
    }

    #[test]
    fn test_expand_tilde_non_tilde_path() {
        let path = PathBuf::from("/foo/bar/baz");
        let expanded = expand_tilde(&path);
        assert_eq!(expanded.to_string_lossy(), "/foo/bar/baz");
    }

    #[test]
    fn test_expand_tilde_relative_path() {
        let path = PathBuf::from("foo/bar");
        let expanded = expand_tilde(&path);
        assert_eq!(expanded.to_string_lossy(), "foo/bar");
    }

    #[test]
    fn test_log_level_from_str() {
        use super::log_level_from_str;
        use tracing::Level;
        assert_eq!(log_level_from_str("trace"), Level::TRACE);
        assert_eq!(log_level_from_str("debug"), Level::DEBUG);
        assert_eq!(log_level_from_str("info"), Level::INFO);
        assert_eq!(log_level_from_str("warn"), Level::WARN);
        assert_eq!(log_level_from_str("warning"), Level::WARN);
        assert_eq!(log_level_from_str("error"), Level::ERROR);
        assert_eq!(log_level_from_str("unknown"), Level::INFO);
    }
}
