use serde::Deserialize;

#[derive(Deserialize)]
pub struct Config {
    pub general: GeneralConfig,
    pub openweathermap: OpenWeatherMapConfig,
}

impl Config {
    pub fn from_file(path: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let data = String::from_utf8(std::fs::read(path)?)?;
        let config: Config = toml::from_str(&data)?;

        Ok(config)
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
