use std::path::{Path, PathBuf};

pub fn expand_tilde(path: &Path) -> PathBuf {
    let s = path.to_string_lossy();
    if s.starts_with("~") {
        if let Some(home) = std::env::var_os("HOME") {
            return PathBuf::from(s.replacen("~", &home.to_string_lossy(), 1));
        }
    }
    path.to_path_buf()
}
