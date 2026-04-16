// Port of Haskell's series generators from Duckling/Time/Types.hs.
// Generates multiple occurrences for cyclic TimeForm variants.
#![allow(clippy::arithmetic_side_effects)]

use super::{
    add_grain, grain_start, pod_interval, resolve_holiday, resolve_holiday_interval,
    resolve_holiday_minute_interval, resolve_season_interval, resolve_simple_datetime,
    resolve_weekend_interval, Direction, EarlyLate, PartOfDay, TimeData, TimeForm,
};
use crate::dimensions::time_grain::Grain;
use chrono::{DateTime, Datelike, FixedOffset, NaiveDate, Timelike};

/// Internal representation matching Haskell's `TimeObject { start, grain, end }`.
#[derive(Debug, Clone)]
pub(crate) struct TimeObject {
    pub start: DateTime<FixedOffset>,
    pub grain: Grain,
    pub end: Option<DateTime<FixedOffset>>,
}

/// Maximum number of series values to generate in each direction.
const SAFE_MAX: usize = 10;

/// Default per-parse budget for `generate_series_depth` invocations.
///
/// Each compose level fans out by ~2·SAFE_MAX, and a single parse can
/// produce hundreds of top-level time candidates (Outlook-style calendar
/// text with rows of weekday/number abbreviations hits this). 250K is ~5×
/// what realistic parses measure at; pathological inputs that previously
/// spun 20+ seconds bail out in a few hundred ms.
pub(crate) const DEFAULT_WORK_BUDGET: usize = 250_000;

/// Threaded work budget for time-resolution. Bounds cumulative
/// `generate_series_depth` calls across one parse.
///
/// Passed explicitly through the resolve chain (rather than a thread-local)
/// so the invariant is visible in types: callers who don't care about
/// budgeting can pass `Budget::unlimited()`.
pub(crate) struct Budget {
    remaining: usize,
}

impl Budget {
    pub(crate) fn new(initial: usize) -> Self {
        Self { remaining: initial }
    }

    /// Charge one unit of work. Returns `false` when exhausted.
    fn consume(&mut self) -> bool {
        if self.remaining == 0 {
            return false;
        }
        self.remaining -= 1;
        true
    }
}

// ============================================================
// TimeObject arithmetic — ports of Haskell's timePlus, timeEnd, timeIntersect
// ============================================================

/// Port of Haskell's `timePlus`.
pub(crate) fn time_plus(t: &TimeObject, grain: Grain, n: i64) -> Option<TimeObject> {
    let new_start = add_grain(t.start, grain, n)?;
    let new_grain = if grain < t.grain { grain } else { t.grain };
    Some(TimeObject {
        start: new_start,
        grain: new_grain,
        end: None,
    })
}

/// Port of Haskell's `timeEnd`: end of a TimeObject's interval.
fn time_end(t: &TimeObject) -> DateTime<FixedOffset> {
    t.end
        .unwrap_or_else(|| add_grain(t.start, t.grain, 1).unwrap_or(t.start))
}

/// Port of Haskell's `timeRound`: round a TimeObject down to a grain boundary.
fn time_round(t: &TimeObject, grain: Grain) -> TimeObject {
    let start = grain_start(t.start, grain);
    TimeObject {
        start,
        grain,
        end: None,
    }
}

/// Port of Haskell's `timeIntersect`.
pub(crate) fn time_intersect(t1: &TimeObject, t2: &TimeObject) -> Option<TimeObject> {
    let s1 = t1.start;
    let s2 = t2.start;
    let e1 = time_end(t1);
    let e2 = time_end(t2);
    let g = if t1.grain < t2.grain {
        t1.grain
    } else {
        t2.grain
    };

    if s1 > s2 {
        return time_intersect(t2, t1);
    }
    // s1 <= s2
    if e1 <= s2 {
        return None;
    }
    if e1 < e2 || (s1 == s2 && e1 == e2 && t1.end.is_some()) {
        Some(TimeObject {
            start: s2,
            grain: g,
            end: t1.end,
        })
    } else {
        Some(TimeObject {
            start: s2,
            grain: g,
            end: t2.end,
        })
    }
}

