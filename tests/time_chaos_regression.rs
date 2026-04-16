use chrono::{FixedOffset, TimeZone};
use duckling::{
    parse, Context, DimensionKind, DimensionValue, IntervalEndpoints, Lang, Locale, Options,
    TimePoint, TimeValue,
};

fn context(locale: Locale) -> Context {
    Context::new(
        FixedOffset::west_opt(5 * 3600)
            .unwrap()
            .with_ymd_and_hms(2013, 2, 12, 4, 30, 0)
            .unwrap(),
        locale,
    )
}

fn parse_time_no_panic(text: &str, locale: Locale) -> Vec<duckling::Entity> {
    let ctx = context(locale);
    let options = Options::default();
    std::panic::catch_unwind(|| parse(text, &locale, &[DimensionKind::Time], &ctx, &options))
        .unwrap_or_else(|_| panic!("parse panicked for locale {locale:?}, input: {text:?}"))
}

fn assert_time_point_sane(_tp: &TimePoint) {}

fn assert_interval_endpoints_sane(ep: &IntervalEndpoints) {
    assert!(
        ep.from.is_some() || ep.to.is_some(),
        "interval endpoint pair is fully unbounded: {ep:?}"
    );
    if let Some(tp) = &ep.from {
        assert_time_point_sane(tp);
    }
    if let Some(tp) = &ep.to {
        assert_time_point_sane(tp);
    }
}

fn assert_time_value_sane(tv: &TimeValue) {
    match tv {
        TimeValue::Single { value, values, .. } => {
            assert_time_point_sane(value);
            assert!(!values.is_empty(), "single time has empty values array");
            assert!(
                values.len() <= 3,
                "single time has too many values: {}",
                values.len()
            );
            for tp in values {
                assert_time_point_sane(tp);
            }
        }
        TimeValue::Interval {
            from, to, values, ..
        } => {
            assert!(
                from.is_some() || to.is_some(),
                "interval is fully unbounded: {tv:?}"
            );
            if let Some(tp) = from {
                assert_time_point_sane(tp);
            }
            if let Some(tp) = to {
                assert_time_point_sane(tp);
            }
            assert!(!values.is_empty(), "interval time has empty values array");
            assert!(
                values.len() <= 3,
                "interval time has too many values: {}",
                values.len()
            );
            for ep in values {
                assert_interval_endpoints_sane(ep);
            }
        }
    }
}

fn assert_entities_sane(text: &str, entities: &[duckling::Entity]) {
    assert!(
        entities.len() <= 1_024,
        "too many time entities ({}) for input: {:?}",
        entities.len(),
        text
    );
    for entity in entities {
        assert!(
            entity.start <= entity.end,
            "invalid entity range {:?} for input {:?}",
            entity,
            text
        );
        assert!(
            entity.end <= text.len(),
            "entity range out of bounds {:?} for input {:?}",
            entity,
            text
        );
        assert_eq!(
            entity.body,
            text[entity.start..entity.end],
            "entity body mismatch for input {:?}",
            text
        );
        match &entity.value {
            DimensionValue::Time(tv) => assert_time_value_sane(tv),
            other => panic!("non-time entity leaked in time-only parse: {other:?}"),
        }
    }
}

