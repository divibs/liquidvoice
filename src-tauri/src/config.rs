use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AppConfig {
    pub api_key: String,
    pub model: String,
    pub hotkey: String,
    pub trigger_mode: TriggerMode,
    pub language: String,
    pub prompt: String,
    pub theme: String,
    pub max_recording_sec: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum TriggerMode {
    PushToTalk,
    Toggle,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            api_key: String::new(),
            model: "gpt-4o-transcribe".into(),
            hotkey: "Ctrl+Space".into(),
            trigger_mode: TriggerMode::PushToTalk,
            language: String::new(),
            prompt: String::new(),
            theme: "blueprint".into(),
            max_recording_sec: 60,
        }
    }
}

const ALLOWED_MODELS: &[&str] = &["gpt-4o-transcribe", "gpt-4o-mini-transcribe"];
const ALLOWED_THEMES: &[&str] = &["blueprint", "signal", "zinc"];

impl AppConfig {
    /// Clamp / normalize user-editable fields before persist or apply.
    pub fn sanitize(mut self) -> Self {
        self.api_key = self.api_key.trim().to_string();
        self.hotkey = self.hotkey.trim().to_string();
        if self.hotkey.is_empty() {
            self.hotkey = AppConfig::default().hotkey;
        }
        if !ALLOWED_MODELS.contains(&self.model.as_str()) {
            self.model = AppConfig::default().model;
        }
        if !ALLOWED_THEMES.contains(&self.theme.as_str()) {
            self.theme = AppConfig::default().theme;
        }
        self.language = self.language.trim().to_lowercase();
        self.prompt = self.prompt.trim().to_string();
        self.max_recording_sec = self.max_recording_sec.clamp(5, 300);
        self
    }
}

pub fn config_path() -> PathBuf {
    let base = dirs::config_dir().unwrap_or_else(|| PathBuf::from("."));
    base.join("liquidvoice").join("config.json")
}

pub fn load() -> AppConfig {
    let path = config_path();
    fs::read_to_string(&path)
        .ok()
        .and_then(|s| serde_json::from_str::<AppConfig>(&s).ok())
        .unwrap_or_default()
        .sanitize()
}

pub fn save(config: &AppConfig) -> Result<(), String> {
    let path = config_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("config dir: {e}"))?;
    }
    let json = serde_json::to_string_pretty(config).map_err(|e| format!("config encode: {e}"))?;
    fs::write(&path, json).map_err(|e| format!("config write: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sanitize_rejects_bad_model_and_empty_hotkey() {
        let cfg = AppConfig {
            model: "gpt-4".into(),
            hotkey: "  ".into(),
            theme: "neon".into(),
            max_recording_sec: 9999,
            ..Default::default()
        }
        .sanitize();
        assert_eq!(cfg.model, "gpt-4o-transcribe");
        assert_eq!(cfg.hotkey, "Ctrl+Space");
        assert_eq!(cfg.theme, "blueprint");
        assert_eq!(cfg.max_recording_sec, 300);
    }
}