/// Port of Haskell's `timeStartsBeforeTheEndOf`.
fn starts_before_end_of(t1: &TimeObject, t2: &TimeObject) -> bool {
    t1.start < time_end(t2)
}

// ============================================================
// Core series generator: timeSequence
// ============================================================

/// Port of Haskell's `timeSequence grain step anchor`.
/// Returns (past, future) bounded to SAFE_MAX elements each.
fn time_sequence(
    grain: Grain,
    step: i64,
    anchor: &TimeObject,
) -> (Vec<TimeObject>, Vec<TimeObject>) {
    let mut future = Vec::with_capacity(SAFE_MAX);
    let mut past = Vec::with_capacity(SAFE_MAX);

    // Future: anchor, anchor+step, anchor+2step, ...
    let mut t = anchor.clone();
    for _ in 0..SAFE_MAX {
        future.push(t.clone());
        match time_plus(&t, grain, step) {
            Some(next) => t = next,
            None => break,
        }
    }

    // Past: anchor-step, anchor-2step, ...
    if let Some(first_past) = time_plus(anchor, grain, -step) {
        let mut t = first_past;
        for _ in 0..SAFE_MAX {
            past.push(t.clone());
            match time_plus(&t, grain, -step) {
                Some(next) => t = next,
                None => break,
            }
        }
    }

    (past, future)
}

// ============================================================
// Per-form series generators (ports from Haskell)
// ============================================================

/// DayOfWeek: weekly cycle. Port of Haskell's `runDayOfTheWeekPredicate`.
fn series_day_of_week(dow: u32, ref_time: &TimeObject) -> (Vec<TimeObject>, Vec<TimeObject>) {
    let current_dow = ref_time.start.weekday().num_days_from_monday();
    let days_until = ((dow as i64) - (current_dow as i64)).rem_euclid(7);
    let rounded = time_round(ref_time, Grain::Day);
    let anchor = match time_plus(&rounded, Grain::Day, days_until) {
        Some(a) => a,
        None => return (vec![], vec![]),
    };
    time_sequence(Grain::Day, 7, &anchor)
}

/// Month: yearly cycle. Port of Haskell's `runMonthPredicate`.
fn series_month(m: u32, ref_time: &TimeObject) -> (Vec<TimeObject>, Vec<TimeObject>) {
    let rounded_year = time_round(ref_time, Grain::Year);
    let rounded = match time_plus(&rounded_year, Grain::Month, (m as i64) - 1) {
        Some(r) => r,
        None => return (vec![], vec![]),
    };
    let anchor = if starts_before_end_of(ref_time, &rounded) {
        rounded
    } else {
        match time_plus(&rounded, Grain::Year, 1) {
            Some(next) => next,
            None => return (vec![], vec![]),
        }
    };
    time_sequence(Grain::Year, 1, &anchor)
}

/// Hour: 12h or 24h cycle. Port of Haskell's `runHourPredicate`.
fn series_hour(h: u32, is_12h: bool, ref_time: &TimeObject) -> (Vec<TimeObject>, Vec<TimeObject>) {
    let current_h = ref_time.start.hour();
    let step: i64 = if is_12h && h <= 12 { 12 } else { 24 };
    let n = h as i64;
    let rounded = time_round(ref_time, Grain::Hour);
    let anchor = match time_plus(
        &rounded,
        Grain::Hour,
        (n - current_h as i64).rem_euclid(step),
    ) {
        Some(a) => a,
        None => return (vec![], vec![]),
    };
    time_sequence(Grain::Hour, step, &anchor)
}

/// HourMinute: same as Hour but with minute set.
fn series_hour_minute(
    h: u32,
    m: u32,
    is_12h: bool,
    ref_time: &TimeObject,
) -> (Vec<TimeObject>, Vec<TimeObject>) {
    // Generate hour series, then compose with minute
    let (hour_past, hour_future) = series_hour(h, is_12h, ref_time);
    let set_minute = |t: &TimeObject| -> Option<TimeObject> {
        let dt = t.start;
        let new_start = dt
            .date_naive()
            .and_hms_opt(dt.hour(), m, 0)?
            .and_local_timezone(*ref_time.start.offset())
            .single()
            .unwrap();
        Some(TimeObject {
            start: new_start,
            grain: Grain::Minute,
            end: None,
        })
    };
    let past: Vec<TimeObject> = hour_past.iter().filter_map(set_minute).collect();
    let future: Vec<TimeObject> = hour_future.iter().filter_map(set_minute).collect();
    (past, future)
}

