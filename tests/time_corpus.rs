// Ported from Duckling/Time/EN/Corpus.hs
// Reference time for tests: 2013-02-12 04:30:00 UTC-02:00
// All expected values from Haskell corpus at /tmp/duckling-haskell/Duckling/Time/EN/Corpus.hs

use chrono::{DateTime, FixedOffset, NaiveDate, NaiveDateTime, TimeZone};
use duckling::{
    parse, Context, DimensionKind, DimensionValue, Entity, Grain, Lang, Locale, Options, TimePoint,
    TimeValue,
};

fn fixed_offset(offset_minutes: i32) -> FixedOffset {
    FixedOffset::east_opt(offset_minutes.checked_mul(60).unwrap()).unwrap()
}

fn local_datetime(
    offset_minutes: i32,
    y: i32,
    m: u32,
    d: u32,
    h: u32,
    mi: u32,
    s: u32,
) -> DateTime<FixedOffset> {
    fixed_offset(offset_minutes)
        .with_ymd_and_hms(y, m, d, h, mi, s)
        .unwrap()
}

fn context_at(reference_time: DateTime<FixedOffset>, locale: Locale) -> Context {
    Context::new(reference_time, locale)
}

fn make_context() -> Context {
    context_at(
        local_datetime(-120, 2013, 2, 12, 4, 30, 0),
        Locale::new(Lang::EN, None),
    )
}

fn parse_time(text: &str) -> Vec<Entity> {
    let locale = Locale::new(Lang::EN, None);
    let context = make_context();
    let options = Options::default();
    parse(text, &locale, &[DimensionKind::Time], &context, &options)
}

/// Build a NaiveDateTime from components (for naive/wall-clock time tests)
fn dt(y: i32, m: u32, d: u32, h: u32, mi: u32, s: u32) -> NaiveDateTime {
    NaiveDate::from_ymd_opt(y, m, d)
        .unwrap()
        .and_hms_opt(h, mi, s)
        .unwrap()
}

fn grain(s: &str) -> Grain {
    Grain::from_str(s)
}

fn check_time_naive(text: &str, expected_value: NaiveDateTime, expected_grain: &str) {
    let entities = parse_time(text);
    let eg = grain(expected_grain);
    let found = entities.iter().any(|e| {
        matches!(&e.value, DimensionValue::Time(TimeValue::Single { value: TimePoint::Naive { value, grain }, .. }) if *value == expected_value && *grain == eg)
    });
    assert!(
        found,
        "Expected naive time value '{:?}' grain '{}' for '{}', got: {:?}",
        expected_value,
        expected_grain,
        text,
        entities
            .iter()
            .map(|e| format!("{:?}={:?}", e.value.dim_kind(), e.value))
            .collect::<Vec<_>>()
    );
}

fn check_time_instant(text: &str, expected_value: NaiveDateTime, expected_grain: &str) {
    let entities = parse_time(text);
    let eg = grain(expected_grain);
    let found = entities.iter().any(|e| {
        matches!(&e.value, DimensionValue::Time(TimeValue::Single { value: TimePoint::Instant { value, grain }, .. }) if value.naive_local() == expected_value && *grain == eg)
    });
    assert!(
        found,
        "Expected instant time value '{:?}' grain '{}' for '{}', got: {:?}",
        expected_value,
        expected_grain,
        text,
        entities
            .iter()
            .map(|e| format!("{:?}={:?}", e.value.dim_kind(), e.value))
            .collect::<Vec<_>>()
    );
}

/// Extract NaiveDateTime and Grain from a TimePoint (works for both Instant and Naive)
fn tp_value_grain(tp: &TimePoint) -> (NaiveDateTime, Grain) {
    match tp {
        TimePoint::Instant { value, grain } => (value.naive_local(), *grain),
        TimePoint::Naive { value, grain } => (*value, *grain),
    }
}

fn check_time_interval(
    text: &str,
    expected_from: NaiveDateTime,
    expected_to: NaiveDateTime,
    expected_grain: &str,
) {
    let entities = parse_time(text);
    let eg = grain(expected_grain);
    let found = entities.iter().any(|e| match &e.value {
        DimensionValue::Time(TimeValue::Interval {
            from: Some(f),
            to: Some(t),
            ..
        }) => {
            let (fv, fg) = tp_value_grain(f);
            let (tv, tg) = tp_value_grain(t);
            fv == expected_from && tv == expected_to && (fg == eg || tg == eg)
        }
        _ => false,
    });
    assert!(
        found,
        "Expected time interval from '{:?}' to '{:?}' grain '{}' for '{}', got: {:?}",
        expected_from,
        expected_to,
        expected_grain,
        text,
        entities
            .iter()
            .map(|e| format!("{:?}={:?}", e.value.dim_kind(), e.value))
            .collect::<Vec<_>>()
    );
}

fn check_time_open_interval_after(text: &str, expected_value: NaiveDateTime, expected_grain: &str) {
    let entities = parse_time(text);
    let eg = grain(expected_grain);
    let found = entities.iter().any(|e| match &e.value {
        DimensionValue::Time(TimeValue::Interval {
            from: Some(f),
            to: None,
            ..
        }) => {
            let (fv, fg) = tp_value_grain(f);
            fv == expected_value && fg == eg
        }
        _ => false,
    });
    assert!(
        found,
        "Expected open interval (after) value '{:?}' grain '{}' for '{}', got: {:?}",
        expected_value,
        expected_grain,
        text,
        entities
            .iter()
            .map(|e| format!("{:?}={:?}", e.value.dim_kind(), e.value))
            .collect::<Vec<_>>()
    );
}

fn check_time_open_interval_before(
    text: &str,
    expected_value: NaiveDateTime,
    expected_grain: &str,
) {
    let entities = parse_time(text);
    let eg = grain(expected_grain);
    let found = entities.iter().any(|e| match &e.value {
        DimensionValue::Time(TimeValue::Interval {
            from: None,
            to: Some(t),
            ..
        }) => {
            let (tv, tg) = tp_value_grain(t);
            tv == expected_value && tg == eg
        }
        _ => false,
    });
    assert!(
        found,
        "Expected open interval (before) value '{:?}' grain '{}' for '{}', got: {:?}",
        expected_value,
        expected_grain,
        text,
        entities
            .iter()
            .map(|e| format!("{:?}={:?}", e.value.dim_kind(), e.value))
            .collect::<Vec<_>>()
    );
}

fn check_no_time(text: &str) {
    let entities = parse_time(text);
    let found = entities
        .iter()
        .any(|e| matches!(&e.value, DimensionValue::Time(_)));
    assert!(
        !found,
        "Expected NO time for '{}', but got: {:?}",
        text,
        entities
            .iter()
            .map(|e| format!("{:?}={:?}", e.value.dim_kind(), e.value))
            .collect::<Vec<_>>()
    );
}

// ============================================================
// Group 1: datetime (2013,2,12,4,30,0) Second - "now"
// ============================================================
#[test]
fn test_time_now() {
    check_time_instant("now", dt(2013, 2, 12, 4, 30, 0), "second");
    check_time_instant("right now", dt(2013, 2, 12, 4, 30, 0), "second");
    check_time_instant("just now", dt(2013, 2, 12, 4, 30, 0), "second");
    check_time_instant("at the moment", dt(2013, 2, 12, 4, 30, 0), "second");
    check_time_instant("ATM", dt(2013, 2, 12, 4, 30, 0), "second");
}

// ============================================================
// Group 2: datetime (2013,2,12,0,0,0) Day - "today"
// ============================================================
#[test]
fn test_time_today() {
    check_time_naive("today", dt(2013, 2, 12, 0, 0, 0), "day");
    check_time_naive("at this time", dt(2013, 2, 12, 0, 0, 0), "day");
}

// ============================================================
// Group 3: datetime (2013,2,1,0,0,0) Month - "2/2013"
// ============================================================
#[test]
fn test_time_month_year_slash() {
    check_time_naive("2/2013", dt(2013, 2, 1, 0, 0, 0), "month");
}

// ============================================================
// Group 4: datetime (2014,1,1,0,0,0) Year - "in 2014"
// ============================================================
#[test]
fn test_time_in_2014() {
    check_time_naive("in 2014", dt(2014, 1, 1, 0, 0, 0), "year");
}

// ============================================================
// Group 5: datetime (2013,2,11,0,0,0) Day - "yesterday"
// ============================================================
#[test]
fn test_time_yesterday() {
    check_time_naive("yesterday", dt(2013, 2, 11, 0, 0, 0), "day");
}

// ============================================================
// Group 6: datetime (2013,2,13,0,0,0) Day - "tomorrow"
// ============================================================
#[test]
fn test_time_tomorrow() {
    check_time_naive("tomorrow", dt(2013, 2, 13, 0, 0, 0), "day");
    check_time_naive("tomorrows", dt(2013, 2, 13, 0, 0, 0), "day");
}

// ============================================================
// Group 7: datetime (2013,2,18,0,0,0) Day - "monday"
// ============================================================
#[test]
fn test_time_monday() {
    check_time_naive("monday", dt(2013, 2, 18, 0, 0, 0), "day");
    check_time_naive("mon.", dt(2013, 2, 18, 0, 0, 0), "day");
    check_time_naive("this monday", dt(2013, 2, 18, 0, 0, 0), "day");
    check_time_naive("Monday, Feb 18", dt(2013, 2, 18, 0, 0, 0), "day");
    check_time_naive("Mon, February 18", dt(2013, 2, 18, 0, 0, 0), "day");
}

// ============================================================
// Group 8: datetime (2013,2,19,0,0,0) Day - "tuesday"
// ============================================================
#[test]
fn test_time_tuesday() {
    check_time_naive("tuesday", dt(2013, 2, 19, 0, 0, 0), "day");
    check_time_naive("Tuesday the 19th", dt(2013, 2, 19, 0, 0, 0), "day");
    check_time_naive("Tuesday 19th", dt(2013, 2, 19, 0, 0, 0), "day");
}

// ============================================================
// Group 9: datetime (2013,8,15,0,0,0) Day - "Thu 15th"
// ============================================================
#[test]
fn test_time_thu_15th() {
    check_time_naive("Thu 15th", dt(2013, 8, 15, 0, 0, 0), "day");
}

// ============================================================
// Group 10: datetime (2013,2,14,0,0,0) Day - "thursday"
// ============================================================
#[test]
fn test_time_thursday() {
    check_time_naive("thursday", dt(2013, 2, 14, 0, 0, 0), "day");
    check_time_naive("thu", dt(2013, 2, 14, 0, 0, 0), "day");
    check_time_naive("thu.", dt(2013, 2, 14, 0, 0, 0), "day");
}

// ============================================================
// Group 11: datetime (2013,2,15,0,0,0) Day - "friday"
// ============================================================
#[test]
fn test_time_friday() {
    check_time_naive("friday", dt(2013, 2, 15, 0, 0, 0), "day");
    check_time_naive("fri", dt(2013, 2, 15, 0, 0, 0), "day");
    check_time_naive("fri.", dt(2013, 2, 15, 0, 0, 0), "day");
}

// ============================================================
// Group 12: datetime (2013,2,16,0,0,0) Day - "saturday"
// ============================================================
#[test]
fn test_time_saturday() {
    check_time_naive("saturday", dt(2013, 2, 16, 0, 0, 0), "day");
    check_time_naive("sat", dt(2013, 2, 16, 0, 0, 0), "day");
    check_time_naive("sat.", dt(2013, 2, 16, 0, 0, 0), "day");
}

// ============================================================
// Group 13: datetime (2013,2,17,0,0,0) Day - "sunday"
// ============================================================
#[test]
fn test_time_sunday() {
    check_time_naive("sunday", dt(2013, 2, 17, 0, 0, 0), "day");
    check_time_naive("sun", dt(2013, 2, 17, 0, 0, 0), "day");
    check_time_naive("sun.", dt(2013, 2, 17, 0, 0, 0), "day");
}

// ============================================================
// Group 14: datetime (2013,3,1,0,0,0) Day - "the 1st of march"
// ============================================================
#[test]
fn test_time_first_of_march() {
    check_time_naive("the 1st of march", dt(2013, 3, 1, 0, 0, 0), "day");
    check_time_naive("first of march", dt(2013, 3, 1, 0, 0, 0), "day");
    check_time_naive("the first of march", dt(2013, 3, 1, 0, 0, 0), "day");
    check_time_naive("march first", dt(2013, 3, 1, 0, 0, 0), "day");
}

// ============================================================
// Group 15: datetime (2013,3,2,0,0,0) Day - "the 2nd of march"
// ============================================================
#[test]
fn test_time_second_of_march() {
    check_time_naive("the 2nd of march", dt(2013, 3, 2, 0, 0, 0), "day");
    check_time_naive("second of march", dt(2013, 3, 2, 0, 0, 0), "day");
    check_time_naive("the second of march", dt(2013, 3, 2, 0, 0, 0), "day");
}

// ============================================================
// Group 16: datetime (2013,3,3,0,0,0) Day - "march 3"
// ============================================================
#[test]
fn test_time_march_3() {
    check_time_naive("march 3", dt(2013, 3, 3, 0, 0, 0), "day");
    check_time_naive("the third of march", dt(2013, 3, 3, 0, 0, 0), "day");
}

// ============================================================
// Group 17: datetime (2013,3,15,0,0,0) Day - "the ides of march"
// ============================================================
#[test]
fn test_time_ides_of_march() {
    check_time_naive("the ides of march", dt(2013, 3, 15, 0, 0, 0), "day");
}

// ============================================================
// Group 18: datetime (2015,3,3,0,0,0) Day - "march 3 2015"
// ============================================================
#[test]
fn test_time_march_3_2015() {
    check_time_naive("march 3 2015", dt(2015, 3, 3, 0, 0, 0), "day");
    check_time_naive("march 3rd 2015", dt(2015, 3, 3, 0, 0, 0), "day");
    check_time_naive("march third 2015", dt(2015, 3, 3, 0, 0, 0), "day");
    check_time_naive("3/3/2015", dt(2015, 3, 3, 0, 0, 0), "day");
    check_time_naive("3/3/15", dt(2015, 3, 3, 0, 0, 0), "day");
    check_time_naive("2015-3-3", dt(2015, 3, 3, 0, 0, 0), "day");
    check_time_naive("2015-03-03", dt(2015, 3, 3, 0, 0, 0), "day");
}

// ============================================================
// Group 19: datetime (2013,2,15,0,0,0) Day - "on the 15th"
// ============================================================
#[test]
fn test_time_february_15() {
    check_time_naive("on the 15th", dt(2013, 2, 15, 0, 0, 0), "day");
    check_time_naive("the 15th of february", dt(2013, 2, 15, 0, 0, 0), "day");
    check_time_naive("15 of february", dt(2013, 2, 15, 0, 0, 0), "day");
    check_time_naive("february the 15th", dt(2013, 2, 15, 0, 0, 0), "day");
    check_time_naive("february 15", dt(2013, 2, 15, 0, 0, 0), "day");
    check_time_naive("15th february", dt(2013, 2, 15, 0, 0, 0), "day");
    check_time_naive("February 15", dt(2013, 2, 15, 0, 0, 0), "day");
}

// ============================================================
// Group 20: datetime (2013,8,8,0,0,0) Day - "Aug 8"
// ============================================================
#[test]
fn test_time_aug_8() {
    check_time_naive("Aug 8", dt(2013, 8, 8, 0, 0, 0), "day");
}

// ============================================================
// Group 21: datetime (2014,3,1,0,0,0) Month - "March in 1 year"
// ============================================================
#[test]
fn test_time_march_in_a_year() {
    check_time_naive("March in 1 year", dt(2014, 3, 1, 0, 0, 0), "month");
    check_time_naive("March in a year", dt(2014, 3, 1, 0, 0, 0), "month");
}

// ============================================================
// Group 22: datetime (2014,7,18,0,0,0) Day - "Fri, Jul 18"
// ============================================================
#[test]
fn test_time_fri_jul_18() {
    check_time_naive("Fri, Jul 18", dt(2014, 7, 18, 0, 0, 0), "day");
    check_time_naive("Jul 18, Fri", dt(2014, 7, 18, 0, 0, 0), "day");
}

// ============================================================
// Group 23: datetime (2014,10,1,0,0,0) Month - "October 2014"
// ============================================================
#[test]
fn test_time_october_2014() {
    check_time_naive("October 2014", dt(2014, 10, 1, 0, 0, 0), "month");
    check_time_naive("2014-10", dt(2014, 10, 1, 0, 0, 0), "month");
    check_time_naive("2014/10", dt(2014, 10, 1, 0, 0, 0), "month");
}

// ============================================================
// Group 24: datetime (2015,4,14,0,0,0) Day - "14april 2015"
// ============================================================
#[test]
fn test_time_april_14_2015() {
    check_time_naive("14april 2015", dt(2015, 4, 14, 0, 0, 0), "day");
    check_time_naive("April 14, 2015", dt(2015, 4, 14, 0, 0, 0), "day");
    check_time_naive("14th April 15", dt(2015, 4, 14, 0, 0, 0), "day");
}

// ============================================================
// Group 25: datetime (2013,2,19,0,0,0) Day - "next tuesday"
// ============================================================
#[test]
fn test_time_next_tuesday() {
    check_time_naive("next tuesday", dt(2013, 2, 19, 0, 0, 0), "day");
    check_time_naive("around next tuesday", dt(2013, 2, 19, 0, 0, 0), "day");
}

// ============================================================
// Groups 26 & 53: datetime (2013,2,22,0,0,0) Day - "friday after next"
// ============================================================
#[test]
fn test_time_friday_after_next() {
    check_time_naive("friday after next", dt(2013, 2, 22, 0, 0, 0), "day");
}

// ============================================================
// Group 27: datetime (2013,3,1,0,0,0) Month - "next March"
// ============================================================
#[test]
fn test_time_next_march() {
    check_time_naive("next March", dt(2013, 3, 1, 0, 0, 0), "month");
}

// ============================================================
// Group 28: datetime (2014,3,1,0,0,0) Month - "March after next"
// ============================================================
#[test]
fn test_time_march_after_next() {
    check_time_naive("March after next", dt(2014, 3, 1, 0, 0, 0), "month");
}

// ============================================================
// Group 29: datetime (2013,2,10,0,0,0) Day - "Sunday, Feb 10"
// ============================================================
#[test]
fn test_time_sunday_feb_10() {
    check_time_naive("Sunday, Feb 10", dt(2013, 2, 10, 0, 0, 0), "day");
}

// ============================================================
// Group 30: datetime (2013,2,13,0,0,0) Day - "Wed, Feb13"
// ============================================================
#[test]
fn test_time_wed_feb13() {
    check_time_naive("Wed, Feb13", dt(2013, 2, 13, 0, 0, 0), "day");
}

// ============================================================
// Group 31: datetime (2013,2,11,0,0,0) Week - "this week"
// ============================================================
#[test]
fn test_time_this_week() {
    check_time_naive("this week", dt(2013, 2, 11, 0, 0, 0), "week");
    check_time_naive("current week", dt(2013, 2, 11, 0, 0, 0), "week");
}

// ============================================================
// Group 32: datetime (2013,2,4,0,0,0) Week - "last week"
// ============================================================
#[test]
fn test_time_last_week() {
    check_time_naive("last week", dt(2013, 2, 4, 0, 0, 0), "week");
    check_time_naive("past week", dt(2013, 2, 4, 0, 0, 0), "week");
    check_time_naive("previous week", dt(2013, 2, 4, 0, 0, 0), "week");
}

// ============================================================
// Group 33: datetime (2013,2,18,0,0,0) Week - "next week"
// ============================================================
#[test]
fn test_time_next_week() {
    check_time_naive("next week", dt(2013, 2, 18, 0, 0, 0), "week");
    check_time_naive("the following week", dt(2013, 2, 18, 0, 0, 0), "week");
    check_time_naive("around next week", dt(2013, 2, 18, 0, 0, 0), "week");
    check_time_naive("upcoming week", dt(2013, 2, 18, 0, 0, 0), "week");
    check_time_naive("coming week", dt(2013, 2, 18, 0, 0, 0), "week");
}

// ============================================================
// Group 34: datetime (2013,1,1,0,0,0) Month - "last month"
// ============================================================
#[test]
fn test_time_last_month() {
    check_time_naive("last month", dt(2013, 1, 1, 0, 0, 0), "month");
}

// ============================================================
// Group 35: datetime (2013,3,1,0,0,0) Month - "next month"
// ============================================================
#[test]
fn test_time_next_month() {
    check_time_naive("next month", dt(2013, 3, 1, 0, 0, 0), "month");
}

// ============================================================
// Group 36: datetime (2013,3,20,0,0,0) Day - "20 of next month"
// ============================================================
#[test]
fn test_time_20_of_next_month() {
    check_time_naive("20 of next month", dt(2013, 3, 20, 0, 0, 0), "day");
    check_time_naive("20th of the next month", dt(2013, 3, 20, 0, 0, 0), "day");
    check_time_naive("20th day of next month", dt(2013, 3, 20, 0, 0, 0), "day");
}

// ============================================================
// Group 37: datetime (2013,2,20,0,0,0) Day - "20th of the current month"
// ============================================================
#[test]
fn test_time_20_of_current_month() {
    check_time_naive("20th of the current month", dt(2013, 2, 20, 0, 0, 0), "day");
    check_time_naive("20 of this month", dt(2013, 2, 20, 0, 0, 0), "day");
}

// ============================================================
// Group 38: datetime (2013,1,20,0,0,0) Day - "20th of the previous month"
// ============================================================
#[test]
fn test_time_20_of_previous_month() {
    check_time_naive(
        "20th of the previous month",
        dt(2013, 1, 20, 0, 0, 0),
        "day",
    );
}

// ============================================================
// Group 39: datetime (2013,1,1,0,0,0) Quarter - "this quarter"
// ============================================================
#[test]
fn test_time_this_quarter() {
    check_time_naive("this quarter", dt(2013, 1, 1, 0, 0, 0), "quarter");
    check_time_naive("this qtr", dt(2013, 1, 1, 0, 0, 0), "quarter");
}

// ============================================================
// Group 40: datetime (2013,4,1,0,0,0) Quarter - "next quarter"
// ============================================================
#[test]
fn test_time_next_quarter() {
    check_time_naive("next quarter", dt(2013, 4, 1, 0, 0, 0), "quarter");
    check_time_naive("next qtr", dt(2013, 4, 1, 0, 0, 0), "quarter");
}

