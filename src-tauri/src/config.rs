use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Config {
    pub input_device: Option<String>,
    pub output_device: Option<String>,
    pub carrier_path: Option<PathBuf>,
    pub mic_gain: f32,
    pub music_gain: f32,
    pub carrier_gain: f32,
    pub carrier_gate_rms: f32,
    pub krisp_bypass_enabled: bool,
}

impl Config {
    pub fn with_defaults() -> Self {
        Self {
            input_device: None,
            output_device: None,
            carrier_path: None,
            mic_gain: 1.0,
            music_gain: 0.8,
            carrier_gain: 0.05,
            carrier_gate_rms: 0.01,
            krisp_bypass_enabled: true,
        }
    }
    pub fn load(path: &Path) -> Self {
        std::fs::read_to_string(path)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_else(Self::with_defaults)
    }
    pub fn save(&self, path: &Path) -> std::io::Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(path, serde_json::to_string_pretty(self).unwrap())
    }
}

pub fn default_data_dir() -> PathBuf {
    dirs_path().join("WaVoic")
}

fn dirs_path() -> PathBuf {
    #[cfg(target_os = "windows")]
    {
        std::env::var_os("APPDATA")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("."))
    }
    #[cfg(not(target_os = "windows"))]
    {
        std::env::var_os("HOME")
            .map(|h| PathBuf::from(h).join(".config"))
            .unwrap_or_else(|| PathBuf::from("."))
    }
}
