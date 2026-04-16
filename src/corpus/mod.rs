// Corpus types and helpers for training, ported from Duckling/Testing/Types.hs
// and Duckling/Time/Corpus.hs

/// English time corpus examples.
pub mod time_en;

use crate::ranking::train::TrainingCorpus;
use crate::resolve::{Context, Options};
use crate::types::{DimensionValue, Entity, TimePoint, TimeValue};
use crate::Grain;
use chrono::NaiveDate;

/// Build a TrainingCorpus from a context and a list of grouped examples.
/// Each group is (Vec<text>, check_fn) — multiple texts that should all pass
/// the same predicate. This mirrors Haskell's `examples` helper which maps
/// a single predicate over multiple texts.
#[allow(clippy::type_complexity)]
pub fn build_corpus(
    context: Context,
    groups: Vec<(Vec<&str>, Box<dyn Fn(&Entity) -> bool>)>,
) -> TrainingCorpus {
    let mut examples: Vec<(String, Box<dyn Fn(&Entity) -> bool>)> = Vec::new();
    for (texts, check) in groups {
        // We need to share the check across texts. Use Rc.
        let check = std::rc::Rc::new(check);
        for text in texts {
            let check = check.clone();
            examples.push((text.to_string(), Box::new(move |e: &Entity| check(e))));
        }
    }
    TrainingCorpus {
        context,
        options: Options { with_latent: false },
        examples,
    }
}

type Check = Box<dyn Fn(&Entity) -> bool>;

/// Port of Haskell's `datetime` check from Time/Corpus.hs.
/// Checks that an entity resolves to a specific naive datetime with the given grain.
pub fn datetime(y: i32, m: u32, d: u32, h: u32, mi: u32, s: u32, grain: Grain) -> Check {
    Box::new(move |e: &Entity| match &e.value {
        DimensionValue::Time(TimeValue::Single {
            value: TimePoint::Naive { value, grain: g },
            ..
        }) => {
            let expected = NaiveDate::from_ymd_opt(y, m, d)
                .unwrap()
                .and_hms_opt(h, mi, s)
                .unwrap();
            *value == expected && *g == grain
        }
        DimensionValue::Time(TimeValue::Single {
            value: TimePoint::Instant { value, grain: g },
            ..
        }) => {
            let expected = NaiveDate::from_ymd_opt(y, m, d)
                .unwrap()
                .and_hms_opt(h, mi, s)
                .unwrap();
            value.naive_local() == expected && *g == grain
        }
        _ => false,
    })
}

/// Port of Haskell's `datetimeHoliday` check.
/// Same as datetime but also checks holiday name in the entity body (we ignore
/// the holiday name check since Rust entities don't carry it separately — the
/// match on datetime is sufficient for training).
#[allow(clippy::too_many_arguments)]
pub fn datetime_holiday(
    y: i32,
    m: u32,
    d: u32,
    h: u32,
    mi: u32,
    s: u32,
    grain: Grain,
    _holiday: &str,
) -> Check {
    datetime(y, m, d, h, mi, s, grain)
}

/// Port of Haskell's `datetimeInterval` check.
#[allow(clippy::too_many_arguments)]
pub fn datetime_interval(
    y1: i32,
    m1: u32,
    d1: u32,
    h1: u32,
    mi1: u32,
    s1: u32,
    y2: i32,
    m2: u32,
    d2: u32,
    h2: u32,
    mi2: u32,
    s2: u32,
    grain: Grain,
) -> Check {
    Box::new(move |e: &Entity| match &e.value {
        DimensionValue::Time(TimeValue::Interval {
            from: Some(from),
            to: Some(to),
            ..
        }) => {
            let expected_from = NaiveDate::from_ymd_opt(y1, m1, d1)
                .unwrap()
                .and_hms_opt(h1, mi1, s1)
                .unwrap();
            let expected_to = NaiveDate::from_ymd_opt(y2, m2, d2)
                .unwrap()
                .and_hms_opt(h2, mi2, s2)
                .unwrap();
            let from_matches = match from {
                TimePoint::Naive { value, grain: g } => *value == expected_from && *g == grain,
                TimePoint::Instant { value, grain: g } => {
                    value.naive_local() == expected_from && *g == grain
                }
            };
            let to_matches = match to {
                TimePoint::Naive { value, .. } => *value == expected_to,
                TimePoint::Instant { value, .. } => value.naive_local() == expected_to,
            };
            from_matches && to_matches
        }
        _ => false,
    })
}

/// Port of Haskell's `datetimeIntervalHoliday` check.
#[allow(clippy::too_many_arguments)]
pub fn datetime_interval_holiday(
    y1: i32,
    m1: u32,
    d1: u32,
    h1: u32,
    mi1: u32,
    s1: u32,
    y2: i32,
    m2: u32,
    d2: u32,
    h2: u32,
    mi2: u32,
    s2: u32,
    grain: Grain,
    _holiday: &str,
) -> Check {
    datetime_interval(y1, m1, d1, h1, mi1, s1, y2, m2, d2, h2, mi2, s2, grain)
}

/// Port of Haskell's `datetimeOpenInterval` check for After direction.
pub fn datetime_open_interval_after(
    y: i32,
    m: u32,
    d: u32,
    h: u32,
    mi: u32,
    s: u32,
    grain: Grain,
) -> Check {
    Box::new(move |e: &Entity| match &e.value {
        DimensionValue::Time(TimeValue::Interval {
            from: Some(from),
            to: None,
            ..
        }) => {
            let expected = NaiveDate::from_ymd_opt(y, m, d)
                .unwrap()
                .and_hms_opt(h, mi, s)
                .unwrap();
            match from {
                TimePoint::Naive { value, grain: g } => *value == expected && *g == grain,
                TimePoint::Instant { value, grain: g } => {
                    value.naive_local() == expected && *g == grain
                }
            }
        }
        _ => false,
    })
}

/// Port of Haskell's `datetimeOpenInterval` check for Before direction.
pub fn datetime_open_interval_before(
    y: i32,
    m: u32,
    d: u32,
    h: u32,
    mi: u32,
    s: u32,
    _grain: Grain,
) -> Check {
    Box::new(move |e: &Entity| match &e.value {
        DimensionValue::Time(TimeValue::Interval {
            from: None,
            to: Some(to),
            ..
        }) => {
            let expected = NaiveDate::from_ymd_opt(y, m, d)
                .unwrap()
                .and_hms_opt(h, mi, s)
                .unwrap();
            match to {
                TimePoint::Naive { value, .. } => *value == expected,
                TimePoint::Instant { value, .. } => value.naive_local() == expected,
            }
        }
        _ => false,
    })
}

/// Helper to create a group of examples sharing the same check.
/// Mirrors Haskell's `examples output ["text1", "text2", ...]`.
pub fn examples(check: Check, texts: Vec<&str>) -> (Vec<&str>, Check) {
    (texts, check)
}