// ============================================================
// Group 41: datetime (2013,7,1,0,0,0) Quarter - "third quarter"
// ============================================================
#[test]
fn test_time_third_quarter() {
    check_time_naive("third quarter", dt(2013, 7, 1, 0, 0, 0), "quarter");
    check_time_naive("3rd quarter", dt(2013, 7, 1, 0, 0, 0), "quarter");
    check_time_naive("third qtr", dt(2013, 7, 1, 0, 0, 0), "quarter");
    check_time_naive("3rd qtr", dt(2013, 7, 1, 0, 0, 0), "quarter");
    check_time_naive("the 3rd qtr", dt(2013, 7, 1, 0, 0, 0), "quarter");
}

// ============================================================
// Group 42: datetime (2018,10,1,0,0,0) Quarter - "4th quarter 2018"
// ============================================================
#[test]
fn test_time_4th_quarter_2018() {
    check_time_naive("4th quarter 2018", dt(2018, 10, 1, 0, 0, 0), "quarter");
    check_time_naive("4th qtr 2018", dt(2018, 10, 1, 0, 0, 0), "quarter");
    check_time_naive("the 4th qtr of 2018", dt(2018, 10, 1, 0, 0, 0), "quarter");
    check_time_naive("18q4", dt(2018, 10, 1, 0, 0, 0), "quarter");
    check_time_naive("2018Q4", dt(2018, 10, 1, 0, 0, 0), "quarter");
}

// ============================================================
// Group 43: datetime (2012,1,1,0,0,0) Year - "last year"
// ============================================================
#[test]
fn test_time_last_year() {
    check_time_naive("last year", dt(2012, 1, 1, 0, 0, 0), "year");
    check_time_naive("last yr", dt(2012, 1, 1, 0, 0, 0), "year");
}

// ============================================================
// Group 44: datetime (2013,1,1,0,0,0) Year - "this year"
// ============================================================
#[test]
fn test_time_this_year() {
    check_time_naive("this year", dt(2013, 1, 1, 0, 0, 0), "year");
    check_time_naive("current year", dt(2013, 1, 1, 0, 0, 0), "year");
    check_time_naive("this yr", dt(2013, 1, 1, 0, 0, 0), "year");
}

// ============================================================
// Group 45: datetime (2014,1,1,0,0,0) Year - "next year"
// ============================================================
#[test]
fn test_time_next_year() {
    check_time_naive("next year", dt(2014, 1, 1, 0, 0, 0), "year");
    check_time_naive("next yr", dt(2014, 1, 1, 0, 0, 0), "year");
}

// ============================================================
// Group 46: datetime (2014,1,1,0,0,0) Year - "in 2014 AD"
// ============================================================
#[test]
fn test_time_in_2014_ad() {
    check_time_naive("in 2014 A.D.", dt(2014, 1, 1, 0, 0, 0), "year");
    check_time_naive("in 2014 AD", dt(2014, 1, 1, 0, 0, 0), "year");
}

// ============================================================
// Group 47: datetime (-2014,1,1,0,0,0) Year - "in 2014 BC"
// ============================================================
#[test]
fn test_time_in_2014_bc() {
    check_time_naive("in 2014 B.C.", dt(-2014, 1, 1, 0, 0, 0), "year");
    check_time_naive("in 2014 BC", dt(-2014, 1, 1, 0, 0, 0), "year");
}

// ============================================================
// Group 48: datetime (14,1,1,0,0,0) Year - "in 14 a.d."
// ============================================================
#[test]
fn test_time_in_14_ad() {
    check_time_naive("in 14 a.d.", dt(14, 1, 1, 0, 0, 0), "year");
}

// ============================================================
// Group 49: datetime (2013,2,10,0,0,0) Day - "last sunday"
// ============================================================
#[test]
fn test_time_last_sunday() {
    check_time_naive("last sunday", dt(2013, 2, 10, 0, 0, 0), "day");
    check_time_naive("sunday from last week", dt(2013, 2, 10, 0, 0, 0), "day");
    check_time_naive("last week's sunday", dt(2013, 2, 10, 0, 0, 0), "day");
}

// ============================================================
// Group 50: datetime (2013,2,5,0,0,0) Day - "last tuesday"
// ============================================================
#[test]
fn test_time_last_tuesday() {
    check_time_naive("last tuesday", dt(2013, 2, 5, 0, 0, 0), "day");
}

// ============================================================
// Group 51: datetime (2013,2,20,0,0,0) Day - "next wednesday"
// ============================================================
#[test]
fn test_time_next_wednesday() {
    check_time_naive("next wednesday", dt(2013, 2, 20, 0, 0, 0), "day");
}

// ============================================================
// Group 52: datetime (2013,2,20,0,0,0) Day - "wednesday of next week"
// ============================================================
#[test]
fn test_time_wednesday_of_next_week() {
    check_time_naive("wednesday of next week", dt(2013, 2, 20, 0, 0, 0), "day");
    check_time_naive("wednesday next week", dt(2013, 2, 20, 0, 0, 0), "day");
    check_time_naive("wednesday after next", dt(2013, 2, 20, 0, 0, 0), "day");
}

// ============================================================
// Group 54: datetime (2013,2,11,0,0,0) Day - "monday of this week"
// ============================================================
#[test]
fn test_time_monday_of_this_week() {
    check_time_naive("monday of this week", dt(2013, 2, 11, 0, 0, 0), "day");
}

// ============================================================
// Group 55: datetime (2013,2,12,0,0,0) Day - "tuesday of this week"
// ============================================================
#[test]
fn test_time_tuesday_of_this_week() {
    check_time_naive("tuesday of this week", dt(2013, 2, 12, 0, 0, 0), "day");
}

// ============================================================
// Group 56: datetime (2013,2,13,0,0,0) Day - "wednesday of this week"
// ============================================================
#[test]
fn test_time_wednesday_of_this_week() {
    check_time_naive("wednesday of this week", dt(2013, 2, 13, 0, 0, 0), "day");
}

// ============================================================
// Group 57: datetime (2013,2,14,0,0,0) Day - "the day after tomorrow"
// ============================================================
#[test]
fn test_time_day_after_tomorrow() {
    check_time_naive("the day after tomorrow", dt(2013, 2, 14, 0, 0, 0), "day");
}

// ============================================================
// Group 58: datetime (2013,2,14,17,0,0) Hour - "day after tomorrow 5pm"
// ============================================================
#[test]
fn test_time_day_after_tomorrow_5pm() {
    check_time_naive("day after tomorrow 5pm", dt(2013, 2, 14, 17, 0, 0), "hour");
}

// ============================================================
// Group 59: datetime (2013,2,10,0,0,0) Day - "the day before yesterday"
// ============================================================
#[test]
fn test_time_day_before_yesterday() {
    check_time_naive("the day before yesterday", dt(2013, 2, 10, 0, 0, 0), "day");
}

// ============================================================
// Group 60: datetime (2013,2,10,8,0,0) Hour - "day before yesterday 8am"
// ============================================================
#[test]
fn test_time_day_before_yesterday_8am() {
    check_time_naive("day before yesterday 8am", dt(2013, 2, 10, 8, 0, 0), "hour");
}

// ============================================================
// Group 61: datetime (2013,3,25,0,0,0) Day - "last Monday of March"
// ============================================================
#[test]
fn test_time_last_monday_of_march() {
    check_time_naive("last Monday of March", dt(2013, 3, 25, 0, 0, 0), "day");
}

// ============================================================
// Group 62: datetime (2014,3,30,0,0,0) Day - "last Sunday of March 2014"
// ============================================================
#[test]
fn test_time_last_sunday_of_march_2014() {
    check_time_naive("last Sunday of March 2014", dt(2014, 3, 30, 0, 0, 0), "day");
}

// ============================================================
// Group 63: datetime (2013,10,3,0,0,0) Day - "third day of october"
// ============================================================
#[test]
fn test_time_third_day_of_october() {
    check_time_naive("third day of october", dt(2013, 10, 3, 0, 0, 0), "day");
}

// ============================================================
// Group 64: datetime (2014,10,6,0,0,0) Week - "first week of october 2014"
// ============================================================
#[test]
fn test_time_first_week_of_october_2014() {
    check_time_naive(
        "first week of october 2014",
        dt(2014, 10, 6, 0, 0, 0),
        "week",
    );
}

// ============================================================
// Group 65: datetime (2018,12,10,0,0,0) Week - "third last week of 2018"
// ============================================================
#[test]
fn test_time_third_last_week_of_2018() {
    check_time_naive("third last week of 2018", dt(2018, 12, 10, 0, 0, 0), "week");
    check_time_naive(
        "the third last week of 2018",
        dt(2018, 12, 10, 0, 0, 0),
        "week",
    );
    check_time_naive(
        "the 3rd last week of 2018",
        dt(2018, 12, 10, 0, 0, 0),
        "week",
    );
}

// ============================================================
// Group 66: datetime (2018,10,15,0,0,0) Week - "2nd last week of October 2018"
// ============================================================
#[test]
fn test_time_2nd_last_week_of_october_2018() {
    check_time_naive(
        "2nd last week of October 2018",
        dt(2018, 10, 15, 0, 0, 0),
        "week",
    );
    check_time_naive(
        "the second last week of October 2018",
        dt(2018, 10, 15, 0, 0, 0),
        "week",
    );
}

// ============================================================
// Group 67: datetime (2013,5,27,0,0,0) Day - "fifth last day of May"
// ============================================================
#[test]
fn test_time_fifth_last_day_of_may() {
    check_time_naive("fifth last day of May", dt(2013, 5, 27, 0, 0, 0), "day");
    check_time_naive("the 5th last day of May", dt(2013, 5, 27, 0, 0, 0), "day");
}

// ============================================================
// Groups 68 & 69: datetime (2013,10,7,0,0,0) Week - "the week of october 6th/7th"
// ============================================================
#[test]
fn test_time_week_of_october() {
    check_time_naive("the week of october 6th", dt(2013, 10, 7, 0, 0, 0), "week");
    check_time_naive("the week of october 7th", dt(2013, 10, 7, 0, 0, 0), "week");
}

// ============================================================
// Group 70: datetime (2015,10,31,0,0,0) Day - "last day of october 2015"
// ============================================================
#[test]
fn test_time_last_day_of_october_2015() {
    check_time_naive("last day of october 2015", dt(2015, 10, 31, 0, 0, 0), "day");
    check_time_naive("last day in october 2015", dt(2015, 10, 31, 0, 0, 0), "day");
}

// ============================================================
// Group 71: datetime (2014,9,22,0,0,0) Week - "last week of september 2014"
// ============================================================
#[test]
fn test_time_last_week_of_september_2014() {
    check_time_naive(
        "last week of september 2014",
        dt(2014, 9, 22, 0, 0, 0),
        "week",
    );
}

// ============================================================
// Group 72: datetime (2013,10,1,0,0,0) Day - "first tuesday of october"
// ============================================================
#[test]
fn test_time_first_tuesday_of_october() {
    check_time_naive("first tuesday of october", dt(2013, 10, 1, 0, 0, 0), "day");
    check_time_naive("first tuesday in october", dt(2013, 10, 1, 0, 0, 0), "day");
}

// ============================================================
// Group 73: datetime (2014,9,16,0,0,0) Day - "third tuesday of september 2014"
// ============================================================
#[test]
fn test_time_third_tuesday_of_september_2014() {
    check_time_naive(
        "third tuesday of september 2014",
        dt(2014, 9, 16, 0, 0, 0),
        "day",
    );
}

// ============================================================
// Group 74: datetime (2014,10,1,0,0,0) Day - "first wednesday of october 2014"
// ============================================================
#[test]
fn test_time_first_wednesday_of_october_2014() {
    check_time_naive(
        "first wednesday of october 2014",
        dt(2014, 10, 1, 0, 0, 0),
        "day",
    );
}

// ============================================================
// Group 75: datetime (2014,10,8,0,0,0) Day - "second wednesday of october 2014"
// ============================================================
#[test]
fn test_time_second_wednesday_of_october_2014() {
    check_time_naive(
        "second wednesday of october 2014",
        dt(2014, 10, 8, 0, 0, 0),
        "day",
    );
}

// ============================================================
// Group 76: datetime (2015,1,13,0,0,0) Day - "third tuesday after christmas 2014"
// ============================================================
#[test]
fn test_time_third_tuesday_after_christmas_2014() {
    check_time_naive(
        "third tuesday after christmas 2014",
        dt(2015, 1, 13, 0, 0, 0),
        "day",
    );
}

// ============================================================
// Group 77: datetime (2013,2,13,3,0,0) Hour - "at 3am"
// ============================================================
#[test]
fn test_time_at_3am() {
    check_time_naive("at 3am", dt(2013, 2, 13, 3, 0, 0), "hour");
    check_time_naive("3 in the AM", dt(2013, 2, 13, 3, 0, 0), "hour");
    check_time_naive("at 3 AM", dt(2013, 2, 13, 3, 0, 0), "hour");
    check_time_naive("3 oclock am", dt(2013, 2, 13, 3, 0, 0), "hour");
    check_time_naive("at three am", dt(2013, 2, 13, 3, 0, 0), "hour");
    check_time_naive("this morning at 3", dt(2013, 2, 13, 3, 0, 0), "hour");
    check_time_naive("3 in the morning", dt(2013, 2, 13, 3, 0, 0), "hour");
    check_time_naive("at 3 in the morning", dt(2013, 2, 13, 3, 0, 0), "hour");
    check_time_naive("early morning @ 3", dt(2013, 2, 13, 3, 0, 0), "hour");
}

#[test]
fn test_time_this_morning_at_10() {
    check_time_naive("this morning @ 10", dt(2013, 2, 12, 10, 0, 0), "hour");
    check_time_naive("this morning at 10am", dt(2013, 2, 12, 10, 0, 0), "hour");
}

#[test]
fn test_time_3_18am() {
    check_time_naive("3:18am", dt(2013, 2, 13, 3, 18, 0), "minute");
    check_time_naive("3:18a", dt(2013, 2, 13, 3, 18, 0), "minute");
    check_time_naive("3h18", dt(2013, 2, 13, 3, 18, 0), "minute");
}

#[test]
fn test_time_at_7_in_3_years() {
    check_time_naive("at 7 in 3 years", dt(2016, 2, 1, 7, 0, 0), "hour");
}

#[test]
fn test_time_at_3pm() {
    check_time_naive("at 3pm", dt(2013, 2, 12, 15, 0, 0), "hour");
    check_time_naive("@ 3pm", dt(2013, 2, 12, 15, 0, 0), "hour");
    check_time_naive("3PM", dt(2013, 2, 12, 15, 0, 0), "hour");
    check_time_naive("3pm", dt(2013, 2, 12, 15, 0, 0), "hour");
    check_time_naive("3 oclock pm", dt(2013, 2, 12, 15, 0, 0), "hour");
    check_time_naive(
        "3 o'clock in the afternoon",
        dt(2013, 2, 12, 15, 0, 0),
        "hour",
    );
    check_time_naive("3ish pm", dt(2013, 2, 12, 15, 0, 0), "hour");
    check_time_naive("3pm approximately", dt(2013, 2, 12, 15, 0, 0), "hour");
    check_time_naive("at about 3pm", dt(2013, 2, 12, 15, 0, 0), "hour");
    check_time_naive("at 3p", dt(2013, 2, 12, 15, 0, 0), "hour");
    check_time_naive("at 3p.", dt(2013, 2, 12, 15, 0, 0), "hour");
}

#[test]
fn test_time_15h00() {
    check_time_naive("15h00", dt(2013, 2, 12, 15, 0, 0), "minute");
    check_time_naive("at 15h00", dt(2013, 2, 12, 15, 0, 0), "minute");
    check_time_naive("15h", dt(2013, 2, 12, 15, 0, 0), "minute");
    check_time_naive("at 15h", dt(2013, 2, 12, 15, 0, 0), "minute");
}

#[test]
fn test_time_quarter_past_3pm() {
    check_time_naive("at 15 past 3pm", dt(2013, 2, 12, 15, 15, 0), "minute");
    check_time_naive("a quarter past 3pm", dt(2013, 2, 12, 15, 15, 0), "minute");
    check_time_naive(
        "for a quarter past 3pm",
        dt(2013, 2, 12, 15, 15, 0),
        "minute",
    );
    check_time_naive(
        "3:15 in the afternoon",
        dt(2013, 2, 12, 15, 15, 0),
        "minute",
    );
    check_time_naive("15:15", dt(2013, 2, 12, 15, 15, 0), "minute");
    check_time_naive("15h15", dt(2013, 2, 12, 15, 15, 0), "minute");
    check_time_naive("3:15pm", dt(2013, 2, 12, 15, 15, 0), "minute");
    check_time_naive("3:15PM", dt(2013, 2, 12, 15, 15, 0), "minute");
    check_time_naive("3:15p", dt(2013, 2, 12, 15, 15, 0), "minute");
    check_time_naive("at 3 15", dt(2013, 2, 12, 15, 15, 0), "minute");
    check_time_naive("15 minutes past 3pm", dt(2013, 2, 12, 15, 15, 0), "minute");
    check_time_naive("15 minutes past 15h", dt(2013, 2, 12, 15, 15, 0), "minute");
}

#[test]
fn test_time_20_past_3pm() {
    check_time_naive("at 20 past 3pm", dt(2013, 2, 12, 15, 20, 0), "minute");
    check_time_naive(
        "3:20 in the afternoon",
        dt(2013, 2, 12, 15, 20, 0),
        "minute",
    );
    check_time_naive("3:20 in afternoon", dt(2013, 2, 12, 15, 20, 0), "minute");
    check_time_naive("twenty after 3pm", dt(2013, 2, 12, 15, 20, 0), "minute");
    check_time_naive("3:20p", dt(2013, 2, 12, 15, 20, 0), "minute");
    check_time_naive("15h20", dt(2013, 2, 12, 15, 20, 0), "minute");
    check_time_naive("at three twenty", dt(2013, 2, 12, 15, 20, 0), "minute");
    check_time_naive("20 minutes past 3pm", dt(2013, 2, 12, 15, 20, 0), "minute");
    check_time_naive(
        "this afternoon at 3:20",
        dt(2013, 2, 12, 15, 20, 0),
        "minute",
    );
    check_time_naive("tonight @ 3:20", dt(2013, 2, 12, 15, 20, 0), "minute");
}

#[test]
fn test_time_half_past_3pm() {
    check_time_naive(
        "at half past three pm",
        dt(2013, 2, 12, 15, 30, 0),
        "minute",
    );
    check_time_naive("half past 3 pm", dt(2013, 2, 12, 15, 30, 0), "minute");
    check_time_naive("15:30", dt(2013, 2, 12, 15, 30, 0), "minute");
    check_time_naive("15h30", dt(2013, 2, 12, 15, 30, 0), "minute");
    check_time_naive("3:30pm", dt(2013, 2, 12, 15, 30, 0), "minute");
    check_time_naive("3:30PM", dt(2013, 2, 12, 15, 30, 0), "minute");
    check_time_naive("330 p.m.", dt(2013, 2, 12, 15, 30, 0), "minute");
    check_time_naive("3:30 p m", dt(2013, 2, 12, 15, 30, 0), "minute");
    check_time_naive("3:30", dt(2013, 2, 12, 15, 30, 0), "minute");
    check_time_naive("half three", dt(2013, 2, 12, 15, 30, 0), "minute");
    check_time_naive("30 minutes past 3 pm", dt(2013, 2, 12, 15, 30, 0), "minute");
}

#[test]
fn test_time_quarter_past_noon() {
    check_time_naive("at 15 past noon", dt(2013, 2, 12, 12, 15, 0), "minute");
    check_time_naive("a quarter past noon", dt(2013, 2, 12, 12, 15, 0), "minute");
    check_time_naive(
        "for a quarter past noon",
        dt(2013, 2, 12, 12, 15, 0),
        "minute",
    );
    check_time_naive(
        "12:15 in the afternoon",
        dt(2013, 2, 12, 12, 15, 0),
        "minute",
    );
    check_time_naive("12:15", dt(2013, 2, 12, 12, 15, 0), "minute");
    check_time_naive("12h15", dt(2013, 2, 12, 12, 15, 0), "minute");
    check_time_naive("12:15pm", dt(2013, 2, 12, 12, 15, 0), "minute");
    check_time_naive("12:15PM", dt(2013, 2, 12, 12, 15, 0), "minute");
    check_time_naive("12:15p", dt(2013, 2, 12, 12, 15, 0), "minute");
    check_time_naive("at 12 15", dt(2013, 2, 12, 12, 15, 0), "minute");
    check_time_naive("15 minutes past noon", dt(2013, 2, 12, 12, 15, 0), "minute");
}

#[test]
fn test_time_nine_fifty_nine_am() {
    check_time_naive("nine fifty nine a m", dt(2013, 2, 12, 9, 59, 0), "minute");
}

#[test]
fn test_time_15_23_24() {
    check_time_naive("15:23:24", dt(2013, 2, 12, 15, 23, 24), "second");
}

#[test]
fn test_time_9_01_10_am() {
    check_time_naive("9:01:10 AM", dt(2013, 2, 12, 9, 1, 10), "second");
}

#[test]
fn test_time_quarter_to_noon() {
    check_time_naive("a quarter to noon", dt(2013, 2, 12, 11, 45, 0), "minute");
    check_time_naive("11:45am", dt(2013, 2, 12, 11, 45, 0), "minute");
    check_time_naive("11h45", dt(2013, 2, 12, 11, 45, 0), "minute");
    check_time_naive("15 to noon", dt(2013, 2, 12, 11, 45, 0), "minute");
}

#[test]
fn test_time_quarter_past_1pm() {
    check_time_naive("a quarter past 1pm", dt(2013, 2, 12, 13, 15, 0), "minute");
    check_time_naive(
        "for a quarter past 1pm",
        dt(2013, 2, 12, 13, 15, 0),
        "minute",
    );
    check_time_naive("1:15pm", dt(2013, 2, 12, 13, 15, 0), "minute");
    check_time_naive("13h15", dt(2013, 2, 12, 13, 15, 0), "minute");
    check_time_naive("15 minutes from 1pm", dt(2013, 2, 12, 13, 15, 0), "minute");
}