/// DayOfMonth: monthly cycle, filtering by month length.
/// Port of Haskell's `runDayOfTheMonthPredicate`.
fn series_day_of_month(d: u32, ref_time: &TimeObject) -> (Vec<TimeObject>, Vec<TimeObject>) {
    let rounded = time_round(ref_time, Grain::Month);
    let current_day = ref_time.start.day();
    let anchor = if current_day <= d {
        rounded
    } else {
        match time_plus(&rounded, Grain::Month, 1) {
            Some(next) => next,
            None => return (vec![], vec![]),
        }
    };

    let enough_days = |t: &TimeObject| -> bool {
        let date = t.start.date_naive();
        let (y, m, _) = (date.year(), date.month(), date.day());
        let max_day = if m == 12 {
            NaiveDate::from_ymd_opt(y + 1, 1, 1)
        } else {
            NaiveDate::from_ymd_opt(y, m + 1, 1)
        }
        .and_then(|next_first| next_first.pred_opt())
        .map(|d| d.day())
        .unwrap_or(28);
        d <= max_day
    };

    let add_days =
        |t: &TimeObject| -> Option<TimeObject> { time_plus(t, Grain::Day, (d as i64) - 1) };

    let mut future = Vec::new();
    let mut t = anchor.clone();
    for _ in 0..SAFE_MAX * 2 {
        if enough_days(&t) {
            if let Some(obj) = add_days(&t) {
                future.push(obj);
                if future.len() >= SAFE_MAX {
                    break;
                }
            }
        }
        match time_plus(&t, Grain::Month, 1) {
            Some(next) => t = next,
            None => break,
        }
    }

    let mut past = Vec::new();
    if let Some(first_past) = time_plus(&anchor, Grain::Month, -1) {
        let mut t = first_past;
        for _ in 0..SAFE_MAX * 2 {
            if enough_days(&t) {
                if let Some(obj) = add_days(&t) {
                    past.push(obj);
                    if past.len() >= SAFE_MAX {
                        break;
                    }
                }
            }
            match time_plus(&t, Grain::Month, -1) {
                Some(next) => t = next,
                None => break,
            }
        }
    }

    (past, future)
}

/// Year: single value, past or future depending on ref_time.
fn series_year(y: i32, ref_time: &TimeObject) -> (Vec<TimeObject>, Vec<TimeObject>) {
    let ref_year = ref_time.start.year();
    let rounded = time_round(ref_time, Grain::Year);
    let diff = (y as i64) - (ref_year as i64);
    let obj = match time_plus(&rounded, Grain::Year, diff) {
        Some(o) => o,
        None => return (vec![], vec![]),
    };
    if ref_year <= y {
        (vec![], vec![obj])
    } else {
        (vec![obj], vec![])
    }
}

/// PartOfDay: daily cycle, interval TimeObjects.
fn series_part_of_day(
    pod: PartOfDay,
    ref_time: &TimeObject,
    early_late: Option<EarlyLate>,
) -> (Vec<TimeObject>, Vec<TimeObject>) {
    let date = ref_time.start.date_naive();
    let (from, to) = pod_interval(pod, date, early_late, *ref_time.start.offset());
    let anchor = TimeObject {
        start: from,
        grain: Grain::Hour,
        end: Some(to),
    };
    // Daily cycle
    let mut future = Vec::new();
    let mut past = Vec::new();
    let mut t = anchor.clone();
    // Check if anchor is before ref_time
    if time_end(&t) <= ref_time.start {
        // Anchor is in the past, move to tomorrow
        past.push(t.clone());
        match shift_interval(&t, Grain::Day, 1) {
            Some(next) => t = next,
            None => return (past, future),
        }
    }
    for _ in 0..SAFE_MAX {
        future.push(t.clone());
        match shift_interval(&t, Grain::Day, 1) {
            Some(next) => t = next,
            None => break,
        }
    }
    // Generate past from anchor going backwards
    if let Some(first_past) = shift_interval(&anchor, Grain::Day, -1) {
        if past.is_empty() {
            past.push(anchor.clone());
        }
        let mut t = first_past;
        for _ in 0..SAFE_MAX {
            past.push(t.clone());
            match shift_interval(&t, Grain::Day, -1) {
                Some(next) => t = next,
                None => break,
            }
        }
    }
    (past, future)
}

