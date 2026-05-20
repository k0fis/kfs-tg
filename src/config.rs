use serde::Deserialize;
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub general: GeneralConfig,
    #[serde(default)]
    pub ui: UiConfig,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct GeneralConfig {
    #[serde(default)]
    pub api_id: Option<i32>,
    #[serde(default)]
    pub api_hash: Option<String>,
}

impl GeneralConfig {
    pub fn effective_api_id(&self) -> i32 {
        self.api_id
            .unwrap_or_else(|| env!("KFS_TG_API_ID").parse().unwrap_or(0))
    }

    pub fn effective_api_hash(&self) -> String {
        self.api_hash
            .clone()
            .unwrap_or_else(|| String::from(env!("KFS_TG_API_HASH")))
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct UiConfig {
    #[serde(default = "default_chat_list_width")]
    pub chat_list_width: u16,
    #[serde(default = "default_true")]
    #[allow(dead_code)]
    pub show_timestamps: bool,
    #[serde(default = "default_date_format")]
    #[allow(dead_code)]
    pub date_format: String,
    #[serde(default = "default_true")]
    pub notifications: bool,
}

impl Default for UiConfig {
    fn default() -> Self {
        Self {
            chat_list_width: default_chat_list_width(),
            show_timestamps: true,
            date_format: default_date_format(),
            notifications: true,
        }
    }
}

fn default_chat_list_width() -> u16 {
    25
}
fn default_true() -> bool {
    true
}
fn default_date_format() -> String {
    "%H:%M".to_string()
}

impl Config {
    pub fn load(path: Option<&str>) -> anyhow::Result<Self> {
        let config_path = if let Some(p) = path {
            PathBuf::from(p)
        } else {
            Self::default_path()
        };

        if config_path.exists() {
            let content = fs::read_to_string(&config_path)?;
            let config: Config = toml::from_str(&content)?;
            Ok(config)
        } else {
            Ok(Config {
                general: GeneralConfig::default(),
                ui: UiConfig::default(),
            })
        }
    }

    fn default_path() -> PathBuf {
        dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("kfs-tg")
            .join("config.toml")
    }

    #[allow(dead_code)]
    pub fn data_dir() -> PathBuf {
        dirs::data_local_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("kfs-tg")
    }
}