#[test]
fn test_time_quarter_past_2pm() {
    check_time_naive("a quarter past 2pm", dt(2013, 2, 12, 14, 15, 0), "minute");
    check_time_naive(
        "for a quarter past 2pm",
        dt(2013, 2, 12, 14, 15, 0),
        "minute",
    );
}

#[test]
fn test_time_quarter_past_8pm() {
    check_time_naive("a quarter past 8pm", dt(2013, 2, 12, 20, 15, 0), "minute");
    check_time_naive(
        "for a quarter past 8pm",
        dt(2013, 2, 12, 20, 15, 0),
        "minute",
    );
}

#[test]
fn test_time_8_tonight() {
    check_time_naive("8 tonight", dt(2013, 2, 12, 20, 0, 0), "hour");
    check_time_naive("tonight at 8 o'clock", dt(2013, 2, 12, 20, 0, 0), "hour");
    check_time_naive("eight tonight", dt(2013, 2, 12, 20, 0, 0), "hour");
    check_time_naive("8 this evening", dt(2013, 2, 12, 20, 0, 0), "hour");
    check_time_naive("at 8 in the evening", dt(2013, 2, 12, 20, 0, 0), "hour");
    check_time_naive("in the evening at eight", dt(2013, 2, 12, 20, 0, 0), "hour");
}

#[test]
fn test_time_7_30pm_fri_sep_20() {
    check_time_naive(
        "at 7:30 PM on Fri, Sep 20",
        dt(2013, 9, 20, 19, 30, 0),
        "minute",
    );
    check_time_naive(
        "at 19h30 on Fri, Sep 20",
        dt(2013, 9, 20, 19, 30, 0),
        "minute",
    );
}

#[test]
fn test_time_saturday_at_9am() {
    check_time_naive("at 9am on Saturday", dt(2013, 2, 16, 9, 0, 0), "hour");
    check_time_naive("Saturday morning at 9", dt(2013, 2, 16, 9, 0, 0), "hour");
    check_time_naive("on Saturday for 9am", dt(2013, 2, 16, 9, 0, 0), "hour");
}

#[test]
fn test_time_fri_jul_18_2014_7pm() {
    check_time_naive(
        "Fri, Jul 18, 2014 07:00 PM",
        dt(2014, 7, 18, 19, 0, 0),
        "minute",
    );
    check_time_naive(
        "Fri, Jul 18, 2014 19h00",
        dt(2014, 7, 18, 19, 0, 0),
        "minute",
    );
    check_time_naive("Fri, Jul 18, 2014 19h", dt(2014, 7, 18, 19, 0, 0), "minute");
}

#[test]
fn test_time_in_a_sec() {
    check_time_instant("in a sec", dt(2013, 2, 12, 4, 30, 1), "second");
    check_time_instant("one second from now", dt(2013, 2, 12, 4, 30, 1), "second");
    check_time_instant("in 1\"", dt(2013, 2, 12, 4, 30, 1), "second");
}

#[test]
fn test_time_in_a_minute() {
    check_time_instant("in a minute", dt(2013, 2, 12, 4, 31, 0), "second");
    check_time_instant("in one minute", dt(2013, 2, 12, 4, 31, 0), "second");
    check_time_instant("in 1'", dt(2013, 2, 12, 4, 31, 0), "second");
}

#[test]
fn test_time_in_2_minutes() {
    check_time_instant("in 2 minutes", dt(2013, 2, 12, 4, 32, 0), "second");
    check_time_instant("in 2 more minutes", dt(2013, 2, 12, 4, 32, 0), "second");
    check_time_instant("2 minutes from now", dt(2013, 2, 12, 4, 32, 0), "second");
    check_time_instant(
        "in a couple of minutes",
        dt(2013, 2, 12, 4, 32, 0),
        "second",
    );
    check_time_instant("in a pair of minutes", dt(2013, 2, 12, 4, 32, 0), "second");
}

#[test]
fn test_time_in_three_minutes() {
    check_time_instant("in three minutes", dt(2013, 2, 12, 4, 33, 0), "second");
    check_time_instant("in a few minutes", dt(2013, 2, 12, 4, 33, 0), "second");
}

#[test]
fn test_time_in_60_minutes() {
    check_time_instant("in 60 minutes", dt(2013, 2, 12, 5, 30, 0), "second");
}

#[test]
fn test_time_in_quarter_of_an_hour() {
    check_time_instant(
        "in a quarter of an hour",
        dt(2013, 2, 12, 4, 45, 0),
        "second",
    );
    check_time_instant("in 1/4h", dt(2013, 2, 12, 4, 45, 0), "second");
    check_time_instant("in 1/4 h", dt(2013, 2, 12, 4, 45, 0), "second");
    check_time_instant("in 1/4 hour", dt(2013, 2, 12, 4, 45, 0), "second");
}

#[test]
fn test_time_in_half_an_hour() {
    check_time_instant("in half an hour", dt(2013, 2, 12, 5, 0, 0), "second");
    check_time_instant("in 1/2h", dt(2013, 2, 12, 5, 0, 0), "second");
    check_time_instant("in 1/2 h", dt(2013, 2, 12, 5, 0, 0), "second");
    check_time_instant("in 1/2 hour", dt(2013, 2, 12, 5, 0, 0), "second");
}

#[test]
fn test_time_in_three_quarters_of_an_hour() {
    check_time_instant(
        "in three-quarters of an hour",
        dt(2013, 2, 12, 5, 15, 0),
        "second",
    );
    check_time_instant("in 3/4h", dt(2013, 2, 12, 5, 15, 0), "second");
    check_time_instant("in 3/4 h", dt(2013, 2, 12, 5, 15, 0), "second");
    check_time_instant("in 3/4 hour", dt(2013, 2, 12, 5, 15, 0), "second");
}

#[test]
fn test_time_in_2_5_hours() {
    check_time_instant("in 2.5 hours", dt(2013, 2, 12, 7, 0, 0), "second");
    check_time_instant("in 2 and an half hours", dt(2013, 2, 12, 7, 0, 0), "second");
}

#[test]
fn test_time_in_one_hour() {
    check_time_instant("in one hour", dt(2013, 2, 12, 5, 30, 0), "minute");
    check_time_instant("in 1h", dt(2013, 2, 12, 5, 30, 0), "minute");
}

#[test]
fn test_time_in_a_couple_hours() {
    check_time_instant("in a couple hours", dt(2013, 2, 12, 6, 30, 0), "minute");
    check_time_instant("in a couple of hours", dt(2013, 2, 12, 6, 30, 0), "minute");
}

#[test]
fn test_time_in_a_few_hours() {
    check_time_instant("in a few hours", dt(2013, 2, 12, 7, 30, 0), "minute");
    check_time_instant("in few hours", dt(2013, 2, 12, 7, 30, 0), "minute");
}

#[test]
fn test_time_in_24_hours() {
    check_time_instant("in 24 hours", dt(2013, 2, 13, 4, 30, 0), "minute");
}

#[test]
fn test_time_in_a_day() {
    check_time_instant("in a day", dt(2013, 2, 13, 4, 0, 0), "hour");
    check_time_instant("a day from now", dt(2013, 2, 13, 4, 0, 0), "hour");
}

#[test]
fn test_time_a_day_from_right_now() {
    check_time_instant("a day from right now", dt(2013, 2, 13, 4, 30, 0), "second");
}

#[test]
fn test_time_3_years_from_today() {
    check_time_naive("3 years from today", dt(2016, 2, 12, 0, 0, 0), "day");
}

#[test]
fn test_time_3_fridays_from_now() {
    check_time_naive("3 fridays from now", dt(2013, 3, 1, 0, 0, 0), "day");
    check_time_naive("three fridays from now", dt(2013, 3, 1, 0, 0, 0), "day");
}

#[test]
fn test_time_2_sundays_from_now() {
    check_time_naive("2 sundays from now", dt(2013, 2, 24, 0, 0, 0), "day");
    check_time_naive("two sundays from now", dt(2013, 2, 24, 0, 0, 0), "day");
}

#[test]
fn test_time_4_tuesdays_from_now() {
    check_time_naive("4 tuesdays from now", dt(2013, 3, 12, 0, 0, 0), "day");
    check_time_naive("four tuesdays from now", dt(2013, 3, 12, 0, 0, 0), "day");
}

#[test]
fn test_time_in_7_days() {
    check_time_instant("in 7 days", dt(2013, 2, 19, 4, 0, 0), "hour");
}

#[test]
fn test_time_in_7_days_at_5pm() {
    check_time_instant("in 7 days at 5pm", dt(2013, 2, 19, 17, 0, 0), "hour");
}

#[test]
fn test_time_in_4_years_at_5pm() {
    check_time_instant("in 4 years at 5pm", dt(2017, 2, 1, 17, 0, 0), "hour");
}

#[test]
fn test_time_in_1_week() {
    check_time_instant("in 1 week", dt(2013, 2, 19, 0, 0, 0), "day");
    check_time_instant("in a week", dt(2013, 2, 19, 0, 0, 0), "day");
}

#[test]
fn test_time_in_about_half_an_hour() {
    check_time_instant("in about half an hour", dt(2013, 2, 12, 5, 0, 0), "second");
}

#[test]
fn test_time_7_days_ago() {
    check_time_instant("7 days ago", dt(2013, 2, 5, 4, 0, 0), "hour");
}

#[test]
fn test_time_14_days_ago() {
    check_time_instant("14 days Ago", dt(2013, 1, 29, 4, 0, 0), "hour");
    check_time_instant("a fortnight ago", dt(2013, 1, 29, 4, 0, 0), "hour");
}

#[test]
fn test_time_a_week_ago() {
    check_time_instant("a week ago", dt(2013, 2, 5, 0, 0, 0), "day");
    check_time_instant("one week ago", dt(2013, 2, 5, 0, 0, 0), "day");
    check_time_instant("1 week ago", dt(2013, 2, 5, 0, 0, 0), "day");
}

#[test]
fn test_time_2_thursdays_ago() {
    check_time_naive("2 thursdays back", dt(2013, 1, 31, 0, 0, 0), "day");
    check_time_naive("2 thursdays ago", dt(2013, 1, 31, 0, 0, 0), "day");
}

#[test]
fn test_time_three_weeks_ago() {
    check_time_instant("three weeks ago", dt(2013, 1, 22, 0, 0, 0), "day");
}

#[test]
fn test_time_three_months_ago() {
    check_time_instant("three months ago", dt(2012, 11, 12, 0, 0, 0), "day");
}

#[test]
fn test_time_first_monday_of_this_month() {
    check_time_naive(
        "the first Monday of this month",
        dt(2013, 2, 4, 0, 0, 0),
        "day",
    );
    check_time_naive(
        "the first Monday of the month",
        dt(2013, 2, 4, 0, 0, 0),
        "day",
    );
    check_time_naive(
        "the first Monday in this month",
        dt(2013, 2, 4, 0, 0, 0),
        "day",
    );
    check_time_naive("first Monday in the month", dt(2013, 2, 4, 0, 0, 0), "day");
}

#[test]
fn test_time_two_years_ago() {
    check_time_instant("two years ago", dt(2011, 2, 1, 0, 0, 0), "month");
}

#[test]
fn test_time_7_days_hence() {
    check_time_instant("7 days hence", dt(2013, 2, 19, 4, 0, 0), "hour");
}

#[test]
fn test_time_14_days_hence() {
    check_time_instant("14 days hence", dt(2013, 2, 26, 4, 0, 0), "hour");
    check_time_instant("a fortnight hence", dt(2013, 2, 26, 4, 0, 0), "hour");
}

#[test]
fn test_time_a_week_hence() {
    check_time_instant("a week hence", dt(2013, 2, 19, 0, 0, 0), "day");
    check_time_instant("one week hence", dt(2013, 2, 19, 0, 0, 0), "day");
    check_time_instant("1 week hence", dt(2013, 2, 19, 0, 0, 0), "day");
}

#[test]
fn test_time_three_weeks_hence() {
    check_time_instant("three weeks hence", dt(2013, 3, 5, 0, 0, 0), "day");
}

#[test]
fn test_time_three_months_hence() {
    check_time_instant("three months hence", dt(2013, 5, 12, 0, 0, 0), "day");
}

#[test]
fn test_time_two_years_hence() {
    check_time_instant("two years hence", dt(2015, 2, 1, 0, 0, 0), "month");
}

#[test]
fn test_time_one_year_after_christmas() {
    check_time_naive("one year After christmas", dt(2013, 12, 25, 0, 0, 0), "day");
    check_time_naive("a year from Christmas", dt(2013, 12, 25, 0, 0, 0), "day");
}

#[test]
fn test_time_interval_10_days_from_18th_dec() {
    check_time_interval(
        "for 10 days from 18th Dec",
        dt(2013, 12, 18, 0, 0, 0),
        dt(2013, 12, 29, 0, 0, 0),
        "day",
    );
    check_time_interval(
        "from 18th Dec for 10 days",
        dt(2013, 12, 18, 0, 0, 0),
        dt(2013, 12, 29, 0, 0, 0),
        "day",
    );
    check_time_interval(
        "18th Dec for 10 days",
        dt(2013, 12, 18, 0, 0, 0),
        dt(2013, 12, 29, 0, 0, 0),
        "day",
    );
}

#[test]
fn test_time_interval_30_min_from_4pm() {
    check_time_interval(
        "for 30' starting from 4pm",
        dt(2013, 2, 12, 16, 0, 0),
        dt(2013, 2, 12, 16, 31, 0),
        "minute",
    );
    check_time_interval(
        "from 4pm for thirty minutes",
        dt(2013, 2, 12, 16, 0, 0),
        dt(2013, 2, 12, 16, 31, 0),
        "minute",
    );
    check_time_interval(
        "4pm for 30 mins",
        dt(2013, 2, 12, 16, 0, 0),
        dt(2013, 2, 12, 16, 31, 0),
        "minute",
    );
    check_time_interval(
        "16h for 30 mins",
        dt(2013, 2, 12, 16, 0, 0),
        dt(2013, 2, 12, 16, 31, 0),
        "minute",
    );
}

#[test]
fn test_time_this_summer() {
    check_time_interval(
        "this Summer",
        dt(2013, 6, 21, 0, 0, 0),
        dt(2013, 9, 24, 0, 0, 0),
        "day",
    );
    check_time_interval(
        "current summer",
        dt(2013, 6, 21, 0, 0, 0),
        dt(2013, 9, 24, 0, 0, 0),
        "day",
    );
}

#[test]
fn test_time_this_winter() {
    check_time_interval(
        "this winter",
        dt(2012, 12, 21, 0, 0, 0),
        dt(2013, 3, 21, 0, 0, 0),
        "day",
    );
}

#[test]
fn test_time_this_season() {
    check_time_interval(
        "this season",
        dt(2012, 12, 21, 0, 0, 0),
        dt(2013, 3, 19, 0, 0, 0),
        "day",
    );
    check_time_interval(
        "current seasons",
        dt(2012, 12, 21, 0, 0, 0),
        dt(2013, 3, 19, 0, 0, 0),
        "day",
    );
}

#[test]
fn test_time_last_season() {
    check_time_interval(
        "last season",
        dt(2012, 9, 23, 0, 0, 0),
        dt(2012, 12, 20, 0, 0, 0),
        "day",
    );
    check_time_interval(
        "past seasons",
        dt(2012, 9, 23, 0, 0, 0),
        dt(2012, 12, 20, 0, 0, 0),
        "day",
    );
    check_time_interval(
        "previous seasons",
        dt(2012, 9, 23, 0, 0, 0),
        dt(2012, 12, 20, 0, 0, 0),
        "day",
    );
}

#[test]
fn test_time_next_season() {
    check_time_interval(
        "next season",
        dt(2013, 3, 20, 0, 0, 0),
        dt(2013, 6, 20, 0, 0, 0),
        "day",
    );
}

#[test]
fn test_time_last_night() {
    check_time_interval(
        "last night",
        dt(2013, 2, 11, 18, 0, 0),
        dt(2013, 2, 12, 0, 0, 0),
        "hour",
    );
    check_time_interval(
        "yesterday evening",
        dt(2013, 2, 11, 18, 0, 0),
        dt(2013, 2, 12, 0, 0, 0),
        "hour",
    );
}

#[test]
fn test_time_late_last_night() {
    check_time_interval(
        "late last night",
        dt(2013, 2, 11, 21, 0, 0),
        dt(2013, 2, 12, 0, 0, 0),
        "hour",
    );
}

#[test]
fn test_time_christmas() {
    check_time_naive("xmas", dt(2013, 12, 25, 0, 0, 0), "day");
    check_time_naive("christmas", dt(2013, 12, 25, 0, 0, 0), "day");
    check_time_naive("christmas day", dt(2013, 12, 25, 0, 0, 0), "day");
}

#[test]
fn test_time_xmas_at_6pm() {
    check_time_naive("xmas at 6 pm", dt(2013, 12, 25, 18, 0, 0), "hour");
}

#[test]
fn test_time_morning_of_xmas() {
    check_time_interval(
        "morning of xmas",
        dt(2013, 12, 25, 0, 0, 0),
        dt(2013, 12, 25, 12, 0, 0),
        "hour",
    );
    check_time_interval(
        "morning of christmas 2013",
        dt(2013, 12, 25, 0, 0, 0),
        dt(2013, 12, 25, 12, 0, 0),
        "hour",
    );
    check_time_interval(
        "morning of this christmas day",
        dt(2013, 12, 25, 0, 0, 0),
        dt(2013, 12, 25, 12, 0, 0),
        "hour",
    );
}

#[test]
fn test_time_new_years_eve() {
    check_time_naive("new year's eve", dt(2013, 12, 31, 0, 0, 0), "day");
    check_time_naive("new years eve", dt(2013, 12, 31, 0, 0, 0), "day");
}

#[test]
fn test_time_new_years_day() {
    check_time_naive("new year's day", dt(2014, 1, 1, 0, 0, 0), "day");
    check_time_naive("new years day", dt(2014, 1, 1, 0, 0, 0), "day");
}

#[test]
fn test_time_valentines_day() {
    check_time_naive("valentine's day", dt(2013, 2, 14, 0, 0, 0), "day");
    check_time_naive("valentine day", dt(2013, 2, 14, 0, 0, 0), "day");
}

#[test]
fn test_time_4th_of_july() {
    check_time_naive("4th of July", dt(2013, 7, 4, 0, 0, 0), "day");
    check_time_naive("4 of july", dt(2013, 7, 4, 0, 0, 0), "day");
}

#[test]
fn test_time_halloween() {
    check_time_naive("halloween", dt(2013, 10, 31, 0, 0, 0), "day");
    check_time_naive("next halloween", dt(2013, 10, 31, 0, 0, 0), "day");
    check_time_naive("Halloween 2013", dt(2013, 10, 31, 0, 0, 0), "day");
}

#[test]
fn test_time_black_friday() {
    check_time_naive("black friday", dt(2013, 11, 29, 0, 0, 0), "day");
    check_time_naive(
        "black friday of this year",
        dt(2013, 11, 29, 0, 0, 0),
        "day",
    );
    check_time_naive("black friday 2013", dt(2013, 11, 29, 0, 0, 0), "day");
}

#[test]
fn test_time_black_friday_2017() {
    check_time_naive("black friday 2017", dt(2017, 11, 24, 0, 0, 0), "day");
}

#[test]
fn test_time_boss_day() {
    check_time_naive("boss's day", dt(2013, 10, 16, 0, 0, 0), "day");
    check_time_naive("boss's", dt(2013, 10, 16, 0, 0, 0), "day");
    check_time_naive("boss day", dt(2013, 10, 16, 0, 0, 0), "day");
    check_time_naive("next boss's day", dt(2013, 10, 16, 0, 0, 0), "day");
}

#[test]
fn test_time_boss_day_2016() {
    check_time_naive("boss's day 2016", dt(2016, 10, 17, 0, 0, 0), "day");
}

#[test]
fn test_time_boss_day_2021() {
    check_time_naive("boss's day 2021", dt(2021, 10, 15, 0, 0, 0), "day");
}

#[test]
fn test_time_mlk_day() {
    check_time_naive("MLK day", dt(2014, 1, 20, 0, 0, 0), "day");
    check_time_naive(
        "next Martin Luther King day",
        dt(2014, 1, 20, 0, 0, 0),
        "day",
    );
    check_time_naive(
        "next Martin Luther King's day",
        dt(2014, 1, 20, 0, 0, 0),
        "day",
    );
    check_time_naive(
        "next Martin Luther Kings day",
        dt(2014, 1, 20, 0, 0, 0),
        "day",
    );
    check_time_naive("this MLK day", dt(2014, 1, 20, 0, 0, 0), "day");
}

#[test]
fn test_time_last_mlk_day() {
    check_time_naive("last MLK Jr. day", dt(2013, 1, 21, 0, 0, 0), "day");
    check_time_naive("MLK day 2013", dt(2013, 1, 21, 0, 0, 0), "day");
}

#[test]
fn test_time_mlk_day_last_year() {
    check_time_naive("MLK day of last year", dt(2012, 1, 16, 0, 0, 0), "day");
    check_time_naive("MLK day 2012", dt(2012, 1, 16, 0, 0, 0), "day");
    check_time_naive(
        "Civil Rights Day of last year",
        dt(2012, 1, 16, 0, 0, 0),
        "day",
    );
}

#[test]
fn test_time_world_vegan_day() {
    check_time_naive("world vegan day", dt(2013, 11, 1, 0, 0, 0), "day");
}

#[test]
fn test_time_easter() {
    check_time_naive("easter", dt(2013, 3, 31, 0, 0, 0), "day");
    check_time_naive("easter 2013", dt(2013, 3, 31, 0, 0, 0), "day");
}

#[test]
fn test_time_last_easter() {
    check_time_naive("last easter", dt(2012, 4, 8, 0, 0, 0), "day");
}

#[test]
fn test_time_easter_monday() {
    check_time_naive("easter mon", dt(2013, 4, 1, 0, 0, 0), "day");
}

#[test]
fn test_time_easter_2010() {
    check_time_naive("easter 2010", dt(2010, 4, 4, 0, 0, 0), "day");
    check_time_naive(
        "Easter Sunday two thousand ten",
        dt(2010, 4, 4, 0, 0, 0),
        "day",
    );
}

#[test]
fn test_time_three_days_after_easter() {
    check_time_naive("three days after Easter", dt(2013, 4, 3, 0, 0, 0), "day");
}

