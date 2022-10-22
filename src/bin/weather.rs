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
    let config =
        waybar_extensions::config::Config::from_file("/home/daniel/.config/waybar/modules.toml")
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
        0..=11 => "N",
        12..=34 => "NNE",
        35..=56 => "NE",
        57..=79 => "ENE",
        80..=101 => "E",
        102..=124 => "ESE",
        125..=146 => "SE",
        147..=169 => "SSE",
        170..=191 => "S",
        192..=214 => "SSW",
        215..=236 => "SW",
        237..=259 => "WSW",
        260..=281 => "W",
        282..=304 => "WNW",
        305..=326 => "NW",
        327..=349 => "NNW",
        350..=360 => "N",
        _ => panic!("Invalid wind direction"),
    }
}
