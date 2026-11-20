use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize)]
pub struct MainConfig {
    #[serde(default)]
    pub request_mode: RequestMode,
    #[serde(default)]
    pub request_order: Vec<String>,
    #[serde(default, rename = "provider")]
    pub custom_providers: Vec<ProviderConfig>,
}

impl Default for MainConfig {
    fn default() -> Self {
        MainConfig {
            request_mode: RequestMode::default(),
            request_order: vec![
                "minehut".to_string(),
                "minekeep".to_string(),
                "mojang".to_string(),
            ],
            custom_providers: vec![
                ProviderConfig {
                    name: "Minehut".to_string(),
                    url: "https://api.minehut.com/mitm/proxy/session/minecraft/".to_string(),
                },
                ProviderConfig {
                    name: "MineKeep".to_string(),
                    url: "https://sessionserver.minekeep.net/session/minecraft/".to_string(),
                },
            ],
        }
    }
}

#[derive(Deserialize, Serialize)]
pub struct ProviderConfig {
    pub name: String,
    pub url: String,
}

#[derive(Deserialize, Serialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum RequestMode {
    Parallel,
    Sequential,
}

impl Default for RequestMode {
    fn default() -> Self {
        RequestMode::Parallel
    }
}

pub fn load_config(path: &str) -> MainConfig {
    let config = match std::fs::read_to_string(path) {
        Ok(contents) => toml::from_str(&contents).expect("invalid config.toml"),
        Err(_) => MainConfig::default(),
    };
    save_config(path, &config);
    config
}

fn save_config(path: &str, config: &MainConfig) {
    match toml::to_string_pretty(config) {
        Ok(serialized) => {
            if let Err(e) = std::fs::write(path, serialized) {
                eprintln!("Failed to save config to {}: {}", path, e);
            }
        }
        Err(e) => eprintln!("Failed to serialize config: {}", e),
    }
}