/// Weekend: weekly cycle, interval TimeObjects.
fn series_weekend(
    ref_time: &TimeObject,
    direction: Option<Direction>,
) -> (Vec<TimeObject>, Vec<TimeObject>) {
    // Get the primary weekend interval
    let (from, to) = match resolve_weekend_interval(ref_time.start, direction) {
        Some(pair) => pair,
        None => return (vec![], vec![]),
    };
    let anchor = TimeObject {
        start: from,
        grain: Grain::Hour,
        end: Some(to),
    };
    // Weekly cycle
    let mut future = Vec::new();
    let mut past = Vec::new();
    let mut t = anchor.clone();
    if t.start <= ref_time.start && time_end(&t) <= ref_time.start {
        past.push(t.clone());
        match shift_interval(&t, Grain::Week, 1) {
            Some(next) => t = next,
            None => return (past, future),
        }
    }
    for _ in 0..SAFE_MAX {
        future.push(t.clone());
        match shift_interval(&t, Grain::Week, 1) {
            Some(next) => t = next,
            None => break,
        }
    }
    if let Some(first_past) = shift_interval(&anchor, Grain::Week, -1) {
        if past.is_empty() {
            past.push(anchor.clone());
        }
        let mut t = first_past;
        for _ in 0..SAFE_MAX {
            past.push(t.clone());
            match shift_interval(&t, Grain::Week, -1) {
                Some(next) => t = next,
                None => break,
            }
        }
    }
    (past, future)
}

/// Season: yearly cycle, interval TimeObjects.
fn series_season(
    s: u32,
    ref_time: &TimeObject,
    direction: Option<Direction>,
) -> (Vec<TimeObject>, Vec<TimeObject>) {
    let (from, to) = resolve_season_interval(s, ref_time.start, direction);
    let anchor = TimeObject {
        start: from,
        grain: Grain::Day,
        end: Some(to),
    };
    // Yearly cycle
    let mut future = Vec::new();
    let mut past = Vec::new();
    let mut t = anchor.clone();
    if time_end(&t) <= ref_time.start {
        past.push(t.clone());
        match shift_interval(&t, Grain::Year, 1) {
            Some(next) => t = next,
            None => return (past, future),
        }
    }
    for _ in 0..SAFE_MAX {
        future.push(t.clone());
        match shift_interval(&t, Grain::Year, 1) {
            Some(next) => t = next,
            None => break,
        }
    }
    if let Some(first_past) = shift_interval(&anchor, Grain::Year, -1) {
        if past.is_empty() {
            past.push(anchor.clone());
        }
        let mut t = first_past;
        for _ in 0..SAFE_MAX {
            past.push(t.clone());
            match shift_interval(&t, Grain::Year, -1) {
                Some(next) => t = next,
                None => break,
            }
        }
    }
    (past, future)
}

