use chrono::{Datelike, Local, NaiveDate};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PrayerTime {
    pub hour: u8,
    pub minute: u8,
}

impl std::fmt::Display for PrayerTime {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{:02}:{:02}", self.hour, self.minute)
    }
}

#[derive(Debug, Clone)]
pub struct PrayerTimes {
    pub fajr: PrayerTime,
    pub dhuhr: PrayerTime,
    pub asr: PrayerTime,
    pub maghrib: PrayerTime,
    pub isha: PrayerTime,
}

impl PrayerTimes {
    /// `(name, "HH:MM")` rows in display order.
    pub fn rows(&self) -> [(&'static str, String); 5] {
        [
            ("Fajr", self.fajr.to_string()),
            ("Dhuhr", self.dhuhr.to_string()),
            ("Asr", self.asr.to_string()),
            ("Maghrib", self.maghrib.to_string()),
            ("Isha", self.isha.to_string()),
        ]
    }
}

pub fn calculate_prayer_times(latitude: f64, longitude: f64) -> PrayerTimes {
    calculate_for_date(Local::now().date_naive(), latitude, longitude)
}

/// Karachi convention (18°/18°), the app's default angles.
pub fn calculate_for_date(date: NaiveDate, latitude: f64, longitude: f64) -> PrayerTimes {
    let (fajr_angle, isha_angle) = (18.0, 18.0);
    let julian = julian_date(date.year(), date.month(), date.day()) - longitude / 360.0;
    let timezone = (longitude / 15.0).round();

    let fajr = sun_angle_time(julian, latitude, fajr_angle, 5.0 / 24.0, false);
    let dhuhr = mid_day(julian, 12.0 / 24.0);
    let asr = asr_time(julian, latitude, 2.0, 13.0 / 24.0);
    let maghrib = sun_angle_time(julian, latitude, 0.833, 18.0 / 24.0, true);
    let isha = sun_angle_time(julian, latitude, isha_angle, 18.0 / 24.0, true);
    let adjust = |time: f64| time + timezone - longitude / 15.0;

    PrayerTimes {
        fajr: to_time(adjust(fajr)),
        dhuhr: to_time(adjust(dhuhr)),
        asr: to_time(adjust(asr)),
        maghrib: to_time(adjust(maghrib)),
        isha: to_time(adjust(isha)),
    }
}

pub fn hijri_date(date: NaiveDate) -> (i32, u8, u8) {
    let mut l = gregorian_jdn(date) - 1_948_438 + 10_632;
    let n = (l - 1) / 10_631;
    l = l - 10_631 * n + 354;
    let j = ((10_985 - l) / 5_316) * ((50 * l) / 17_719) + (l / 5_670) * ((43 * l) / 15_238);
    l = l - ((30 - j) / 15) * ((17_719 * j) / 50) - (j / 16) * ((15_238 * j) / 43) + 29;
    let month = (24 * l) / 709;
    let day = l - (709 * month) / 24;
    let year = 30 * n + j - 30;
    (year as i32, month as u8, day as u8)
}

pub fn hijri_month_name(month: u8) -> &'static str {
    const MONTHS: [&str; 12] = [
        "Muharram",
        "Safar",
        "Rabi' al-Awwal",
        "Rabi' al-Thani",
        "Jumada al-Awwal",
        "Jumada al-Thani",
        "Rajab",
        "Sha'ban",
        "Ramadan",
        "Shawwal",
        "Dhu al-Qi'dah",
        "Dhu al-Hijjah",
    ];
    MONTHS
        .get(month.saturating_sub(1) as usize)
        .copied()
        .unwrap_or("")
}

fn sun_angle_time(julian: f64, latitude: f64, angle: f64, time: f64, after: bool) -> f64 {
    let (declination, _) = sun_position(julian + time);
    let noon = mid_day(julian, time);
    let numerator = -sin_deg(angle) - sin_deg(declination) * sin_deg(latitude);
    let denominator = cos_deg(declination) * cos_deg(latitude);
    let delta = acos_deg((numerator / denominator).clamp(-1.0, 1.0)) / 15.0;
    if after {
        noon + delta
    } else {
        noon - delta
    }
}

