use anyhow::Result;
use serde::{Deserialize, Serialize};

use super::paths;
use crate::ui::i18n::Lang;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    #[serde(default)]
    pub player: PlayerSettings,
    #[serde(default)]
    pub cache: CacheSettings,
    #[serde(default)]
    pub display: DisplaySettings,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerSettings {
    #[serde(default = "default_volume")]
    pub volume: u8,
    #[serde(default = "default_true")]
    pub replay_gain: bool,
    #[serde(default)]
    pub default_play_mode: PlayMode,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PlayMode {
    #[default]
    Sequential,
    Shuffle,
    RepeatOne,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheSettings {
    #[serde(default = "default_cache_size")]
    pub max_size_mb: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DisplaySettings {
    #[serde(default)]
    pub kids_mode: bool,
    #[serde(default)]
    pub language: Lang,
    #[serde(default = "default_cover_scale")]
    pub cover_scale: u8,
}

fn default_volume() -> u8 {
    80
}
fn default_true() -> bool {
    true
}
fn default_cache_size() -> u64 {
    2048
}
fn default_cover_scale() -> u8 {
    100
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            player: PlayerSettings::default(),
            cache: CacheSettings::default(),
            display: DisplaySettings::default(),
        }
    }
}

impl Default for PlayerSettings {
    fn default() -> Self {
        Self {
            volume: default_volume(),
            replay_gain: true,
            default_play_mode: PlayMode::default(),
        }
    }
}

impl Default for CacheSettings {
    fn default() -> Self {
        Self {
            max_size_mb: default_cache_size(),
        }
    }
}

impl Default for DisplaySettings {
    fn default() -> Self {
        Self {
            kids_mode: false,
            language: Lang::default(),
            cover_scale: default_cover_scale(),
        }
    }
}

impl Settings {
    pub fn load() -> Result<Self> {
        let path = paths::config_file()?;
        if path.exists() {
            let content = std::fs::read_to_string(&path)?;
            Ok(toml::from_str(&content)?)
        } else {
            let settings = Self::default();
            settings.save()?;
            Ok(settings)
        }
    }

    pub fn save(&self) -> Result<()> {
        let path = paths::config_file()?;
        let content = toml::to_string_pretty(self)?;
        std::fs::write(path, content)?;
        Ok(())
    }
}