fn chaos_cases() -> Vec<String> {
    let mut cases = vec![
        "February 30th 2024 at 25:61:61".to_string(),
        "31/02/2025 24:00 tomorrow yesterday".to_string(),
        "from from from 3pm to to to 2pm yesterday tomorrow".to_string(),
        "between 12am and 12am and 12am and never".to_string(),
        "next friday 25:00 PST EST CET GMT+99 UTC-77".to_string(),
        "2024-03-10 02:30 EST EDT PST PDT GMT BST".to_string(),
        "....next....week....??!!".to_string(),
        "((((((((((((((tomorrow))))))))))))))".to_string(),
        "in --5 days".to_string(),
        "in +-+-+-3 weeks".to_string(),
        "in 999999999999999999999999999999 days".to_string(),
        "999999999999999999999999 years from now".to_string(),
        "in -999999999999999999 months".to_string(),
        "last 9999999999999999 centuries".to_string(),
        "next 2147483647 quarters".to_string(),
        "after 9223372036854775807 seconds".to_string(),
        "before -9223372036854775808 minutes".to_string(),
        "yesterday tomorrow today now then soon eventually maybe".to_string(),
        "monday tuesday wednesday thursday friday saturday sunday forever".to_string(),
        "january february march april may june july august september october november december".to_string(),
        "Q0 Q5 Q999 of year -1000000000".to_string(),
        "year 0000, year -0001, year +99999999".to_string(),
        "13/13/1313 at 13:13:13".to_string(),
        "00/00/0000 00:00".to_string(),
        "24:00 25:00 26:00".to_string(),
        "12:60 99:99 7:77pm".to_string(),
        "tomorrow\0at\x003pm".to_string(),
        " \u{200b}\u{200d}\u{2060}\u{feff}tomorrow\u{200b} at \u{200d}3pm\u{2060}".to_string(),
        "🕒🕓🕔 tomorrow 😈 at 13:37 // next ⏳ week".to_string(),
        "２０００年１３月４０日 ٢٥:٦١".to_string(),
        "İstanbul yarın 3pm, Москва завтра 5pm, 東京 明日 9時".to_string(),
        "next next next next next next monday".to_string(),
        "last last last last friday".to_string(),
        "from 3pm - to - ??? - 4pm - maybe".to_string(),
        "after before since until around about near circa noon".to_string(),
        "3pm-2pm-1pm-0pm".to_string(),
        "tomorrow tomorrow tomorrow tomorrow tomorrow".to_string(),
        "quarter past quarter to half past quarter after midnight noon".to_string(),
        "this monday of last year after next week before yesterday".to_string(),
        "midnight at noon at midnight at noon".to_string(),
        "week 0 of month 0 of year 0".to_string(),
        "at 3pm on feb 29 on non-leap years forever".to_string(),
        "DST gap: 2024-03-10 02:30 America/New_York".to_string(),
        "DST fold: 2024-11-03 01:30 America/New_York".to_string(),
        "rfc3339-ish 2025-99-99T99:99:99Z".to_string(),
        "cron vibes: 61 25 32 13 *".to_string(),
        "log line: [ERROR] 9999-99-99 99:99:99 retry in -999999h".to_string(),
        "calendar soup: mon 13/13 tue 32/01 wed 00/12".to_string(),
        "range soup: from -infinity to +infinity tomorrow".to_string(),
        "NTP drift: now now now now now".to_string(),
        "ASCII control \u{0001}\u{0002}\u{0003} tomorrow \u{0007}".to_string(),
        "mixed separators 2025..03..11 -- 08::15::00".to_string(),
        "date math: (today+1d)-(-2w)+(3m)-999999y".to_string(),
        "garbage: <<time>> {{{{now}}}} [[[later]]]".to_string(),
        "human: maybe around lunch-ish tomorrow-ish maybe-ish".to_string(),
        "messy reservation block:\nTuesday, March 11, 2025 at 8:15 PM\n(773) 348-8886\nlocation: 2300 N. Lincoln Park West Chicago, IL 60614".to_string(),
    ];

    cases.push(format!("in {} years", "9".repeat(2000)));
    cases.push(format!("in {} months", "1".repeat(3000)));
    cases.push(format!("{}monday", "next ".repeat(512)));
    cases.push(format!("{}friday", "last ".repeat(512)));
    cases.push(format!(
        "{}\n{}\n{}",
        "tomorrow ".repeat(300),
        "yesterday ".repeat(300),
        "in 999999999999999 days"
    ));
    cases.push(format!(
        "from {} to {}",
        "next ".repeat(256) + "monday",
        "last ".repeat(256) + "friday"
    ));
    cases
}

#[test]
fn test_time_chaos_no_panic_en_us() {
    let locale = Locale::new(Lang::EN, Some(duckling::Region::US));
    for text in chaos_cases() {
        let entities = parse_time_no_panic(&text, locale);
        assert_entities_sane(&text, &entities);
    }
}

#[test]
fn test_time_chaos_no_panic_es_and_fr() {
    let locales = [
        Locale::new(Lang::ES, Some(duckling::Region::ES)),
        Locale::new(Lang::FR, None),
    ];

    let subset = [
        "mañana a las 25:99",
        "demain à 99h99",
        "in 9999999999999999 days",
        "from from from 3pm to to to 2pm yesterday tomorrow",
        "tomorrow\0at\x003pm",
        "🕒 tomorrow 😈 at 13:37",
        "DST gap: 2024-03-10 02:30 America/New_York",
        "between 12am and 12am and never",
        "year +99999999 and -99999999",
    ];

    for locale in locales {
        for text in subset {
            let entities = parse_time_no_panic(text, locale);
            assert_entities_sane(text, &entities);
        }
    }
}

#[test]
fn test_time_chaos_unrepresentable_relatives_return_no_time() {
    let locale = Locale::new(Lang::EN, None);
    let cases = [
        "in 9999999999999999 days",
        "in -9999999999999999 days",
        "in 9223372036854775807 weeks",
        "in -9223372036854775808 weeks",
        "in 9999999999999999 months",
        "in 9999999999999999 years",
        "in 9999999999999999 quarters",
    ];

    for text in cases {
        let entities = parse_time_no_panic(text, locale);
        let has_time = entities
            .iter()
            .any(|e| matches!(e.value, DimensionValue::Time(_)));
        assert!(
            !has_time,
            "expected no Time entity for unrepresentable relative input {:?}, got {:?}",
            text, entities
        );
    }
}