fn asr_time(julian: f64, latitude: f64, factor: f64, time: f64) -> f64 {
    let (declination, _) = sun_position(julian + time);
    let angle = -acot_deg(factor + tan_deg((latitude - declination).abs()));
    sun_angle_time(julian, latitude, angle, time, true)
}

fn mid_day(julian: f64, time: f64) -> f64 {
    let (_, equation) = sun_position(julian + time);
    fix_hour(12.0 - equation)
}

fn sun_position(julian: f64) -> (f64, f64) {
    let days = julian - 2_451_545.0;
    let mean_anomaly = fix_angle(357.529 + 0.985_600_28 * days);
    let mean_longitude = fix_angle(280.459 + 0.985_647_36 * days);
    let ecliptic_longitude = fix_angle(
        mean_longitude + 1.915 * sin_deg(mean_anomaly) + 0.020 * sin_deg(2.0 * mean_anomaly),
    );
    let obliquity = 23.439 - 0.000_000_36 * days;
    let right_ascension = fix_hour(
        atan2_deg(
            cos_deg(obliquity) * sin_deg(ecliptic_longitude),
            cos_deg(ecliptic_longitude),
        ) / 15.0,
    );
    let equation = mean_longitude / 15.0 - right_ascension;
    let declination = asin_deg(sin_deg(obliquity) * sin_deg(ecliptic_longitude));
    (declination, equation)
}

fn julian_date(year: i32, month: u32, day: u32) -> f64 {
    gregorian_jdn(NaiveDate::from_ymd_opt(year, month, day).expect("valid date")) as f64 - 0.5
}

fn gregorian_jdn(date: NaiveDate) -> i64 {
    let a = (14 - date.month() as i64) / 12;
    let y = date.year() as i64 + 4_800 - a;
    let m = date.month() as i64 + 12 * a - 3;
    date.day() as i64 + (153 * m + 2) / 5 + 365 * y + y / 4 - y / 100 + y / 400 - 32_045
}

fn to_time(value: f64) -> PrayerTime {
    let value = fix_hour(value + 0.5 / 60.0);
    let hour = value.floor() as u8;
    let minute = ((value - f64::from(hour)) * 60.0).floor() as u8;
    PrayerTime { hour, minute }
}

fn fix_angle(value: f64) -> f64 {
    wrap_circle(value, 360.0)
}
fn fix_hour(value: f64) -> f64 {
    wrap_circle(value, 24.0)
}
fn wrap_circle(value: f64, modulus: f64) -> f64 {
    ((value % modulus) + modulus) % modulus
}
fn sin_deg(value: f64) -> f64 {
    value.to_radians().sin()
}
fn cos_deg(value: f64) -> f64 {
    value.to_radians().cos()
}
fn tan_deg(value: f64) -> f64 {
    value.to_radians().tan()
}
fn asin_deg(value: f64) -> f64 {
    value.asin().to_degrees()
}
fn acos_deg(value: f64) -> f64 {
    value.acos().to_degrees()
}
fn atan2_deg(y: f64, x: f64) -> f64 {
    y.atan2(x).to_degrees()
}
fn acot_deg(value: f64) -> f64 {
    (1.0 / value).atan().to_degrees()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dhaka_prayer_times_land_in_plausible_hours() {
        let date = NaiveDate::from_ymd_opt(2026, 9, 18).unwrap();
        let times = calculate_for_date(date, 23.8103, 90.4125);
        assert!((4..=6).contains(&times.fajr.hour));
        assert!((17..=19).contains(&times.maghrib.hour));
    }

    #[test]
    fn hijri_date_is_plausible() {
        let date = NaiveDate::from_ymd_opt(2026, 9, 18).unwrap();
        assert_eq!(hijri_date(date), (1448, 4, 7));
    }

    #[test]
    fn rows_are_zero_padded_and_ordered() {
        let date = NaiveDate::from_ymd_opt(2026, 9, 18).unwrap();
        let rows = calculate_for_date(date, 23.8103, 90.4125).rows();
        assert_eq!(rows[0].0, "Fajr");
        assert_eq!(rows[4].0, "Isha");
        assert!(rows.iter().all(|(_, time)| time.len() == 5));
    }
}
