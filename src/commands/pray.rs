use crate::config::Config;
use crate::prayer::{calculate_prayer_times, hijri_date, hijri_month_name, PrayerTimes};
use chrono::Local;
use serde::Deserialize;
use std::time::Duration;

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
                print_raw(&t.fajr, &t.dhuhr, &t.asr, &t.maghrib, &t.isha);
                return Ok(());
            }
            Err(error) => eprintln!("City lookup failed ({error}); using configured coordinates."),
        }
    }

    let times = calculate_prayer_times(config.latitude, config.longitude, config.calc_method);
    let (year, month, day) = hijri_date(now.date_naive());
    println!(
        "Prayer Times for {:.4}, {:.4} — {}",
        config.latitude,
        config.longitude,
        now.format("%A %d %b %Y")
    );
    println!("Hijri: {} {} {}\n", day, hijri_month_name(month), year);
    print_times(&times);
    Ok(())
}

fn fetch_city(city: &str, country: &str) -> Result<ApiResponse, String> {
    let date = Local::now().format("%d-%m-%Y").to_string();
    let url = format!("https://api.aladhan.com/v1/timingsByCity/{date}");
    reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(15))
        .user_agent("qari-cli/0.1.0")
        .build()
        .map_err(|error| error.to_string())?
        .get(url)
        .query(&[
            ("city", city),
            ("country", country),
            ("method", "1"),
            ("school", "1"),
        ])
        .send()
        .and_then(reqwest::blocking::Response::error_for_status)
        .and_then(reqwest::blocking::Response::json)
        .map_err(|error| error.to_string())
}

fn print_times(times: &PrayerTimes) {
    print_raw(
        &times.fajr.to_string(),
        &times.dhuhr.to_string(),
        &times.asr.to_string(),
        &times.maghrib.to_string(),
        &times.isha.to_string(),
    );
}

fn print_raw(fajr: &str, dhuhr: &str, asr: &str, maghrib: &str, isha: &str) {
    for (name, time) in [
        ("Fajr", fajr),
        ("Dhuhr", dhuhr),
        ("Asr", asr),
        ("Maghrib", maghrib),
        ("Isha", isha),
    ] {
        println!("  {name:<8}{}", clean_time(time));
    }
}

fn clean_time(value: &str) -> &str {
    value.split_whitespace().next().unwrap_or(value)
}