/// Holiday: yearly cycle (or single if year given).
fn series_holiday(
    name: &str,
    year_opt: Option<i32>,
    ref_time: &TimeObject,
    _direction: Option<Direction>,
) -> (Vec<TimeObject>, Vec<TimeObject>) {
    if let Some(year) = year_opt {
        // Fixed year → single value
        if let Some(date) = resolve_holiday(name, year) {
            let obj = TimeObject {
                start: date
                    .and_hms_opt(0, 0, 0)
                    .unwrap()
                    .and_local_timezone(*ref_time.start.offset())
                    .single()
                    .unwrap(),
                grain: Grain::Day,
                end: None,
            };
            if obj.start >= ref_time.start {
                return (vec![], vec![obj]);
            } else {
                return (vec![obj], vec![]);
            }
        }
        return (vec![], vec![]);
    }

    // No year → yearly cycle
    let ref_year = ref_time.start.year();
    let mut future = Vec::new();
    let mut past = Vec::new();

    // Try generating occurrences around the reference year
    for offset in -3i32..=6 {
        let y = ref_year.saturating_add(offset);
        // Check for interval holidays first
        if let Some((from_date, to_date)) = resolve_holiday_interval(name, y) {
            let obj = TimeObject {
                start: from_date
                    .and_hms_opt(0, 0, 0)
                    .unwrap()
                    .and_local_timezone(*ref_time.start.offset())
                    .single()
                    .unwrap(),
                grain: Grain::Day,
                end: Some(
                    to_date
                        .and_hms_opt(0, 0, 0)
                        .unwrap()
                        .and_local_timezone(*ref_time.start.offset())
                        .single()
                        .unwrap(),
                ),
            };
            if obj.start >= ref_time.start {
                future.push(obj);
            } else {
                past.push(obj);
            }
        } else if let Some((from_dt, to_dt)) =
            resolve_holiday_minute_interval(name, y, *ref_time.start.offset())
        {
            let obj = TimeObject {
                start: from_dt,
                grain: Grain::Minute,
                end: Some(to_dt),
            };
            if obj.start >= ref_time.start {
                future.push(obj);
            } else {
                past.push(obj);
            }
        } else if let Some(date) = resolve_holiday(name, y) {
            let obj = TimeObject {
                start: date
                    .and_hms_opt(0, 0, 0)
                    .unwrap()
                    .and_local_timezone(*ref_time.start.offset())
                    .single()
                    .unwrap(),
                grain: Grain::Day,
                end: None,
            };
            if obj.start >= ref_time.start {
                future.push(obj);
            } else {
                past.push(obj);
            }
        }
    }

    past.reverse(); // Most recent past first
    (past, future)
}

/// DateMDY: yearly cycle if no year, single if year given.
fn series_date_mdy(
    month: u32,
    day: u32,
    year: Option<i32>,
    ref_time: &TimeObject,
) -> (Vec<TimeObject>, Vec<TimeObject>) {
    if let Some(y) = year {
        let date = NaiveDate::from_ymd_opt(y, month, day);
        match date {
            Some(d) => {
                let obj = TimeObject {
                    start: d
                        .and_hms_opt(0, 0, 0)
                        .unwrap()
                        .and_local_timezone(*ref_time.start.offset())
                        .single()
                        .unwrap(),
                    grain: Grain::Day,
                    end: None,
                };
                if obj.start >= ref_time.start {
                    (vec![], vec![obj])
                } else {
                    (vec![obj], vec![])
                }
            }
            None => (vec![], vec![]),
        }
    } else {
        // Yearly cycle
        let ref_year = ref_time.start.year();
        let mut future = Vec::new();
        let mut past = Vec::new();
        for offset in -3i32..=6 {
            let y = ref_year.saturating_add(offset);
            if let Some(d) = NaiveDate::from_ymd_opt(y, month, day) {
                let obj = TimeObject {
                    start: d
                        .and_hms_opt(0, 0, 0)
                        .unwrap()
                        .and_local_timezone(*ref_time.start.offset())
                        .single()
                        .unwrap(),
                    grain: Grain::Day,
                    end: None,
                };
                if obj.start >= ref_time.start {
                    future.push(obj);
                } else {
                    past.push(obj);
                }
            }
        }
        past.reverse();
        (past, future)
    }
}

/// Shift an interval TimeObject by n grains (preserving the end offset).
fn shift_interval(t: &TimeObject, grain: Grain, n: i64) -> Option<TimeObject> {
    let new_start = add_grain(t.start, grain, n)?;
    let new_end = t.end.and_then(|e| add_grain(e, grain, n));
    Some(TimeObject {
        start: new_start,
        grain: t.grain,
        end: new_end,
    })
}

/// Single-value helper: wraps a (DateTime, grain) as a one-element series.
fn single_value(
    dt: DateTime<FixedOffset>,
    grain: Grain,
    ref_time: &TimeObject,
) -> (Vec<TimeObject>, Vec<TimeObject>) {
    let obj = TimeObject {
        start: dt,
        grain,
        end: None,
    };
    if dt >= ref_time.start {
        (vec![], vec![obj])
    } else {
        (vec![obj], vec![])
    }
}