#[test]
fn test_time_maundy_thursday() {
    check_time_naive("Maundy Thursday", dt(2013, 3, 28, 0, 0, 0), "day");
    check_time_naive("Covenant thu", dt(2013, 3, 28, 0, 0, 0), "day");
    check_time_naive("Thu of Mysteries", dt(2013, 3, 28, 0, 0, 0), "day");
}

#[test]
fn test_time_pentecost() {
    check_time_naive("Pentecost", dt(2013, 5, 19, 0, 0, 0), "day");
    check_time_naive("white sunday 2013", dt(2013, 5, 19, 0, 0, 0), "day");
}

#[test]
fn test_time_whit_monday() {
    check_time_naive("whit monday", dt(2013, 5, 20, 0, 0, 0), "day");
    check_time_naive("Monday of the Holy Spirit", dt(2013, 5, 20, 0, 0, 0), "day");
}

#[test]
fn test_time_palm_sunday() {
    check_time_naive("palm sunday", dt(2013, 3, 24, 0, 0, 0), "day");
    check_time_naive("branch sunday 2013", dt(2013, 3, 24, 0, 0, 0), "day");
}

#[test]
fn test_time_trinity_sunday() {
    check_time_naive("trinity sunday", dt(2013, 5, 26, 0, 0, 0), "day");
}

#[test]
fn test_time_pancake_day() {
    check_time_naive("pancake day 2013", dt(2013, 2, 12, 0, 0, 0), "day");
    check_time_naive("mardi gras", dt(2013, 2, 12, 0, 0, 0), "day");
}

#[test]
fn test_time_st_patricks_day() {
    check_time_naive("st patrick's day 2013", dt(2013, 3, 17, 0, 0, 0), "day");
    check_time_naive("st paddy's day", dt(2013, 3, 17, 0, 0, 0), "day");
    check_time_naive("saint paddy's day", dt(2013, 3, 17, 0, 0, 0), "day");
    check_time_naive("saint patricks day", dt(2013, 3, 17, 0, 0, 0), "day");
}

#[test]
fn test_time_lent_2018() {
    check_time_interval(
        "lent 2018",
        dt(2018, 2, 14, 0, 0, 0),
        dt(2018, 4, 1, 0, 0, 0),
        "day",
    );
}

#[test]
fn test_time_orthodox_easter_2018() {
    check_time_naive("orthodox easter 2018", dt(2018, 4, 8, 0, 0, 0), "day");
}

#[test]
fn test_time_orthodox_good_friday_2020() {
    check_time_naive("orthodox good friday 2020", dt(2020, 4, 17, 0, 0, 0), "day");
    check_time_naive(
        "orthodox great friday 2020",
        dt(2020, 4, 17, 0, 0, 0),
        "day",
    );
}

#[test]
fn test_time_clean_monday_2018() {
    check_time_naive("clean monday 2018", dt(2018, 2, 19, 0, 0, 0), "day");
    check_time_naive(
        "orthodox shrove monday two thousand eighteen",
        dt(2018, 2, 19, 0, 0, 0),
        "day",
    );
}

#[test]
fn test_time_lazarus_saturday_2018() {
    check_time_naive("lazarus saturday 2018", dt(2018, 3, 31, 0, 0, 0), "day");
}

#[test]
fn test_time_great_fast_2018() {
    check_time_interval(
        "great fast 2018",
        dt(2018, 2, 19, 0, 0, 0),
        dt(2018, 3, 31, 0, 0, 0),
        "day",
    );
}

#[test]
fn test_time_this_evening() {
    check_time_interval(
        "this evening",
        dt(2013, 2, 12, 18, 0, 0),
        dt(2013, 2, 13, 0, 0, 0),
        "hour",
    );
    check_time_interval(
        "today evening",
        dt(2013, 2, 12, 18, 0, 0),
        dt(2013, 2, 13, 0, 0, 0),
        "hour",
    );
    check_time_interval(
        "tonight",
        dt(2013, 2, 12, 18, 0, 0),
        dt(2013, 2, 13, 0, 0, 0),
        "hour",
    );
}

#[test]
fn test_time_this_past_weekend() {
    check_time_interval(
        "this past weekend",
        dt(2013, 2, 8, 18, 0, 0),
        dt(2013, 2, 11, 0, 0, 0),
        "hour",
    );
}

#[test]
fn test_time_tomorrow_evening() {
    check_time_interval(
        "tomorrow evening",
        dt(2013, 2, 13, 18, 0, 0),
        dt(2013, 2, 14, 0, 0, 0),
        "hour",
    );
}

#[test]
fn test_time_tomorrow_lunch() {
    check_time_interval(
        "tomorrow lunch",
        dt(2013, 2, 13, 12, 0, 0),
        dt(2013, 2, 13, 14, 0, 0),
        "hour",
    );
    check_time_interval(
        "tomorrow at lunch",
        dt(2013, 2, 13, 12, 0, 0),
        dt(2013, 2, 13, 14, 0, 0),
        "hour",
    );
}

#[test]
fn test_time_yesterday_evening() {
    check_time_interval(
        "yesterday evening",
        dt(2013, 2, 11, 18, 0, 0),
        dt(2013, 2, 12, 0, 0, 0),
        "hour",
    );
}

#[test]
fn test_time_this_weekend() {
    check_time_interval(
        "this week-end",
        dt(2013, 2, 15, 18, 0, 0),
        dt(2013, 2, 18, 0, 0, 0),
        "hour",
    );
}

#[test]
fn test_time_monday_morning() {
    check_time_interval(
        "monday mOrnIng",
        dt(2013, 2, 18, 0, 0, 0),
        dt(2013, 2, 18, 12, 0, 0),
        "hour",
    );
}

#[test]
fn test_time_monday_early_morning() {
    check_time_interval(
        "monday early in the morning",
        dt(2013, 2, 18, 0, 0, 0),
        dt(2013, 2, 18, 9, 0, 0),
        "hour",
    );
    check_time_interval(
        "monday early morning",
        dt(2013, 2, 18, 0, 0, 0),
        dt(2013, 2, 18, 9, 0, 0),
        "hour",
    );
    check_time_interval(
        "monday in the early hours of the morning",
        dt(2013, 2, 18, 0, 0, 0),
        dt(2013, 2, 18, 9, 0, 0),
        "hour",
    );
}

#[test]
fn test_time_late_tonight() {
    check_time_interval(
        "late tonight",
        dt(2013, 2, 12, 21, 0, 0),
        dt(2013, 2, 13, 0, 0, 0),
        "hour",
    );
    check_time_interval(
        "late tonite",
        dt(2013, 2, 12, 21, 0, 0),
        dt(2013, 2, 13, 0, 0, 0),
        "hour",
    );
}

#[test]
fn test_time_february_15th_morning() {
    check_time_interval(
        "february the 15th in the morning",
        dt(2013, 2, 15, 0, 0, 0),
        dt(2013, 2, 15, 12, 0, 0),
        "hour",
    );
    check_time_interval(
        "15 of february in the morning",
        dt(2013, 2, 15, 0, 0, 0),
        dt(2013, 2, 15, 12, 0, 0),
        "hour",
    );
    check_time_interval(
        "morning of the 15th of february",
        dt(2013, 2, 15, 0, 0, 0),
        dt(2013, 2, 15, 12, 0, 0),
        "hour",
    );
}

#[test]
fn test_time_last_2_seconds() {
    check_time_interval(
        "last 2 seconds",
        dt(2013, 2, 12, 4, 29, 58),
        dt(2013, 2, 12, 4, 30, 0),
        "second",
    );
    check_time_interval(
        "last two seconds",
        dt(2013, 2, 12, 4, 29, 58),
        dt(2013, 2, 12, 4, 30, 0),
        "second",
    );
}

#[test]
fn test_time_next_3_seconds() {
    check_time_interval(
        "next 3 seconds",
        dt(2013, 2, 12, 4, 30, 1),
        dt(2013, 2, 12, 4, 30, 4),
        "second",
    );
    check_time_interval(
        "next three seconds",
        dt(2013, 2, 12, 4, 30, 1),
        dt(2013, 2, 12, 4, 30, 4),
        "second",
    );
}

#[test]
fn test_time_last_2_minutes() {
    check_time_interval(
        "last 2 minutes",
        dt(2013, 2, 12, 4, 28, 0),
        dt(2013, 2, 12, 4, 30, 0),
        "minute",
    );
    check_time_interval(
        "last two minutes",
        dt(2013, 2, 12, 4, 28, 0),
        dt(2013, 2, 12, 4, 30, 0),
        "minute",
    );
}

#[test]
fn test_time_next_3_minutes() {
    check_time_interval(
        "next 3 minutes",
        dt(2013, 2, 12, 4, 31, 0),
        dt(2013, 2, 12, 4, 34, 0),
        "minute",
    );
    check_time_interval(
        "next three minutes",
        dt(2013, 2, 12, 4, 31, 0),
        dt(2013, 2, 12, 4, 34, 0),
        "minute",
    );
}

#[test]
fn test_time_last_1_hour() {
    check_time_interval(
        "last 1 hour",
        dt(2013, 2, 12, 3, 0, 0),
        dt(2013, 2, 12, 4, 0, 0),
        "hour",
    );
    check_time_interval(
        "last one hour",
        dt(2013, 2, 12, 3, 0, 0),
        dt(2013, 2, 12, 4, 0, 0),
        "hour",
    );
}

#[test]
fn test_time_next_3_hours() {
    check_time_interval(
        "next 3 hours",
        dt(2013, 2, 12, 5, 0, 0),
        dt(2013, 2, 12, 8, 0, 0),
        "hour",
    );
    check_time_interval(
        "next three hours",
        dt(2013, 2, 12, 5, 0, 0),
        dt(2013, 2, 12, 8, 0, 0),
        "hour",
    );
}

#[test]
fn test_time_last_2_days() {
    check_time_interval(
        "last 2 days",
        dt(2013, 2, 10, 0, 0, 0),
        dt(2013, 2, 12, 0, 0, 0),
        "day",
    );
    check_time_interval(
        "last two days",
        dt(2013, 2, 10, 0, 0, 0),
        dt(2013, 2, 12, 0, 0, 0),
        "day",
    );
    check_time_interval(
        "past 2 days",
        dt(2013, 2, 10, 0, 0, 0),
        dt(2013, 2, 12, 0, 0, 0),
        "day",
    );
}

#[test]
fn test_time_next_3_days() {
    check_time_interval(
        "next 3 days",
        dt(2013, 2, 13, 0, 0, 0),
        dt(2013, 2, 16, 0, 0, 0),
        "day",
    );
    check_time_interval(
        "next three days",
        dt(2013, 2, 13, 0, 0, 0),
        dt(2013, 2, 16, 0, 0, 0),
        "day",
    );
}

#[test]
fn test_time_next_few_days() {
    check_time_interval(
        "next few days",
        dt(2013, 2, 13, 0, 0, 0),
        dt(2013, 2, 16, 0, 0, 0),
        "day",
    );
}

#[test]
fn test_time_last_2_weeks() {
    check_time_interval(
        "last 2 weeks",
        dt(2013, 1, 28, 0, 0, 0),
        dt(2013, 2, 11, 0, 0, 0),
        "week",
    );
    check_time_interval(
        "last two weeks",
        dt(2013, 1, 28, 0, 0, 0),
        dt(2013, 2, 11, 0, 0, 0),
        "week",
    );
    check_time_interval(
        "past 2 weeks",
        dt(2013, 1, 28, 0, 0, 0),
        dt(2013, 2, 11, 0, 0, 0),
        "week",
    );
}

#[test]
fn test_time_next_3_weeks() {
    check_time_interval(
        "next 3 weeks",
        dt(2013, 2, 18, 0, 0, 0),
        dt(2013, 3, 11, 0, 0, 0),
        "week",
    );
    check_time_interval(
        "next three weeks",
        dt(2013, 2, 18, 0, 0, 0),
        dt(2013, 3, 11, 0, 0, 0),
        "week",
    );
}

#[test]
fn test_time_last_2_months() {
    check_time_interval(
        "last 2 months",
        dt(2012, 12, 1, 0, 0, 0),
        dt(2013, 2, 1, 0, 0, 0),
        "month",
    );
    check_time_interval(
        "last two months",
        dt(2012, 12, 1, 0, 0, 0),
        dt(2013, 2, 1, 0, 0, 0),
        "month",
    );
}

#[test]
fn test_time_next_3_months() {
    check_time_interval(
        "next 3 months",
        dt(2013, 3, 1, 0, 0, 0),
        dt(2013, 6, 1, 0, 0, 0),
        "month",
    );
    check_time_interval(
        "next three months",
        dt(2013, 3, 1, 0, 0, 0),
        dt(2013, 6, 1, 0, 0, 0),
        "month",
    );
}

#[test]
fn test_time_last_2_years() {
    check_time_interval(
        "last 2 years",
        dt(2011, 1, 1, 0, 0, 0),
        dt(2013, 1, 1, 0, 0, 0),
        "year",
    );
    check_time_interval(
        "last two years",
        dt(2011, 1, 1, 0, 0, 0),
        dt(2013, 1, 1, 0, 0, 0),
        "year",
    );
}

#[test]
fn test_time_next_3_years() {
    check_time_interval(
        "next 3 years",
        dt(2014, 1, 1, 0, 0, 0),
        dt(2017, 1, 1, 0, 0, 0),
        "year",
    );
    check_time_interval(
        "next three years",
        dt(2014, 1, 1, 0, 0, 0),
        dt(2017, 1, 1, 0, 0, 0),
        "year",
    );
}

#[test]
fn test_time_july_13_to_15() {
    check_time_interval(
        "July 13-15",
        dt(2013, 7, 13, 0, 0, 0),
        dt(2013, 7, 16, 0, 0, 0),
        "day",
    );
    check_time_interval(
        "July 13 to 15",
        dt(2013, 7, 13, 0, 0, 0),
        dt(2013, 7, 16, 0, 0, 0),
        "day",
    );
    check_time_interval(
        "July 13 thru 15",
        dt(2013, 7, 13, 0, 0, 0),
        dt(2013, 7, 16, 0, 0, 0),
        "day",
    );
    check_time_interval(
        "July 13 through 15",
        dt(2013, 7, 13, 0, 0, 0),
        dt(2013, 7, 16, 0, 0, 0),
        "day",
    );
    check_time_interval(
        "July 13 - July 15",
        dt(2013, 7, 13, 0, 0, 0),
        dt(2013, 7, 16, 0, 0, 0),
        "day",
    );
}

#[test]
fn test_time_from_july_13_to_15() {
    check_time_interval(
        "from July 13-15",
        dt(2013, 7, 13, 0, 0, 0),
        dt(2013, 7, 16, 0, 0, 0),
        "day",
    );
    check_time_interval(
        "from 13 to 15 July",
        dt(2013, 7, 13, 0, 0, 0),
        dt(2013, 7, 16, 0, 0, 0),
        "day",
    );
    check_time_interval(
        "from 13th to 15th July",
        dt(2013, 7, 13, 0, 0, 0),
        dt(2013, 7, 16, 0, 0, 0),
        "day",
    );
    check_time_interval(
        "from the 13 to 15 July",
        dt(2013, 7, 13, 0, 0, 0),
        dt(2013, 7, 16, 0, 0, 0),
        "day",
    );
    check_time_interval(
        "from the 13th to 15th July",
        dt(2013, 7, 13, 0, 0, 0),
        dt(2013, 7, 16, 0, 0, 0),
        "day",
    );
    check_time_interval(
        "from the 13th to the 15th July",
        dt(2013, 7, 13, 0, 0, 0),
        dt(2013, 7, 16, 0, 0, 0),
        "day",
    );
    check_time_interval(
        "from the 13 to the 15 July",
        dt(2013, 7, 13, 0, 0, 0),
        dt(2013, 7, 16, 0, 0, 0),
        "day",
    );
}

#[test]
fn test_time_from_13_to_15_of_july() {
    check_time_interval(
        "from 13 to 15 of July",
        dt(2013, 7, 13, 0, 0, 0),
        dt(2013, 7, 16, 0, 0, 0),
        "day",
    );
    check_time_interval(
        "from 13th to 15 of July",
        dt(2013, 7, 13, 0, 0, 0),
        dt(2013, 7, 16, 0, 0, 0),
        "day",
    );
    check_time_interval(
        "from 13 to 15th of July",
        dt(2013, 7, 13, 0, 0, 0),
        dt(2013, 7, 16, 0, 0, 0),
        "day",
    );
    check_time_interval(
        "from 13th to 15th of July",
        dt(2013, 7, 13, 0, 0, 0),
        dt(2013, 7, 16, 0, 0, 0),
        "day",
    );
    check_time_interval(
        "from 13 to the 15 of July",
        dt(2013, 7, 13, 0, 0, 0),
        dt(2013, 7, 16, 0, 0, 0),
        "day",
    );
    check_time_interval(
        "from 13th to the 15 of July",
        dt(2013, 7, 13, 0, 0, 0),
        dt(2013, 7, 16, 0, 0, 0),
        "day",
    );
    check_time_interval(
        "from 13 to the 15th of July",
        dt(2013, 7, 13, 0, 0, 0),
        dt(2013, 7, 16, 0, 0, 0),
        "day",
    );
    check_time_interval(
        "from 13th to the 15th of July",
        dt(2013, 7, 13, 0, 0, 0),
        dt(2013, 7, 16, 0, 0, 0),
        "day",
    );
    check_time_interval(
        "from the 13 to the 15 of July",
        dt(2013, 7, 13, 0, 0, 0),
        dt(2013, 7, 16, 0, 0, 0),
        "day",
    );
    check_time_interval(
        "from the 13th to the 15 of July",
        dt(2013, 7, 13, 0, 0, 0),
        dt(2013, 7, 16, 0, 0, 0),
        "day",
    );
    check_time_interval(
        "from the 13 to the 15th of July",
        dt(2013, 7, 13, 0, 0, 0),
        dt(2013, 7, 16, 0, 0, 0),
        "day",
    );
    check_time_interval(
        "from the 13th to the 15th of July",
        dt(2013, 7, 13, 0, 0, 0),
        dt(2013, 7, 16, 0, 0, 0),
        "day",
    );
}

#[test]
fn test_time_aug_8_to_12() {
    check_time_interval(
        "Aug 8 - Aug 12",
        dt(2013, 8, 8, 0, 0, 0),
        dt(2013, 8, 13, 0, 0, 0),
        "day",
    );
}

#[test]
fn test_time_930_to_1100() {
    check_time_interval(
        "9:30 - 11:00",
        dt(2013, 2, 12, 9, 30, 0),
        dt(2013, 2, 12, 11, 1, 0),
        "minute",
    );
    check_time_interval(
        "9h30 - 11h00",
        dt(2013, 2, 12, 9, 30, 0),
        dt(2013, 2, 12, 11, 1, 0),
        "minute",
    );
    check_time_interval(
        "9h30 - 11h",
        dt(2013, 2, 12, 9, 30, 0),
        dt(2013, 2, 12, 11, 1, 0),
        "minute",
    );
}

#[test]
fn test_time_930_to_1100_cst() {
    check_time_interval(
        "9:30 - 11:00 CST",
        dt(2013, 2, 12, 13, 30, 0),
        dt(2013, 2, 12, 15, 1, 0),
        "minute",
    );
    check_time_interval(
        "9h30 - 11h00 CST",
        dt(2013, 2, 12, 13, 30, 0),
        dt(2013, 2, 12, 15, 1, 0),
        "minute",
    );
    check_time_interval(
        "9h30 - 11h CST",
        dt(2013, 2, 12, 13, 30, 0),
        dt(2013, 2, 12, 15, 1, 0),
        "minute",
    );
}

#[test]
fn test_time_1500_gmt_to_1800_gmt() {
    check_time_interval(
        "15:00 GMT - 18:00 GMT",
        dt(2013, 2, 12, 13, 0, 0),
        dt(2013, 2, 12, 16, 1, 0),
        "minute",
    );
    check_time_interval(
        "15h00 GMT - 18h00 GMT",
        dt(2013, 2, 12, 13, 0, 0),
        dt(2013, 2, 12, 16, 1, 0),
        "minute",
    );
    check_time_interval(
        "15h GMT - 18h GMT",
        dt(2013, 2, 12, 13, 0, 0),
        dt(2013, 2, 12, 16, 1, 0),
        "minute",
    );
}

#[test]
fn test_time_iso8601_interval() {
    check_time_interval(
        "2015-03-28 17:00:00/2015-03-29 21:00:00",
        dt(2015, 3, 28, 17, 0, 0),
        dt(2015, 3, 29, 21, 0, 1),
        "second",
    );
}

#[test]
fn test_time_930_to_1100_on_thursday() {
    check_time_interval(
        "from 9:30 - 11:00 on Thursday",
        dt(2013, 2, 14, 9, 30, 0),
        dt(2013, 2, 14, 11, 1, 0),
        "minute",
    );
    check_time_interval(
        "between 9:30 and 11:00 on thursday",
        dt(2013, 2, 14, 9, 30, 0),
        dt(2013, 2, 14, 11, 1, 0),
        "minute",
    );
    check_time_interval(
        "between 9h30 and 11h00 on thursday",
        dt(2013, 2, 14, 9, 30, 0),
        dt(2013, 2, 14, 11, 1, 0),
        "minute",
    );
    check_time_interval(
        "9:30 - 11:00 on Thursday",
        dt(2013, 2, 14, 9, 30, 0),
        dt(2013, 2, 14, 11, 1, 0),
        "minute",
    );
    check_time_interval(
        "9h30 - 11h00 on Thursday",
        dt(2013, 2, 14, 9, 30, 0),
        dt(2013, 2, 14, 11, 1, 0),
        "minute",
    );
    check_time_interval(
        "later than 9:30 but before 11:00 on Thursday",
        dt(2013, 2, 14, 9, 30, 0),
        dt(2013, 2, 14, 11, 1, 0),
        "minute",
    );
    check_time_interval(
        "Thursday from 9:30 to 11:00",
        dt(2013, 2, 14, 9, 30, 0),
        dt(2013, 2, 14, 11, 1, 0),
        "minute",
    );
    check_time_interval(
        "from 9:30 untill 11:00 on thursday",
        dt(2013, 2, 14, 9, 30, 0),
        dt(2013, 2, 14, 11, 1, 0),
        "minute",
    );
    check_time_interval(
        "Thursday from 9:30 untill 11:00",
        dt(2013, 2, 14, 9, 30, 0),
        dt(2013, 2, 14, 11, 1, 0),
        "minute",
    );
    check_time_interval(
        "9:30 till 11:00 on Thursday",
        dt(2013, 2, 14, 9, 30, 0),
        dt(2013, 2, 14, 11, 1, 0),
        "minute",
    );
}

