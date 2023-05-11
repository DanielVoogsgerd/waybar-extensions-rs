use std::path::Path;

use serde::Deserialize;

#[derive(Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct Config {
    pub general: GeneralConfig,
    pub openweathermap: OpenWeatherMapConfig,
    pub org_clock: OrgClockConfig,
    pub unfinished_projects: UnfinishedProjectsConfig,
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
            .ok_or("Could not load configuration file")?;

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

#[derive(Deserialize)]
pub struct OrgClockConfig {
    pub notify_time: u32,
    pub notify_interval: u32,
    pub alert_time: u32,
}

#[derive(Deserialize)]
pub struct UnfinishedProjectsConfig {
    pub project_dirs: Vec<String>,
    pub max_file_depth: Option<usize>,
    pub max_project_depth: Option<usize>,
    pub active_age: u64,
    pub warning_age: u64,
    pub critical_age: u64,
}