/// Single interval value helper.
#[allow(dead_code)]
fn single_interval(
    from: DateTime<FixedOffset>,
    to: DateTime<FixedOffset>,
    grain: Grain,
    ref_time: &TimeObject,
) -> (Vec<TimeObject>, Vec<TimeObject>) {
    let obj = TimeObject {
        start: from,
        grain,
        end: Some(to),
    };
    if from >= ref_time.start {
        (vec![], vec![obj])
    } else {
        (vec![obj], vec![])
    }
}

// ============================================================
// Composed series: port of Haskell's runCompose
// ============================================================

/// Port of Haskell's `runCompose pred1 pred2`, threading a mutable `Budget`
/// through both predicates. For each occurrence of pred2, find the first
/// match of pred1 within it.
fn run_compose_with(
    mut pred1: impl FnMut(&TimeObject, &mut Budget) -> (Vec<TimeObject>, Vec<TimeObject>),
    mut pred2: impl FnMut(&TimeObject, &mut Budget) -> (Vec<TimeObject>, Vec<TimeObject>),
    ref_time: &TimeObject,
    budget: &mut Budget,
) -> (Vec<TimeObject>, Vec<TimeObject>) {
    let (past2, future2) = pred2(ref_time, budget);

    let mut compute_serie = |tokens: &[TimeObject], budget: &mut Budget| -> Vec<TimeObject> {
        let mut results = Vec::new();
        for time1 in tokens.iter().take(SAFE_MAX) {
            let (_, inner_future) = pred1(time1, budget);
            for t in inner_future.iter() {
                if starts_before_end_of(t, time1) {
                    if let Some(isect) = time_intersect(time1, t) {
                        results.push(isect);
                        break;
                    }
                }
            }
        }
        results
    };

    let backward = compute_serie(&past2, budget);
    let forward = compute_serie(&future2, budget);
    (backward, forward)
}

// ============================================================
// Main dispatch: generate_series
// ============================================================

/// Generate (past, future) series for a TimeData.
/// This is the main entry point matching Haskell's `runPredicate timePred refTime tc`.
pub(crate) fn generate_series(
    data: &TimeData,
    ref_time: DateTime<FixedOffset>,
    budget: &mut Budget,
) -> (Vec<TimeObject>, Vec<TimeObject>) {
    if !budget.consume() {
        return (vec![], vec![]);
    }

    let ref_obj = TimeObject {
        start: ref_time,
        grain: Grain::Second,
        end: None,
    };

    match &data.form {
        TimeForm::DayOfWeek(dow) => series_day_of_week(*dow, &ref_obj),
        TimeForm::Month(m) => series_month(*m, &ref_obj),
        TimeForm::Hour(h, is_12h) => series_hour(*h, *is_12h, &ref_obj),
        TimeForm::HourMinute(h, m, is_12h) => series_hour_minute(*h, *m, *is_12h, &ref_obj),
        TimeForm::DayOfMonth(d) => series_day_of_month(*d, &ref_obj),
        TimeForm::Year(y) => series_year(*y, &ref_obj),
        TimeForm::PartOfDay(pod) => series_part_of_day(*pod, &ref_obj, data.early_late),
        TimeForm::Weekend => series_weekend(&ref_obj, data.direction),
        TimeForm::Season(s) => series_season(*s, &ref_obj, data.direction),
        TimeForm::Holiday(name, year_opt) => {
            series_holiday(name, *year_opt, &ref_obj, data.direction)
        }
        TimeForm::DateMDY { month, day, year } => series_date_mdy(*month, *day, *year, &ref_obj),

        // Composed forms: use run_compose. No explicit depth cap; the budget
        // already bounds total work, and legitimate grammars (e.g. VI
        // intersect chains) can legitimately nest deeper than any fixed cap.
        TimeForm::Composed(primary, secondary) => {
            let p = primary.clone();
            let s = secondary.clone();
            run_compose_with(
                |t, b| generate_series(&p, t.start, b),
                |t, b| generate_series(&s, t.start, b),
                &ref_obj,
                budget,
            )
        }

        // Single-value forms: use existing resolve_simple_datetime
        _ => match resolve_simple_datetime(&data.form, ref_time, data.direction) {
            Some((dt, grain_str)) => {
                let grain = Grain::from_str(grain_str);
                single_value(dt, grain, &ref_obj)
            }
            None => (vec![], vec![]),
        },
    }
}