#[test]
fn test_time_tomorrow_between_1_and_230() {
    check_time_interval(
        "tomorrow in between 1-2:30 ish",
        dt(2013, 2, 13, 1, 0, 0),
        dt(2013, 2, 13, 2, 31, 0),
        "minute",
    );
}

#[test]
fn test_time_3_to_4_pm() {
    check_time_interval(
        "3-4pm",
        dt(2013, 2, 12, 15, 0, 0),
        dt(2013, 2, 12, 17, 0, 0),
        "hour",
    );
    check_time_interval(
        "from 3 to 4 in the PM",
        dt(2013, 2, 12, 15, 0, 0),
        dt(2013, 2, 12, 17, 0, 0),
        "hour",
    );
    check_time_interval(
        "around 3-4pm",
        dt(2013, 2, 12, 15, 0, 0),
        dt(2013, 2, 12, 17, 0, 0),
        "hour",
    );
}

#[test]
fn test_time_330_to_6_pm() {
    check_time_interval(
        "3:30 to 6 PM",
        dt(2013, 2, 12, 15, 30, 0),
        dt(2013, 2, 12, 18, 1, 0),
        "minute",
    );
    check_time_interval(
        "3:30-6 p.m.",
        dt(2013, 2, 12, 15, 30, 0),
        dt(2013, 2, 12, 18, 1, 0),
        "minute",
    );
    check_time_interval(
        "3:30-6:00pm",
        dt(2013, 2, 12, 15, 30, 0),
        dt(2013, 2, 12, 18, 1, 0),
        "minute",
    );
    check_time_interval(
        "15h30-18h",
        dt(2013, 2, 12, 15, 30, 0),
        dt(2013, 2, 12, 18, 1, 0),
        "minute",
    );
    check_time_interval(
        "from 3:30 to six p.m.",
        dt(2013, 2, 12, 15, 30, 0),
        dt(2013, 2, 12, 18, 1, 0),
        "minute",
    );
    check_time_interval(
        "from 3:30 to 6:00pm",
        dt(2013, 2, 12, 15, 30, 0),
        dt(2013, 2, 12, 18, 1, 0),
        "minute",
    );
    check_time_interval(
        "later than 3:30pm but before 6pm",
        dt(2013, 2, 12, 15, 30, 0),
        dt(2013, 2, 12, 18, 1, 0),
        "minute",
    );
    check_time_interval(
        "between 3:30pm and 6 pm",
        dt(2013, 2, 12, 15, 30, 0),
        dt(2013, 2, 12, 18, 1, 0),
        "minute",
    );
}

#[test]
fn test_time_3pm_to_6pm_second_grain() {
    check_time_interval(
        "3pm - 6:00:00pm",
        dt(2013, 2, 12, 15, 0, 0),
        dt(2013, 2, 12, 18, 0, 1),
        "second",
    );
}

#[test]
fn test_time_8am_to_1pm() {
    check_time_interval(
        "8am - 1pm",
        dt(2013, 2, 12, 8, 0, 0),
        dt(2013, 2, 12, 14, 0, 0),
        "hour",
    );
}

#[test]
fn test_time_thursday_9a_to_11a() {
    check_time_interval(
        "Thursday from 9a to 11a",
        dt(2013, 2, 14, 9, 0, 0),
        dt(2013, 2, 14, 12, 0, 0),
        "hour",
    );
    check_time_interval(
        "this Thu 9-11am",
        dt(2013, 2, 14, 9, 0, 0),
        dt(2013, 2, 14, 12, 0, 0),
        "hour",
    );
}

#[test]
fn test_time_1130_to_130() {
    check_time_interval(
        "11:30-1:30",
        dt(2013, 2, 12, 11, 30, 0),
        dt(2013, 2, 12, 13, 31, 0),
        "minute",
    );
}

#[test]
fn test_time_130pm_sat_sep_21() {
    check_time_naive(
        "1:30 PM on Sat, Sep 21",
        dt(2013, 9, 21, 13, 30, 0),
        "minute",
    );
}

#[test]
fn test_time_within_2_weeks() {
    check_time_interval(
        "Within 2 weeks",
        dt(2013, 2, 12, 4, 30, 0),
        dt(2013, 2, 26, 0, 0, 0),
        "second",
    );
}

#[test]
fn test_time_by_2pm() {
    check_time_interval(
        "by 2:00pm",
        dt(2013, 2, 12, 4, 30, 0),
        dt(2013, 2, 12, 14, 0, 0),
        "second",
    );
}

#[test]
fn test_time_eod() {
    check_time_interval(
        "EOD",
        dt(2013, 2, 12, 17, 0, 0),
        dt(2013, 2, 13, 0, 0, 0),
        "hour",
    );
    check_time_interval(
        "end of day",
        dt(2013, 2, 12, 17, 0, 0),
        dt(2013, 2, 13, 0, 0, 0),
        "hour",
    );
    check_time_interval(
        "end of the day",
        dt(2013, 2, 12, 17, 0, 0),
        dt(2013, 2, 13, 0, 0, 0),
        "hour",
    );
    check_time_interval(
        "the end of the day",
        dt(2013, 2, 12, 17, 0, 0),
        dt(2013, 2, 13, 0, 0, 0),
        "hour",
    );
}

#[test]
fn test_time_by_eod() {
    check_time_interval(
        "by EOD",
        dt(2013, 2, 12, 4, 30, 0),
        dt(2013, 2, 13, 0, 0, 0),
        "second",
    );
    check_time_interval(
        "by end of day",
        dt(2013, 2, 12, 4, 30, 0),
        dt(2013, 2, 13, 0, 0, 0),
        "second",
    );
    check_time_interval(
        "by the end of the day",
        dt(2013, 2, 12, 4, 30, 0),
        dt(2013, 2, 13, 0, 0, 0),
        "second",
    );
}

#[test]
fn test_time_by_eom() {
    check_time_interval(
        "by EOM",
        dt(2013, 2, 12, 4, 30, 0),
        dt(2013, 3, 1, 0, 0, 0),
        "second",
    );
    check_time_interval(
        "by the EOM",
        dt(2013, 2, 12, 4, 30, 0),
        dt(2013, 3, 1, 0, 0, 0),
        "second",
    );
    check_time_interval(
        "by end of the month",
        dt(2013, 2, 12, 4, 30, 0),
        dt(2013, 3, 1, 0, 0, 0),
        "second",
    );
    check_time_interval(
        "by the end of month",
        dt(2013, 2, 12, 4, 30, 0),
        dt(2013, 3, 1, 0, 0, 0),
        "second",
    );
}

#[test]
fn test_time_eom() {
    check_time_interval(
        "EOM",
        dt(2013, 2, 21, 0, 0, 0),
        dt(2013, 3, 1, 0, 0, 0),
        "day",
    );
    check_time_interval(
        "the EOM",
        dt(2013, 2, 21, 0, 0, 0),
        dt(2013, 3, 1, 0, 0, 0),
        "day",
    );
    check_time_interval(
        "at the EOM",
        dt(2013, 2, 21, 0, 0, 0),
        dt(2013, 3, 1, 0, 0, 0),
        "day",
    );
    check_time_interval(
        "the end of the month",
        dt(2013, 2, 21, 0, 0, 0),
        dt(2013, 3, 1, 0, 0, 0),
        "day",
    );
    check_time_interval(
        "end of the month",
        dt(2013, 2, 21, 0, 0, 0),
        dt(2013, 3, 1, 0, 0, 0),
        "day",
    );
    check_time_interval(
        "at the end of month",
        dt(2013, 2, 21, 0, 0, 0),
        dt(2013, 3, 1, 0, 0, 0),
        "day",
    );
}

#[test]
fn test_time_bom() {
    check_time_interval(
        "BOM",
        dt(2013, 2, 1, 0, 0, 0),
        dt(2013, 2, 11, 0, 0, 0),
        "day",
    );
    check_time_interval(
        "the BOM",
        dt(2013, 2, 1, 0, 0, 0),
        dt(2013, 2, 11, 0, 0, 0),
        "day",
    );
    check_time_interval(
        "at the BOM",
        dt(2013, 2, 1, 0, 0, 0),
        dt(2013, 2, 11, 0, 0, 0),
        "day",
    );
    check_time_interval(
        "beginning of the month",
        dt(2013, 2, 1, 0, 0, 0),
        dt(2013, 2, 11, 0, 0, 0),
        "day",
    );
    check_time_interval(
        "the beginning of the month",
        dt(2013, 2, 1, 0, 0, 0),
        dt(2013, 2, 11, 0, 0, 0),
        "day",
    );
    check_time_interval(
        "at the beginning of month",
        dt(2013, 2, 1, 0, 0, 0),
        dt(2013, 2, 11, 0, 0, 0),
        "day",
    );
}

#[test]
fn test_time_by_end_of_next_month() {
    check_time_interval(
        "by the end of next month",
        dt(2013, 2, 12, 4, 30, 0),
        dt(2013, 4, 1, 0, 0, 0),
        "second",
    );
}

#[test]
fn test_time_4pm_cet() {
    check_time_instant("4pm CET", dt(2013, 2, 12, 13, 0, 0), "minute");
}

#[test]
fn test_time_thursday_8_gmt() {
    check_time_instant("Thursday 8:00 GMT", dt(2013, 2, 14, 6, 0, 0), "minute");
    check_time_instant("Thursday 8:00 gmt", dt(2013, 2, 14, 6, 0, 0), "minute");
    check_time_instant("Thursday 8h00 GMT", dt(2013, 2, 14, 6, 0, 0), "minute");
    check_time_instant("Thursday 8h00 gmt", dt(2013, 2, 14, 6, 0, 0), "minute");
    check_time_instant("Thursday 8h GMT", dt(2013, 2, 14, 6, 0, 0), "minute");
    check_time_instant("Thursday 8h gmt", dt(2013, 2, 14, 6, 0, 0), "minute");
    check_time_instant("Thu at 8 GMT", dt(2013, 2, 14, 6, 0, 0), "minute");
    check_time_instant("Thu at 8 gmt", dt(2013, 2, 14, 6, 0, 0), "minute");
    check_time_instant("Thursday 9 am BST", dt(2013, 2, 14, 6, 0, 0), "minute");
    check_time_instant("Thursday 9 am (BST)", dt(2013, 2, 14, 6, 0, 0), "minute");
}

#[test]
fn test_time_thursday_8_pst() {
    check_time_instant("Thursday 8:00 PST", dt(2013, 2, 14, 14, 0, 0), "minute");
    check_time_instant("Thursday 8:00 pst", dt(2013, 2, 14, 14, 0, 0), "minute");
    check_time_instant("Thursday 8h00 PST", dt(2013, 2, 14, 14, 0, 0), "minute");
    check_time_instant("Thursday 8h00 pst", dt(2013, 2, 14, 14, 0, 0), "minute");
    check_time_instant("Thursday 8h PST", dt(2013, 2, 14, 14, 0, 0), "minute");
    check_time_instant("Thursday 8h pst", dt(2013, 2, 14, 14, 0, 0), "minute");
    check_time_instant("Thu at 8 am PST", dt(2013, 2, 14, 14, 0, 0), "minute");
    check_time_instant("Thu at 8 am pst", dt(2013, 2, 14, 14, 0, 0), "minute");
    check_time_instant(
        "Thursday at 9:30pm ist",
        dt(2013, 2, 14, 14, 0, 0),
        "minute",
    );
}

#[test]
fn test_time_today_at_2pm() {
    check_time_naive("today at 2pm", dt(2013, 2, 12, 14, 0, 0), "hour");
    check_time_naive("at 2pm", dt(2013, 2, 12, 14, 0, 0), "hour");
    check_time_naive("this afternoon at 2", dt(2013, 2, 12, 14, 0, 0), "hour");
    check_time_naive("this evening at 2", dt(2013, 2, 12, 14, 0, 0), "hour");
    check_time_naive("tonight at 2", dt(2013, 2, 12, 14, 0, 0), "hour");
}

#[test]
fn test_time_3pm_tomorrow() {
    check_time_naive("3pm tomorrow", dt(2013, 2, 13, 15, 0, 0), "hour");
}

#[test]
fn test_time_today_in_one_hour() {
    check_time_naive("today in one hour", dt(2013, 2, 12, 5, 30, 0), "minute");
}

#[test]
fn test_time_asap() {
    check_time_open_interval_after("ASAP", dt(2013, 2, 12, 4, 30, 0), "second");
    check_time_open_interval_after("as soon as possible", dt(2013, 2, 12, 4, 30, 0), "second");
}

#[test]
fn test_time_until_2pm() {
    check_time_open_interval_before("until 2:00pm", dt(2013, 2, 12, 14, 0, 0), "minute");
    check_time_open_interval_before("through 2:00pm", dt(2013, 2, 12, 14, 0, 0), "minute");
}

#[test]
fn test_time_after_2pm() {
    check_time_open_interval_after("after 2 pm", dt(2013, 2, 12, 14, 0, 0), "hour");
    check_time_open_interval_after("from 2 pm", dt(2013, 2, 12, 14, 0, 0), "hour");
    check_time_open_interval_after("since 2pm", dt(2013, 2, 12, 14, 0, 0), "hour");
}

#[test]
fn test_time_anytime_after_2014() {
    check_time_open_interval_after("anytime after 2014", dt(2014, 1, 1, 0, 0, 0), "year");
    check_time_open_interval_after("since 2014", dt(2014, 1, 1, 0, 0, 0), "year");
}

#[test]
fn test_time_before_2014() {
    check_time_open_interval_before("sometimes before 2014", dt(2014, 1, 1, 0, 0, 0), "year");
    check_time_open_interval_before("through 2014", dt(2014, 1, 1, 0, 0, 0), "year");
}

#[test]
fn test_time_after_5_days() {
    check_time_open_interval_after("after 5 days", dt(2013, 2, 17, 4, 0, 0), "hour");
}

#[test]
fn test_time_before_11_am() {
    check_time_open_interval_before("before 11 am", dt(2013, 2, 12, 11, 0, 0), "hour");
}

#[test]
fn test_time_in_the_afternoon() {
    check_time_interval(
        "in the afternoon",
        dt(2013, 2, 12, 12, 0, 0),
        dt(2013, 2, 12, 19, 0, 0),
        "hour",
    );
}

#[test]
fn test_time_8am_until_6() {
    check_time_interval(
        "8am until 6",
        dt(2013, 2, 12, 8, 0, 0),
        dt(2013, 2, 12, 19, 0, 0),
        "hour",
    );
}

#[test]
fn test_time_at_130pm() {
    check_time_naive("at 1:30pm", dt(2013, 2, 12, 13, 30, 0), "minute");
    check_time_naive("1:30pm", dt(2013, 2, 12, 13, 30, 0), "minute");
    check_time_naive("at 13h30", dt(2013, 2, 12, 13, 30, 0), "minute");
    check_time_naive("13h30", dt(2013, 2, 12, 13, 30, 0), "minute");
}

#[test]
fn test_time_in_15_minutes() {
    check_time_instant("in 15 minutes", dt(2013, 2, 12, 4, 45, 0), "second");
    check_time_instant("in 15'", dt(2013, 2, 12, 4, 45, 0), "second");
    check_time_instant("in 15", dt(2013, 2, 12, 4, 45, 0), "second");
}

#[test]
fn test_time_after_lunch() {
    check_time_interval(
        "after lunch",
        dt(2013, 2, 12, 13, 0, 0),
        dt(2013, 2, 12, 17, 0, 0),
        "hour",
    );
}

#[test]
fn test_time_after_school() {
    check_time_interval(
        "after school",
        dt(2013, 2, 12, 15, 0, 0),
        dt(2013, 2, 12, 21, 0, 0),
        "hour",
    );
}

#[test]
fn test_time_1030() {
    check_time_naive("10:30", dt(2013, 2, 12, 10, 30, 0), "minute");
    check_time_naive("approximately 1030", dt(2013, 2, 12, 10, 30, 0), "minute");
}

#[test]
fn test_time_this_morning() {
    check_time_interval(
        "this morning",
        dt(2013, 2, 12, 0, 0, 0),
        dt(2013, 2, 12, 12, 0, 0),
        "hour",
    );
}

#[test]
fn test_time_next_monday() {
    check_time_naive("next monday", dt(2013, 2, 18, 0, 0, 0), "day");
}

#[test]
fn test_time_at_noon() {
    check_time_naive("at 12pm", dt(2013, 2, 12, 12, 0, 0), "hour");
    check_time_naive("at noon", dt(2013, 2, 12, 12, 0, 0), "hour");
    check_time_naive("midday", dt(2013, 2, 12, 12, 0, 0), "hour");
    check_time_naive("the midday", dt(2013, 2, 12, 12, 0, 0), "hour");
    check_time_naive("mid day", dt(2013, 2, 12, 12, 0, 0), "hour");
}

#[test]
fn test_time_at_midnight() {
    check_time_naive("at 12am", dt(2013, 2, 13, 0, 0, 0), "hour");
    check_time_naive("at midnight", dt(2013, 2, 13, 0, 0, 0), "hour");
    check_time_naive("this morning at 12", dt(2013, 2, 13, 0, 0, 0), "hour");
    check_time_naive("this evening at 12", dt(2013, 2, 13, 0, 0, 0), "hour");
    check_time_naive("this afternoon at 12", dt(2013, 2, 13, 0, 0, 0), "hour");
}

#[test]
fn test_time_9_tomorrow_morning() {
    check_time_naive("9 tomorrow morning", dt(2013, 2, 13, 9, 0, 0), "hour");
    check_time_naive("9 tomorrow", dt(2013, 2, 13, 9, 0, 0), "hour");
}

#[test]
fn test_time_9_tomorrow_evening() {
    check_time_naive("9 tomorrow evening", dt(2013, 2, 13, 21, 0, 0), "hour");
}

#[test]
fn test_time_march() {
    check_time_naive("in March", dt(2013, 3, 1, 0, 0, 0), "month");
    check_time_naive("during March", dt(2013, 3, 1, 0, 0, 0), "month");
}

#[test]
fn test_may_and_march_as_verbs() {
    // "may" and "march" should not be parsed as months when used as verbs
    let results = parse_time("I may be incomplete or out of date, and we march to the beat.");
    let has_time = results
        .iter()
        .any(|e| matches!(&e.value, DimensionValue::Time(_)));
    assert!(
        !has_time,
        "Should not parse 'may'/'march' as months in verb context, got: {:?}",
        results
    );
}

#[test]
fn test_may_and_march_as_months() {
    // But they should still work when used with date context
    check_time_naive("in May", dt(2013, 5, 1, 0, 0, 0), "month");
    check_time_naive("May 3rd", dt(2013, 5, 3, 0, 0, 0), "day");
    check_time_naive("last May", dt(2012, 5, 1, 0, 0, 0), "month");
    check_time_naive("next March", dt(2013, 3, 1, 0, 0, 0), "month");
    check_time_naive("March 15", dt(2013, 3, 15, 0, 0, 0), "day");
    check_time_naive("March 15, 2015", dt(2015, 3, 15, 0, 0, 0), "day");
}

#[test]
fn test_iso_datetime_no_spurious_interval() {
    // "2025-03-17 at 4.35.24 PM" should parse as a single datetime with no spurious interval
    check_time_naive(
        "2025-03-17 at 4.35.24 PM",
        dt(2025, 3, 17, 16, 35, 24),
        "second",
    );
    let results = parse_time("2025-03-17 at 4.35.24 PM");
    let has_interval = results
        .iter()
        .any(|e| matches!(&e.value, DimensionValue::Time(TimeValue::Interval { .. })));
    assert!(
        !has_interval,
        "Should not produce spurious interval, got: {:?}",
        results
    );
}

#[test]
#[ignore = "debug helper that intentionally panics"]
fn test_debug_issues() {
    let r1 = parse_time("Apr 1 2018");
    eprintln!("'Apr 1 2018': {:?}", r1);
    let r2 = parse_time("Apr 1");
    eprintln!("'Apr 1': {:?}", r2);
    let r3 = parse_time("Apr 1 2018 at 6:03pm");
    eprintln!("'Apr 1 2018 at 6:03pm': {:?}", r3);
    let r4 = parse_time("April 1, 2018 6:03pm");
    eprintln!("'April 1, 2018 6:03pm': {:?}", r4);
    let r5 = parse_time("last April 1");
    eprintln!("'last April 1': {:?}", r5);
    let r6 = parse_time("last April");
    eprintln!("'last April': {:?}", r6);
    panic!("debug output above");
}

#[test]
fn test_dot_separated_times() {
    // Dot separator support (matching Haskell's [:.] in time regexes)
    check_time_naive("4.35 PM", dt(2013, 2, 12, 16, 35, 0), "minute");
    check_time_naive("10.30", dt(2013, 2, 12, 10, 30, 0), "minute");
    check_time_naive("4.35.24 PM", dt(2013, 2, 12, 16, 35, 24), "second");
}

#[test]
fn test_iso_date_no_spurious_interval() {
    // "2018-04-01" should parse as a single date, not also as an interval
    let results = parse_time("On 2018-04-01 we met.");
    let date_count = results
        .iter()
        .filter(|e| matches!(&e.value, DimensionValue::Time(_)))
        .count();
    let has_correct_date = results.iter().any(|e| {
        matches!(&e.value, DimensionValue::Time(TimeValue::Single { value: TimePoint::Naive { value, grain: Grain::Day }, .. })
            if value.date() == chrono::NaiveDate::from_ymd_opt(2018, 4, 1).unwrap())
    });
    assert!(
        has_correct_date,
        "Expected date 2018-04-01, got: {:?}",
        results
    );
    assert_eq!(
        date_count, 1,
        "Expected exactly 1 time entity (no spurious interval), got: {:?}",
        results
    );
}

#[test]
fn test_iso8601_t_separator_z_parses_as_single_datetime() {
    let text = "2018-04-01T18:03:40Z";
    let locale = Locale::new(Lang::EN, None);
    let context = context_at(local_datetime(0, 2013, 2, 12, 4, 30, 0), locale);
    let options = Options::default();
    let entities = parse(text, &locale, &[DimensionKind::Time], &context, &options);
    let found = entities.iter().any(|e| {
        matches!(
            &e.value,
            DimensionValue::Time(TimeValue::Single { value: TimePoint::Instant { value, grain: Grain::Minute }, .. })
                if value.naive_local() == dt(2018, 4, 1, 18, 3, 40)
        )
    });
    assert!(
        found,
        "Expected ISO-8601 datetime parse for {:?}, got: {:?}",
        text, entities
    );
}

