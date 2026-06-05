use chrono::{Datelike, Duration, Local, NaiveDate};

/// 闭区间日期范围（"YYYY-MM-DD"，本地时区）。
pub struct DateRange {
    pub start: String,
    pub end: String,
}

fn fmt(d: NaiveDate) -> String {
    d.format("%Y-%m-%d").to_string()
}

pub fn today() -> String {
    fmt(Local::now().date_naive())
}

pub fn today_range() -> DateRange {
    let t = Local::now().date_naive();
    DateRange { start: fmt(t), end: fmt(t) }
}

pub fn yesterday_range() -> DateRange {
    let y = Local::now().date_naive() - Duration::days(1);
    DateRange { start: fmt(y), end: fmt(y) }
}

/// 本周（周一为起点）至今。
pub fn this_week_range() -> DateRange {
    let today = Local::now().date_naive();
    let weekday = today.weekday().num_days_from_monday() as i64; // 周一=0
    let start = today - Duration::days(weekday);
    DateRange { start: fmt(start), end: fmt(today) }
}

/// 本月 1 号至今。
pub fn this_month_range() -> DateRange {
    let today = Local::now().date_naive();
    let start = NaiveDate::from_ymd_opt(today.year(), today.month(), 1).unwrap_or(today);
    DateRange { start: fmt(start), end: fmt(today) }
}

/// 趋势百分比：clamp 到 [-100,100]；previous==0 时 current>0 记为 100，否则 0。
pub fn calculate_trend(current: i64, previous: i64) -> f64 {
    if previous == 0 {
        return if current > 0 { 100.0 } else { 0.0 };
    }
    let t = (current - previous) as f64 / previous as f64 * 100.0;
    t.clamp(-100.0, 100.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trend_zero_previous() {
        assert_eq!(calculate_trend(5, 0), 100.0);
        assert_eq!(calculate_trend(0, 0), 0.0);
    }

    #[test]
    fn trend_percentage_and_clamp() {
        assert_eq!(calculate_trend(150, 100), 50.0);
        assert_eq!(calculate_trend(50, 100), -50.0);
        assert_eq!(calculate_trend(1000, 100), 100.0); // clamp +900 -> 100
        assert_eq!(calculate_trend(0, 100), -100.0);
    }

    #[test]
    fn week_range_starts_monday_and_ends_today() {
        let r = this_week_range();
        let start = NaiveDate::parse_from_str(&r.start, "%Y-%m-%d").unwrap();
        assert_eq!(start.weekday().num_days_from_monday(), 0);
        assert!(r.start <= r.end);
    }
}
