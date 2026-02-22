use std::path::{Path, PathBuf};

pub fn ensure_settings_dir() -> PathBuf {
    let path = std::env::var("LOCALAPPDATA")
        .map(|dir| PathBuf::from(dir).join("WSL USB Manager"))
        .expect("LOCALAPPDATA environment variable must be set");

    let _ = std::fs::create_dir_all(&path);
    write_persistent_example(&path);
    path
}

/// Temporary example of saving some data.
fn write_persistent_example(dir: &Path) {
    use std::time::SystemTime;

    let timestamp = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map(|d| d.as_secs().to_string())
        .unwrap_or_default();
    let _ = std::fs::write(dir.join("persistent_example.txt"), timestamp);
}