#[test]
fn test_time_additional_regression_inputs() {
    let should_parse = [
        "On 2018-04-01 we met.",
        "2018-04-01 18:03",
        "April 1, 2018",
        "1 April 2018",
        "last April",
        "next April 1",
        "April 1",
        "in 2 days",
        "April 1, 2018 6:03pm",
        "end of day",
        "this weekend",
        "Tuesday, March 11, 2025 at 8:15 PM\n(773) 348-8886\nlocation: 2300 N. Lincoln Park West  Chicago, IL United States 60614",
    ];
    for text in should_parse {
        let results = parse_time(text);
        assert!(
            !results.is_empty(),
            "Expected at least one time parse for {:?}, got none",
            text
        );
    }

    // Duration-like values under Time-only parsing: ensure they are exercised.
    let _ = parse_time("3 hours");
    let _ = parse_time("3h");
}

#[test]
fn test_time_additional_regression_inputs_extreme_values() {
    let _ = parse_time("in 999999999 months");
    let _ = parse_time("in 9999999999999999 days");
}

#[test]
fn test_last_april_first_is_previous_occurrence() {
    // DateMDY without a year is a cyclic form (like DayOfWeek/Month), so
    // ok_for_this_next is true and "last" attaches correctly.
    check_time_naive("last April 1", dt(2012, 4, 1, 0, 0, 0), "day");
}

#[test]
fn test_last_april_first_march_2018_ref() {
    // Regression: with ref time in March 2018, "last April 1" should resolve
    // to 2017-04-01 (the most recent past April 1), not 2018-04-01.
    let locale = Locale::new(Lang::EN, None);
    let context = context_at(local_datetime(0, 2018, 3, 15, 12, 0, 0), locale);
    let options = Options::default();
    let entities = parse(
        "last April 1",
        &locale,
        &[DimensionKind::Time],
        &context,
        &options,
    );
    let expected = dt(2017, 4, 1, 0, 0, 0);
    let found = entities.iter().any(|e| {
        matches!(&e.value, DimensionValue::Time(TimeValue::Single { value: TimePoint::Naive { value, grain: Grain::Day }, .. }) if *value == expected)
    });
    assert!(
        found,
        "Expected 'last April 1' to resolve to 2017-04-01 with March 2018 ref, got: {:?}",
        entities
            .iter()
            .map(|e| format!("{:?}", e.value))
            .collect::<Vec<_>>()
    );
}

#[test]
fn test_time_tomorrow_afternoon_at_5() {
    check_time_naive("tomorrow afternoon at 5", dt(2013, 2, 13, 17, 0, 0), "hour");
    check_time_naive("at 5 tomorrow afternoon", dt(2013, 2, 13, 17, 0, 0), "hour");
    check_time_naive("at 5pm tomorrow", dt(2013, 2, 13, 17, 0, 0), "hour");
    check_time_naive("tomorrow at 5pm", dt(2013, 2, 13, 17, 0, 0), "hour");
    check_time_naive("tomorrow evening at 5", dt(2013, 2, 13, 17, 0, 0), "hour");
}

#[test]
fn test_time_tomorrow_afternoon() {
    check_time_interval(
        "tomorrow afternoon",
        dt(2013, 2, 13, 12, 0, 0),
        dt(2013, 2, 13, 19, 0, 0),
        "hour",
    );
    check_time_interval(
        "tomorrow afternoonish",
        dt(2013, 2, 13, 12, 0, 0),
        dt(2013, 2, 13, 19, 0, 0),
        "hour",
    );
}

#[test]
fn test_time_1pm_to_2pm_tomorrow() {
    check_time_interval(
        "1pm-2pm tomorrow",
        dt(2013, 2, 13, 13, 0, 0),
        dt(2013, 2, 13, 15, 0, 0),
        "hour",
    );
}

#[test]
fn test_time_on_the_first() {
    check_time_naive("on the first", dt(2013, 3, 1, 0, 0, 0), "day");
    check_time_naive("the 1st", dt(2013, 3, 1, 0, 0, 0), "day");
}

#[test]
fn test_time_at_1030_am() {
    check_time_naive("at 1030", dt(2013, 2, 12, 10, 30, 0), "minute");
    check_time_naive("around 1030", dt(2013, 2, 12, 10, 30, 0), "minute");
    check_time_naive("ten thirty am", dt(2013, 2, 12, 10, 30, 0), "minute");
}

#[test]
fn test_time_730_in_the_evening() {
    check_time_naive(
        "at 730 in the evening",
        dt(2013, 2, 12, 19, 30, 0),
        "minute",
    );
    check_time_naive("seven thirty p.m.", dt(2013, 2, 12, 19, 30, 0), "minute");
}

#[test]
fn test_time_tomorrow_at_150ish() {
    check_time_naive("tomorrow at 150ish", dt(2013, 2, 13, 1, 50, 0), "minute");
}

#[test]
fn test_time_tonight_at_11() {
    check_time_naive("tonight at 11", dt(2013, 2, 12, 23, 0, 0), "hour");
    check_time_naive("this evening at 11", dt(2013, 2, 12, 23, 0, 0), "hour");
    check_time_naive("this afternoon at 11", dt(2013, 2, 12, 23, 0, 0), "hour");
    check_time_naive("tonight at 11pm", dt(2013, 2, 12, 23, 0, 0), "hour");
}

#[test]
fn test_time_at_423() {
    check_time_naive("at 4:23", dt(2013, 2, 12, 4, 23, 0), "minute");
    check_time_naive("4:23am", dt(2013, 2, 12, 4, 23, 0), "minute");
    check_time_naive("four twenty-three a m", dt(2013, 2, 12, 4, 23, 0), "minute");
}

#[test]
fn test_time_closest_monday_to_oct_5th() {
    check_time_naive(
        "the closest Monday to Oct 5th",
        dt(2013, 10, 7, 0, 0, 0),
        "day",
    );
}

#[test]
fn test_time_second_closest_monday_to_oct_5th() {
    check_time_naive(
        "the second closest Mon to October fifth",
        dt(2013, 9, 30, 0, 0, 0),
        "day",
    );
}

#[test]
fn test_time_early_march() {
    check_time_interval(
        "early March",
        dt(2013, 3, 1, 0, 0, 0),
        dt(2013, 3, 11, 0, 0, 0),
        "day",
    );
}

#[test]
fn test_time_mid_march() {
    check_time_interval(
        "mid March",
        dt(2013, 3, 11, 0, 0, 0),
        dt(2013, 3, 21, 0, 0, 0),
        "day",
    );
}

#[test]
fn test_time_late_march() {
    check_time_interval(
        "late March",
        dt(2013, 3, 21, 0, 0, 0),
        dt(2013, 4, 1, 0, 0, 0),
        "day",
    );
}

#[test]
fn test_time_last_weekend_of_october() {
    check_time_interval(
        "last weekend of October",
        dt(2013, 10, 25, 18, 0, 0),
        dt(2013, 10, 28, 0, 0, 0),
        "hour",
    );
    check_time_interval(
        "last week-end in October",
        dt(2013, 10, 25, 18, 0, 0),
        dt(2013, 10, 28, 0, 0, 0),
        "hour",
    );
    check_time_interval(
        "last week end of October",
        dt(2013, 10, 25, 18, 0, 0),
        dt(2013, 10, 28, 0, 0, 0),
        "hour",
    );
}

#[test]
fn test_time_all_week() {
    check_time_interval(
        "all week",
        dt(2013, 2, 11, 0, 0, 0),
        dt(2013, 2, 17, 0, 0, 0),
        "day",
    );
}

#[test]
fn test_time_rest_of_the_week() {
    check_time_interval(
        "rest of the week",
        dt(2013, 2, 12, 0, 0, 0),
        dt(2013, 2, 17, 0, 0, 0),
        "day",
    );
}

#[test]
fn test_time_last_weekend_of_july() {
    check_time_interval(
        "last wkend of July",
        dt(2013, 7, 26, 18, 0, 0),
        dt(2013, 7, 29, 0, 0, 0),
        "hour",
    );
}

#[test]
fn test_time_last_weekend_of_october_2017() {
    check_time_interval(
        "last weekend of October 2017",
        dt(2017, 10, 27, 18, 0, 0),
        dt(2017, 10, 30, 0, 0, 0),
        "hour",
    );
}

#[test]
fn test_time_august_27th_to_29th() {
    check_time_interval(
        "August 27th - 29th",
        dt(2013, 8, 27, 0, 0, 0),
        dt(2013, 8, 30, 0, 0, 0),
        "day",
    );
    check_time_interval(
        "from August 27th - 29th",
        dt(2013, 8, 27, 0, 0, 0),
        dt(2013, 8, 30, 0, 0, 0),
        "day",
    );
}

#[test]
fn test_time_23rd_to_26th_oct() {
    check_time_interval(
        "23rd to 26th Oct",
        dt(2013, 10, 23, 0, 0, 0),
        dt(2013, 10, 27, 0, 0, 0),
        "day",
    );
}

#[test]
fn test_time_1_to_8_september() {
    check_time_interval(
        "1-8 september",
        dt(2013, 9, 1, 0, 0, 0),
        dt(2013, 9, 9, 0, 0, 0),
        "day",
    );
}

#[test]
fn test_time_12_to_16_september() {
    check_time_interval(
        "12 to 16 september",
        dt(2013, 9, 12, 0, 0, 0),
        dt(2013, 9, 17, 0, 0, 0),
        "day",
    );
}

#[test]
fn test_time_19th_to_21st_aug() {
    check_time_interval(
        "19th To 21st aug",
        dt(2013, 8, 19, 0, 0, 0),
        dt(2013, 8, 22, 0, 0, 0),
        "day",
    );
}

#[test]
fn test_time_end_of_april() {
    check_time_interval(
        "end of April",
        dt(2013, 4, 21, 0, 0, 0),
        dt(2013, 5, 1, 0, 0, 0),
        "day",
    );
    check_time_interval(
        "at the end of April",
        dt(2013, 4, 21, 0, 0, 0),
        dt(2013, 5, 1, 0, 0, 0),
        "day",
    );
}

#[test]
fn test_time_beginning_of_january() {
    check_time_interval(
        "beginning of January",
        dt(2014, 1, 1, 0, 0, 0),
        dt(2014, 1, 11, 0, 0, 0),
        "day",
    );
    check_time_interval(
        "at the beginning of January",
        dt(2014, 1, 1, 0, 0, 0),
        dt(2014, 1, 11, 0, 0, 0),
        "day",
    );
}

#[test]
fn test_time_end_of_2012() {
    check_time_interval(
        "end of 2012",
        dt(2012, 9, 1, 0, 0, 0),
        dt(2013, 1, 1, 0, 0, 0),
        "month",
    );
    check_time_interval(
        "at the end of 2012",
        dt(2012, 9, 1, 0, 0, 0),
        dt(2013, 1, 1, 0, 0, 0),
        "month",
    );
}

#[test]
fn test_time_beginning_of_2017() {
    check_time_interval(
        "beginning of 2017",
        dt(2017, 1, 1, 0, 0, 0),
        dt(2017, 4, 1, 0, 0, 0),
        "month",
    );
    check_time_interval(
        "at the beginning of 2017",
        dt(2017, 1, 1, 0, 0, 0),
        dt(2017, 4, 1, 0, 0, 0),
        "month",
    );
}

#[test]
fn test_time_beginning_of_year() {
    check_time_interval(
        "beginning of year",
        dt(2013, 1, 1, 0, 0, 0),
        dt(2013, 4, 1, 0, 0, 0),
        "month",
    );
    check_time_interval(
        "the beginning of the year",
        dt(2013, 1, 1, 0, 0, 0),
        dt(2013, 4, 1, 0, 0, 0),
        "month",
    );
    check_time_interval(
        "the BOY",
        dt(2013, 1, 1, 0, 0, 0),
        dt(2013, 4, 1, 0, 0, 0),
        "month",
    );
    check_time_interval(
        "BOY",
        dt(2013, 1, 1, 0, 0, 0),
        dt(2013, 4, 1, 0, 0, 0),
        "month",
    );
}

#[test]
fn test_time_by_eoy() {
    check_time_interval(
        "by EOY",
        dt(2013, 2, 12, 4, 30, 0),
        dt(2014, 1, 1, 0, 0, 0),
        "second",
    );
    check_time_interval(
        "by the EOY",
        dt(2013, 2, 12, 4, 30, 0),
        dt(2014, 1, 1, 0, 0, 0),
        "second",
    );
    check_time_interval(
        "by end of the year",
        dt(2013, 2, 12, 4, 30, 0),
        dt(2014, 1, 1, 0, 0, 0),
        "second",
    );
    check_time_interval(
        "by the end of year",
        dt(2013, 2, 12, 4, 30, 0),
        dt(2014, 1, 1, 0, 0, 0),
        "second",
    );
}

#[test]
fn test_time_eoy() {
    check_time_interval(
        "EOY",
        dt(2013, 9, 1, 0, 0, 0),
        dt(2014, 1, 1, 0, 0, 0),
        "month",
    );
    check_time_interval(
        "the EOY",
        dt(2013, 9, 1, 0, 0, 0),
        dt(2014, 1, 1, 0, 0, 0),
        "month",
    );
    check_time_interval(
        "at the EOY",
        dt(2013, 9, 1, 0, 0, 0),
        dt(2014, 1, 1, 0, 0, 0),
        "month",
    );
    check_time_interval(
        "the end of the year",
        dt(2013, 9, 1, 0, 0, 0),
        dt(2014, 1, 1, 0, 0, 0),
        "month",
    );
    check_time_interval(
        "end of the year",
        dt(2013, 9, 1, 0, 0, 0),
        dt(2014, 1, 1, 0, 0, 0),
        "month",
    );
    check_time_interval(
        "at the end of year",
        dt(2013, 9, 1, 0, 0, 0),
        dt(2014, 1, 1, 0, 0, 0),
        "month",
    );
}

#[test]
fn test_time_beginning_of_this_week() {
    check_time_interval(
        "beginning of this week",
        dt(2013, 2, 11, 0, 0, 0),
        dt(2013, 2, 14, 0, 0, 0),
        "day",
    );
    check_time_interval(
        "beginning of current week",
        dt(2013, 2, 11, 0, 0, 0),
        dt(2013, 2, 14, 0, 0, 0),
        "day",
    );
    check_time_interval(
        "at the beginning of this week",
        dt(2013, 2, 11, 0, 0, 0),
        dt(2013, 2, 14, 0, 0, 0),
        "day",
    );
    check_time_interval(
        "at the beginning of current week",
        dt(2013, 2, 11, 0, 0, 0),
        dt(2013, 2, 14, 0, 0, 0),
        "day",
    );
}

#[test]
fn test_time_beginning_of_coming_week() {
    check_time_interval(
        "beginning of coming week",
        dt(2013, 2, 18, 0, 0, 0),
        dt(2013, 2, 21, 0, 0, 0),
        "day",
    );
    check_time_interval(
        "at the beginning of coming week",
        dt(2013, 2, 18, 0, 0, 0),
        dt(2013, 2, 21, 0, 0, 0),
        "day",
    );
}

#[test]
fn test_time_beginning_of_last_week() {
    check_time_interval(
        "beginning of last week",
        dt(2013, 2, 4, 0, 0, 0),
        dt(2013, 2, 7, 0, 0, 0),
        "day",
    );
    check_time_interval(
        "beginning of past week",
        dt(2013, 2, 4, 0, 0, 0),
        dt(2013, 2, 7, 0, 0, 0),
        "day",
    );
    check_time_interval(
        "beginning of previous week",
        dt(2013, 2, 4, 0, 0, 0),
        dt(2013, 2, 7, 0, 0, 0),
        "day",
    );
    check_time_interval(
        "at the beginning of last week",
        dt(2013, 2, 4, 0, 0, 0),
        dt(2013, 2, 7, 0, 0, 0),
        "day",
    );
    check_time_interval(
        "at the beginning of past week",
        dt(2013, 2, 4, 0, 0, 0),
        dt(2013, 2, 7, 0, 0, 0),
        "day",
    );
    check_time_interval(
        "at the beginning of previous week",
        dt(2013, 2, 4, 0, 0, 0),
        dt(2013, 2, 7, 0, 0, 0),
        "day",
    );
}

#[test]
fn test_time_beginning_of_next_week() {
    check_time_interval(
        "beginning of next week",
        dt(2013, 2, 18, 0, 0, 0),
        dt(2013, 2, 21, 0, 0, 0),
        "day",
    );
    check_time_interval(
        "beginning of the following week",
        dt(2013, 2, 18, 0, 0, 0),
        dt(2013, 2, 21, 0, 0, 0),
        "day",
    );
    check_time_interval(
        "beginning of around next week",
        dt(2013, 2, 18, 0, 0, 0),
        dt(2013, 2, 21, 0, 0, 0),
        "day",
    );
    check_time_interval(
        "at the beginning of next week",
        dt(2013, 2, 18, 0, 0, 0),
        dt(2013, 2, 21, 0, 0, 0),
        "day",
    );
    check_time_interval(
        "at the beginning of the following week",
        dt(2013, 2, 18, 0, 0, 0),
        dt(2013, 2, 21, 0, 0, 0),
        "day",
    );
    check_time_interval(
        "at the beginning of around next week",
        dt(2013, 2, 18, 0, 0, 0),
        dt(2013, 2, 21, 0, 0, 0),
        "day",
    );
}

#[test]
fn test_time_end_of_this_week() {
    check_time_interval(
        "end of this week",
        dt(2013, 2, 15, 0, 0, 0),
        dt(2013, 2, 18, 0, 0, 0),
        "day",
    );
    check_time_interval(
        "end of current week",
        dt(2013, 2, 15, 0, 0, 0),
        dt(2013, 2, 18, 0, 0, 0),
        "day",
    );
    check_time_interval(
        "at the end of this week",
        dt(2013, 2, 15, 0, 0, 0),
        dt(2013, 2, 18, 0, 0, 0),
        "day",
    );
    check_time_interval(
        "at the end of current week",
        dt(2013, 2, 15, 0, 0, 0),
        dt(2013, 2, 18, 0, 0, 0),
        "day",
    );
}

#[test]
fn test_time_end_of_coming_week() {
    check_time_interval(
        "end of coming week",
        dt(2013, 2, 22, 0, 0, 0),
        dt(2013, 2, 25, 0, 0, 0),
        "day",
    );
    check_time_interval(
        "at the end of coming week",
        dt(2013, 2, 22, 0, 0, 0),
        dt(2013, 2, 25, 0, 0, 0),
        "day",
    );
}

#[test]
fn test_time_end_of_last_week() {
    check_time_interval(
        "end of last week",
        dt(2013, 2, 8, 0, 0, 0),
        dt(2013, 2, 11, 0, 0, 0),
        "day",
    );
    check_time_interval(
        "end of past week",
        dt(2013, 2, 8, 0, 0, 0),
        dt(2013, 2, 11, 0, 0, 0),
        "day",
    );
    check_time_interval(
        "end of previous week",
        dt(2013, 2, 8, 0, 0, 0),
        dt(2013, 2, 11, 0, 0, 0),
        "day",
    );
    check_time_interval(
        "at the end of last week",
        dt(2013, 2, 8, 0, 0, 0),
        dt(2013, 2, 11, 0, 0, 0),
        "day",
    );
    check_time_interval(
        "at the end of past week",
        dt(2013, 2, 8, 0, 0, 0),
        dt(2013, 2, 11, 0, 0, 0),
        "day",
    );
    check_time_interval(
        "at the end of previous week",
        dt(2013, 2, 8, 0, 0, 0),
        dt(2013, 2, 11, 0, 0, 0),
        "day",
    );
}

#[test]
fn test_time_end_of_next_week() {
    check_time_interval(
        "end of next week",
        dt(2013, 2, 22, 0, 0, 0),
        dt(2013, 2, 25, 0, 0, 0),
        "day",
    );
    check_time_interval(
        "end of the following week",
        dt(2013, 2, 22, 0, 0, 0),
        dt(2013, 2, 25, 0, 0, 0),
        "day",
    );
    check_time_interval(
        "end of around next week",
        dt(2013, 2, 22, 0, 0, 0),
        dt(2013, 2, 25, 0, 0, 0),
        "day",
    );
    check_time_interval(
        "at the end of next week",
        dt(2013, 2, 22, 0, 0, 0),
        dt(2013, 2, 25, 0, 0, 0),
        "day",
    );
    check_time_interval(
        "at the end of the following week",
        dt(2013, 2, 22, 0, 0, 0),
        dt(2013, 2, 25, 0, 0, 0),
        "day",
    );
    check_time_interval(
        "at the end of around next week",
        dt(2013, 2, 22, 0, 0, 0),
        dt(2013, 2, 25, 0, 0, 0),
        "day",
    );
}

// ============================================================
// Remaining holidays (groups 297+)
// ============================================================

#[test]
fn test_chinese_new_year() {
    check_time_naive("chinese new year", dt(2014, 1, 31, 0, 0, 0), "day");
    check_time_naive(
        "chinese lunar new year's day",
        dt(2014, 1, 31, 0, 0, 0),
        "day",
    );
}

#[test]
fn test_last_chinese_new_year() {
    check_time_naive("last chinese new year", dt(2013, 2, 10, 0, 0, 0), "day");
    check_time_naive(
        "last chinese lunar new year's day",
        dt(2013, 2, 10, 0, 0, 0),
        "day",
    );
    check_time_naive("last chinese new years", dt(2013, 2, 10, 0, 0, 0), "day");
}

#[test]
fn test_chinese_new_year_2018() {
    check_time_naive(
        "chinese new year's day 2018",
        dt(2018, 2, 16, 0, 0, 0),
        "day",
    );
}

#[test]
fn test_yom_kippur_2018() {
    check_time_naive("yom kippur 2018", dt(2018, 9, 18, 0, 0, 0), "day");
}

#[test]
fn test_shemini_atzeret_2018() {
    check_time_naive("shemini atzeret 2018", dt(2018, 9, 30, 0, 0, 0), "day");
}

