// Persists user settings to a JSON file in the platform app-config directory.
// macOS: ~/Library/Application Support/pkap/settings.json
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Serialize, Deserialize, Clone, Default, Debug)]
pub struct Profile {
    pub name: String,
    pub fps: u32,
    pub quality: String,
    pub format: String,
}

#[derive(Serialize, Deserialize, Clone, Default, Debug)]
pub struct SettingsFile {
    pub fps: Option<u32>,
    pub quality: Option<String>,
    pub format: Option<String>,
    pub save_folder: Option<String>,
    pub countdown: Option<bool>,
    pub profiles: Option<Vec<Profile>>,
}

// Returns the path to settings.json inside the app config directory.
pub fn settings_path(config_dir: &PathBuf) -> PathBuf {
    config_dir.join("settings.json")
}

// Reads settings from disk. Returns defaults if the file doesn't exist yet.
pub fn load(config_dir: &PathBuf) -> SettingsFile {
    let path = settings_path(config_dir);
    if !path.exists() {
        return SettingsFile::default();
    }
    let json = std::fs::read_to_string(&path).unwrap_or_default();
    serde_json::from_str(&json).unwrap_or_default()
}

// Writes settings to disk, creating the config directory if needed.
pub fn save(config_dir: &PathBuf, settings: &SettingsFile) {
    let path = settings_path(config_dir);
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if let Ok(json) = serde_json::to_string_pretty(settings) {
        let _ = std::fs::write(&path, json);
    }
}
