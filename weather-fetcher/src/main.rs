use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::time::{Duration, SystemTime};

#[derive(Deserialize, Debug)]
struct OwmResponse {
    main: Main,
    weather: Vec<Weather>,
    wind: Wind,
    sys: Sys,
}

#[derive(Deserialize, Debug)]
struct Main {
    temp: f32,
    feels_like: f32,
    temp_min: f32,
    temp_max: f32,
    pressure: u32,
    humidity: u32,
}

#[derive(Deserialize, Debug)]
struct Weather {
    description: String,
    icon: String,
}

#[derive(Deserialize, Debug)]
struct Wind {
    speed: f32,
}

#[derive(Deserialize, Debug)]
struct Sys {
    sunrise: i64,
    sunset: i64,
}

#[derive(Serialize)]
struct WaybarOutput {
    text: String,
    alt: String,
    tooltip: String,
    class: String,
}

fn main() -> Result<()> {
    // 1. 環境変数から設定を取得 (Nixのラップ機能で流し込む)
    let city = std::env::var("CITY_NAME").unwrap_or_else(|_| "Unknown".to_string());
    let lat = std::env::var("LAT").context("LAT is not set")?;
    let lon = std::env::var("LON").context("LON is not set")?;
    let api_key = std::env::var("OWM_KEY").context("OWM_KEY is not set")?;

    let cache_dir = dirs::cache_dir()
        .context("Could not find cache dir")?
        .join("rbn");
    let cache_path = cache_dir.join("weather_cache.json");
    fs::create_dir_all(&cache_dir)?;

    // 2. キャッシュチェック (30分)
    if let Ok(metadata) = fs::metadata(&cache_path) {
        if let Ok(modified) = metadata.modified() {
            if SystemTime::now().duration_since(modified)?.as_secs() < 1800 {
                let cached = fs::read_to_string(&cache_path)?;
                println!("{}", cached);
                return Ok(());
            }
        }
    }

    // 3. APIリクエスト
    let url = format!(
        "https://api.openweathermap.org/data/2.5/weather?lat={}&lon={}&appid={}&units=metric&lang=en",
        lat, lon, api_key
    );

    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(5))
        .build()?;

    let res = client.get(url).send();

    match res {
        Ok(response) if response.status().is_success() => {
            let data: OwmResponse = response.json()?;

            let (icon, class) = match data.weather[0].icon.as_str() {
                "01d" => ("", "sunny"),
                "01n" => ("", "clear-night"),
                "02d" => ("", "cloudy"),
                "02n" => ("", "cloudy"),
                "03d" | "03n" => ("", "cloudy"),
                "04d" | "04n" => ("", "cloudy"),
                "09d" | "09n" => ("", "rain"),
                "10d" => ("", "rain"),
                "10n" => ("", "rain"),
                "11d" | "11n" => ("", "thunder"),
                "13d" | "13n" => ("󰙿", "snow"),
                "50d" | "50n" => ("", "mist"),
                _ => ("", "unknown"),
            };

            let sunrise = chrono::DateTime::from_timestamp(data.sys.sunrise, 0)
                .map(|dt| dt.with_timezone(&chrono::Local).format("%H:%M").to_string())
                .unwrap_or_default();
            let sunset = chrono::DateTime::from_timestamp(data.sys.sunset, 0)
                .map(|dt| dt.with_timezone(&chrono::Local).format("%H:%M").to_string())
                .unwrap_or_default();

            let tooltip = format!(
                "Location: {}\rCondition: {}\rTemperature: {:.0}°C (Feels: {:.0}°C)\rPressure: {} hPa\rHigh/Low: {:.0}°C / {:.0}°C\rHumidity: {}%\rWind: {:.1}m/s\r\rSunrise: {}\rSunset: {}",
                city, data.weather[0].description, data.main.temp, data.main.feels_like,
                data.main.pressure, data.main.temp_max, data.main.temp_min, data.main.humidity,
                data.wind.speed, sunrise, sunset
            );

            let output = WaybarOutput {
                text: format!("{} {:.0}°C", icon, data.main.temp),
                alt: city,
                tooltip,
                class: class.to_string(),
            };

            let json_output = serde_json::to_string(&output)?;
            fs::write(&cache_path, &json_output)?;
            println!("{}", json_output);
        }
        _ => {
            if cache_path.exists() {
                println!("{}", fs::read_to_string(&cache_path)?);
            } else {
                println!(r#"{{"text":"󰤭 ", "tooltip":"Fetch Failed"}}"#);
            }
        }
    }

    Ok(())
}
