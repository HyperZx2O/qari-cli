use crate::config::Config;
use crate::prayer::{calculate_prayer_times, hijri_date, hijri_month_name};
use chrono::Local;
use serde::Deserialize;

#[derive(Deserialize)]
struct ApiResponse {
    data: ApiData,
}
#[derive(Deserialize)]
struct ApiData {
    timings: ApiTimings,
    date: ApiDate,
}
#[derive(Deserialize)]
struct ApiTimings {
    #[serde(rename = "Fajr")]
    fajr: String,
    #[serde(rename = "Dhuhr")]
    dhuhr: String,
    #[serde(rename = "Asr")]
    asr: String,
    #[serde(rename = "Maghrib")]
    maghrib: String,
    #[serde(rename = "Isha")]
    isha: String,
}
#[derive(Deserialize)]
struct ApiDate {
    hijri: ApiHijri,
}
#[derive(Deserialize)]
struct ApiHijri {
    day: String,
    month: ApiMonth,
    year: String,
}
#[derive(Deserialize)]
struct ApiMonth {
    en: String,
}

pub fn run(config: &Config, city: Option<&str>, country: Option<&str>) -> Result<(), String> {
    let now = Local::now();
    if let Some(city) = city {
        match fetch_city(city, country.unwrap_or("BD")) {
            Ok(response) => {
                println!(
                    "Prayer Times for {}, {} — {}",
                    city,
                    country.unwrap_or("BD"),
                    now.format("%A %d %b %Y")
                );
                println!(
                    "Hijri: {} {} {}\n",
                    response.data.date.hijri.day,
                    response.data.date.hijri.month.en,
                    response.data.date.hijri.year
                );
                let t = response.data.timings;
                print_rows(api_rows(&t));
                return Ok(());
            }
            Err(error) => eprintln!("City lookup failed ({error}); using configured coordinates."),
        }
    }

    let times = calculate_prayer_times(config.latitude, config.longitude);
    let (year, month, day) = hijri_date(now.date_naive());
    println!(
        "Prayer Times for {:.4}, {:.4} — {}",
        config.latitude,
        config.longitude,
        now.format("%A %d %b %Y")
    );
    println!("Hijri: {} {} {}\n", day, hijri_month_name(month), year);
    print_rows(times.rows());
    Ok(())
}

fn fetch_city(city: &str, country: &str) -> Result<ApiResponse, String> {
    let date = Local::now().format("%d-%m-%Y").to_string();
    let url = format!("https://api.aladhan.com/v1/timingsByCity/{date}");
    let body = crate::data::http_get(
        "AlAdhan",
        &url,
        &[
            ("city", city),
            ("country", country),
            ("method", "1"),
            ("school", "1"),
        ],
        15,
    )?;
    serde_json::from_str(&body)
        .map_err(|error| format!("AlAdhan returned an unexpected payload: {error}"))
}

/// AlAdhan serves times like `05:12 (BST)`: keep the clock part.
fn api_rows(timings: &ApiTimings) -> [(&'static str, String); 5] {
    [
        ("Fajr", clean_time(&timings.fajr).to_string()),
        ("Dhuhr", clean_time(&timings.dhuhr).to_string()),
        ("Asr", clean_time(&timings.asr).to_string()),
        ("Maghrib", clean_time(&timings.maghrib).to_string()),
        ("Isha", clean_time(&timings.isha).to_string()),
    ]
}

fn print_rows(rows: [(&str, String); 5]) {
    for (name, time) in rows {
        println!("  {name:<8}{time}");
    }
}

fn clean_time(value: &str) -> &str {
    value.split_whitespace().next().unwrap_or(value)
}
