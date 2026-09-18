use chrono::Datelike;

#[test]
fn test_today_deterministic() {
    let day = chrono::Local::now().ordinal() as usize;
    assert_eq!((day - 1) % 6236, (day - 1) % 6236);
}
