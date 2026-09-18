#![allow(dead_code)]

#[path = "../src/prayer.rs"]
mod prayer;

use chrono::NaiveDate;

#[test]
fn test_dhaka_prayer_times_range() {
    let date = NaiveDate::from_ymd_opt(2026, 9, 18).unwrap();
    let times = prayer::calculate_for_date(date, 23.8103, 90.4125, 0);
    assert!((4..=6).contains(&times.fajr.hour));
    assert!((17..=19).contains(&times.maghrib.hour));
}

#[test]
fn test_hijri_date_is_plausible() {
    let date = NaiveDate::from_ymd_opt(2026, 9, 18).unwrap();
    assert_eq!(prayer::hijri_date(date), (1448, 4, 7));
}