#[test]
fn test_simchat_torah_2018() {
    check_time_naive("simchat torah 2018", dt(2018, 10, 1, 0, 0, 0), "day");
}

#[test]
fn test_tisha_bav_2018() {
    check_time_naive("tisha b'av 2018", dt(2018, 7, 21, 0, 0, 0), "day");
}

#[test]
fn test_yom_haatzmaut_2018() {
    check_time_naive("yom haatzmaut 2018", dt(2018, 4, 18, 0, 0, 0), "day");
}

#[test]
fn test_lag_bomer_2017() {
    check_time_naive("lag b'omer 2017", dt(2017, 5, 13, 0, 0, 0), "day");
}

#[test]
fn test_yom_hashoah_2018() {
    check_time_naive("Yom Hashoah 2018", dt(2018, 4, 11, 0, 0, 0), "day");
    check_time_naive("Holocaust Day 2018", dt(2018, 4, 11, 0, 0, 0), "day");
}

#[test]
fn test_rosh_hashanah_2018() {
    check_time_interval(
        "rosh hashanah 2018",
        dt(2018, 9, 9, 0, 0, 0),
        dt(2018, 9, 12, 0, 0, 0),
        "day",
    );
    check_time_interval(
        "rosh hashana 2018",
        dt(2018, 9, 9, 0, 0, 0),
        dt(2018, 9, 12, 0, 0, 0),
        "day",
    );
    check_time_interval(
        "rosh hashanna 2018",
        dt(2018, 9, 9, 0, 0, 0),
        dt(2018, 9, 12, 0, 0, 0),
        "day",
    );
}

#[test]
fn test_chanukah_2018() {
    check_time_interval(
        "Chanukah 2018",
        dt(2018, 12, 2, 0, 0, 0),
        dt(2018, 12, 10, 0, 0, 0),
        "day",
    );
    check_time_interval(
        "hanukah 2018",
        dt(2018, 12, 2, 0, 0, 0),
        dt(2018, 12, 10, 0, 0, 0),
        "day",
    );
    check_time_interval(
        "hannukkah 2018",
        dt(2018, 12, 2, 0, 0, 0),
        dt(2018, 12, 10, 0, 0, 0),
        "day",
    );
}

#[test]
fn test_passover_2018() {
    check_time_interval(
        "passover 2018",
        dt(2018, 3, 30, 0, 0, 0),
        dt(2018, 4, 8, 0, 0, 0),
        "day",
    );
}

#[test]
fn test_sukkot_2018() {
    check_time_interval(
        "feast of the ingathering 2018",
        dt(2018, 9, 23, 0, 0, 0),
        dt(2018, 10, 2, 0, 0, 0),
        "day",
    );
    check_time_interval(
        "succos 2018",
        dt(2018, 9, 23, 0, 0, 0),
        dt(2018, 10, 2, 0, 0, 0),
        "day",
    );
}

#[test]
fn test_shavuot_2018() {
    check_time_interval(
        "shavuot 2018",
        dt(2018, 5, 19, 0, 0, 0),
        dt(2018, 5, 22, 0, 0, 0),
        "day",
    );
}

#[test]
fn test_mawlid_2017() {
    check_time_naive("mawlid al-nabawi 2017", dt(2017, 11, 30, 0, 0, 0), "day");
}

#[test]
fn test_eid_al_fitr_1950() {
    check_time_naive("Eid al-Fitr 1950", dt(1950, 7, 16, 0, 0, 0), "day");
}

#[test]
fn test_eid_al_fitr_1975() {
    check_time_naive("Eid al-Fitr 1975", dt(1975, 10, 6, 0, 0, 0), "day");
}

#[test]
fn test_eid_al_fitr_1988() {
    check_time_naive("Eid al-Fitr 1988", dt(1988, 5, 16, 0, 0, 0), "day");
}

#[test]
fn test_eid_al_fitr_2018() {
    check_time_naive("Eid al-Fitr 2018", dt(2018, 6, 15, 0, 0, 0), "day");
}

#[test]
fn test_eid_al_fitr_2034() {
    check_time_naive("Eid al-Fitr 2034", dt(2034, 12, 12, 0, 0, 0), "day");
}

#[test]
fn test_eid_al_fitr_2046() {
    check_time_naive("Eid al-Fitr 2046", dt(2046, 8, 4, 0, 0, 0), "day");
}

#[test]
fn test_eid_al_fitr_2050() {
    check_time_naive("Eid al-Fitr 2050", dt(2050, 6, 21, 0, 0, 0), "day");
}

#[test]
fn test_eid_al_adha_2018() {
    check_time_naive("Eid al-Adha 2018", dt(2018, 8, 21, 0, 0, 0), "day");
    check_time_naive("id ul-adha 2018", dt(2018, 8, 21, 0, 0, 0), "day");
    check_time_naive("sacrifice feast 2018", dt(2018, 8, 21, 0, 0, 0), "day");
    check_time_naive("Bakr Id 2018", dt(2018, 8, 21, 0, 0, 0), "day");
}

#[test]
fn test_eid_al_adha_1980() {
    check_time_naive("Eid al-Adha 1980", dt(1980, 10, 19, 0, 0, 0), "day");
    check_time_naive("id ul-adha 1980", dt(1980, 10, 19, 0, 0, 0), "day");
    check_time_naive("sacrifice feast 1980", dt(1980, 10, 19, 0, 0, 0), "day");
    check_time_naive("Bakr Id 1980", dt(1980, 10, 19, 0, 0, 0), "day");
}

#[test]
fn test_eid_al_adha_1966() {
    check_time_naive("Eid al-Adha 1966", dt(1966, 4, 1, 0, 0, 0), "day");
    check_time_naive("id ul-adha 1966", dt(1966, 4, 1, 0, 0, 0), "day");
    check_time_naive("sacrifice feast 1966", dt(1966, 4, 1, 0, 0, 0), "day");
    check_time_naive("Bakr Id 1966", dt(1966, 4, 1, 0, 0, 0), "day");
}

#[test]
fn test_eid_al_adha_1974() {
    check_time_naive("Eid al-Adha 1974", dt(1974, 1, 3, 0, 0, 0), "day");
    check_time_naive("id ul-adha 1974", dt(1974, 1, 3, 0, 0, 0), "day");
    check_time_naive("sacrifice feast 1974", dt(1974, 1, 3, 0, 0, 0), "day");
    check_time_naive("Bakr Id 1974", dt(1974, 1, 3, 0, 0, 0), "day");
}

#[test]
fn test_laylat_al_qadr_2017() {
    check_time_naive("laylat al kadr 2017", dt(2017, 6, 22, 0, 0, 0), "day");
    check_time_naive("night of measures 2017", dt(2017, 6, 22, 0, 0, 0), "day");
}

#[test]
fn test_laylat_al_qadr_2018() {
    check_time_naive("laylat al-qadr 2018", dt(2018, 6, 11, 0, 0, 0), "day");
    check_time_naive("night of power 2018", dt(2018, 6, 11, 0, 0, 0), "day");
}

#[test]
fn test_islamic_new_year_2018() {
    check_time_naive("Islamic New Year 2018", dt(2018, 9, 11, 0, 0, 0), "day");
    check_time_naive("Amun Jadid 2018", dt(2018, 9, 11, 0, 0, 0), "day");
}

#[test]
fn test_ashura_2017() {
    check_time_naive("day of Ashura 2017", dt(2017, 9, 30, 0, 0, 0), "day");
}

#[test]
fn test_tu_bishvat_2018() {
    check_time_naive("tu bishvat 2018", dt(2018, 1, 30, 0, 0, 0), "day");
}

#[test]
fn test_jamat_ul_vida_2017() {
    check_time_naive("Jamat Ul-Vida 2017", dt(2017, 6, 23, 0, 0, 0), "day");
    check_time_naive("Jumu'atul-Wida 2017", dt(2017, 6, 23, 0, 0, 0), "day");
}

#[test]
fn test_jamat_ul_vida_2018() {
    check_time_naive("Jamat Ul-Vida 2018", dt(2018, 6, 8, 0, 0, 0), "day");
    check_time_naive("Jumu'atul-Wida 2018", dt(2018, 6, 8, 0, 0, 0), "day");
}

#[test]
fn test_isra_and_miraj_2018() {
    check_time_naive("isra and mi'raj 2018", dt(2018, 4, 13, 0, 0, 0), "day");
    check_time_naive(
        "the prophet's ascension 2018",
        dt(2018, 4, 13, 0, 0, 0),
        "day",
    );
}

#[test]
fn test_night_journey_2019() {
    check_time_naive("the night journey 2019", dt(2019, 4, 3, 0, 0, 0), "day");
    check_time_naive("ascension to heaven 2019", dt(2019, 4, 3, 0, 0, 0), "day");
}

#[test]
fn test_ramadan_1950() {
    check_time_interval(
        "Ramadan 1950",
        dt(1950, 6, 17, 0, 0, 0),
        dt(1950, 7, 16, 0, 0, 0),
        "day",
    );
}

#[test]
fn test_ramadan_1977() {
    check_time_interval(
        "Ramadan 1977",
        dt(1977, 8, 15, 0, 0, 0),
        dt(1977, 9, 14, 0, 0, 0),
        "day",
    );
}

#[test]
fn test_ramadan_2018() {
    check_time_interval(
        "Ramadan 2018",
        dt(2018, 5, 16, 0, 0, 0),
        dt(2018, 6, 15, 0, 0, 0),
        "day",
    );
}

#[test]
fn test_ramadan_2034() {
    check_time_interval(
        "Ramadan 2034",
        dt(2034, 11, 12, 0, 0, 0),
        dt(2034, 12, 12, 0, 0, 0),
        "day",
    );
}

#[test]
fn test_ramadan_2046() {
    check_time_interval(
        "Ramadan 2046",
        dt(2046, 7, 5, 0, 0, 0),
        dt(2046, 8, 4, 0, 0, 0),
        "day",
    );
}

#[test]
fn test_ramadan_2050() {
    check_time_interval(
        "Ramadan 2050",
        dt(2050, 5, 22, 0, 0, 0),
        dt(2050, 6, 21, 0, 0, 0),
        "day",
    );
}

#[test]
fn test_dhanatrayodashi_2017() {
    check_time_naive("dhanatrayodashi in 2017", dt(2017, 10, 17, 0, 0, 0), "day");
}

#[test]
fn test_dhanteras_2019() {
    check_time_naive("dhanteras 2019", dt(2019, 10, 25, 0, 0, 0), "day");
}

#[test]
fn test_kali_chaudas_2019() {
    check_time_naive("kali chaudas 2019", dt(2019, 10, 26, 0, 0, 0), "day");
    check_time_naive(
        "choti diwali two thousand nineteen",
        dt(2019, 10, 26, 0, 0, 0),
        "day",
    );
}

#[test]
fn test_diwali_2019() {
    check_time_naive("diwali 2019", dt(2019, 10, 27, 0, 0, 0), "day");
    check_time_naive("Deepavali in 2019", dt(2019, 10, 27, 0, 0, 0), "day");
    check_time_naive(
        "Lakshmi Puja six years hence",
        dt(2019, 10, 27, 0, 0, 0),
        "day",
    );
}

#[test]
fn test_bhai_dooj_2019() {
    check_time_naive("bhai dooj 2019", dt(2019, 10, 29, 0, 0, 0), "day");
}

#[test]
fn test_chhath_2019() {
    check_time_naive("chhath 2019", dt(2019, 11, 2, 0, 0, 0), "day");
    check_time_naive("dala puja 2019", dt(2019, 11, 2, 0, 0, 0), "day");
    check_time_naive("Surya Shashthi in 2019", dt(2019, 11, 2, 0, 0, 0), "day");
}

#[test]
fn test_maha_saptami_2021() {
    check_time_naive("Maha Saptami 2021", dt(2021, 10, 12, 0, 0, 0), "day");
}

#[test]
fn test_dussehra_2018() {
    check_time_naive("Dussehra 2018", dt(2018, 10, 18, 0, 0, 0), "day");
    check_time_naive(
        "vijayadashami in five years",
        dt(2018, 10, 18, 0, 0, 0),
        "day",
    );
}

#[test]
fn test_navaratri_2018() {
    check_time_interval(
        "navaratri 2018",
        dt(2018, 10, 9, 0, 0, 0),
        dt(2018, 10, 19, 0, 0, 0),
        "day",
    );
    check_time_interval(
        "durga puja in 2018",
        dt(2018, 10, 9, 0, 0, 0),
        dt(2018, 10, 19, 0, 0, 0),
        "day",
    );
}

#[test]
fn test_karva_chauth_2018() {
    check_time_naive("karva chauth 2018", dt(2018, 10, 27, 0, 0, 0), "day");
    check_time_naive("karva chauth in 2018", dt(2018, 10, 27, 0, 0, 0), "day");
}

#[test]
fn test_ratha_yatra_2018() {
    check_time_naive("ratha-yatra 2018", dt(2018, 7, 14, 0, 0, 0), "day");
}

#[test]
fn test_rakhi_2018() {
    check_time_naive("rakhi 2018", dt(2018, 8, 26, 0, 0, 0), "day");
}

#[test]
fn test_mahavir_jayanti_2020() {
    check_time_naive("mahavir jayanti 2020", dt(2020, 4, 6, 0, 0, 0), "day");
    check_time_naive(
        "mahaveer janma kalyanak 2020",
        dt(2020, 4, 6, 0, 0, 0),
        "day",
    );
}

#[test]
fn test_maha_shivaratri_2020() {
    check_time_naive("maha shivaratri 2020", dt(2020, 2, 21, 0, 0, 0), "day");
}

#[test]
fn test_saraswati_jayanti_2018() {
    check_time_naive("saraswati jayanti 2018", dt(2018, 2, 10, 0, 0, 0), "day");
}

#[test]
fn test_pongal_2018() {
    check_time_naive("pongal 2018", dt(2018, 1, 14, 0, 0, 0), "day");
    check_time_naive("makara sankranthi 2018", dt(2018, 1, 14, 0, 0, 0), "day");
}

#[test]
fn test_bogi_pandigai_2018() {
    check_time_naive("bogi pandigai 2018", dt(2018, 1, 13, 0, 0, 0), "day");
}

#[test]
fn test_maattu_pongal_2018() {
    check_time_naive("maattu pongal 2018", dt(2018, 1, 15, 0, 0, 0), "day");
}

#[test]
fn test_kaanum_pongal_2018() {
    check_time_naive("kaanum pongal 2018", dt(2018, 1, 16, 0, 0, 0), "day");
    check_time_naive("kanni pongal 2018", dt(2018, 1, 16, 0, 0, 0), "day");
}

#[test]
fn test_makar_sankranti_2019() {
    check_time_naive("makar sankranti 2019", dt(2019, 1, 15, 0, 0, 0), "day");
    check_time_naive("maghi in 2019", dt(2019, 1, 15, 0, 0, 0), "day");
}

#[test]
fn test_vaisakhi_2018() {
    check_time_naive("Vaisakhi 2018", dt(2018, 4, 14, 0, 0, 0), "day");
    check_time_naive("baisakhi in 2018", dt(2018, 4, 14, 0, 0, 0), "day");
    check_time_naive("Vasakhi 2018", dt(2018, 4, 14, 0, 0, 0), "day");
    check_time_naive("vaishakhi 2018", dt(2018, 4, 14, 0, 0, 0), "day");
}

#[test]
fn test_onam_2018() {
    check_time_naive("onam 2018", dt(2018, 8, 24, 0, 0, 0), "day");
    check_time_naive("Thiru Onam 2018", dt(2018, 8, 24, 0, 0, 0), "day");
    check_time_naive("Thiruvonam 2018", dt(2018, 8, 24, 0, 0, 0), "day");
}

#[test]
fn test_vasant_panchami_2019() {
    check_time_naive("vasant panchami in 2019", dt(2019, 2, 10, 0, 0, 0), "day");
    check_time_naive("basant panchami 2019", dt(2019, 2, 10, 0, 0, 0), "day");
}

#[test]
fn test_chhoti_holi_2019() {
    check_time_naive("chhoti holi 2019", dt(2019, 3, 20, 0, 0, 0), "day");
    check_time_naive("holika dahan 2019", dt(2019, 3, 20, 0, 0, 0), "day");
    check_time_naive("kamudu pyre 2019", dt(2019, 3, 20, 0, 0, 0), "day");
}

#[test]
fn test_krishna_janmashtami_2019() {
    check_time_naive("krishna janmashtami 2019", dt(2019, 8, 23, 0, 0, 0), "day");
    check_time_naive("gokulashtami 2019", dt(2019, 8, 23, 0, 0, 0), "day");
}

#[test]
fn test_holi_2019() {
    check_time_naive("holi 2019", dt(2019, 3, 21, 0, 0, 0), "day");
    check_time_naive("dhulandi 2019", dt(2019, 3, 21, 0, 0, 0), "day");
    check_time_naive("phagwah 2019", dt(2019, 3, 21, 0, 0, 0), "day");
}

#[test]
fn test_parsi_new_year_2018() {
    check_time_naive("Parsi New Year 2018", dt(2018, 8, 17, 0, 0, 0), "day");
    check_time_naive("Jamshedi Navroz 2018", dt(2018, 8, 17, 0, 0, 0), "day");
}

#[test]
fn test_parsi_new_year_2022() {
    check_time_naive("jamshedi Navroz 2022", dt(2022, 8, 16, 0, 0, 0), "day");
    check_time_naive("parsi new year 2022", dt(2022, 8, 16, 0, 0, 0), "day");
}

#[test]
fn test_gysd_2013() {
    check_time_interval(
        "GYSD 2013",
        dt(2013, 4, 26, 0, 0, 0),
        dt(2013, 4, 29, 0, 0, 0),
        "day",
    );
    check_time_interval(
        "global youth service day",
        dt(2013, 4, 26, 0, 0, 0),
        dt(2013, 4, 29, 0, 0, 0),
        "day",
    );
}

#[test]
fn test_vesak() {
    check_time_naive("vesak", dt(2013, 5, 24, 0, 0, 0), "day");
    check_time_naive("vaisakha", dt(2013, 5, 24, 0, 0, 0), "day");
    check_time_naive("Buddha day", dt(2013, 5, 24, 0, 0, 0), "day");
    check_time_naive("Buddha Purnima", dt(2013, 5, 24, 0, 0, 0), "day");
}

#[test]
fn test_earth_hour() {
    check_time_interval(
        "earth hour",
        dt(2013, 3, 23, 20, 30, 0),
        dt(2013, 3, 23, 21, 31, 0),
        "minute",
    );
}

#[test]
fn test_earth_hour_2016() {
    check_time_interval(
        "earth hour 2016",
        dt(2016, 3, 19, 20, 30, 0),
        dt(2016, 3, 19, 21, 31, 0),
        "minute",
    );
}

#[test]
fn test_purim() {
    check_time_naive("purim", dt(2013, 2, 23, 0, 0, 0), "day");
}

#[test]
fn test_shushan_purim() {
    check_time_naive("Shushan Purim", dt(2013, 2, 24, 0, 0, 0), "day");
}

#[test]
fn test_guru_gobind_singh_jayanti() {
    check_time_naive("guru gobind singh birthday", dt(2014, 1, 7, 0, 0, 0), "day");
    check_time_naive(
        "guru gobind singh jayanti 2014",
        dt(2014, 1, 7, 0, 0, 0),
        "day",
    );
    check_time_naive("guru gobind singh jayanti", dt(2014, 1, 7, 0, 0, 0), "day");
    check_time_naive("Guru Govind Singh Jayanti", dt(2014, 1, 7, 0, 0, 0), "day");
}

#[test]
fn test_koningsdag_2018() {
    check_time_naive("Koningsdag 2018", dt(2018, 4, 27, 0, 0, 0), "day");
    check_time_naive("koningsdag 2018", dt(2018, 4, 27, 0, 0, 0), "day");
    check_time_naive("king's day 2018", dt(2018, 4, 27, 0, 0, 0), "day");
    check_time_naive("King's Day 2018", dt(2018, 4, 27, 0, 0, 0), "day");
}

#[test]
fn test_koningsdag_2014() {
    check_time_naive("Koningsdag 2014", dt(2014, 4, 26, 0, 0, 0), "day");
    check_time_naive("koningsdag 2014", dt(2014, 4, 26, 0, 0, 0), "day");
    check_time_naive("King's Day 2014", dt(2014, 4, 26, 0, 0, 0), "day");
    check_time_naive("king's day 2014", dt(2014, 4, 26, 0, 0, 0), "day");
}

#[test]
fn test_rabindra_jayanti_2018() {
    check_time_naive("rabindra jayanti 2018", dt(2018, 5, 9, 0, 0, 0), "day");
    check_time_naive("Rabindranath Jayanti 2018", dt(2018, 5, 9, 0, 0, 0), "day");
    check_time_naive("Rabindra Jayanti 2018", dt(2018, 5, 9, 0, 0, 0), "day");
}

#[test]
fn test_rabindra_jayanti_2019() {
    check_time_naive("rabindra jayanti 2019", dt(2019, 5, 9, 0, 0, 0), "day");
    check_time_naive("Rabindranath Jayanti 2019", dt(2019, 5, 9, 0, 0, 0), "day");
    check_time_naive("Rabindra Jayanti 2019", dt(2019, 5, 9, 0, 0, 0), "day");
}

#[test]
fn test_guru_ravidas_jayanti_2018() {
    check_time_naive("guru Ravidas jayanti 2018", dt(2018, 1, 31, 0, 0, 0), "day");
    check_time_naive(
        "Guru Ravidass birthday 2018",
        dt(2018, 1, 31, 0, 0, 0),
        "day",
    );
    check_time_naive(
        "guru ravidass Jayanti 2018",
        dt(2018, 1, 31, 0, 0, 0),
        "day",
    );
}

#[test]
fn test_guru_ravidas_jayanti_2019() {
    check_time_naive(
        "Guru Ravidass Jayanti 2019",
        dt(2019, 2, 19, 0, 0, 0),
        "day",
    );
    check_time_naive(
        "Guru Ravidas Birthday 2019",
        dt(2019, 2, 19, 0, 0, 0),
        "day",
    );
    check_time_naive("guru ravidas jayanti 2019", dt(2019, 2, 19, 0, 0, 0), "day");
}

