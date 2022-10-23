use waybar_extensions::weather;

use chrono::{DateTime, Local, NaiveDateTime, Utc};

#[derive(serde::Serialize)]
pub struct WaybarResponse {
    text: String,
    tooltip: String,
    class: Vec<String>,
}

#[tokio::main]
async fn main() {
    // Configuration can be found in XDG_CONFIG_HOME/waybar/modules.toml
    let config_path = xdg::BaseDirectories::with_prefix("waybar")
        .expect("Could not waybar configuration")
        .find_config_file("modules.toml")
        .expect("Could not find modules.toml");

    let config = waybar_extensions::config::Config::from_file(&config_path)
        .expect("Could not read configuration");

    let weather = weather::current::get(
        config.general.lat,
        config.general.lon,
        &config.openweathermap.api_key,
    )
    .await
    .expect("No current weather found");

    let temp = weather.main.temp - 273.15;
    let description = weather.weather[0].main.clone();

    let text = format!("{temp:.1} °C with {description}");

    let rain_info = if let Some(rain) = weather.rain {
        rain.n1h
    } else {
        0f64
    };
    let wind_speed = weather.wind.speed;
    let wind_direction = get_wind_direction(weather.wind.deg as u16);
    let min_temp = weather.main.temp_min - 273.15;
    let max_temp = weather.main.temp_max - 273.15;
    let sunrise_utc =
        DateTime::<Utc>::from_utc(NaiveDateTime::from_timestamp(weather.sys.sunrise, 0), Utc);
    let sunrise = DateTime::<Local>::from(sunrise_utc).format("%H:%M");
    let sunset_utc =
        DateTime::<Utc>::from_utc(NaiveDateTime::from_timestamp(weather.sys.sunset, 0), Utc);
    let sunset = DateTime::<Local>::from(sunset_utc).format("%H:%M");

    let tooltip = format!("Rain: {rain_info} mm\nWind: {wind_speed} km/h ({wind_direction})\nTemperature: {min_temp:.1} - {max_temp:.1} °C\nSunrise: {sunrise}\nSunset: {sunset}");

    let response = WaybarResponse {
        text,
        tooltip,
        class: vec![],
    };
    let waybar_response =
        serde_json::to_string(&response).expect("Could not format waybar response");

    println!("{:}", waybar_response);
}

fn get_wind_direction(angle: u16) -> &'static str {
    match angle {
        000..=011 => "N",
        012..=033 => "NNE",
        034..=056 => "NE",
        057..=078 => "ENE",
        079..=101 => "E",
        102..=123 => "ESE",
        124..=146 => "SE",
        147..=168 => "SSE",
        169..=191 => "S",
        192..=213 => "SSW",
        214..=236 => "SW",
        237..=258 => "WSW",
        259..=281 => "W",
        282..=303 => "WNW",
        304..=326 => "NW",
        327..=348 => "NNW",
        349..=360 => "N",
        _ => panic!("Invalid wind direction"),
    }
}
