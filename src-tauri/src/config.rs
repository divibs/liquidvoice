use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use tauri_plugin_global_shortcut::Shortcut;

use crate::secret;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AppConfig {
    /// Legacy single key (kept in sync with the active provider key).
    pub api_key: String,
    /// Saved OpenAI key (used when model is OpenAI).
    pub openai_api_key: String,
    /// Saved DashScope / Qwen key (used when model is Qwen).
    pub qwen_api_key: String,
    pub model: String,
    pub hotkey: String,
    pub trigger_mode: TriggerMode,
    pub language: String,
    pub prompt: String,
    pub theme: String,
    pub max_recording_sec: u32,
    /// Legacy flag (unused). Prefer `frost_strength`.
    pub glass_blur: bool,
    /// Simulated frosted-glass intensity on the overlay pill (0 = clear, 100 = heavy frost).
    pub frost_strength: u32,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum TriggerMode {
    PushToTalk,
    Toggle,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            api_key: String::new(),
            openai_api_key: String::new(),
            qwen_api_key: String::new(),
            model: "gpt-4o-mini-transcribe".into(),
            hotkey: "Ctrl+Space".into(),
            trigger_mode: TriggerMode::PushToTalk,
            language: String::new(),
            prompt: String::new(),
            theme: "blueprint".into(),
            max_recording_sec: 60,
            glass_blur: false,
            frost_strength: 72,
        }
    }
}

const ALLOWED_MODELS: &[&str] = &[
    "gpt-4o-transcribe",
    "gpt-4o-mini-transcribe",
    "qwen-audio-3.0-asr-flash",
];
const ALLOWED_THEMES: &[&str] = &["blueprint", "signal", "zinc"];

pub fn is_qwen_model(model: &str) -> bool {
    model == "qwen-audio-3.0-asr-flash"
}

impl AppConfig {
    pub fn active_api_key(&self) -> &str {
        if is_qwen_model(&self.model) {
            &self.qwen_api_key
        } else {
            &self.openai_api_key
        }
    }

    /// Clamp / normalize user-editable fields before persist or apply.
    pub fn sanitize(mut self) -> Self {
        self.api_key = self.api_key.trim().to_string();
        self.openai_api_key = self.openai_api_key.trim().to_string();
        self.qwen_api_key = self.qwen_api_key.trim().to_string();
        self.hotkey = self.hotkey.trim().to_string();
        // Normalize through the parser so the stored string always matches the
        // actually-bound shortcut (case, spacing). Unparseable -> default.
        match self.hotkey.parse::<Shortcut>() {
            Ok(s) => self.hotkey = s.to_string(),
            Err(_) => self.hotkey = AppConfig::default().hotkey,
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
        self.frost_strength = self.frost_strength.clamp(0, 100);

        // Migrate legacy `api_key` into the empty provider slot.
        if self.openai_api_key.is_empty() && self.qwen_api_key.is_empty() && !self.api_key.is_empty()
        {
            if is_qwen_model(&self.model) {
                self.qwen_api_key = self.api_key.clone();
            } else {
                self.openai_api_key = self.api_key.clone();
            }
        }
        // Keep legacy field pointing at the active provider key.
        self.api_key = self.active_api_key().to_string();
        self
    }
}

pub fn config_path() -> PathBuf {
    let base = dirs::config_dir().unwrap_or_else(|| PathBuf::from("."));
    base.join("liquidvoice").join("config.json")
}

pub fn load() -> AppConfig {
    let path = config_path();
    let mut config = fs::read_to_string(&path)
        .ok()
        .and_then(|s| serde_json::from_str::<AppConfig>(&s).ok())
        .unwrap_or_default();
    // Decrypt keys before sanitize (which migrates the legacy field).
    config.openai_api_key = secret::decrypt(&config.openai_api_key).unwrap_or_default();
    config.qwen_api_key = secret::decrypt(&config.qwen_api_key).unwrap_or_default();
    config.api_key = secret::decrypt(&config.api_key).unwrap_or_default();
    config.sanitize()
}

pub fn save(config: &AppConfig) -> Result<(), String> {
    let path = config_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("config dir: {e}"))?;
    }
    let mut stored = config.clone();
    stored.api_key = secret::encrypt(&stored.api_key);
    stored.openai_api_key = secret::encrypt(&stored.openai_api_key);
    stored.qwen_api_key = secret::encrypt(&stored.qwen_api_key);
    let json =
        serde_json::to_string_pretty(&stored).map_err(|e| format!("config encode: {e}"))?;
    atomic_write(&path, &json).map_err(|e| format!("config write: {e}"))
}

/// Write via temp file + rename so a crash mid-write cannot truncate the config.
fn atomic_write(path: &Path, data: &str) -> std::io::Result<()> {
    let tmp = path.with_extension("json.tmp");
    fs::write(&tmp, data)?;
    #[cfg(windows)]
    {
        // fs::rename cannot replace an existing file on Windows.
        let _ = fs::remove_file(path);
    }
    fs::rename(&tmp, path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sanitize_rejects_bad_model_and_hotkeys() {
        let cfg = AppConfig {
            model: "gpt-4".into(),
            hotkey: "  ".into(),
            theme: "neon".into(),
            max_recording_sec: 9999,
            glass_blur: true,
            ..Default::default()
        }
        .sanitize();
        assert_eq!(cfg.model, "gpt-4o-mini-transcribe");
        assert_eq!(cfg.hotkey, "Ctrl+Space");
        assert_eq!(cfg.theme, "blueprint");
        assert_eq!(cfg.max_recording_sec, 300);
        assert_eq!(cfg.frost_strength, 72);

        // Unparseable hotkey falls back to the default, not a phantom binding.
        let cfg2 = AppConfig {
            hotkey: "not a hotkey".into(),
            ..Default::default()
        }
        .sanitize();
        assert_eq!(cfg2.hotkey, "Ctrl+Space");
        assert!(cfg2.hotkey.parse::<Shortcut>().is_ok());

        // Valid hotkey is normalized through the parser (round-trips cleanly).
        let cfg3 = AppConfig {
            hotkey: "ctrl+space".into(),
            ..Default::default()
        }
        .sanitize();
        assert!(cfg3.hotkey.parse::<Shortcut>().is_ok());
        assert_eq!(cfg3.hotkey.parse::<Shortcut>().unwrap().to_string(), cfg3.hotkey);
    }

    #[test]
    fn sanitize_keeps_qwen_model() {
        let cfg = AppConfig {
            model: "qwen-audio-3.0-asr-flash".into(),
            ..Default::default()
        }
        .sanitize();
        assert_eq!(cfg.model, "qwen-audio-3.0-asr-flash");
        assert!(is_qwen_model(&cfg.model));
    }

    #[test]
    fn migrates_legacy_api_key_and_preserves_both_providers() {
        let cfg = AppConfig {
            api_key: "sk-legacy".into(),
            model: "gpt-4o-mini-transcribe".into(),
            ..Default::default()
        }
        .sanitize();
        assert_eq!(cfg.openai_api_key, "sk-legacy");
        assert!(cfg.qwen_api_key.is_empty());

        let cfg2 = AppConfig {
            openai_api_key: "sk-oai".into(),
            qwen_api_key: "sk-qwen".into(),
            model: "qwen-audio-3.0-asr-flash".into(),
            ..Default::default()
        }
        .sanitize();
        assert_eq!(cfg2.active_api_key(), "sk-qwen");
        assert_eq!(cfg2.api_key, "sk-qwen");
    }
}
