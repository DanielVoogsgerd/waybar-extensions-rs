use std::path::Path;

use serde::Deserialize;

#[derive(Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct Config {
    pub general: GeneralConfig,
    pub openweathermap: OpenWeatherMapConfig,
}

impl Config {
    pub fn from_file(path: &Path) -> Result<Self, Box<dyn std::error::Error>> {
        let data = String::from_utf8(std::fs::read(path)?)?;
        let config: Config = toml::from_str(&data)?;

        Ok(config)
    }

    pub fn load(app_name: &str, file_name: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let config_path = xdg::BaseDirectories::with_prefix(app_name)
            .or(Err("Could not load app configuration"))?
            .find_config_file(file_name)
            .ok_or_else(|| "Could not load configuration file")?;

        Self::from_file(&config_path)
    }
}

#[derive(Deserialize)]
pub struct GeneralConfig {
    pub lat: f32,
    pub lon: f32,
}

#[derive(Deserialize)]
pub struct OpenWeatherMapConfig {
    pub api_key: String,
}