#[test]
fn test_valmiki_jayanti_2019() {
    check_time_naive("valmiki jayanti 2019", dt(2019, 10, 13, 0, 0, 0), "day");
    check_time_naive("Valmiki Jayanti 2019", dt(2019, 10, 13, 0, 0, 0), "day");
    check_time_naive("pargat diwas 2019", dt(2019, 10, 13, 0, 0, 0), "day");
}

#[test]
fn test_valmiki_jayanti_2018() {
    check_time_naive(
        "maharishi valmiki jayanti 2018",
        dt(2018, 10, 24, 0, 0, 0),
        "day",
    );
    check_time_naive("pargat diwas 2018", dt(2018, 10, 24, 0, 0, 0), "day");
    check_time_naive("Pargat Diwas 2018", dt(2018, 10, 24, 0, 0, 0), "day");
}

#[test]
fn test_ganesh_chaturthi_2019() {
    check_time_naive("ganesh chaturthi 2019", dt(2019, 9, 2, 0, 0, 0), "day");
}

#[test]
fn test_rama_navami_2020() {
    check_time_naive("rama navami 2020", dt(2020, 4, 2, 0, 0, 0), "day");
}

#[test]
fn test_ugadi_2018() {
    check_time_naive("Ugadi 2018", dt(2018, 3, 18, 0, 0, 0), "day");
    check_time_naive("ugadi 2018", dt(2018, 3, 18, 0, 0, 0), "day");
    check_time_naive("yugadi 2018", dt(2018, 3, 18, 0, 0, 0), "day");
    check_time_naive("Yugadi 2018", dt(2018, 3, 18, 0, 0, 0), "day");
    check_time_naive("samvatsaradi 2018", dt(2018, 3, 18, 0, 0, 0), "day");
    check_time_naive("chaitra sukladi 2018", dt(2018, 3, 18, 0, 0, 0), "day");
    check_time_naive("chaitra sukhladi 2018", dt(2018, 3, 18, 0, 0, 0), "day");
}

#[test]
fn test_closest_xmas() {
    check_time_naive(
        "the closest xmas to today",
        dt(2012, 12, 25, 0, 0, 0),
        "day",
    );
}

#[test]
fn test_second_closest_xmas() {
    check_time_naive(
        "the second closest xmas to today",
        dt(2013, 12, 25, 0, 0, 0),
        "day",
    );
}

#[test]
fn test_3rd_closest_xmas() {
    check_time_naive(
        "the 3rd closest xmas to today",
        dt(2011, 12, 25, 0, 0, 0),
        "day",
    );
}

#[test]
fn test_last_friday_of_october() {
    check_time_naive("last friday of october", dt(2013, 10, 25, 0, 0, 0), "day");
    check_time_naive("last friday in october", dt(2013, 10, 25, 0, 0, 0), "day");
}

#[test]
fn test_upcoming_two_weeks() {
    check_time_naive("upcoming two weeks", dt(2013, 2, 25, 0, 0, 0), "week");
    check_time_naive("upcoming two week", dt(2013, 2, 25, 0, 0, 0), "week");
    check_time_naive("upcoming 2 weeks", dt(2013, 2, 25, 0, 0, 0), "week");
    check_time_naive("upcoming 2 week", dt(2013, 2, 25, 0, 0, 0), "week");
    check_time_naive("two upcoming weeks", dt(2013, 2, 25, 0, 0, 0), "week");
    check_time_naive("two upcoming week", dt(2013, 2, 25, 0, 0, 0), "week");
    check_time_naive("2 upcoming weeks", dt(2013, 2, 25, 0, 0, 0), "week");
    check_time_naive("2 upcoming week", dt(2013, 2, 25, 0, 0, 0), "week");
}

#[test]
fn test_upcoming_two_days() {
    check_time_naive("upcoming two days", dt(2013, 2, 14, 0, 0, 0), "day");
    check_time_naive("upcoming two day", dt(2013, 2, 14, 0, 0, 0), "day");
    check_time_naive("upcoming 2 days", dt(2013, 2, 14, 0, 0, 0), "day");
    check_time_naive("upcoming 2 day", dt(2013, 2, 14, 0, 0, 0), "day");
    check_time_naive("two upcoming days", dt(2013, 2, 14, 0, 0, 0), "day");
    check_time_naive("two upcoming day", dt(2013, 2, 14, 0, 0, 0), "day");
    check_time_naive("2 upcoming days", dt(2013, 2, 14, 0, 0, 0), "day");
    check_time_naive("2 upcoming day", dt(2013, 2, 14, 0, 0, 0), "day");
}

#[test]
fn test_upcoming_two_months() {
    check_time_naive("upcoming two months", dt(2013, 4, 1, 0, 0, 0), "month");
    check_time_naive("upcoming two month", dt(2013, 4, 1, 0, 0, 0), "month");
    check_time_naive("upcoming 2 months", dt(2013, 4, 1, 0, 0, 0), "month");
    check_time_naive("upcoming 2 month", dt(2013, 4, 1, 0, 0, 0), "month");
    check_time_naive("two upcoming months", dt(2013, 4, 1, 0, 0, 0), "month");
    check_time_naive("two upcoming month", dt(2013, 4, 1, 0, 0, 0), "month");
    check_time_naive("2 upcoming months", dt(2013, 4, 1, 0, 0, 0), "month");
    check_time_naive("2 upcoming month", dt(2013, 4, 1, 0, 0, 0), "month");
}

#[test]
fn test_upcoming_two_quarters() {
    check_time_naive("upcoming two quarters", dt(2013, 7, 1, 0, 0, 0), "quarter");
    check_time_naive("upcoming two quarter", dt(2013, 7, 1, 0, 0, 0), "quarter");
    check_time_naive("upcoming 2 quarters", dt(2013, 7, 1, 0, 0, 0), "quarter");
    check_time_naive("upcoming 2 quarter", dt(2013, 7, 1, 0, 0, 0), "quarter");
    check_time_naive("two upcoming quarters", dt(2013, 7, 1, 0, 0, 0), "quarter");
    check_time_naive("two upcoming quarter", dt(2013, 7, 1, 0, 0, 0), "quarter");
    check_time_naive("2 upcoming quarters", dt(2013, 7, 1, 0, 0, 0), "quarter");
    check_time_naive("2 upcoming quarter", dt(2013, 7, 1, 0, 0, 0), "quarter");
}

#[test]
fn test_upcoming_two_years() {
    check_time_naive("upcoming two years", dt(2015, 1, 1, 0, 0, 0), "year");
    check_time_naive("upcoming two year", dt(2015, 1, 1, 0, 0, 0), "year");
    check_time_naive("upcoming 2 years", dt(2015, 1, 1, 0, 0, 0), "year");
    check_time_naive("upcoming 2 year", dt(2015, 1, 1, 0, 0, 0), "year");
    check_time_naive("two upcoming years", dt(2015, 1, 1, 0, 0, 0), "year");
    check_time_naive("two upcoming year", dt(2015, 1, 1, 0, 0, 0), "year");
    check_time_naive("2 upcoming years", dt(2015, 1, 1, 0, 0, 0), "year");
    check_time_naive("2 upcoming year", dt(2015, 1, 1, 0, 0, 0), "year");
}

#[test]
fn test_20_minutes_to_2pm_tomorrow() {
    check_time_naive(
        "20 minutes to 2pm tomorrow",
        dt(2013, 2, 13, 13, 40, 0),
        "minute",
    );
}

#[test]
fn test_first_monday_of_last_month() {
    check_time_naive("first monday of last month", dt(2013, 1, 7, 0, 0, 0), "day");
}

#[test]
fn test_first_tuesday_of_last_month() {
    check_time_naive(
        "first tuesday of last month",
        dt(2013, 1, 1, 0, 0, 0),
        "day",
    );
}

#[test]
fn test_second_monday_of_last_month() {
    check_time_naive(
        "second monday of last month",
        dt(2013, 1, 14, 0, 0, 0),
        "day",
    );
}

#[test]
fn test_next_saturday() {
    check_time_naive("next saturday", dt(2013, 2, 23, 0, 0, 0), "day");
}

#[test]
fn test_next_monday() {
    check_time_naive("next monday", dt(2013, 2, 18, 0, 0, 0), "day");
}

// ============================================================
// defaultCorpus additions (US date format tests + Thanksgiving)
// ============================================================

#[test]
fn test_us_date_format_2_15() {
    check_time_naive("2/15", dt(2013, 2, 15, 0, 0, 0), "day");
    check_time_naive("on 2/15", dt(2013, 2, 15, 0, 0, 0), "day");
    check_time_naive("2 / 15", dt(2013, 2, 15, 0, 0, 0), "day");
    check_time_naive("2-15", dt(2013, 2, 15, 0, 0, 0), "day");
    check_time_naive("2 - 15", dt(2013, 2, 15, 0, 0, 0), "day");
}

#[test]
fn test_us_date_format_10_31_1974() {
    check_time_naive("10/31/1974", dt(1974, 10, 31, 0, 0, 0), "day");
    check_time_naive("10/31/74", dt(1974, 10, 31, 0, 0, 0), "day");
    check_time_naive("10-31-74", dt(1974, 10, 31, 0, 0, 0), "day");
    check_time_naive("10.31.1974", dt(1974, 10, 31, 0, 0, 0), "day");
    check_time_naive("31/Oct/1974", dt(1974, 10, 31, 0, 0, 0), "day");
    check_time_naive("31-Oct-74", dt(1974, 10, 31, 0, 0, 0), "day");
    check_time_naive("31st Oct 1974", dt(1974, 10, 31, 0, 0, 0), "day");
}

#[test]
fn test_date_with_time_4_25() {
    check_time_naive("4/25 at 4:00pm", dt(2013, 4, 25, 16, 0, 0), "minute");
    check_time_naive("4/25 at 16h00", dt(2013, 4, 25, 16, 0, 0), "minute");
    check_time_naive("4/25 at 16h", dt(2013, 4, 25, 16, 0, 0), "minute");
}

#[test]
fn test_thanksgiving_2013() {
    check_time_naive("thanksgiving day", dt(2013, 11, 28, 0, 0, 0), "day");
    check_time_naive("thanksgiving", dt(2013, 11, 28, 0, 0, 0), "day");
    check_time_naive("thanksgiving 2013", dt(2013, 11, 28, 0, 0, 0), "day");
    check_time_naive("this thanksgiving", dt(2013, 11, 28, 0, 0, 0), "day");
    check_time_naive("next thanksgiving day", dt(2013, 11, 28, 0, 0, 0), "day");
    check_time_naive("thanksgiving in 9 months", dt(2013, 11, 28, 0, 0, 0), "day");
    check_time_naive(
        "thanksgiving 9 months from now",
        dt(2013, 11, 28, 0, 0, 0),
        "day",
    );
}

#[test]
fn test_thanksgiving_next_year() {
    check_time_naive(
        "thanksgiving of next year",
        dt(2014, 11, 27, 0, 0, 0),
        "day",
    );
    check_time_naive("thanksgiving in a year", dt(2014, 11, 27, 0, 0, 0), "day");
    check_time_naive("thanksgiving 2014", dt(2014, 11, 27, 0, 0, 0), "day");
}

#[test]
fn test_last_thanksgiving() {
    check_time_naive("last thanksgiving", dt(2012, 11, 22, 0, 0, 0), "day");
    check_time_naive("thanksgiving day 2012", dt(2012, 11, 22, 0, 0, 0), "day");
    check_time_naive(
        "thanksgiving 3 months ago",
        dt(2012, 11, 22, 0, 0, 0),
        "day",
    );
    check_time_naive("thanksgiving 1 year ago", dt(2012, 11, 22, 0, 0, 0), "day");
}

#[test]
fn test_thanksgiving_2016() {
    check_time_naive("thanksgiving 2016", dt(2016, 11, 24, 0, 0, 0), "day");
    check_time_naive("thanksgiving in 3 years", dt(2016, 11, 24, 0, 0, 0), "day");
}

#[test]
fn test_thanksgiving_2017() {
    check_time_naive("thanksgiving 2017", dt(2017, 11, 23, 0, 0, 0), "day");
}

// ============================================================
// negativeCorpus - should produce no time entities
// ============================================================

#[test]
fn test_negative_laughing_out_loud() {
    check_no_time("laughing out loud");
}

#[test]
fn test_negative_1_adult() {
    check_no_time("1 adult");
}

#[test]
fn test_negative_we_are_separated() {
    check_no_time("we are separated");
}

#[test]
fn test_negative_25() {
    check_no_time("25");
}

#[test]
fn test_negative_this_is_the_one() {
    check_no_time("this is the one");
}

#[test]
fn test_negative_this_one() {
    check_no_time("this one");
}

#[test]
fn test_negative_this_past_one() {
    check_no_time("this past one");
}

#[test]
fn test_negative_at_single() {
    check_no_time("at single");
}

#[test]
fn test_negative_at_a_couple_of() {
    check_no_time("at a couple of");
}

#[test]
fn test_negative_at_pairs() {
    check_no_time("at pairs");
}

#[test]
fn test_negative_at_a_few() {
    check_no_time("at a few");
}

#[test]
fn test_negative_at_dozens() {
    check_no_time("at dozens");
}

#[test]
fn test_negative_single_oclock() {
    check_no_time("single o'clock");
}

#[test]
fn test_negative_dozens_oclock() {
    check_no_time("dozens o'clock");
}

#[test]
fn test_negative_rat_6() {
    check_no_time("Rat 6");
    check_no_time("rat 6");
}

#[test]
fn test_negative_3_30() {
    check_no_time("3 30");
}

#[test]
fn test_negative_three_twenty() {
    check_no_time("three twenty");
}

#[test]
fn test_negative_phone_number_dot() {
    check_no_time("at 650.650.6500");
}

#[test]
fn test_negative_phone_number_dash() {
    check_no_time("at 650-650-6500");
}

#[test]
fn test_negative_two_sixty_am() {
    check_no_time("two sixty a m");
}

#[test]
fn test_negative_pay_abc_2000() {
    check_no_time("Pay ABC 2000");
}

#[test]
fn test_negative_4a() {
    check_no_time("4a");
}

#[test]
fn test_negative_4a_dot() {
    check_no_time("4a.");
}

#[test]
fn test_negative_a4_a5() {
    check_no_time("A4 A5");
}

#[test]
fn test_negative_palm() {
    check_no_time("palm");
}

#[test]
fn test_negative_mlk_day_typo() {
    check_no_time("Martin Luther King' day");
}

#[test]
fn test_negative_two_three() {
    check_no_time("two three");
}

// ============================================================
// latentCorpus - Latent: needs withLatent = true
// These tests are commented out because they require latent
// parsing mode to be enabled.
// ============================================================

#[test]
fn test_latent_examples() {
    // Latent: needs withLatent = true
    // check_time_naive("the 24", dt(2013, 2, 24, 0, 0, 0), "day");
    // check_time_naive("On 24th", dt(2013, 2, 24, 0, 0, 0), "day");

    // Latent: needs withLatent = true
    // check_time_naive("7", dt(2013, 2, 12, 7, 0, 0), "hour");
    // check_time_naive("7a", dt(2013, 2, 12, 7, 0, 0), "hour");

    // Latent: needs withLatent = true
    // check_time_naive("7p", dt(2013, 2, 12, 19, 0, 0), "hour");

    // Latent: needs withLatent = true
    // check_time_naive("ten thirty", dt(2013, 2, 12, 10, 30, 0), "minute");
    // check_time_naive("ten-thirty", dt(2013, 2, 12, 10, 30, 0), "minute");

    // Latent: needs withLatent = true
    // check_time_naive("1974", dt(1974, 1, 1, 0, 0, 0), "year");

    // Latent: needs withLatent = true
    // check_time_naive("May", dt(2013, 5, 1, 0, 0, 0), "month");

    // Latent: needs withLatent = true
    // check_time_interval("morning", dt(2013, 2, 12, 0, 0, 0), dt(2013, 2, 12, 12, 0, 0), "hour");

    // Latent: needs withLatent = true
    // check_time_interval("afternoon", dt(2013, 2, 12, 12, 0, 0), dt(2013, 2, 12, 19, 0, 0), "hour");

    // Latent: needs withLatent = true
    // check_time_interval("evening", dt(2013, 2, 12, 18, 0, 0), dt(2013, 2, 13, 0, 0, 0), "hour");

    // Latent: needs withLatent = true
    // check_time_interval("night", dt(2013, 2, 12, 18, 0, 0), dt(2013, 2, 13, 0, 0, 0), "hour");

    // Latent: needs withLatent = true
    // check_time_interval("the week", dt(2013, 2, 12, 0, 0, 0), dt(2013, 2, 17, 0, 0, 0), "day");

    // Latent: needs withLatent = true
    // check_time_naive("twelve zero three", dt(2013, 2, 12, 12, 3, 0), "minute");
    // check_time_naive("twelve o three", dt(2013, 2, 12, 12, 3, 0), "minute");
    // check_time_naive("twelve ou three", dt(2013, 2, 12, 12, 3, 0), "minute");
    // check_time_naive("twelve oh three", dt(2013, 2, 12, 12, 3, 0), "minute");
    // check_time_naive("twelve-zero-three", dt(2013, 2, 12, 12, 3, 0), "minute");
    // check_time_naive("twelve-oh-three", dt(2013, 2, 12, 12, 3, 0), "minute");

    // Latent: needs withLatent = true
    // check_time_interval("1960 - 1961", dt(1960, 1, 1, 0, 0, 0), dt(1962, 1, 1, 0, 0, 0), "year");

    // Latent: needs withLatent = true
    // check_time_naive("tonight 815", dt(2013, 2, 12, 20, 15, 0), "minute");
}

// ============================================================
// Weekend resolution with custom reference times
// Tests Haskell predNth semantics for this/next/last weekend
// ============================================================

fn parse_time_with_context(text: &str, context: &Context) -> Vec<Entity> {
    let locale = Locale::new(Lang::EN, None);
    let options = Options::default();
    parse(text, &locale, &[DimensionKind::Time], context, &options)
}

fn check_time_interval_with_context(
    text: &str,
    context: &Context,
    expected_from: NaiveDateTime,
    expected_to: NaiveDateTime,
    expected_grain: &str,
) {
    let entities = parse_time_with_context(text, context);
    let eg = grain(expected_grain);
    let found = entities.iter().any(|e| match &e.value {
        DimensionValue::Time(TimeValue::Interval {
            from: Some(f),
            to: Some(t),
            ..
        }) => {
            let (fv, fg) = tp_value_grain(f);
            let (tv, tg) = tp_value_grain(t);
            fv == expected_from && tv == expected_to && (fg == eg || tg == eg)
        }
        _ => false,
    });
    assert!(
        found,
        "Expected time interval from '{:?}' to '{:?}' grain '{}' for '{}', got: {:?}",
        expected_from,
        expected_to,
        expected_grain,
        text,
        entities
            .iter()
            .map(|e| format!("{:?}={:?}", e.value.dim_kind(), e.value))
            .collect::<Vec<_>>()
    );
}

/// Saturday Feb 9, 2013 10:00 — inside a weekend
fn make_saturday_context() -> Context {
    context_at(
        local_datetime(-120, 2013, 2, 9, 10, 0, 0),
        Locale::new(Lang::EN, None),
    )
}

/// Friday Feb 8, 2013 20:00 — inside a weekend (Friday evening)
fn make_friday_evening_context() -> Context {
    context_at(
        local_datetime(-120, 2013, 2, 8, 20, 0, 0),
        Locale::new(Lang::EN, None),
    )
}

#[test]
fn test_weekend_inside_saturday_this() {
    // "this weekend" from Saturday → current weekend
    let ctx = make_saturday_context();
    check_time_interval_with_context(
        "this weekend",
        &ctx,
        dt(2013, 2, 8, 18, 0, 0),
        dt(2013, 2, 11, 0, 0, 0),
        "hour",
    );
}

#[test]
fn test_weekend_inside_saturday_next() {
    // "next weekend" from Saturday → skip current, next weekend
    let ctx = make_saturday_context();
    check_time_interval_with_context(
        "next weekend",
        &ctx,
        dt(2013, 2, 15, 18, 0, 0),
        dt(2013, 2, 18, 0, 0, 0),
        "hour",
    );
}

#[test]
fn test_weekend_inside_saturday_last() {
    // "last weekend" from Saturday → previous weekend
    let ctx = make_saturday_context();
    check_time_interval_with_context(
        "last weekend",
        &ctx,
        dt(2013, 2, 1, 18, 0, 0),
        dt(2013, 2, 4, 0, 0, 0),
        "hour",
    );
}

#[test]
fn test_weekend_inside_friday_evening_this() {
    // "this weekend" from Friday evening → current weekend
    let ctx = make_friday_evening_context();
    check_time_interval_with_context(
        "this weekend",
        &ctx,
        dt(2013, 2, 8, 18, 0, 0),
        dt(2013, 2, 11, 0, 0, 0),
        "hour",
    );
}

#[test]
fn test_weekend_inside_friday_evening_next() {
    // "next weekend" from Friday evening → skip current, next weekend
    let ctx = make_friday_evening_context();
    check_time_interval_with_context(
        "next weekend",
        &ctx,
        dt(2013, 2, 15, 18, 0, 0),
        dt(2013, 2, 18, 0, 0, 0),
        "hour",
    );
}

#[test]
fn test_weekend_outside_tuesday_this() {
    // "this weekend" from Tuesday (default ref_time) → upcoming weekend
    // This is the existing test_time_this_weekend but explicit
    check_time_interval(
        "this weekend",
        dt(2013, 2, 15, 18, 0, 0),
        dt(2013, 2, 18, 0, 0, 0),
        "hour",
    );
}

#[test]
fn test_weekend_outside_tuesday_last() {
    // "last weekend" from Tuesday → most recent past weekend
    check_time_interval(
        "last weekend",
        dt(2013, 2, 8, 18, 0, 0),
        dt(2013, 2, 11, 0, 0, 0),
        "hour",
    );
}

// ============================================================
// diffCorpus - uses different reference time (2013-02-15 04:30:00 UTC-2)
// These tests are commented out because they require a different
// reference time than the default (2013-02-12T04:30:00+00:00).
// ============================================================

#[test]
fn test_diff_corpus_examples() {
    // diffCorpus: requires reference time 2013-02-15 04:30:00 UTC-2
    // check_time_naive("3 fridays from now", dt(2013, 3, 8, 0, 0, 0), "day");
    // check_time_naive("three fridays from now", dt(2013, 3, 8, 0, 0, 0), "day");
}
