pub mod ar;
pub mod bg;
pub mod ca;
pub mod da;
pub mod de;
pub mod el;
pub mod en;
pub mod es;
pub mod fr;
pub mod ga;
pub mod he;
pub mod hr;
pub mod hu;
pub mod it;
pub mod ja;
pub mod ka;
pub mod ko;
pub mod nb;
pub mod nl;
pub mod pl;
pub mod pt;
pub mod ro;
pub mod ru;
pub(crate) mod series;
pub mod sv;
pub mod tr;
pub mod uk;
pub mod vi;
pub mod zh;

use crate::dimensions::time_grain::Grain;
use crate::resolve::Context;
use crate::types::{DimensionValue, IntervalEndpoints, TimePoint, TimeValue};
use chrono::{
    DateTime, Datelike, Duration, FixedOffset, NaiveDate, NaiveDateTime, TimeZone, Timelike,
};
#[cfg(not(debug_assertions))]
use std::panic::{catch_unwind, AssertUnwindSafe};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PartOfDay {
    Morning,
    Afternoon,
    Evening,
    Night,
    Lunch,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IntervalDirection {
    After,
    Before,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EarlyLate {
    Early,
    Late,
    Mid,
}

#[derive(Debug, Clone)]
pub struct TimeData {
    pub form: TimeForm,
    pub direction: Option<Direction>,
    pub latent: bool,
    pub open_interval_direction: Option<IntervalDirection>,
    pub early_late: Option<EarlyLate>,
    /// Intentional deviation from Haskell's `hasTimezone :: Bool`:
    /// We store the actual timezone name (e.g. "CET", "EST") instead of just a flag,
    /// preserving more information for downstream consumers.
    pub timezone: Option<String>,
    /// Haskell's notImmediate: if true and the first future value coincides with
    /// the reference time, skip to the next occurrence.
    /// Used by "this Monday" (predNth 0 True) when today is Monday.
    pub not_immediate: bool,
    /// Matches Haskell's `okForThisNext :: Bool` (default `False`).
    /// Set to `true` on DayOfWeek, Month, Weekend, Season, Holiday, and select
    /// PartOfDay forms. Gates "this/next/last <time>" rules via `is_ok_with_this_next`.
    pub ok_for_this_next: bool,
    /// Matches Haskell's `holiday :: Maybe Text`.
    /// Set from `TimeForm::Holiday(name, _)` and propagated through composition.
    pub holiday: Option<String>,
}

#[derive(Debug, Clone)]
pub enum TimeForm {
    DayOfWeek(u32),             // 0=Monday .. 6=Sunday
    Month(u32),                 // 1..12
    DayOfMonth(u32),            // 1..31
    Hour(u32, bool),            // hour, is_12h_ambiguous
    HourMinute(u32, u32, bool), // hour, minute, is_12h_ambiguous
    HourMinuteSecond(u32, u32, u32),
    Year(i32),
    Now,
    Today,
    Tomorrow,
    Yesterday,
    RelativeGrain {
        n: i64,
        grain: Grain,
    },
    DateMDY {
        month: u32,
        day: u32,
        year: Option<i32>,
    },
    PartOfDay(PartOfDay),
    Weekend,
    Season(u32),                  // 0=spring, 1=summer, 2=fall, 3=winter
    Holiday(String, Option<i32>), // name, optional year
    GrainOffset {
        grain: Grain,
        offset: i32,
    },
    NthGrain {
        n: i64,
        grain: Grain,
        past: bool,
        interval: bool,
    },
    Quarter(u32),
    QuarterYear(u32, i32),
    DayAfterTomorrow,
    DayBeforeYesterday,
    BeginEnd {
        begin: bool,
        target: Box<TimeForm>,
    },
    Interval(Box<TimeData>, Box<TimeData>, bool), // from, to, open (true=don't adjust end)
    AllGrain(Grain),
    RestOfGrain(Grain),
    Composed(Box<TimeData>, Box<TimeData>),
    // "first Monday of March", "second Tuesday of last month"
    NthDOWOfTime {
        n: i32,
        dow: u32,
        base: Box<TimeData>,
    },
    // "last Friday of October"
    LastDOWOfTime {
        dow: u32,
        base: Box<TimeData>,
    },
    // "last week of September", "last weekend of July"
    LastCycleOfTime {
        grain: Grain,
        base: Box<TimeData>,
    },
    // "2 Sundays from now", "3 Fridays from now"
    NDOWsFromTime {
        n: i32,
        dow: u32,
        base: Box<TimeData>,
    },
    // "closest Monday to Oct 5th", "closest xmas to today"
    NthClosestToTime {
        n: i32,
        target: Box<TimeData>,
        base: Box<TimeData>,
    },
    // "Nth week of month" (e.g., "first week of October 2014")
    NthGrainOfTime {
        n: i32,
        grain: Grain,
        base: Box<TimeData>,
    },
    // "last day of October", "5th last day of May"
    NthLastDayOfTime {
        n: i32,
        base: Box<TimeData>,
    },
    // Haskell: durationAfter — "<duration> after <time>"
    // Shifts each occurrence of base by n grains, picks nearest future
    DurationAfter {
        n: i64,
        grain: Grain,
        base: Box<TimeData>,
    },
    // Haskell: cycleNthAfter with negative n — "<ordinal> last <grain> of <time>"
    NthLastCycleOfTime {
        n: i32,
        grain: Grain,
        base: Box<TimeData>,
    },
}

#[derive(Debug, Clone, Copy)]
pub enum Direction {
    Past,
    Future,
    FarFuture, // "after next" — skip one extra occurrence
}

/// Extract holiday name from a TimeForm, propagating through Composed/Interval
/// forms using Haskell's `h1 <|> h2` semantics (first non-None wins).
fn extract_holiday(form: &TimeForm) -> Option<String> {
    match form {
        TimeForm::Holiday(name, _) => Some(name.clone()),
        TimeForm::Composed(a, b) => a.holiday.clone().or_else(|| b.holiday.clone()),
        TimeForm::Interval(a, b, _) => a.holiday.clone().or_else(|| b.holiday.clone()),
        _ => None,
    }
}

impl TimeData {
    pub fn new(form: TimeForm) -> Self {
        let ok_for_this_next = matches!(
            form,
            TimeForm::DayOfWeek(_)
                | TimeForm::Month(_)
                | TimeForm::Weekend
                | TimeForm::Season(_)
                | TimeForm::Holiday(..)
                | TimeForm::PartOfDay(_)
        );
        let holiday = extract_holiday(&form);
        TimeData {
            form,
            direction: None,
            latent: false,
            open_interval_direction: None,
            early_late: None,
            timezone: None,
            not_immediate: false,
            ok_for_this_next,
            holiday,
        }
    }

    pub fn latent(form: TimeForm) -> Self {
        let ok_for_this_next = matches!(
            form,
            TimeForm::DayOfWeek(_)
                | TimeForm::Month(_)
                | TimeForm::Weekend
                | TimeForm::Season(_)
                | TimeForm::Holiday(..)
                | TimeForm::PartOfDay(_)
        );
        let holiday = extract_holiday(&form);
        TimeData {
            form,
            direction: None,
            latent: true,
            open_interval_direction: None,
            early_late: None,
            timezone: None,
            not_immediate: false,
            ok_for_this_next,
            holiday,
        }
    }

    /// Manually mark this TimeData as ok for "this/next/last" rules.
    /// Matches Haskell's `mkOkForThisNext`.
    #[allow(dead_code)]
    pub fn mk_ok_for_this_next(mut self) -> Self {
        self.ok_for_this_next = true;
        self
    }
}

// ============================================================
// Instant vs Naive classification
// ============================================================

/// Returns true if the form represents an absolute time pinned to a UTC instant.
/// Now and RelativeGrain are instant; DurationAfter and Composed inherit from their base/primary.
fn is_instant_form(form: &TimeForm) -> bool {
    match form {
        TimeForm::Now => true,
        TimeForm::RelativeGrain { .. } => true,
        TimeForm::DurationAfter { base, .. } => is_instant_form(&base.form),
        TimeForm::Composed(primary, _) => is_instant_form(&primary.form),
        _ => false,
    }
}

// ============================================================
// Values array generation (Haskell's values = take 3 $ future)
// ============================================================

/// Generate the values array (up to 3 TimePoints) using the series generator.
/// Matches Haskell's `values <- Just $ take 3 $ if null future then past else future`.
fn generate_extra_values(
    data: &TimeData,
    ref_time: DateTime<FixedOffset>,
    primary: &TimePoint,
    budget: &mut series::Budget,
) -> Vec<TimePoint> {
    let is_instant = is_instant_form(&data.form);

    let (past, future) = series::generate_series(data, ref_time, budget);
    let source = if future.is_empty() { &past } else { &future };

    let to_time_point = |obj: &series::TimeObject| -> TimePoint {
        match obj.end {
            None => {
                // Simple value
                if is_instant {
                    TimePoint::Instant {
                        value: obj.start,
                        grain: obj.grain,
                    }
                } else {
                    TimePoint::Naive {
                        value: obj.start.naive_local(),
                        grain: obj.grain,
                    }
                }
            }
            Some(_) => {
                // Interval start point (use start)
                if is_instant {
                    TimePoint::Instant {
                        value: obj.start,
                        grain: obj.grain,
                    }
                } else {
                    TimePoint::Naive {
                        value: obj.start.naive_local(),
                        grain: obj.grain,
                    }
                }
            }
        }
    };

    let values: Vec<TimePoint> = source.iter().take(3).map(to_time_point).collect();

    // If the series produced values, use them; otherwise just wrap the primary
    if values.is_empty() {
        vec![primary.clone()]
    } else {
        values
    }
}

/// Generate interval values array (up to 3 interval endpoint pairs).
#[allow(dead_code)]
fn generate_interval_values(
    data: &TimeData,
    ref_time: DateTime<FixedOffset>,
    primary_from: &Option<TimePoint>,
    primary_to: &Option<TimePoint>,
    is_instant: bool,
    budget: &mut series::Budget,
) -> Vec<IntervalEndpoints> {
    let (past, future) = series::generate_series(data, ref_time, budget);
    let source = if future.is_empty() { &past } else { &future };

    let to_endpoints = |obj: &series::TimeObject| -> IntervalEndpoints {
        let from_point = if is_instant {
            Some(TimePoint::Instant {
                value: obj.start,
                grain: obj.grain,
            })
        } else {
            Some(TimePoint::Naive {
                value: obj.start.naive_local(),
                grain: obj.grain,
            })
        };
        let to_point = obj.end.map(|e| {
            if is_instant {
                TimePoint::Instant {
                    value: e,
                    grain: obj.grain,
                }
            } else {
                TimePoint::Naive {
                    value: e.naive_local(),
                    grain: obj.grain,
                }
            }
        });
        IntervalEndpoints {
            from: from_point,
            to: to_point,
        }
    };

    let values: Vec<IntervalEndpoints> = source.iter().take(3).map(to_endpoints).collect();

    if values.is_empty() {
        vec![IntervalEndpoints {
            from: primary_from.clone(),
            to: primary_to.clone(),
        }]
    } else {
        values
    }
}

// ============================================================
// Main resolve entry point
// ============================================================

pub fn resolve(
    data: &TimeData,
    context: &Context,
    with_latent: bool,
    budget: &mut series::Budget,
) -> Option<DimensionValue> {
    if data.latent && !with_latent {
        return None;
    }
    let ref_time = context.reference_time;
    if has_unrepresentable_relative(data, ref_time) {
        return None;
    }

    let has_tz = data.timezone.as_deref().and_then(timezone_offset).is_some();
    let apply_tz = |dt: DateTime<FixedOffset>| -> DateTime<FixedOffset> {
        data.timezone
            .as_deref()
            .and_then(|tz_name| apply_named_timezone(dt, tz_name, context))
            .unwrap_or(dt)
    };

    // 1. Open intervals (ASAP, after/before/since/until + time)
    if let Some(dir) = data.open_interval_direction {
        let (dt, grain_str) = safe_resolve_simple_datetime(&data.form, ref_time, data.direction)?;
        let grain = if has_tz {
            Grain::Minute
        } else {
            Grain::from_str(grain_str)
        };
        let point = if has_tz || is_instant_form(&data.form) {
            TimePoint::Instant {
                value: apply_tz(dt),
                grain,
            }
        } else {
            TimePoint::Naive {
                value: dt.naive_local(),
                grain,
            }
        };
        return match dir {
            IntervalDirection::After => {
                let from = Some(point);
                Some(DimensionValue::Time(TimeValue::Interval {
                    from: from.clone(),
                    to: None,
                    values: vec![IntervalEndpoints { from, to: None }],
                    holiday: data.holiday.clone(),
                }))
            }
            IntervalDirection::Before => {
                let to = Some(point);
                Some(DimensionValue::Time(TimeValue::Interval {
                    from: None,
                    to: to.clone(),
                    values: vec![IntervalEndpoints { from: None, to }],
                    holiday: data.holiday.clone(),
                }))
            }
        };
    }

    // 2. Try to resolve as interval (pass context for per-endpoint timezone)
    if let Some(mut tv) = safe_try_resolve_as_interval(data, ref_time, context) {
        // Propagate holiday from TimeData into the resolved TimeValue
        if data.holiday.is_some() {
            match &mut tv {
                TimeValue::Single { holiday, .. } | TimeValue::Interval { holiday, .. } => {
                    if holiday.is_none() {
                        *holiday = data.holiday.clone();
                    }
                }
            }
        }
        return Some(DimensionValue::Time(tv));
    }

    // 3. Simple value
    let (dt, grain_str) = safe_resolve_simple_datetime(&data.form, ref_time, data.direction)?;
    let grain = if has_tz {
        Grain::Minute
    } else {
        Grain::from_str(grain_str)
    };
    let is_instant = has_tz || is_instant_form(&data.form);
    let point = if is_instant {
        TimePoint::Instant {
            value: apply_tz(dt),
            grain,
        }
    } else {
        TimePoint::Naive {
            value: dt.naive_local(),
            grain,
        }
    };
    // Generate additional values from the series
    let extra_values = generate_extra_values(data, context.reference_time, &point, budget);
    Some(DimensionValue::Time(TimeValue::Single {
        value: point.clone(),
        values: extra_values,
        holiday: data.holiday.clone(),
    }))
}

fn safe_try_resolve_as_interval(
    data: &TimeData,
    ref_time: DateTime<FixedOffset>,
    context: &Context,
) -> Option<TimeValue> {
    #[cfg(debug_assertions)]
    {
        try_resolve_as_interval(data, ref_time, context)
    }

    #[cfg(not(debug_assertions))]
    {
        catch_unwind(AssertUnwindSafe(|| {
            try_resolve_as_interval(data, ref_time, context)
        }))
        .ok()
        .flatten()
    }
}

fn safe_resolve_simple_datetime(
    form: &TimeForm,
    ref_time: DateTime<FixedOffset>,
    direction: Option<Direction>,
) -> Option<(DateTime<FixedOffset>, &'static str)> {
    #[cfg(debug_assertions)]
    {
        resolve_simple_datetime(form, ref_time, direction)
    }

    #[cfg(not(debug_assertions))]
    {
        catch_unwind(AssertUnwindSafe(|| {
            resolve_simple_datetime(form, ref_time, direction)
        }))
        .ok()
        .flatten()
    }
}

fn duration_for_grain(n: i64, grain: Grain) -> Option<Duration> {
    match grain {
        Grain::Second => Duration::try_seconds(n),
        Grain::Minute => Duration::try_minutes(n),
        Grain::Hour => Duration::try_hours(n),
        Grain::Day => Duration::try_days(n),
        Grain::Week => Duration::try_weeks(n),
        _ => None,
    }
}

fn relative_can_be_represented(base: DateTime<FixedOffset>, n: i64, grain: Grain) -> bool {
    match grain {
        Grain::NoGrain | Grain::Second | Grain::Minute | Grain::Hour | Grain::Day | Grain::Week => {
            duration_for_grain(n, grain)
                .and_then(|d| base.checked_add_signed(d))
                .is_some()
        }
        Grain::Month => add_months(base, n).is_some(),
        Grain::Quarter => n.checked_mul(3).and_then(|m| add_months(base, m)).is_some(),
        Grain::Year => add_years(base, n).is_some(),
    }
}

fn has_unrepresentable_relative(data: &TimeData, ref_time: DateTime<FixedOffset>) -> bool {
    fn check_form(form: &TimeForm, ref_time: DateTime<FixedOffset>) -> bool {
        match form {
            TimeForm::RelativeGrain { n, grain } => {
                !relative_can_be_represented(ref_time, *n, *grain)
            }
            TimeForm::DurationAfter { n, grain, base } => {
                !relative_can_be_represented(ref_time, *n, *grain)
                    || check_form(&base.form, ref_time)
            }
            TimeForm::Composed(a, b) | TimeForm::Interval(a, b, _) => {
                check_form(&a.form, ref_time) || check_form(&b.form, ref_time)
            }
            TimeForm::BeginEnd { target, .. } => check_form(target, ref_time),
            TimeForm::NthDOWOfTime { base, .. }
            | TimeForm::LastDOWOfTime { base, .. }
            | TimeForm::LastCycleOfTime { base, .. }
            | TimeForm::NDOWsFromTime { base, .. }
            | TimeForm::NthGrainOfTime { base, .. }
            | TimeForm::NthLastDayOfTime { base, .. }
            | TimeForm::NthLastCycleOfTime { base, .. } => check_form(&base.form, ref_time),
            TimeForm::NthClosestToTime { target, base, .. } => {
                check_form(&target.form, ref_time) || check_form(&base.form, ref_time)
            }
            _ => false,
        }
    }
    check_form(&data.form, ref_time)
}

/// Map timezone abbreviation to a fixed offset.
fn timezone_offset(tz: &str) -> Option<FixedOffset> {
    let minutes: i32 = match tz.to_uppercase().as_str() {
        "UTC" | "GMT" | "WET" => 0,
        "CET" => 60,
        "CEST" | "EET" => 120,
        "IST" => 330, // Indian Standard Time (UTC+5:30)
        "EEST" => 180,
        "EST" => -300,
        "EDT" => -240,
        "CST" => -360,
        "CDT" => -300,
        "MST" => -420,
        "MDT" => -360,
        "PST" => -480,
        "PDT" => -420,
        "BST" | "WEST" => 60,
        "JST" | "KST" => 540,
        "HKT" | "SGT" => 480,
        "AEST" => 600,
        "AEDT" => 660,
        "ACST" => 570,
        "ACDT" => 630,
        "AWST" => 480,
        "NZST" => 720,
        "NZDT" => 780,
        _ => return None,
    };
    FixedOffset::east_opt(minutes.checked_mul(60)?)
}

fn reinterpret_in_timezone(
    dt: DateTime<FixedOffset>,
    source_offset: FixedOffset,
    target_offset: FixedOffset,
) -> Option<DateTime<FixedOffset>> {
    let source = source_offset
        .from_local_datetime(&dt.naive_local())
        .single()?;
    Some(source.with_timezone(&target_offset))
}

/// Reinterpret a timezone-tagged clock time in the provided zone, then convert it
/// back into the context zone so the absolute instant is preserved.
fn apply_named_timezone(
    dt: DateTime<FixedOffset>,
    tz_name: &str,
    context: &Context,
) -> Option<DateTime<FixedOffset>> {
    reinterpret_in_timezone(dt, timezone_offset(tz_name)?, context.timezone())
}

// ============================================================
// Interval resolution
// ============================================================

fn try_resolve_as_interval(
    data: &TimeData,
    ref_time: DateTime<FixedOffset>,
    context: &Context,
) -> Option<TimeValue> {
    match &data.form {
        TimeForm::Interval(from_data, to_data, open) => {
            // Special case: from=Now → "by <time>" open interval
            if matches!(&from_data.form, TimeForm::Now) {
                let from_dt = ref_time;
                let to_dt = match &to_data.form {
                    TimeForm::BeginEnd { begin, target } => {
                        let (_, end) =
                            resolve_begin_end(*begin, target, ref_time, to_data.direction)?;
                        end
                    }
                    _ => {
                        let (dt, _) =
                            resolve_simple_datetime(&to_data.form, ref_time, to_data.direction)?;
                        dt
                    }
                };
                let g = Grain::Second;
                let from_point = Some(TimePoint::Instant {
                    value: from_dt,
                    grain: g,
                });
                let to_point = Some(if is_instant_form(&to_data.form) {
                    TimePoint::Instant {
                        value: to_dt,
                        grain: g,
                    }
                } else {
                    TimePoint::Naive {
                        value: to_dt.naive_local(),
                        grain: g,
                    }
                });
                return Some(TimeValue::Interval {
                    from: from_point.clone(),
                    to: to_point.clone(),
                    values: vec![IntervalEndpoints {
                        from: from_point,
                        to: to_point,
                    }],
                    holiday: data.holiday.clone(),
                });
            }

            let (mut from_dt, _from_grain) =
                resolve_simple_datetime(&from_data.form, ref_time, from_data.direction)?;
            let (mut to_dt, _) =
                resolve_simple_datetime(&to_data.form, ref_time, to_data.direction)?;

            // Apply per-endpoint timezone shifts (Haskell: each predicate carries its own shift)
            let has_endpoint_tz = from_data
                .timezone
                .as_deref()
                .and_then(timezone_offset)
                .is_some()
                || to_data
                    .timezone
                    .as_deref()
                    .and_then(timezone_offset)
                    .is_some();

            // Get the raw hours for AM/PM disambiguation
            let from_hour = match &from_data.form {
                TimeForm::Hour(h, _) => Some(*h),
                TimeForm::HourMinute(h, _, _) => Some(*h),
                _ => None,
            };
            let to_hour = match &to_data.form {
                TimeForm::Hour(h, _) => Some(*h),
                TimeForm::HourMinute(h, _, _) => Some(*h),
                _ => None,
            };
            let from_is_12h = match &from_data.form {
                TimeForm::Hour(_, b) => *b,
                TimeForm::HourMinute(_, _, b) => *b,
                _ => false,
            };
            let to_is_12h = match &to_data.form {
                TimeForm::Hour(_, b) => *b,
                TimeForm::HourMinute(_, _, b) => *b,
                _ => false,
            };

            // Disambiguate "from" Hour based on "to" Hour context
            // e.g., "3-4pm" → from=Hour(3, true), to=Hour(16, false) → from should be 15:00
            if from_is_12h {
                if let Some(to_h) = to_hour {
                    if let Some(from_h) = from_hour {
                        if to_h >= 12 && from_h < 12 {
                            let mins = match &from_data.form {
                                TimeForm::HourMinute(_, m, _) => *m,
                                _ => 0,
                            };
                            from_dt = ref_time
                                .date_naive()
                                .and_hms_opt(from_h.saturating_add(12), mins, 0)
                                .unwrap_or(from_dt.naive_local())
                                .and_local_timezone(*ref_time.offset())
                                .single()
                                .unwrap();
                        }
                    }
                }
            }
            // Disambiguate "to" based on "from" context
            // e.g., "8am until 6" → to=Hour(6, true), from is 8am → to should be 18:00
            if to_is_12h {
                if let Some(from_h) = from_hour {
                    if let Some(to_h) = to_hour {
                        if to_h < 12 && from_h < to_h.saturating_add(12) && !from_is_12h {
                            let mins = match &to_data.form {
                                TimeForm::HourMinute(_, m, _) => *m,
                                _ => 0,
                            };
                            to_dt = ref_time
                                .date_naive()
                                .and_hms_opt(to_h.saturating_add(12), mins, 0)
                                .unwrap_or(to_dt.naive_local())
                                .and_local_timezone(*ref_time.offset())
                                .single()
                                .unwrap();
                        }
                    }
                }
            }

            // Ensure from <= to (if from ended up after to on same day, fix)
            if from_dt > to_dt {
                from_dt = Duration::try_days(1)
                    .and_then(|d| from_dt.checked_sub_signed(d))
                    .unwrap_or(from_dt);
            }
            // For closed intervals, add 1 unit of the finer grain (matching Haskell)
            let to_dt = if *open {
                to_dt
            } else {
                adjust_interval_end_with_from(to_dt, &to_data.form, &from_data.form)?
            };

            // Apply per-endpoint timezone shifts after all disambiguation
            if let Some(tz_name) = from_data.timezone.as_deref() {
                from_dt = apply_named_timezone(from_dt, tz_name, context).unwrap_or(from_dt);
            }
            let to_dt = if let Some(tz_name) = to_data.timezone.as_deref() {
                apply_named_timezone(to_dt, tz_name, context).unwrap_or(to_dt)
            } else {
                to_dt
            };

            // Use the finer grain of from and to (matching Haskell: min g1 g2)
            let to_grain = form_grain(&to_data.form);
            let from_g = form_grain(&from_data.form);
            let interval_grain = if has_endpoint_tz {
                "minute"
            } else if from_g < to_grain {
                from_g.as_str()
            } else {
                to_grain.as_str()
            };
            if has_endpoint_tz {
                let g = Grain::from_str(interval_grain);
                let from_point = Some(TimePoint::Instant {
                    value: from_dt,
                    grain: g,
                });
                let to_point = Some(TimePoint::Instant {
                    value: to_dt,
                    grain: g,
                });
                Some(TimeValue::Interval {
                    from: from_point.clone(),
                    to: to_point.clone(),
                    values: vec![IntervalEndpoints {
                        from: from_point,
                        to: to_point,
                    }],
                    holiday: data.holiday.clone(),
                })
            } else {
                let mut iv = make_interval(from_dt, to_dt, interval_grain);
                if let TimeValue::Interval {
                    ref mut holiday, ..
                } = iv
                {
                    *holiday = data.holiday.clone();
                }
                Some(iv)
            }
        }
        TimeForm::NthGrain {
            n,
            grain,
            past,
            interval: true,
        } => {
            let (from, to) = resolve_nth_grain_interval(*n, *grain, *past, ref_time)?;
            Some(make_interval(from, to, grain.as_str()))
        }
        TimeForm::PartOfDay(pod) => {
            let date = ref_time.date_naive();
            let (from, to) = pod_interval(*pod, date, data.early_late, *ref_time.offset());
            Some(make_interval(from, to, "hour"))
        }
        TimeForm::Weekend => {
            let (from, to) = resolve_weekend_interval(ref_time, data.direction)?;
            Some(make_interval(from, to, "hour"))
        }
        TimeForm::AllGrain(g) => {
            let (from, to) = resolve_all_grain(*g, ref_time)?;
            let grain = grain_for_all_rest(*g);
            Some(make_interval(from, to, grain))
        }
        TimeForm::RestOfGrain(g) => {
            let from = grain_start(ref_time, g.lower());
            let to = {
                let period_start = grain_start(ref_time, *g);
                let next = add_grain(period_start, *g, 1)?;
                // For "rest of the week", end on Sunday (not Monday)
                if *g == Grain::Week {
                    Duration::try_days(1)
                        .and_then(|d| next.checked_sub_signed(d))
                        .unwrap_or(next)
                } else {
                    next
                }
            };
            // "rest of the week" truncates "from" to day boundary
            let from = if *g >= Grain::Day {
                grain_start(ref_time, Grain::Day)
            } else {
                from
            };
            let grain = grain_for_all_rest(*g);
            Some(make_interval(from, to, grain))
        }
        TimeForm::BeginEnd { begin, target } => {
            let (from, to) = resolve_begin_end(*begin, target, ref_time, data.direction)?;
            let grain = begin_end_grain(target);
            Some(make_interval(from, to, grain))
        }
        TimeForm::Season(s) => {
            let (from, to) = resolve_season_interval(*s, ref_time, data.direction);
            Some(make_interval(from, to, "day"))
        }
        // early/mid/late + Month → interval (e.g., "early March", "late October")
        TimeForm::Month(_m) if data.early_late.is_some() => {
            let (month_start, _) = resolve_simple_datetime(&data.form, ref_time, data.direction)?;
            let y = month_start.year();
            let m = month_start.month();
            let month_end = add_grain(
                NaiveDate::from_ymd_opt(y, m, 1)
                    .unwrap()
                    .and_hms_opt(0, 0, 0)
                    .unwrap()
                    .and_local_timezone(*ref_time.offset())
                    .single()
                    .unwrap(),
                Grain::Month,
                1,
            )?;
            let (from, to) = match data.early_late {
                Some(EarlyLate::Early) => {
                    // Beginning of month: day 1 to day 11
                    (
                        make_date(y, m, 1, *ref_time.offset()),
                        make_date(y, m, 11, *ref_time.offset()),
                    )
                }
                Some(EarlyLate::Mid) => {
                    // Middle of month: day 11 to day 21
                    (
                        make_date(y, m, 11, *ref_time.offset()),
                        make_date(y, m, 21, *ref_time.offset()),
                    )
                }
                Some(EarlyLate::Late) => {
                    // End of month: day 21 to end
                    (make_date(y, m, 21, *ref_time.offset()), month_end)
                }
                None => (make_date(y, m, 1, *ref_time.offset()), month_end), // shouldn't happen
            };
            Some(make_interval(from, to, "day"))
        }
        TimeForm::Composed(primary, secondary) => {
            // Helper: check if a form is a clock time
            fn is_clock_time(f: &TimeForm) -> bool {
                matches!(
                    f,
                    TimeForm::Hour(_, _)
                        | TimeForm::HourMinute(_, _, _)
                        | TimeForm::HourMinuteSecond(_, _, _)
                )
            }
            // Helper: check if a TimeData tree contains a clock time
            fn has_clock_time(td: &TimeData) -> bool {
                is_clock_time(&td.form)
                    || matches!(&td.form, TimeForm::Composed(a, b) if has_clock_time(a) || has_clock_time(b))
            }
            // Helper: disambiguate hour based on PartOfDay context
            fn disambiguate_hour(h: u32, is_12h: bool, pod: Option<&PartOfDay>) -> u32 {
                if !is_12h {
                    return h;
                }
                if h == 12 {
                    return match pod {
                        Some(_) => 0,
                        None => h,
                    };
                }
                match pod {
                    Some(PartOfDay::Afternoon)
                    | Some(PartOfDay::Evening)
                    | Some(PartOfDay::Night) => {
                        if h < 12 {
                            h.saturating_add(12)
                        } else {
                            h
                        }
                    }
                    Some(PartOfDay::Morning) => {
                        if h == 12 {
                            0
                        } else {
                            h
                        }
                    }
                    _ => h,
                }
            }

            // Composed forms where secondary is PartOfDay → interval
            // BUT only if there's no clock time that should disambiguate instead
            if let TimeForm::PartOfDay(pod) = &secondary.form {
                if !has_clock_time(primary) {
                    let (date_dt, _) =
                        resolve_simple_datetime(&primary.form, ref_time, primary.direction)?;
                    let date = date_dt.date_naive();
                    let (from, to) = pod_interval(
                        *pod,
                        date,
                        data.early_late.or(secondary.early_late),
                        *ref_time.offset(),
                    );
                    return Some(make_interval(from, to, "hour"));
                }
            }
            // Composed forms where primary is PartOfDay → interval
            // BUT only if there's no clock time in secondary
            if let TimeForm::PartOfDay(pod) = &primary.form {
                if !has_clock_time(secondary) {
                    let (date_dt, _) =
                        resolve_simple_datetime(&secondary.form, ref_time, secondary.direction)?;
                    let date = date_dt.date_naive();
                    let (from, to) = pod_interval(
                        *pod,
                        date,
                        data.early_late.or(primary.early_late),
                        *ref_time.offset(),
                    );
                    return Some(make_interval(from, to, "hour"));
                }
            }
            // Check nested Composed forms for PartOfDay intervals
            // e.g., Composed(DayOfWeek, Composed(PartOfDay, ...))
            if let TimeForm::Composed(sub_a, sub_b) = &secondary.form {
                if let TimeForm::PartOfDay(pod) = &sub_a.form {
                    if !has_clock_time(sub_b) && !has_clock_time(primary) {
                        let (date_dt, _) =
                            resolve_simple_datetime(&primary.form, ref_time, primary.direction)?;
                        let date = date_dt.date_naive();
                        let (from, to) = pod_interval(
                            *pod,
                            date,
                            data.early_late.or(sub_a.early_late),
                            *ref_time.offset(),
                        );
                        return Some(make_interval(from, to, "hour"));
                    }
                }
                if let TimeForm::PartOfDay(pod) = &sub_b.form {
                    if !has_clock_time(sub_a) && !has_clock_time(primary) {
                        let (date_dt, _) =
                            resolve_simple_datetime(&primary.form, ref_time, primary.direction)?;
                        let date = date_dt.date_naive();
                        let (from, to) = pod_interval(
                            *pod,
                            date,
                            data.early_late.or(sub_b.early_late),
                            *ref_time.offset(),
                        );
                        return Some(make_interval(from, to, "hour"));
                    }
                }
                // Handle Composed(ClockTime, PartOfDay) in secondary — use POD
                // for AM/PM disambiguation on a date primary.
                // e.g., Composed(DayOfWeek(Sat), Composed(Hour(9), PartOfDay(Morning)))
                let clock_pod: Option<(&TimeData, &PartOfDay)> = if is_clock_time(&sub_a.form) {
                    if let TimeForm::PartOfDay(pod) = &sub_b.form {
                        Some((sub_a, pod))
                    } else {
                        None
                    }
                } else if is_clock_time(&sub_b.form) {
                    if let TimeForm::PartOfDay(pod) = &sub_a.form {
                        Some((sub_b, pod))
                    } else {
                        None
                    }
                } else {
                    None
                };
                if let Some((clock, pod)) = clock_pod {
                    let (date_dt, _) =
                        resolve_simple_datetime(&primary.form, ref_time, primary.direction)?;
                    match &clock.form {
                        TimeForm::Hour(h, is_12h) => {
                            let hour = disambiguate_hour(*h, *is_12h, Some(pod));
                            let mut dt = date_dt
                                .date_naive()
                                .and_hms_opt(hour, 0, 0)?
                                .and_local_timezone(*ref_time.offset())
                                .single()
                                .unwrap();
                            if dt <= ref_time && date_dt.date_naive() == ref_time.date_naive() {
                                dt = Duration::try_days(1)
                                    .and_then(|d| dt.checked_add_signed(d))
                                    .unwrap_or(dt);
                            }
                            {
                                let point = TimePoint::Naive {
                                    value: dt.naive_local(),
                                    grain: Grain::Hour,
                                };
                                return Some(TimeValue::Single {
                                    value: point.clone(),
                                    values: vec![point],
                                    holiday: data.holiday.clone(),
                                });
                            }
                        }
                        TimeForm::HourMinute(h, m, is_12h) => {
                            let hour = disambiguate_hour(*h, *is_12h, Some(pod));
                            let mut dt = date_dt
                                .date_naive()
                                .and_hms_opt(hour, *m, 0)?
                                .and_local_timezone(*ref_time.offset())
                                .single()
                                .unwrap();
                            if dt <= ref_time && date_dt.date_naive() == ref_time.date_naive() {
                                dt = Duration::try_days(1)
                                    .and_then(|d| dt.checked_add_signed(d))
                                    .unwrap_or(dt);
                            }
                            {
                                let point = TimePoint::Naive {
                                    value: dt.naive_local(),
                                    grain: Grain::Minute,
                                };
                                return Some(TimeValue::Single {
                                    value: point.clone(),
                                    values: vec![point],
                                    holiday: data.holiday.clone(),
                                });
                            }
                        }
                        _ => {}
                    }
                }
            }
            if let TimeForm::Composed(sub_a, sub_b) = &primary.form {
                if let TimeForm::PartOfDay(pod) = &sub_a.form {
                    if !has_clock_time(sub_b) && !has_clock_time(secondary) {
                        // Combine the non-PartOfDay part (sub_b) with secondary to get the date
                        // e.g., Composed(Morning, Christmas) + Year(2013) → resolve Christmas+2013 first
                        let (date_dt, _) = resolve_composed(sub_b, secondary, ref_time)?;
                        let date = date_dt.date_naive();
                        let (from, to) = pod_interval(
                            *pod,
                            date,
                            data.early_late.or(sub_a.early_late),
                            *ref_time.offset(),
                        );
                        return Some(make_interval(from, to, "hour"));
                    }
                }
                if let TimeForm::PartOfDay(pod) = &sub_b.form {
                    if !has_clock_time(sub_a) && !has_clock_time(secondary) {
                        // Combine the non-PartOfDay part (sub_a) with secondary to get the date
                        let (date_dt, _) = resolve_composed(sub_a, secondary, ref_time)?;
                        let date = date_dt.date_naive();
                        let (from, to) = pod_interval(
                            *pod,
                            date,
                            data.early_late.or(sub_b.early_late),
                            *ref_time.offset(),
                        );
                        return Some(make_interval(from, to, "hour"));
                    }
                }
                // Handle Composed(ClockTime, PartOfDay) in primary — use POD
                // for AM/PM disambiguation on a date secondary.
                // e.g., Composed(Composed(Hour(9), PartOfDay(Morning)), DayOfWeek(Sat))
                let clock_pod: Option<(&TimeData, &PartOfDay)> = if is_clock_time(&sub_a.form) {
                    if let TimeForm::PartOfDay(pod) = &sub_b.form {
                        Some((sub_a, pod))
                    } else {
                        None
                    }
                } else if is_clock_time(&sub_b.form) {
                    if let TimeForm::PartOfDay(pod) = &sub_a.form {
                        Some((sub_b, pod))
                    } else {
                        None
                    }
                } else {
                    None
                };
                if let Some((clock, pod)) = clock_pod {
                    let (date_dt, _) =
                        resolve_simple_datetime(&secondary.form, ref_time, secondary.direction)?;
                    match &clock.form {
                        TimeForm::Hour(h, is_12h) => {
                            let hour = disambiguate_hour(*h, *is_12h, Some(pod));
                            let mut dt = date_dt
                                .date_naive()
                                .and_hms_opt(hour, 0, 0)?
                                .and_local_timezone(*ref_time.offset())
                                .single()
                                .unwrap();
                            if dt <= ref_time && date_dt.date_naive() == ref_time.date_naive() {
                                dt = Duration::try_days(1)
                                    .and_then(|d| dt.checked_add_signed(d))
                                    .unwrap_or(dt);
                            }
                            {
                                let point = TimePoint::Naive {
                                    value: dt.naive_local(),
                                    grain: Grain::Hour,
                                };
                                return Some(TimeValue::Single {
                                    value: point.clone(),
                                    values: vec![point],
                                    holiday: data.holiday.clone(),
                                });
                            }
                        }
                        TimeForm::HourMinute(h, m, is_12h) => {
                            let hour = disambiguate_hour(*h, *is_12h, Some(pod));
                            let mut dt = date_dt
                                .date_naive()
                                .and_hms_opt(hour, *m, 0)?
                                .and_local_timezone(*ref_time.offset())
                                .single()
                                .unwrap();
                            if dt <= ref_time && date_dt.date_naive() == ref_time.date_naive() {
                                dt = Duration::try_days(1)
                                    .and_then(|d| dt.checked_add_signed(d))
                                    .unwrap_or(dt);
                            }
                            {
                                let point = TimePoint::Naive {
                                    value: dt.naive_local(),
                                    grain: Grain::Minute,
                                };
                                return Some(TimeValue::Single {
                                    value: point.clone(),
                                    values: vec![point],
                                    holiday: data.holiday.clone(),
                                });
                            }
                        }
                        _ => {}
                    }
                }
            }
            // Interval + date (or date + interval) composition
            // e.g., "1pm-2pm tomorrow" = Composed(Interval(1pm, 2pm), Tomorrow)
            // or "Thursday from 9a to 11a" = Composed(DayOfWeek, Interval(9, 11))
            fn is_date_form(f: &TimeForm) -> bool {
                matches!(
                    f,
                    TimeForm::Tomorrow
                        | TimeForm::Yesterday
                        | TimeForm::DayOfWeek(_)
                        | TimeForm::DayOfMonth(_)
                        | TimeForm::DateMDY { .. }
                        | TimeForm::Composed(..)
                        | TimeForm::Today
                        | TimeForm::DayAfterTomorrow
                        | TimeForm::DayBeforeYesterday
                        | TimeForm::GrainOffset { .. }
                        | TimeForm::Holiday(..)
                )
            }

            if let TimeForm::Interval(from_td, to_td, open) = &primary.form {
                if is_date_form(&secondary.form) {
                    let (date_dt, _) =
                        resolve_simple_datetime(&secondary.form, ref_time, secondary.direction)?;
                    let date = date_dt.date_naive();
                    let (from_dt, _from_grain) =
                        resolve_on_date(&from_td.form, date, ref_time, from_td.direction)?;
                    let (mut to_dt, _) =
                        resolve_on_date(&to_td.form, date, ref_time, to_td.direction)?;
                    if !open {
                        to_dt = adjust_interval_end_with_from(to_dt, &to_td.form, &from_td.form)?;
                    }
                    let to_grain = form_grain(&to_td.form);
                    let from_g = form_grain(&from_td.form);
                    let interval_grain = if from_g < to_grain { from_g } else { to_grain };
                    return Some(make_interval(from_dt, to_dt, interval_grain.as_str()));
                }
            }
            if let TimeForm::Interval(from_td, to_td, open) = &secondary.form {
                if is_date_form(&primary.form) {
                    let (date_dt, _) =
                        resolve_simple_datetime(&primary.form, ref_time, primary.direction)?;
                    let date = date_dt.date_naive();
                    let (from_dt, _) =
                        resolve_on_date(&from_td.form, date, ref_time, from_td.direction)?;
                    let (mut to_dt, _) =
                        resolve_on_date(&to_td.form, date, ref_time, to_td.direction)?;
                    if !open {
                        to_dt = adjust_interval_end_with_from(to_dt, &to_td.form, &from_td.form)?;
                    }
                    let to_grain = form_grain(&to_td.form);
                    let from_g = form_grain(&from_td.form);
                    let interval_grain = if from_g < to_grain { from_g } else { to_grain };
                    return Some(make_interval(from_dt, to_dt, interval_grain.as_str()));
                }
            }
            None // not an interval composed form
        }
        // NthGrainOfTime and LastCycleOfTime are resolved as simple values in resolve_simple_datetime
        TimeForm::Holiday(name, year_opt) => {
            let year = year_opt.unwrap_or(ref_time.year());
            // Check for minute-level intervals (Earth Hour)
            if let Some((from_dt, to_dt)) =
                resolve_holiday_minute_interval(name, year, *ref_time.offset())
            {
                return Some(make_interval(from_dt, to_dt, "minute"));
            }
            // Check for day-level intervals
            if let Some((from_date, to_date)) = resolve_holiday_interval(name, year) {
                let from_dt = from_date
                    .and_hms_opt(0, 0, 0)
                    .unwrap()
                    .and_local_timezone(*ref_time.offset())
                    .single()
                    .unwrap();
                let to_dt = to_date
                    .and_hms_opt(0, 0, 0)
                    .unwrap()
                    .and_local_timezone(*ref_time.offset())
                    .single()
                    .unwrap();
                return Some(make_interval(from_dt, to_dt, "day"));
            }
            None
        }
        // "last weekend of July" → weekend interval
        TimeForm::LastDOWOfTime { dow: 5, base } => {
            // dow=5 (Saturday) means weekend. Find last Saturday, build weekend interval.
            let (base_dt, _) = resolve_simple_datetime(&base.form, ref_time, base.direction)?;
            let base_start = match &base.form {
                TimeForm::Month(_) => NaiveDate::from_ymd_opt(base_dt.year(), base_dt.month(), 1)
                    .unwrap()
                    .and_hms_opt(0, 0, 0)
                    .unwrap()
                    .and_local_timezone(*ref_time.offset())
                    .single()
                    .unwrap(),
                TimeForm::GrainOffset {
                    grain: Grain::Month,
                    offset,
                } => {
                    let first = start_of_month(ref_time);
                    add_months(first, i64::from(*offset))?
                }
                _ => grain_start(base_dt, Grain::Month),
            };
            let sat = last_dow_of_month(base_start.year(), base_start.month(), 5);
            let friday = sat
                .checked_sub_signed(chrono::Duration::try_days(1).unwrap_or_default())
                .unwrap_or(sat);
            let from = friday
                .and_hms_opt(18, 0, 0)
                .unwrap()
                .and_local_timezone(*ref_time.offset())
                .single()
                .unwrap();
            let monday = sat
                .checked_add_signed(chrono::Duration::try_days(2).unwrap_or_default())
                .unwrap_or(sat);
            let to = monday
                .and_hms_opt(0, 0, 0)
                .unwrap()
                .and_local_timezone(*ref_time.offset())
                .single()
                .unwrap();
            Some(make_interval(from, to, "hour"))
        }
        _ => None,
    }
}

/// Resolve a time form onto a specific date (for interval + date composition)
fn resolve_on_date(
    form: &TimeForm,
    date: NaiveDate,
    ref_time: DateTime<FixedOffset>,
    direction: Option<Direction>,
) -> Option<(DateTime<FixedOffset>, &'static str)> {
    Some(match form {
        TimeForm::Hour(h, _is_12h) => {
            let dt = date
                .and_hms_opt(*h, 0, 0)
                .unwrap_or(date.and_hms_opt(0, 0, 0).unwrap())
                .and_local_timezone(*ref_time.offset())
                .single()
                .unwrap();
            (dt, "hour")
        }
        TimeForm::HourMinute(h, m, _is_12h) => {
            let dt = date
                .and_hms_opt(*h, *m, 0)
                .unwrap_or(date.and_hms_opt(0, 0, 0).unwrap())
                .and_local_timezone(*ref_time.offset())
                .single()
                .unwrap();
            (dt, "minute")
        }
        TimeForm::HourMinuteSecond(h, m, s) => {
            let dt = date
                .and_hms_opt(*h, *m, *s)
                .unwrap_or(date.and_hms_opt(0, 0, 0).unwrap())
                .and_local_timezone(*ref_time.offset())
                .single()
                .unwrap();
            (dt, "second")
        }
        TimeForm::Composed(_primary, _secondary) => {
            // Resolve composed time on this date
            let (dt, grain) = resolve_simple_datetime(form, ref_time, direction)?;
            let new_dt = date
                .and_hms_opt(dt.hour(), dt.minute(), dt.second())
                .unwrap_or(date.and_hms_opt(0, 0, 0).unwrap())
                .and_local_timezone(*ref_time.offset())
                .single()
                .unwrap();
            (new_dt, grain)
        }
        _ => {
            // For non-clock forms, resolve normally but override the date
            let (dt, grain) = resolve_simple_datetime(form, ref_time, direction)?;
            (dt, grain)
        }
    })
}

fn make_interval(from: DateTime<FixedOffset>, to: DateTime<FixedOffset>, grain: &str) -> TimeValue {
    let g = Grain::from_str(grain);
    let from_point = Some(TimePoint::Naive {
        value: from.naive_local(),
        grain: g,
    });
    let to_point = Some(TimePoint::Naive {
        value: to.naive_local(),
        grain: g,
    });
    TimeValue::Interval {
        from: from_point.clone(),
        to: to_point.clone(),
        values: vec![IntervalEndpoints {
            from: from_point,
            to: to_point,
        }],
        holiday: None,
    }
}

/// For closed intervals, add one grain to "to".
/// Uses the finer grain between from and to (matching Haskell behavior).
fn adjust_interval_end_with_from(
    to: DateTime<FixedOffset>,
    to_form: &TimeForm,
    from_form: &TimeForm,
) -> Option<DateTime<FixedOffset>> {
    let to_grain = form_grain(to_form);
    let from_grain = form_grain(from_form);
    // Match Haskell: g2' = if g1 < Day && g2 < Day then min(g1,g2) else g2
    let adjust_grain = if from_grain < Grain::Day && to_grain < Grain::Day {
        if from_grain < to_grain {
            from_grain
        } else {
            to_grain
        }
    } else {
        to_grain
    };
    Some(match adjust_grain {
        Grain::NoGrain | Grain::Second => Duration::try_seconds(1)
            .and_then(|d| to.checked_add_signed(d))
            .unwrap_or(to),
        Grain::Minute => Duration::try_minutes(1)
            .and_then(|d| to.checked_add_signed(d))
            .unwrap_or(to),
        Grain::Hour => Duration::try_hours(1)
            .and_then(|d| to.checked_add_signed(d))
            .unwrap_or(to),
        Grain::Day => Duration::try_days(1)
            .and_then(|d| to.checked_add_signed(d))
            .unwrap_or(to),
        Grain::Week => Duration::try_weeks(1)
            .and_then(|d| to.checked_add_signed(d))
            .unwrap_or(to),
        Grain::Month => add_months(to, 1)?,
        Grain::Quarter => add_months(to, 3)?,
        Grain::Year => add_years(to, 1)?,
    })
}

/// Get the grain of a TimeForm
pub(super) fn form_grain(f: &TimeForm) -> Grain {
    match f {
        TimeForm::Year(_) => Grain::Year,
        TimeForm::Month(_) => Grain::Month,
        TimeForm::Quarter(_) | TimeForm::QuarterYear(_, _) => Grain::Quarter,
        TimeForm::DayOfWeek(_)
        | TimeForm::DayOfMonth(_)
        | TimeForm::DateMDY { .. }
        | TimeForm::Today
        | TimeForm::Tomorrow
        | TimeForm::Yesterday
        | TimeForm::DayAfterTomorrow
        | TimeForm::DayBeforeYesterday
        | TimeForm::Holiday(_, _) => Grain::Day,
        TimeForm::Now => Grain::NoGrain,
        TimeForm::Hour(_, _) | TimeForm::PartOfDay(_) => Grain::Hour,
        TimeForm::HourMinute(_, _, _) => Grain::Minute,
        TimeForm::HourMinuteSecond(_, _, _) => Grain::Second,
        TimeForm::DurationAfter { grain, .. } => *grain,
        TimeForm::Composed(a, b) => {
            let ga = form_grain(&a.form);
            let gb = form_grain(&b.form);
            if ga < gb {
                ga
            } else {
                gb
            }
        }
        TimeForm::Interval(from_td, to_td, _) => {
            let gf = form_grain(&from_td.form);
            let gt = form_grain(&to_td.form);
            if gf < gt {
                gf
            } else {
                gt
            }
        }
        _ => Grain::Day,
    }
}

// ============================================================
// Simple datetime resolution (returns DateTime + grain string)
// ============================================================

pub(super) fn resolve_simple_datetime(
    form: &TimeForm,
    ref_time: DateTime<FixedOffset>,
    direction: Option<Direction>,
) -> Option<(DateTime<FixedOffset>, &'static str)> {
    Some(match form {
        TimeForm::Now => (ref_time, "second"),
        TimeForm::Today => (midnight(ref_time), "day"),
        TimeForm::Tomorrow => (
            midnight(
                Duration::try_days(1)
                    .and_then(|d| ref_time.checked_add_signed(d))
                    .unwrap_or(ref_time),
            ),
            "day",
        ),
        TimeForm::Yesterday => (
            midnight(
                Duration::try_days(1)
                    .and_then(|d| ref_time.checked_sub_signed(d))
                    .unwrap_or(ref_time),
            ),
            "day",
        ),
        TimeForm::DayAfterTomorrow => (
            midnight(
                Duration::try_days(2)
                    .and_then(|d| ref_time.checked_add_signed(d))
                    .unwrap_or(ref_time),
            ),
            "day",
        ),
        TimeForm::DayBeforeYesterday => (
            midnight(
                Duration::try_days(2)
                    .and_then(|d| ref_time.checked_sub_signed(d))
                    .unwrap_or(ref_time),
            ),
            "day",
        ),
        TimeForm::DayOfWeek(dow) => (resolve_dow(*dow, ref_time, direction), "day"),
        TimeForm::Month(m) => (resolve_month(*m, ref_time, direction), "month"),
        TimeForm::DayOfMonth(d) => {
            // Future-first: if day is past in current month, use next month
            let date = NaiveDate::from_ymd_opt(ref_time.year(), ref_time.month(), *d);
            match date {
                Some(date) => {
                    let dt = date
                        .and_hms_opt(0, 0, 0)
                        .unwrap()
                        .and_local_timezone(*ref_time.offset())
                        .single()
                        .unwrap();
                    if dt < ref_time && !matches!(direction, Some(Direction::Past)) {
                        // Try next month
                        let next = if ref_time.month() == 12 {
                            NaiveDate::from_ymd_opt(ref_time.year().saturating_add(1), 1, *d)
                        } else {
                            NaiveDate::from_ymd_opt(
                                ref_time.year(),
                                ref_time.month().saturating_add(1),
                                *d,
                            )
                        };
                        match next {
                            Some(nd) => (
                                nd.and_hms_opt(0, 0, 0)
                                    .unwrap()
                                    .and_local_timezone(*ref_time.offset())
                                    .single()
                                    .unwrap(),
                                "day",
                            ),
                            None => (dt, "day"),
                        }
                    } else {
                        (dt, "day")
                    }
                }
                None => (ref_time, "day"),
            }
        }
        TimeForm::Hour(h, _) => {
            let mut dt = ref_time
                .date_naive()
                .and_hms_opt(*h, 0, 0)
                .unwrap_or(ref_time.naive_local())
                .and_local_timezone(*ref_time.offset())
                .single()
                .unwrap();
            // Future-first: if past, use tomorrow
            if dt <= ref_time && !matches!(direction, Some(Direction::Past)) {
                dt = Duration::try_days(1)
                    .and_then(|d| dt.checked_add_signed(d))
                    .unwrap_or(dt);
            }
            (dt, "hour")
        }
        TimeForm::HourMinute(h, m, is_12h) => {
            let today = ref_time
                .date_naive()
                .and_hms_opt(*h, *m, 0)
                .unwrap_or(ref_time.naive_local())
                .and_local_timezone(*ref_time.offset())
                .single()
                .unwrap();
            if *is_12h && *h < 12 {
                // 12h ambiguous: try both AM and PM
                let am = today;
                let pm = ref_time
                    .date_naive()
                    .and_hms_opt(h.saturating_add(12), *m, 0)
                    .unwrap_or(ref_time.naive_local())
                    .and_local_timezone(*ref_time.offset())
                    .single()
                    .unwrap();
                if matches!(direction, Some(Direction::Past)) {
                    // Past direction: pick most recent past
                    if pm <= ref_time {
                        (pm, "minute")
                    } else if am <= ref_time {
                        (am, "minute")
                    } else {
                        (
                            Duration::try_days(1)
                                .and_then(|d| pm.checked_sub_signed(d))
                                .unwrap_or(pm),
                            "minute",
                        )
                    }
                } else if am.hour() == ref_time.hour() {
                    // AM is in current hour → keep AM (e.g., "at 4:23" when ref is 4:30)
                    (am, "minute")
                } else {
                    // Pick nearest future (AM first, then PM, then tomorrow AM)
                    if am > ref_time {
                        (am, "minute")
                    } else if pm > ref_time {
                        (pm, "minute")
                    } else {
                        (
                            Duration::try_days(1)
                                .and_then(|d| am.checked_add_signed(d))
                                .unwrap_or(am),
                            "minute",
                        )
                    }
                }
            } else if *is_12h && *h == 12 {
                // "12:15" ambiguous - noon vs midnight
                let noon = today; // h=12 already
                let midnight = ref_time
                    .date_naive()
                    .and_hms_opt(0, *m, 0)
                    .unwrap_or(ref_time.naive_local())
                    .and_local_timezone(*ref_time.offset())
                    .single()
                    .unwrap();
                if noon > ref_time {
                    (noon, "minute")
                } else if Duration::try_days(1)
                    .and_then(|d| midnight.checked_add_signed(d))
                    .unwrap_or(midnight)
                    > ref_time
                {
                    (
                        Duration::try_days(1)
                            .and_then(|d| midnight.checked_add_signed(d))
                            .unwrap_or(midnight),
                        "minute",
                    )
                } else {
                    (
                        Duration::try_days(1)
                            .and_then(|d| noon.checked_add_signed(d))
                            .unwrap_or(noon),
                        "minute",
                    )
                }
            } else {
                // 24h/explicit AM/PM: future-first with current-hour tolerance
                if today <= ref_time
                    && today.hour() != ref_time.hour()
                    && !matches!(direction, Some(Direction::Past))
                {
                    (
                        Duration::try_days(1)
                            .and_then(|d| today.checked_add_signed(d))
                            .unwrap_or(today),
                        "minute",
                    )
                } else {
                    (today, "minute")
                }
            }
        }
        TimeForm::HourMinuteSecond(h, m, s) => {
            let dt = ref_time
                .date_naive()
                .and_hms_opt(*h, *m, *s)
                .unwrap_or(ref_time.naive_local())
                .and_local_timezone(*ref_time.offset())
                .single()
                .unwrap();
            (dt, "second")
        }
        TimeForm::Year(y) => {
            let dt = NaiveDate::from_ymd_opt(*y, 1, 1)
                .unwrap_or(ref_time.date_naive())
                .and_hms_opt(0, 0, 0)
                .unwrap()
                .and_local_timezone(*ref_time.offset())
                .single()
                .unwrap();
            (dt, "year")
        }
        TimeForm::RelativeGrain { n, grain } => resolve_relative_grain(*n, *grain, ref_time)?,
        TimeForm::DateMDY { month, day, year } => {
            let build = |y: i32| -> Option<DateTime<FixedOffset>> {
                Some(
                    NaiveDate::from_ymd_opt(y, *month, *day)?
                        .and_hms_opt(0, 0, 0)?
                        .and_local_timezone(*ref_time.offset())
                        .single()
                        .unwrap(),
                )
            };
            let dt = if let Some(y) = year {
                build(*y)?
            } else {
                let ref_year = ref_time.year();
                let this_year = build(ref_year)?;
                let this_date = this_year.date_naive();
                let ref_date = ref_time.date_naive();
                match direction {
                    Some(Direction::Past) => {
                        if this_date < ref_date {
                            this_year
                        } else {
                            build(ref_year.checked_sub(1)?)?
                        }
                    }
                    Some(Direction::Future) => {
                        if this_date > ref_date {
                            this_year
                        } else {
                            build(ref_year.checked_add(1)?)?
                        }
                    }
                    _ => {
                        if this_date < ref_date {
                            build(ref_year.checked_add(1)?)?
                        } else {
                            this_year
                        }
                    }
                }
            };
            (dt, "day")
        }
        TimeForm::GrainOffset { grain, offset } => resolve_grain_offset(*grain, *offset, ref_time)?,
        TimeForm::Quarter(q) => {
            let month = q.saturating_sub(1).saturating_mul(3).saturating_add(1);
            let dt = NaiveDate::from_ymd_opt(ref_time.year(), month, 1)
                .unwrap_or(ref_time.date_naive())
                .and_hms_opt(0, 0, 0)
                .unwrap()
                .and_local_timezone(*ref_time.offset())
                .single()
                .unwrap();
            (dt, "quarter")
        }
        TimeForm::QuarterYear(q, y) => {
            let month = q.saturating_sub(1).saturating_mul(3).saturating_add(1);
            let dt = NaiveDate::from_ymd_opt(*y, month, 1)
                .unwrap_or(ref_time.date_naive())
                .and_hms_opt(0, 0, 0)
                .unwrap()
                .and_local_timezone(*ref_time.offset())
                .single()
                .unwrap();
            (dt, "quarter")
        }
        TimeForm::Holiday(name, year_opt) => {
            if let Some(year) = year_opt {
                // Explicit year: resolve holiday for that year
                match resolve_holiday(name, *year) {
                    Some(date) => (
                        date.and_hms_opt(0, 0, 0)
                            .unwrap()
                            .and_local_timezone(*ref_time.offset())
                            .single()
                            .unwrap(),
                        "day",
                    ),
                    None => (midnight(ref_time), "day"),
                }
            } else {
                // No explicit year: use direction to pick the right occurrence
                resolve_holiday_with_direction(name, ref_time, direction)
            }
        }
        TimeForm::PartOfDay(_) => {
            // Standalone PartOfDay as simple value (shouldn't normally reach here
            // since try_resolve_as_interval handles it, but fallback)
            (
                ref_time
                    .date_naive()
                    .and_hms_opt(12, 0, 0)
                    .unwrap()
                    .and_local_timezone(*ref_time.offset())
                    .single()
                    .unwrap(),
                "hour",
            )
        }
        TimeForm::Weekend => {
            // Fallback simple value
            let dow = ref_time.weekday().num_days_from_monday();
            let days_to_sat = if dow <= 4 {
                5_i64.saturating_sub(dow as i64)
            } else {
                12_i64.saturating_sub(dow as i64)
            };
            let dt = midnight(
                Duration::try_days(days_to_sat)
                    .and_then(|d| ref_time.checked_add_signed(d))
                    .unwrap_or(ref_time),
            );
            (dt, "week")
        }
        TimeForm::Season(_) | TimeForm::AllGrain(_) | TimeForm::RestOfGrain(_) => {
            // Fallback — intervals should be caught by try_resolve_as_interval
            (midnight(ref_time), "day")
        }
        TimeForm::BeginEnd { target, .. } => {
            // Fallback
            resolve_simple_datetime(target, ref_time, direction)?
        }
        TimeForm::Interval(from, _, _) => {
            resolve_simple_datetime(&from.form, ref_time, from.direction)?
        }
        TimeForm::NthGrain { n, grain, past, .. } => {
            // "upcoming 2 days" = cycleNth Day 2 = start of the day 2 days from now
            let signed_n = if *past { n.checked_neg()? } else { *n };
            let base = grain_start(ref_time, *grain);
            let dt = add_grain(base, *grain, signed_n)?;
            (dt, grain.as_str())
        }
        TimeForm::NthDOWOfTime { n, dow, base } => {
            // "first Monday of March" = find the Nth occurrence of DOW within base period
            let (base_dt, _) = resolve_simple_datetime(&base.form, ref_time, base.direction)?;
            let base_start = match &base.form {
                TimeForm::Month(_) => NaiveDate::from_ymd_opt(base_dt.year(), base_dt.month(), 1)
                    .unwrap()
                    .and_hms_opt(0, 0, 0)
                    .unwrap()
                    .and_local_timezone(*ref_time.offset())
                    .single()
                    .unwrap(),
                TimeForm::GrainOffset {
                    grain: Grain::Month,
                    offset,
                } => {
                    let first = start_of_month(ref_time);
                    add_months(first, i64::from(*offset))?
                }
                TimeForm::GrainOffset {
                    grain: Grain::Year,
                    offset,
                } => {
                    let jan1 = start_of_year(ref_time);
                    add_years(jan1, i64::from(*offset))?
                }
                _ => grain_start(base_dt, Grain::Month),
            };
            let dt = nth_dow_of_month(base_start.year(), base_start.month(), *dow, *n as u32);
            (
                dt.and_hms_opt(0, 0, 0)
                    .unwrap()
                    .and_local_timezone(*ref_time.offset())
                    .single()
                    .unwrap(),
                "day",
            )
        }
        TimeForm::LastDOWOfTime { dow, base } => {
            let (base_dt, _) = resolve_simple_datetime(&base.form, ref_time, base.direction)?;
            let base_start = match &base.form {
                TimeForm::Month(_) => NaiveDate::from_ymd_opt(base_dt.year(), base_dt.month(), 1)
                    .unwrap()
                    .and_hms_opt(0, 0, 0)
                    .unwrap()
                    .and_local_timezone(*ref_time.offset())
                    .single()
                    .unwrap(),
                TimeForm::GrainOffset {
                    grain: Grain::Month,
                    offset,
                } => {
                    let first = start_of_month(ref_time);
                    add_months(first, i64::from(*offset))?
                }
                _ => grain_start(base_dt, Grain::Month),
            };
            let dt = last_dow_of_month(base_start.year(), base_start.month(), *dow);
            (
                dt.and_hms_opt(0, 0, 0)
                    .unwrap()
                    .and_local_timezone(*ref_time.offset())
                    .single()
                    .unwrap(),
                "day",
            )
        }
        TimeForm::NDOWsFromTime { n, dow, base } => {
            // "2 Sundays from now" = find the Nth DOW after base
            // "2 Thursdays ago" = find the Nth DOW before base (n is negative)
            let (base_dt, _) = resolve_simple_datetime(&base.form, ref_time, base.direction)?;
            let base_dow = base_dt.weekday().num_days_from_monday();
            let target_dow = *dow;
            if *n > 0 {
                let days_ahead =
                    (i64::from(target_dow).wrapping_sub(i64::from(base_dow))).rem_euclid(7);
                let first = if days_ahead == 0 {
                    Duration::try_days(7)
                        .and_then(|d| base_dt.checked_add_signed(d))
                        .unwrap_or(base_dt)
                } else {
                    Duration::try_days(days_ahead)
                        .and_then(|d| base_dt.checked_add_signed(d))
                        .unwrap_or(base_dt)
                };
                let weeks = (*n as i64).saturating_sub(1);
                let dt = Duration::try_weeks(weeks)
                    .and_then(|d| first.checked_add_signed(d))
                    .unwrap_or(first);
                (midnight(dt), "day")
            } else {
                // Past: find Nth DOW before base
                let abs_n = (n.saturating_neg()) as i64;
                let days_back =
                    (i64::from(base_dow).wrapping_sub(i64::from(target_dow))).rem_euclid(7);
                let first = if days_back == 0 {
                    Duration::try_days(7)
                        .and_then(|d| base_dt.checked_sub_signed(d))
                        .unwrap_or(base_dt)
                } else {
                    Duration::try_days(days_back)
                        .and_then(|d| base_dt.checked_sub_signed(d))
                        .unwrap_or(base_dt)
                };
                let weeks = abs_n.saturating_sub(1);
                let dt = Duration::try_weeks(weeks)
                    .and_then(|d| first.checked_sub_signed(d))
                    .unwrap_or(first);
                (midnight(dt), "day")
            }
        }
        TimeForm::NthClosestToTime { n, target, base } => {
            // Haskell: predNthClosest — merge past/future candidates sorted by distance
            // Generates past and future occurrences of target relative to base,
            // then picks the nth closest by absolute distance.
            let (base_dt, _) = resolve_simple_datetime(&base.form, ref_time, base.direction)?;
            let mut candidates: Vec<DateTime<FixedOffset>> = Vec::new();

            // Determine the cyclic grain for generating candidates
            // Holidays and months cycle yearly; DOWs cycle weekly
            let cycle_grain = match &target.form {
                TimeForm::Holiday(..)
                | TimeForm::Month(_)
                | TimeForm::DateMDY { .. }
                | TimeForm::Composed(..)
                | TimeForm::DayOfMonth(_) => Grain::Year,
                TimeForm::DayOfWeek(_) => Grain::Week,
                _ => form_grain(&target.form),
            };

            // Generate candidates in both directions from base
            let range = (*n).max(5).saturating_add(2); // enough to find nth closest
            for i in (range.saturating_neg())..=(range) {
                let offset_ref = add_grain(base_dt, cycle_grain, i64::from(i))?;
                let (dt, _) = resolve_simple_datetime(&target.form, offset_ref, None)?;
                if !candidates
                    .iter()
                    .any(|c| c.signed_duration_since(dt).num_seconds().abs() < 86400)
                {
                    candidates.push(dt);
                }
            }
            candidates.sort_by_key(|c| (c.signed_duration_since(base_dt).num_seconds()).abs());
            let idx = ((*n).max(1).saturating_sub(1)) as usize;
            let result = candidates.get(idx).copied().unwrap_or(base_dt);
            let out_grain = form_grain(&target.form);
            (result, out_grain.as_str())
        }
        TimeForm::NthGrainOfTime { n, grain, base } => {
            // "first week of October 2014" → first Monday-aligned week within October
            let (base_dt, _) = resolve_simple_datetime(&base.form, ref_time, base.direction)?;
            let base_start = grain_start(base_dt, target_grain(&base.form));
            if *grain == Grain::Week {
                // Find first Monday on or after base_start
                let dow = base_start.weekday().num_days_from_monday();
                let first_monday = if dow == 0 {
                    base_start
                } else {
                    Duration::try_days((7u32.saturating_sub(dow)) as i64)
                        .and_then(|d| base_start.checked_add_signed(d))
                        .unwrap_or(base_start)
                };
                let dt = Duration::try_weeks((*n).saturating_sub(1) as i64)
                    .and_then(|d| first_monday.checked_add_signed(d))
                    .unwrap_or(first_monday);
                (dt, "week")
            } else if *grain == Grain::Day {
                // "third day of october" = Oct 3
                let dt = add_grain(base_start, Grain::Day, i64::from(n.checked_sub(1)?))?;
                (dt, "day")
            } else {
                let dt = add_grain(base_start, *grain, i64::from(n.checked_sub(1)?))?;
                (dt, grain.as_str())
            }
        }
        TimeForm::NthLastDayOfTime { n, base } => {
            // "last day of October 2015", "5th last day of May"
            let (base_dt, _) = resolve_simple_datetime(&base.form, ref_time, base.direction)?;
            let base_start = match &base.form {
                TimeForm::Month(_) => NaiveDate::from_ymd_opt(base_dt.year(), base_dt.month(), 1)
                    .unwrap()
                    .and_hms_opt(0, 0, 0)
                    .unwrap()
                    .and_local_timezone(*ref_time.offset())
                    .single()
                    .unwrap(),
                TimeForm::GrainOffset {
                    grain: Grain::Month,
                    offset,
                } => {
                    let first = start_of_month(ref_time);
                    add_months(first, i64::from(*offset))?
                }
                _ => grain_start(base_dt, Grain::Month),
            };
            let month_end = add_grain(base_start, Grain::Month, 1)?;
            let dt = Duration::try_days(*n as i64).and_then(|d| month_end.checked_sub_signed(d))?;
            (dt, "day")
        }
        TimeForm::LastCycleOfTime { grain, base } => {
            // "last week of September" → last full week within the period
            let (base_dt, _) = resolve_simple_datetime(&base.form, ref_time, base.direction)?;
            let base_start = grain_start(base_dt, target_grain(&base.form));
            let base_end = add_grain(base_start, target_grain(&base.form), 1)?;
            if *grain == Grain::Week {
                // Find last Monday such that the week (Mon-Sun) fits within the period
                // Start from base_end and go backwards
                let end_dow = base_end.weekday().num_days_from_monday();
                // Last Monday before base_end
                let last_mon = Duration::try_days((end_dow as i64).saturating_add(7))
                    .and_then(|d| base_end.checked_sub_signed(d))
                    .unwrap_or(base_end);
                // If this week fits in the period (Sun <= base_end - 1 day), use it
                let week_end = Duration::try_days(7)
                    .and_then(|d| last_mon.checked_add_signed(d))
                    .unwrap_or(last_mon); // next Monday = exclusive end
                let dt = if week_end <= base_end {
                    last_mon
                } else {
                    Duration::try_weeks(1)
                        .and_then(|d| last_mon.checked_sub_signed(d))
                        .unwrap_or(last_mon)
                };
                (dt, "week")
            } else {
                let dt = add_grain(base_end, *grain, -1)?;
                (dt, grain.as_str())
            }
        }
        TimeForm::DurationAfter { n, grain, base } => {
            // Haskell: durationAfter — shift each occurrence of base by duration, pick nearest future
            let (future_base, _) = resolve_simple_datetime(&base.form, ref_time, base.direction)?;
            let (past_base, _) =
                resolve_simple_datetime(&base.form, ref_time, Some(Direction::Past))?;
            let add_dur = |dt: DateTime<FixedOffset>| -> Option<DateTime<FixedOffset>> {
                match grain {
                    Grain::Year => add_years(dt, *n),
                    Grain::Month => add_months(dt, *n),
                    Grain::Quarter => add_months(dt, n.checked_mul(3)?),
                    Grain::Week => Duration::try_weeks(*n).and_then(|d| dt.checked_add_signed(d)),
                    Grain::Day => Duration::try_days(*n).and_then(|d| dt.checked_add_signed(d)),
                    Grain::Hour => Duration::try_hours(*n).and_then(|d| dt.checked_add_signed(d)),
                    Grain::Minute => {
                        Duration::try_minutes(*n).and_then(|d| dt.checked_add_signed(d))
                    }
                    Grain::NoGrain | Grain::Second => {
                        Duration::try_seconds(*n).and_then(|d| dt.checked_add_signed(d))
                    }
                }
            };
            let future_result = add_dur(future_base)?;
            let past_result = add_dur(past_base)?;
            // Pick nearest-future result
            let result = if past_result >= ref_time
                && (future_result < ref_time || past_result <= future_result)
            {
                past_result
            } else {
                future_result
            };
            // Haskell: mergeDuration uses min(base_grain, duration_grain) for TimeObject grain
            let base_grain = form_grain(&base.form);
            let out_grain = if base_grain < *grain {
                base_grain
            } else {
                *grain
            };
            (result, out_grain.as_str())
        }
        TimeForm::NthLastCycleOfTime { n, grain, base } => {
            // Haskell: cycleNthAfter True grain (-n) $ cycleNthAfter True (timeGrain td) 1 td
            // Position at end of base period, count backward n complete cycles of grain
            let (base_dt, _) = resolve_simple_datetime(&base.form, ref_time, base.direction)?;
            let base_grain = target_grain(&base.form);
            let base_start = grain_start(base_dt, base_grain);
            let base_end = add_grain(base_start, base_grain, 1)?;
            if *grain == Grain::Week {
                // Find the last COMPLETE week within the period (Mon+7 <= base_end)
                let end_dow = base_end.weekday().num_days_from_monday();
                let last_mon = if end_dow == 0 {
                    Duration::try_weeks(1).and_then(|d| base_end.checked_sub_signed(d))?
                } else {
                    Duration::try_days(end_dow as i64)
                        .and_then(|d| base_end.checked_sub_signed(d))?
                };
                // Check if this week fits completely
                let last_complete_mon = if Duration::try_weeks(1)
                    .and_then(|d| last_mon.checked_add_signed(d))?
                    <= base_end
                {
                    last_mon
                } else {
                    Duration::try_weeks(1).and_then(|d| last_mon.checked_sub_signed(d))?
                };
                // Go back (n-1) more weeks
                let dt = Duration::try_weeks((*n as i64).saturating_sub(1))
                    .and_then(|d| last_complete_mon.checked_sub_signed(d))?;
                (dt, "week")
            } else {
                // For day grain: go back n grains from end of period
                let dt = add_grain(base_end, *grain, i64::from(n.checked_neg()?))?;
                (dt, grain.as_str())
            }
        }
        TimeForm::Composed(primary, secondary) => resolve_composed(primary, secondary, ref_time)?,
    })
}

// ============================================================
// Day of week resolution
// ============================================================

fn resolve_dow(
    dow: u32,
    ref_time: DateTime<FixedOffset>,
    direction: Option<Direction>,
) -> DateTime<FixedOffset> {
    let current = ref_time.weekday().num_days_from_monday();
    let target = dow;

    let days = match direction {
        Some(Direction::Future) | Some(Direction::FarFuture) => {
            // "next DOW": go to next week's instance
            let days_to_next_monday = if current == 0 {
                7u32
            } else {
                7u32.saturating_sub(current)
            };
            days_to_next_monday.saturating_add(target) as i64
        }
        Some(Direction::Past) => {
            // "last DOW": most recent past, excluding today
            let back = match target.cmp(&current) {
                std::cmp::Ordering::Less => current.saturating_sub(target),
                std::cmp::Ordering::Greater => 7u32.saturating_sub(target.saturating_sub(current)),
                std::cmp::Ordering::Equal => 7, // same day = last week
            };
            (back as i64).saturating_neg()
        }
        None => {
            // Plain DOW: nearest future, skip today
            let ahead = match target.cmp(&current) {
                std::cmp::Ordering::Greater => target.saturating_sub(current),
                std::cmp::Ordering::Less => 7u32.saturating_sub(current.saturating_sub(target)),
                std::cmp::Ordering::Equal => 7, // same day = next week
            };
            ahead as i64
        }
    };

    midnight(
        Duration::try_days(days)
            .and_then(|d| ref_time.checked_add_signed(d))
            .unwrap_or(ref_time),
    )
}

// ============================================================
// Month resolution
// ============================================================

fn resolve_month(
    m: u32,
    ref_time: DateTime<FixedOffset>,
    direction: Option<Direction>,
) -> DateTime<FixedOffset> {
    let next_occurrence = if m > ref_time.month() || (m == ref_time.month() && direction.is_none())
    {
        NaiveDate::from_ymd_opt(ref_time.year(), m, 1)
    } else {
        NaiveDate::from_ymd_opt(ref_time.year().saturating_add(1), m, 1)
    };

    let base = next_occurrence
        .unwrap_or(ref_time.date_naive())
        .and_hms_opt(0, 0, 0)
        .unwrap()
        .and_local_timezone(*ref_time.offset())
        .single()
        .unwrap();

    match direction {
        Some(Direction::Past) => {
            // Most recent past occurrence
            let year = if m < ref_time.month() || (m == ref_time.month()) {
                ref_time.year()
            } else {
                ref_time.year().saturating_sub(1)
            };
            NaiveDate::from_ymd_opt(year, m, 1)
                .unwrap_or(ref_time.date_naive())
                .and_hms_opt(0, 0, 0)
                .unwrap()
                .and_local_timezone(*ref_time.offset())
                .single()
                .unwrap()
        }
        Some(Direction::FarFuture) => {
            // "March after next": skip one occurrence
            let year = if m >= ref_time.month() {
                ref_time.year().saturating_add(1)
            } else {
                ref_time.year().saturating_add(2)
            };
            NaiveDate::from_ymd_opt(year, m, 1)
                .unwrap_or(ref_time.date_naive())
                .and_hms_opt(0, 0, 0)
                .unwrap()
                .and_local_timezone(*ref_time.offset())
                .single()
                .unwrap()
        }
        _ => base,
    }
}

// ============================================================
// RelativeGrain resolution (in N <grain>, N <grain> ago)
// ============================================================

fn resolve_relative_grain(
    n: i64,
    grain: Grain,
    ref_time: DateTime<FixedOffset>,
) -> Option<(DateTime<FixedOffset>, &'static str)> {
    let lower = grain.lower();
    let result = match grain {
        Grain::NoGrain | Grain::Second => {
            Duration::try_seconds(n).and_then(|d| ref_time.checked_add_signed(d))?
        }
        Grain::Minute => Duration::try_minutes(n).and_then(|d| ref_time.checked_add_signed(d))?,
        Grain::Hour => Duration::try_hours(n).and_then(|d| ref_time.checked_add_signed(d))?,
        Grain::Day => Duration::try_days(n).and_then(|d| ref_time.checked_add_signed(d))?,
        Grain::Week => Duration::try_weeks(n).and_then(|d| ref_time.checked_add_signed(d))?,
        Grain::Month => add_months(ref_time, n)?,
        Grain::Quarter => add_months(ref_time, n.checked_mul(3)?)?,
        Grain::Year => add_years(ref_time, n)?,
    };
    // Truncate to lower grain boundary
    let truncated = grain_start(result, lower);
    Some((truncated, lower.as_str()))
}

// ============================================================
// GrainOffset resolution (this/last/next week/month/year/quarter)
// ============================================================

fn resolve_grain_offset(
    grain: Grain,
    offset: i32,
    ref_time: DateTime<FixedOffset>,
) -> Option<(DateTime<FixedOffset>, &'static str)> {
    let result = match grain {
        Grain::Week => {
            let monday = start_of_week(ref_time);
            Duration::try_weeks(offset as i64).and_then(|d| monday.checked_add_signed(d))?
        }
        Grain::Month => {
            let first = start_of_month(ref_time);
            add_months(first, i64::from(offset))?
        }
        Grain::Year => {
            let jan1 = start_of_year(ref_time);
            add_years(jan1, i64::from(offset))?
        }
        Grain::Quarter => {
            let qstart = start_of_quarter(ref_time);
            add_months(qstart, i64::from(offset).checked_mul(3)?)?
        }
        Grain::Day => Duration::try_days(offset as i64)
            .and_then(|d| midnight(ref_time).checked_add_signed(d))?,
        _ => {
            // For smaller grains, just offset from ref_time
            let secs = grain.in_seconds(offset as i64).unwrap_or(0);
            Duration::try_seconds(secs).and_then(|d| ref_time.checked_add_signed(d))?
        }
    };
    Some((result, grain.as_str()))
}

// ============================================================
// Composed form resolution
// ============================================================

fn resolve_composed(
    primary: &TimeData,
    secondary: &TimeData,
    ref_time: DateTime<FixedOffset>,
) -> Option<(DateTime<FixedOffset>, &'static str)> {
    // Helper: disambiguate hour based on PartOfDay context
    fn disambiguate_hour(h: u32, is_12h: bool, pod: Option<&PartOfDay>) -> u32 {
        if !is_12h {
            return h;
        }
        // Haskell logic: (start < 12 || hours == 12) → AM
        // When hour == 12 with is_12h, always treat as midnight (0) in any PartOfDay context
        if h == 12 {
            return match pod {
                Some(_) => 0, // "this morning/afternoon/evening at 12" → midnight
                None => h,    // No PartOfDay context → keep 12
            };
        }
        match pod {
            Some(PartOfDay::Afternoon) | Some(PartOfDay::Evening) | Some(PartOfDay::Night) => {
                if h < 12 {
                    h.saturating_add(12)
                } else {
                    h
                }
            }
            Some(PartOfDay::Morning) => {
                if h == 12 {
                    0
                } else {
                    h
                }
            }
            _ => h,
        }
    }

    // Extract PartOfDay from a TimeData (possibly nested in Composed)
    fn extract_pod(td: &TimeData) -> Option<&PartOfDay> {
        match &td.form {
            TimeForm::PartOfDay(pod) => Some(pod),
            TimeForm::Composed(a, b) => extract_pod(a).or_else(|| extract_pod(b)),
            _ => None,
        }
    }

    // Extract base date from a possibly nested Composed form
    fn extract_date(
        td: &TimeData,
        ref_time: DateTime<FixedOffset>,
    ) -> Option<DateTime<FixedOffset>> {
        match &td.form {
            TimeForm::Composed(a, b) => {
                // Try to get the date part (non-PartOfDay component)
                match (&a.form, &b.form) {
                    (TimeForm::PartOfDay(_), _) => extract_date(b, ref_time),
                    (_, TimeForm::PartOfDay(_)) => extract_date(a, ref_time),
                    _ => {
                        // Resolve full composition (e.g., DOW + DateMDY → proper intersection)
                        let (dt, _) = resolve_composed(a, b, ref_time)?;
                        Some(dt)
                    }
                }
            }
            _ => {
                let (dt, _) = resolve_simple_datetime(&td.form, ref_time, td.direction)?;
                Some(dt)
            }
        }
    }

    // If secondary is a clock time, combine with primary's date
    match &secondary.form {
        TimeForm::Hour(h, is_12h) => {
            let pod = extract_pod(primary);
            let hour = disambiguate_hour(*h, *is_12h, pod);
            let date_dt = extract_date(primary, ref_time)?;
            let mut dt = date_dt
                .date_naive()
                .and_hms_opt(hour, 0, 0)
                .unwrap_or(date_dt.naive_local())
                .and_local_timezone(*ref_time.offset())
                .single()
                .unwrap();
            // Future-first: if result is past and date is today, advance to tomorrow
            if dt <= ref_time && date_dt.date_naive() == ref_time.date_naive() {
                dt = Duration::try_days(1)
                    .and_then(|d| dt.checked_add_signed(d))
                    .unwrap_or(dt);
            }
            return Some((dt, "hour"));
        }
        TimeForm::HourMinute(h, m, is_12h) => {
            let pod = extract_pod(primary);
            let hour = disambiguate_hour(*h, *is_12h, pod);
            let date_dt = extract_date(primary, ref_time)?;
            let mut dt = date_dt
                .date_naive()
                .and_hms_opt(hour, *m, 0)
                .unwrap_or(date_dt.naive_local())
                .and_local_timezone(*ref_time.offset())
                .single()
                .unwrap();
            if dt <= ref_time && date_dt.date_naive() == ref_time.date_naive() {
                dt = Duration::try_days(1)
                    .and_then(|d| dt.checked_add_signed(d))
                    .unwrap_or(dt);
            }
            return Some((dt, "minute"));
        }
        TimeForm::HourMinuteSecond(h, m, s) => {
            let (date_dt, _) = resolve_simple_datetime(&primary.form, ref_time, primary.direction)?;
            let dt = date_dt
                .date_naive()
                .and_hms_opt(*h, *m, *s)
                .unwrap_or(date_dt.naive_local())
                .and_local_timezone(*ref_time.offset())
                .single()
                .unwrap();
            return Some((dt, "second"));
        }
        TimeForm::Now => {
            // "X from right now" — add duration to ref_time directly (no truncation)
            if let TimeForm::RelativeGrain { n, grain } = &primary.form {
                let result =
                    match grain {
                        Grain::Year => add_years(ref_time, *n)?,
                        Grain::Month => add_months(ref_time, *n)?,
                        Grain::Week => {
                            Duration::try_weeks(*n).and_then(|d| ref_time.checked_add_signed(d))?
                        }
                        Grain::Day => {
                            Duration::try_days(*n).and_then(|d| ref_time.checked_add_signed(d))?
                        }
                        Grain::Hour => {
                            Duration::try_hours(*n).and_then(|d| ref_time.checked_add_signed(d))?
                        }
                        Grain::Minute => Duration::try_minutes(*n)
                            .and_then(|d| ref_time.checked_add_signed(d))?,
                        Grain::NoGrain | Grain::Second => Duration::try_seconds(*n)
                            .and_then(|d| ref_time.checked_add_signed(d))?,
                        Grain::Quarter => add_months(ref_time, n.checked_mul(3)?)?,
                    };
                return Some((result, "second"));
            }
            let (dt, _) = resolve_simple_datetime(&primary.form, ref_time, primary.direction)?;
            return Some((dt, "second"));
        }
        _ => {}
    }

    // If primary is a clock time, combine with secondary's date
    match &primary.form {
        TimeForm::Hour(h, is_12h) => {
            let pod = extract_pod(secondary);
            let hour = disambiguate_hour(*h, *is_12h, pod);
            let date_dt = extract_date(secondary, ref_time)?;
            let mut dt = date_dt
                .date_naive()
                .and_hms_opt(hour, 0, 0)
                .unwrap_or(date_dt.naive_local())
                .and_local_timezone(*ref_time.offset())
                .single()
                .unwrap();
            // Future-first: if result is past and date is today, advance to tomorrow
            if dt <= ref_time && date_dt.date_naive() == ref_time.date_naive() {
                dt = Duration::try_days(1)
                    .and_then(|d| dt.checked_add_signed(d))
                    .unwrap_or(dt);
            }
            return Some((dt, "hour"));
        }
        TimeForm::HourMinute(h, m, is_12h) => {
            let pod = extract_pod(secondary);
            let hour = disambiguate_hour(*h, *is_12h, pod);
            let date_dt = extract_date(secondary, ref_time)?;
            let mut dt = date_dt
                .date_naive()
                .and_hms_opt(hour, *m, 0)
                .unwrap_or(date_dt.naive_local())
                .and_local_timezone(*ref_time.offset())
                .single()
                .unwrap();
            if dt <= ref_time && date_dt.date_naive() == ref_time.date_naive() {
                dt = Duration::try_days(1)
                    .and_then(|d| dt.checked_add_signed(d))
                    .unwrap_or(dt);
            }
            return Some((dt, "minute"));
        }
        TimeForm::HourMinuteSecond(h, m, s) => {
            let date_dt = extract_date(secondary, ref_time)?;
            let dt = date_dt
                .date_naive()
                .and_hms_opt(*h, *m, *s)
                .unwrap_or(date_dt.naive_local())
                .and_local_timezone(*ref_time.offset())
                .single()
                .unwrap();
            return Some((dt, "second"));
        }
        _ => {}
    }

    // If secondary is a date-like form, use secondary for the date context
    match &secondary.form {
        TimeForm::RelativeGrain { n, grain } => {
            // Special case: Holiday + duration (Haskell: intersect td (inDurationInterval dd))
            // "thanksgiving in a year" → find thanksgiving in the year of (ref + 1yr)
            // "thanksgiving 3 months ago" → find thanksgiving in the year of (ref - 3months)
            // "three days after easter" → find nearest easter, add 3 days
            if let TimeForm::Holiday(name, _) = &primary.form {
                if *grain >= Grain::Day {
                    // For day-level durations (days, weeks), add to nearest holiday occurrence
                    if *grain == Grain::Day || *grain == Grain::Week {
                        let (future_base, _) = resolve_holiday_with_direction(name, ref_time, None);
                        let (past_base, _) =
                            resolve_holiday_with_direction(name, ref_time, Some(Direction::Past));
                        let add_dur = |base: DateTime<FixedOffset>| -> DateTime<FixedOffset> {
                            match grain {
                                Grain::Week => Duration::try_weeks(*n)
                                    .and_then(|d| base.checked_add_signed(d))
                                    .unwrap_or(base),
                                Grain::Day => Duration::try_days(*n)
                                    .and_then(|d| base.checked_add_signed(d))
                                    .unwrap_or(base),
                                _ => base,
                            }
                        };
                        let future_result = add_dur(future_base);
                        let past_result = add_dur(past_base);
                        let result = if past_result >= ref_time
                            && (future_result < ref_time || past_result <= future_result)
                        {
                            past_result
                        } else {
                            future_result
                        };
                        return Some((result, "day"));
                    }
                    // For month/year/quarter durations: compute target time, find holiday in target year
                    // Matches Haskell's intersect(holiday, inDurationInterval(dd))
                    let target_time = match grain {
                        Grain::Year => add_years(ref_time, *n)?,
                        Grain::Month => add_months(ref_time, *n)?,
                        Grain::Quarter => add_months(ref_time, n.checked_mul(3)?)?,
                        _ => ref_time,
                    };
                    let target_year = target_time.year();
                    if let Some(date) = resolve_holiday(name, target_year) {
                        return Some((
                            date.and_hms_opt(0, 0, 0)
                                .unwrap()
                                .and_local_timezone(*ref_time.offset())
                                .single()
                                .unwrap(),
                            "day",
                        ));
                    }
                }
            }

            // "today in one hour" → add to ref_time (not midnight)
            // "3 years from today" → add years but keep today's date
            let is_day_level_base = matches!(
                &primary.form,
                TimeForm::Holiday(..)
                    | TimeForm::DateMDY { .. }
                    | TimeForm::DayOfMonth(_)
                    | TimeForm::DayOfWeek(_)
                    | TimeForm::Tomorrow
                    | TimeForm::Yesterday
                    | TimeForm::DayAfterTomorrow
                    | TimeForm::DayBeforeYesterday
                    | TimeForm::Today
            );

            fn apply_duration(
                base: DateTime<FixedOffset>,
                n: i64,
                grain: &Grain,
            ) -> Option<DateTime<FixedOffset>> {
                match grain {
                    Grain::Year => add_years(base, n),
                    Grain::Month => add_months(base, n),
                    Grain::Week => Duration::try_weeks(n).and_then(|d| base.checked_add_signed(d)),
                    Grain::Day => Duration::try_days(n).and_then(|d| base.checked_add_signed(d)),
                    Grain::Hour => Duration::try_hours(n).and_then(|d| base.checked_add_signed(d)),
                    Grain::Minute => {
                        Duration::try_minutes(n).and_then(|d| base.checked_add_signed(d))
                    }
                    Grain::NoGrain | Grain::Second => {
                        Duration::try_seconds(n).and_then(|d| base.checked_add_signed(d))
                    }
                    Grain::Quarter => add_months(base, n.checked_mul(3)?),
                }
            }

            let base = match &primary.form {
                TimeForm::Today | TimeForm::Now => ref_time,
                _ => {
                    // For day-level bases (holidays, dates), try both past and future
                    // occurrences and pick the one whose result is nearest-future
                    if is_day_level_base && *n > 0 {
                        let future_base = {
                            let (dt, _) = resolve_simple_datetime(
                                &primary.form,
                                ref_time,
                                primary.direction,
                            )?;
                            dt
                        };
                        let past_base = {
                            let (dt, _) = resolve_simple_datetime(
                                &primary.form,
                                ref_time,
                                Some(Direction::Past),
                            )?;
                            dt
                        };
                        let future_result = apply_duration(future_base, *n, grain)?;
                        let past_result = apply_duration(past_base, *n, grain)?;
                        // Pick nearest future, or if both future, pick closest
                        if past_result >= ref_time
                            && (future_result < ref_time || past_result <= future_result)
                        {
                            past_base
                        } else {
                            future_base
                        }
                    } else {
                        let (dt, _) =
                            resolve_simple_datetime(&primary.form, ref_time, primary.direction)?;
                        dt
                    }
                }
            };
            let result = apply_duration(base, *n, grain)?;
            if is_day_level_base && *grain >= Grain::Day {
                return Some((grain_start(result, Grain::Day), "day"));
            }
            let out_grain = grain.lower().as_str();
            return Some((grain_start(result, grain.lower()), out_grain));
        }
        TimeForm::GrainOffset { grain, offset } => {
            // "Monday of this week", "20th of next month"
            let offset_base = match grain {
                Grain::Week => {
                    let monday = start_of_week(ref_time);
                    Duration::try_weeks(*offset as i64)
                        .and_then(|d| monday.checked_add_signed(d))
                        .unwrap_or(monday)
                }
                Grain::Month => {
                    let first = start_of_month(ref_time);
                    add_months(first, i64::from(*offset))?
                }
                Grain::Year => {
                    let jan1 = start_of_year(ref_time);
                    add_years(jan1, i64::from(*offset))?
                }
                _ => ref_time,
            };
            // For DOW within a specific week, directly compute the day
            if *grain == Grain::Week {
                if let TimeForm::DayOfWeek(dow) = &primary.form {
                    let dt = Duration::try_days(*dow as i64)
                        .and_then(|d| offset_base.checked_add_signed(d))
                        .unwrap_or(offset_base);
                    return Some((dt, "day"));
                }
            }
            // For DayOfMonth within a specific month, directly compute
            if *grain == Grain::Month {
                if let TimeForm::DayOfMonth(day) = &primary.form {
                    let dt = NaiveDate::from_ymd_opt(offset_base.year(), offset_base.month(), *day)
                        .unwrap_or(offset_base.date_naive())
                        .and_hms_opt(0, 0, 0)
                        .unwrap()
                        .and_local_timezone(*ref_time.offset())
                        .single()
                        .unwrap();
                    return Some((dt, "day"));
                }
            }
            // Resolve primary in the context of the offset base
            let (result, primary_grain) =
                resolve_simple_datetime(&primary.form, offset_base, primary.direction)?;
            return Some((result, primary_grain));
        }
        _ => {}
    }

    // DayOfMonth + Month (or vice versa) → resolve as month-day intersection.
    // Matches Haskell's `monthDay m d = intersect' (month m, dayOfMonth d)`.
    // Propagates direction from inner components (Haskell's `direction = d1 <|> d2`).
    fn resolve_month_day(
        month: u32,
        day: u32,
        direction: Option<Direction>,
        ref_time: DateTime<FixedOffset>,
    ) -> Option<(DateTime<FixedOffset>, &'static str)> {
        let build = |y: i32| -> Option<DateTime<FixedOffset>> {
            Some(
                NaiveDate::from_ymd_opt(y, month, day)?
                    .and_hms_opt(0, 0, 0)?
                    .and_local_timezone(*ref_time.offset())
                    .single()
                    .unwrap(),
            )
        };
        let ref_year = ref_time.year();
        let this_year = build(ref_year)?;
        let this_date = this_year.date_naive();
        let ref_date = ref_time.date_naive();
        let dt = match direction {
            Some(Direction::Past) => {
                if this_date < ref_date {
                    this_year
                } else {
                    build(ref_year.checked_sub(1)?)?
                }
            }
            Some(Direction::Future) => {
                if this_date > ref_date {
                    this_year
                } else {
                    build(ref_year.checked_add(1)?)?
                }
            }
            _ => {
                // Future-first default
                if this_date < ref_date {
                    build(ref_year.checked_add(1)?)?
                } else {
                    this_year
                }
            }
        };
        Some((dt, "day"))
    }

    if let TimeForm::DayOfMonth(day) = &primary.form {
        if let TimeForm::Month(month) = &secondary.form {
            let direction = primary.direction.or(secondary.direction);
            return resolve_month_day(*month, *day, direction, ref_time);
        }
    }
    if let TimeForm::Month(month) = &primary.form {
        if let TimeForm::DayOfMonth(day) = &secondary.form {
            let direction = primary.direction.or(secondary.direction);
            return resolve_month_day(*month, *day, direction, ref_time);
        }
    }

    // Any date-like + Year → resolve date in that year
    if let TimeForm::Year(y) = &secondary.form {
        // Holiday + Year: re-resolve the holiday for the target year
        if let TimeForm::Holiday(name, _) = &primary.form {
            match resolve_holiday(name, *y) {
                Some(date) => {
                    return Some((
                        date.and_hms_opt(0, 0, 0)
                            .unwrap()
                            .and_local_timezone(*ref_time.offset())
                            .single()
                            .unwrap(),
                        "day",
                    ))
                }
                None => return Some((midnight(ref_time), "day")),
            }
        }
        let (base, base_grain) =
            resolve_simple_datetime(&primary.form, ref_time, primary.direction)?;
        match &primary.form {
            TimeForm::DayOfMonth(_) | TimeForm::DateMDY { .. } | TimeForm::Composed(..) => {
                let dt = NaiveDate::from_ymd_opt(*y, base.month(), base.day())
                    .unwrap_or(ref_time.date_naive())
                    .and_hms_opt(0, 0, 0)
                    .unwrap()
                    .and_local_timezone(*ref_time.offset())
                    .single()
                    .unwrap();
                return Some((dt, "day"));
            }
            TimeForm::Month(m) => {
                let dt = NaiveDate::from_ymd_opt(*y, *m, 1)
                    .unwrap_or(ref_time.date_naive())
                    .and_hms_opt(0, 0, 0)
                    .unwrap()
                    .and_local_timezone(*ref_time.offset())
                    .single()
                    .unwrap();
                return Some((dt, "month"));
            }
            _ => {
                // Generic: just override the year
                let dt = NaiveDate::from_ymd_opt(*y, base.month(), base.day())
                    .unwrap_or(ref_time.date_naive())
                    .and_hms_opt(base.hour(), base.minute(), base.second())
                    .unwrap()
                    .and_local_timezone(*ref_time.offset())
                    .single()
                    .unwrap();
                return Some((dt, base_grain));
            }
        }
    }
    // Year + Month → resolve month in that year
    if let TimeForm::Year(y) = &primary.form {
        if let TimeForm::Month(m) = &secondary.form {
            let dt = NaiveDate::from_ymd_opt(*y, *m, 1)
                .unwrap_or(ref_time.date_naive())
                .and_hms_opt(0, 0, 0)
                .unwrap()
                .and_local_timezone(*ref_time.offset())
                .single()
                .unwrap();
            return Some((dt, "month"));
        }
    }

    // If primary is a PartOfDay and secondary has a date, combine
    if let TimeForm::PartOfDay(_) = &primary.form {
        let (date_dt, _) = resolve_simple_datetime(&secondary.form, ref_time, secondary.direction)?;
        return Some((date_dt, "hour"));
    }

    // Helper: extract (month, day) from a Composed(Month, DayOfMonth) form
    fn extract_md_from_composed(form: &TimeForm) -> Option<(u32, u32)> {
        if let TimeForm::Composed(a, b) = form {
            if let (TimeForm::Month(m), TimeForm::DayOfMonth(d)) = (&a.form, &b.form) {
                return Some((*m, *d));
            }
            if let (TimeForm::DayOfMonth(d), TimeForm::Month(m)) = (&a.form, &b.form) {
                return Some((*m, *d));
            }
        }
        None
    }

    // DOW + specific date → use the date, validating DOW match
    // If DOW doesn't match, find the next year where it does
    // Must come BEFORE the nested Composed handler to avoid premature year-fixing
    if let TimeForm::DayOfWeek(dow) = &primary.form {
        match &secondary.form {
            TimeForm::DateMDY { month, day, year } => {
                let (date_dt, _) =
                    resolve_simple_datetime(&secondary.form, ref_time, secondary.direction)?;
                return Some((
                    find_dow_date_intersection(*dow, date_dt, *month, *day, *year),
                    "day",
                ));
            }
            TimeForm::Holiday(..) => {
                return resolve_simple_datetime(&secondary.form, ref_time, secondary.direction);
            }
            TimeForm::DayOfMonth(day) => {
                return Some((find_dow_dom_intersection(*dow, *day, ref_time), "day"));
            }
            _ => {
                // Handle Composed(Month, DayOfMonth) — use year: None so DOW search works
                if let Some((month, day)) = extract_md_from_composed(&secondary.form) {
                    let (date_dt, _) = resolve_month_day(month, day, None, ref_time)?;
                    return Some((
                        find_dow_date_intersection(*dow, date_dt, month, day, None),
                        "day",
                    ));
                }
            }
        }
    }
    // Specific date + DOW → use the date, validating DOW match
    if let TimeForm::DayOfWeek(dow) = &secondary.form {
        match &primary.form {
            TimeForm::DateMDY { month, day, year } => {
                let (date_dt, _) =
                    resolve_simple_datetime(&primary.form, ref_time, primary.direction)?;
                return Some((
                    find_dow_date_intersection(*dow, date_dt, *month, *day, *year),
                    "day",
                ));
            }
            TimeForm::Holiday(..) => {
                return resolve_simple_datetime(&primary.form, ref_time, primary.direction);
            }
            TimeForm::DayOfMonth(day) => {
                return Some((find_dow_dom_intersection(*dow, *day, ref_time), "day"));
            }
            _ => {
                if let Some((month, day)) = extract_md_from_composed(&primary.form) {
                    let (date_dt, _) = resolve_month_day(month, day, None, ref_time)?;
                    return Some((
                        find_dow_date_intersection(*dow, date_dt, month, day, None),
                        "day",
                    ));
                }
            }
        }
    }

    // Nested Composed: if one side is Composed, try to resolve it first
    if let TimeForm::Composed(a, b) = &primary.form {
        let (primary_dt, _primary_grain) = resolve_composed(a, b, ref_time)?;
        let new_primary = TimeData::new(TimeForm::DateMDY {
            month: primary_dt.month(),
            day: primary_dt.day(),
            year: Some(primary_dt.year()),
        });
        return resolve_composed(&new_primary, secondary, ref_time);
    }
    if let TimeForm::Composed(a, b) = &secondary.form {
        let (secondary_dt, _secondary_grain) = resolve_composed(a, b, ref_time)?;
        let new_secondary = TimeData::new(TimeForm::DateMDY {
            month: secondary_dt.month(),
            day: secondary_dt.day(),
            year: Some(secondary_dt.year()),
        });
        return resolve_composed(primary, &new_secondary, ref_time);
    }

    // GrainOffset + DOW: find the DOW within the grain period
    // e.g., "last week's sunday" → Sunday of last week
    if let TimeForm::DayOfWeek(dow) = &secondary.form {
        if let TimeForm::GrainOffset { .. } = &primary.form {
            let (period_start, _) =
                resolve_simple_datetime(&primary.form, ref_time, primary.direction)?;
            let period_date = period_start.date_naive();
            let current_dow = period_date.weekday().num_days_from_monday();
            let days_to_target = if *dow >= current_dow {
                dow.saturating_sub(current_dow) as i64
            } else {
                dow.saturating_add(7).saturating_sub(current_dow) as i64
            };
            let target_date = chrono::Duration::try_days(days_to_target)
                .and_then(|d| period_date.checked_add_signed(d))
                .unwrap_or(period_date);
            let dt = target_date
                .and_hms_opt(0, 0, 0)
                .unwrap()
                .and_local_timezone(*ref_time.offset())
                .single()
                .unwrap();
            return Some((dt, "day"));
        }
    }
    // DOW + GrainOffset: same as above but reversed
    if let TimeForm::DayOfWeek(dow) = &primary.form {
        if let TimeForm::GrainOffset {
            grain: _,
            offset: _,
        } = &secondary.form
        {
            let (period_start, _) =
                resolve_simple_datetime(&secondary.form, ref_time, secondary.direction)?;
            let period_date = period_start.date_naive();
            let current_dow = period_date.weekday().num_days_from_monday();
            let days_to_target = if *dow >= current_dow {
                dow.saturating_sub(current_dow) as i64
            } else {
                dow.saturating_add(7).saturating_sub(current_dow) as i64
            };
            let target_date = chrono::Duration::try_days(days_to_target)
                .and_then(|d| period_date.checked_add_signed(d))
                .unwrap_or(period_date);
            let dt = target_date
                .and_hms_opt(0, 0, 0)
                .unwrap()
                .and_local_timezone(*ref_time.offset())
                .single()
                .unwrap();
            return Some((dt, "day"));
        }
    }

    // Month + DayOfWeek or DayOfWeek + Month → resolve DOW within that month
    // (e.g., "Jul 18" where 18 is DOW? no, this shouldn't happen)

    // Default: resolve secondary if it's more specific than primary
    fn form_specificity(f: &TimeForm) -> u32 {
        match f {
            TimeForm::DateMDY { year: Some(_), .. } => 6,
            TimeForm::DateMDY { .. } | TimeForm::Composed(..) => 5,
            TimeForm::DayOfMonth(_) => 4,
            TimeForm::Month(_) => 3,
            TimeForm::DayOfWeek(_) => 2,
            TimeForm::Year(_) => 1,
            _ => 0,
        }
    }
    let s_primary = form_specificity(&primary.form);
    let s_secondary = form_specificity(&secondary.form);
    if s_secondary > s_primary && s_secondary > 0 {
        return resolve_simple_datetime(&secondary.form, ref_time, secondary.direction);
    }

    // Default: resolve primary
    resolve_simple_datetime(&primary.form, ref_time, primary.direction)
}

// ============================================================
// NthGrain interval resolution (last/next N <grain>)
// ============================================================

fn resolve_nth_grain_interval(
    n: i64,
    grain: Grain,
    past: bool,
    ref_time: DateTime<FixedOffset>,
) -> Option<(DateTime<FixedOffset>, DateTime<FixedOffset>)> {
    let base = grain_start(ref_time, grain);

    if past {
        let from = add_grain(base, grain, n.checked_neg()?)?;
        let to = base;
        Some((from, to))
    } else {
        let from = add_grain(base, grain, 1)?;
        let to = add_grain(base, grain, 1_i64.checked_add(n)?)?;
        Some((from, to))
    }
}

// ============================================================
// Weekend interval
// ============================================================

pub(super) fn resolve_weekend_interval(
    ref_time: DateTime<FixedOffset>,
    direction: Option<Direction>,
) -> Option<(DateTime<FixedOffset>, DateTime<FixedOffset>)> {
    let dow = ref_time.weekday().num_days_from_monday(); // Mon=0..Sun=6
    let hour = ref_time.hour();

    // Haskell: weekend = interval' Open (intersect (dayOfWeek 5, hour 18), intersect (dayOfWeek 1, hour 0))
    // Inside weekend: [Fri 18:00, Mon 00:00)
    let inside_weekend = (dow == 4 && hour >= 18) || dow == 5 || dow == 6;

    // Find the Friday of the most-recent-or-current weekend
    let most_recent_friday = {
        let days_back = if dow == 4 && hour >= 18 {
            0 // Fri evening: today
        } else if dow == 5 {
            1 // Sat: yesterday
        } else if dow == 6 {
            2 // Sun: 2 days ago
        } else if dow == 4 {
            7 // Fri before 18: last week's Fri
        } else {
            i64::from(dow).checked_add(3)? // Mon=3, Tue=4, Wed=5, Thu=6
        };
        Duration::try_days(days_back).and_then(|d| midnight(ref_time).checked_sub_signed(d))?
    };

    // Choose which Friday based on direction (matching Haskell's predNth semantics)
    let friday = match direction {
        None => {
            // "this weekend" — predNth 0 False: current if inside, upcoming if outside
            if inside_weekend {
                most_recent_friday
            } else {
                let days_fwd = 4_i64.checked_sub(i64::from(dow))?; // days to next Friday
                Duration::try_days(days_fwd)
                    .and_then(|d| midnight(ref_time).checked_add_signed(d))?
            }
        }
        Some(Direction::Future) => {
            // "next weekend" — predNth 0 True: skip current if inside, upcoming if outside
            if inside_weekend {
                Duration::try_days(7).and_then(|d| most_recent_friday.checked_add_signed(d))?
            } else {
                let days_fwd = 4_i64.checked_sub(i64::from(dow))?;
                Duration::try_days(days_fwd)
                    .and_then(|d| midnight(ref_time).checked_add_signed(d))?
            }
        }
        Some(Direction::Past) => {
            // "last weekend" — predNth (-1) False: previous if inside, most-recent-past if outside
            if inside_weekend {
                Duration::try_days(7).and_then(|d| most_recent_friday.checked_sub_signed(d))?
            } else {
                most_recent_friday
            }
        }
        Some(Direction::FarFuture) => {
            // "weekend after next" — skip one extra
            if inside_weekend {
                Duration::try_days(14).and_then(|d| most_recent_friday.checked_add_signed(d))?
            } else {
                let days_fwd = 4_i64.checked_sub(i64::from(dow))?;
                let upcoming = Duration::try_days(days_fwd)
                    .and_then(|d| midnight(ref_time).checked_add_signed(d))?;
                Duration::try_days(7).and_then(|d| upcoming.checked_add_signed(d))?
            }
        }
    };

    let from = Duration::try_hours(18).and_then(|d| friday.checked_add_signed(d))?;
    let to = Duration::try_hours(54).and_then(|d| from.checked_add_signed(d))?;
    Some((from, to))
}

// ============================================================
// AllGrain interval
// ============================================================

fn resolve_all_grain(
    grain: Grain,
    ref_time: DateTime<FixedOffset>,
) -> Option<(DateTime<FixedOffset>, DateTime<FixedOffset>)> {
    let from = grain_start(ref_time, grain);
    // "all week" end = start of next period minus 1 day for week
    // Actually: all week = [Mon, Sun) = 6 days
    let to = match grain {
        Grain::Week => Duration::try_days(6)
            .and_then(|d| from.checked_add_signed(d))
            .unwrap_or(from), // Mon to Sun (not including next Mon)
        _ => add_grain(from, grain, 1)?,
    };
    Some((from, to))
}

fn grain_for_all_rest(grain: Grain) -> &'static str {
    match grain {
        Grain::Week | Grain::Month | Grain::Year => grain.lower().as_str(),
        _ => grain.as_str(),
    }
}

// ============================================================
// BeginEnd interval
// ============================================================

fn resolve_begin_end(
    begin: bool,
    target: &TimeForm,
    ref_time: DateTime<FixedOffset>,
    direction: Option<Direction>,
) -> Option<(DateTime<FixedOffset>, DateTime<FixedOffset>)> {
    // Resolve the target period start
    let (period_start, grain) = match target {
        TimeForm::GrainOffset { grain, offset } => {
            let (dt, _) = resolve_grain_offset(*grain, *offset, ref_time)?;
            (dt, *grain)
        }
        TimeForm::Month(m) => {
            let dt = resolve_month(*m, ref_time, direction);
            (
                NaiveDate::from_ymd_opt(dt.year(), dt.month(), 1)
                    .unwrap()
                    .and_hms_opt(0, 0, 0)
                    .unwrap()
                    .and_local_timezone(*ref_time.offset())
                    .single()
                    .unwrap(),
                Grain::Month,
            )
        }
        TimeForm::Year(y) => {
            let dt = NaiveDate::from_ymd_opt(*y, 1, 1)
                .unwrap()
                .and_hms_opt(0, 0, 0)
                .unwrap()
                .and_local_timezone(*ref_time.offset())
                .single()
                .unwrap();
            (dt, Grain::Year)
        }
        _ => {
            let (dt, _) = resolve_simple_datetime(target, ref_time, direction)?;
            let g = target_grain(target);
            (grain_start(dt, g), g)
        }
    };

    // Use hardcoded boundaries matching Haskell Duckling
    Some(match grain {
        Grain::Week => {
            if begin {
                // Beginning of week: Mon-Wed (dayOfWeek 1-3)
                (
                    period_start,
                    Duration::try_days(3)
                        .and_then(|d| period_start.checked_add_signed(d))
                        .unwrap_or(period_start),
                )
            } else {
                // End of week: Fri-Sun (dayOfWeek 5-7)
                (
                    Duration::try_days(4)
                        .and_then(|d| period_start.checked_add_signed(d))
                        .unwrap_or(period_start),
                    Duration::try_days(7)
                        .and_then(|d| period_start.checked_add_signed(d))
                        .unwrap_or(period_start),
                )
            }
        }
        Grain::Month => {
            let y = period_start.year();
            let m = period_start.month();
            if begin {
                // Beginning of month: day 1 to day 11 (exclusive)
                let end = make_date(y, m, 11, *ref_time.offset());
                (period_start, end)
            } else {
                // End of month: day 21 to end of month
                let start = make_date(y, m, 21, *ref_time.offset());
                let end = add_grain(period_start, Grain::Month, 1)?;
                (start, end)
            }
        }
        Grain::Year => {
            let y = period_start.year();
            if begin {
                // Beginning of year: month 1 to month 4 (Jan-Mar)
                let end = make_date(y, 4, 1, *ref_time.offset());
                (period_start, end)
            } else {
                // End of year: month 9 to end of year (Sep-Dec)
                let start = make_date(y, 9, 1, *ref_time.offset());
                let end = make_date(y.saturating_add(1), 1, 1, *ref_time.offset());
                (start, end)
            }
        }
        Grain::Day => {
            if begin {
                // Beginning of day: 00:00 to 08:00
                let end = Duration::try_hours(8)
                    .and_then(|d| period_start.checked_add_signed(d))
                    .unwrap_or(period_start);
                (period_start, end)
            } else {
                // End of day: 17:00 to 00:00
                let start = Duration::try_hours(17)
                    .and_then(|d| period_start.checked_add_signed(d))
                    .unwrap_or(period_start);
                let end = Duration::try_days(1)
                    .and_then(|d| period_start.checked_add_signed(d))
                    .unwrap_or(period_start);
                (start, end)
            }
        }
        _ => {
            // Fallback: divide into thirds
            let period_end = add_grain(period_start, grain, 1)?;
            let total_secs = period_end.signed_duration_since(period_start).num_seconds();
            let portion = total_secs.checked_div(3).unwrap_or(0);
            if begin {
                let to = Duration::try_seconds(portion)
                    .and_then(|d| period_start.checked_add_signed(d))
                    .unwrap_or(period_start);
                let to = grain_start(to, grain.lower());
                (period_start, to)
            } else {
                let from = Duration::try_seconds(portion)
                    .and_then(|d| period_end.checked_sub_signed(d))
                    .unwrap_or(period_end);
                let from = grain_start(from, grain.lower());
                (from, period_end)
            }
        }
    })
}

fn begin_end_grain(target: &TimeForm) -> &'static str {
    match target {
        TimeForm::GrainOffset { grain, .. } => grain.lower().as_str(),
        _ => {
            let g = target_grain(target);
            g.lower().as_str()
        }
    }
}

fn target_grain(form: &TimeForm) -> Grain {
    match form {
        TimeForm::GrainOffset { grain, .. } => *grain,
        TimeForm::Month(_) => Grain::Month,
        TimeForm::Year(_) => Grain::Year,
        TimeForm::DayOfWeek(_) => Grain::Day,
        TimeForm::Quarter(_) | TimeForm::QuarterYear(_, _) => Grain::Quarter,
        TimeForm::Composed(a, b) => {
            // For composed forms, use the coarser grain (the containing period)
            // Month+Year → Month, DOW+Month → Month, etc.
            let ga = target_grain(&a.form);
            let gb = target_grain(&b.form);
            // The coarser grain is the one with higher enum value (Year > Month > Day etc)
            // But we want the containing period, which is the finer of the two
            // Actually: Composed(Month(10), Year(2015)) → "October 2015" → grain=Month
            // The grain should be the finer of the two base grains
            if ga < gb {
                ga
            } else {
                gb
            }
        }
        _ => Grain::Day,
    }
}

// ============================================================
// Season interval
// ============================================================

pub(super) fn resolve_season_interval(
    season: u32,
    ref_time: DateTime<FixedOffset>,
    direction: Option<Direction>,
) -> (DateTime<FixedOffset>, DateTime<FixedOffset>) {
    let year = ref_time.year();
    let offset = *ref_time.offset();

    // Season dates (Northern Hemisphere meteorological approximation)
    let (from, to) = match season {
        0 => {
            // Spring: ~Mar 20 to ~Jun 21
            (
                make_date(year, 3, 20, offset),
                make_date(year, 6, 21, offset),
            )
        }
        1 => {
            // Summer: ~Jun 21 to ~Sep 24
            (
                make_date(year, 6, 21, offset),
                make_date(year, 9, 24, offset),
            )
        }
        2 => {
            // Fall: ~Sep 23 to ~Dec 21
            (
                make_date(year, 9, 23, offset),
                make_date(year, 12, 21, offset),
            )
        }
        3 => {
            // Winter: ~Dec 21 prev to ~Mar 21
            (
                make_date(year.saturating_sub(1), 12, 21, offset),
                make_date(year, 3, 21, offset),
            )
        }
        _ => (make_date(year, 1, 1, offset), make_date(year, 4, 1, offset)),
    };

    match direction {
        Some(Direction::Past) => {
            // Previous season
            let (_from, _to) = match season {
                0 => (
                    make_date(year.saturating_sub(1), 3, 20, offset),
                    make_date(year.saturating_sub(1), 6, 21, offset),
                ),
                1 => (
                    make_date(year.saturating_sub(1), 6, 21, offset),
                    make_date(year.saturating_sub(1), 9, 24, offset),
                ),
                2 => (
                    make_date(year.saturating_sub(1), 9, 23, offset),
                    make_date(year.saturating_sub(1), 12, 21, offset),
                ),
                3 => (
                    make_date(year.saturating_sub(2), 12, 21, offset),
                    make_date(year.saturating_sub(1), 3, 21, offset),
                ),
                _ => (from, to),
            };
            // But if we're looking for last season (not this specific one), use
            // the season before the current one
            let current_season = current_season_number(ref_time);
            let prev_season = if current_season == 0 {
                3
            } else {
                current_season.saturating_sub(1)
            };
            season_dates(prev_season, year, direction, offset)
        }
        Some(Direction::Future) | Some(Direction::FarFuture) => {
            let current_season = current_season_number(ref_time);
            let next_season = current_season.saturating_add(1) % 4;
            season_dates(next_season, year, direction, offset)
        }
        None => {
            if season == 99 {
                // Generic "season" → use current season with slightly different dates
                let current = current_season_number(ref_time);
                generic_season_dates(current, year, offset)
            } else {
                // Specific season like "this Summer" → use that season
                // If the season hasn't started yet this year, show this year's
                // If it's already past, show this year's (it's "this" not "next")
                season_dates(season, year, None, offset)
            }
        }
    }
}

fn current_season_number(ref_time: DateTime<FixedOffset>) -> u32 {
    let month = ref_time.month();
    let day = ref_time.day();
    match month {
        12 if day >= 21 => 3,
        1 | 2 => 3,
        3 if day < 20 => 3,
        3..=5 => 0,
        6 if day < 21 => 0,
        6..=8 => 1,
        9 if day < 23 => 1,
        9..=11 => 2,
        12 => 2,
        _ => 0,
    }
}

fn season_dates(
    season: u32,
    year: i32,
    direction: Option<Direction>,
    offset: FixedOffset,
) -> (DateTime<FixedOffset>, DateTime<FixedOffset>) {
    match season {
        0 => (
            make_date(year, 3, 20, offset),
            make_date(year, 6, 20, offset),
        ),
        1 => (
            make_date(year, 6, 21, offset),
            make_date(year, 9, 24, offset),
        ),
        2 => match direction {
            Some(Direction::Past) => (
                make_date(year.saturating_sub(1), 9, 23, offset),
                make_date(year.saturating_sub(1), 12, 20, offset),
            ),
            _ => (
                make_date(year, 9, 23, offset),
                make_date(year, 12, 20, offset),
            ),
        },
        3 => (
            make_date(year.saturating_sub(1), 12, 21, offset),
            make_date(year, 3, 21, offset),
        ),
        _ => (make_date(year, 1, 1, offset), make_date(year, 4, 1, offset)),
    }
}

/// Generic "this season" / "current season" uses slightly different boundary dates
fn generic_season_dates(
    season: u32,
    year: i32,
    offset: FixedOffset,
) -> (DateTime<FixedOffset>, DateTime<FixedOffset>) {
    match season {
        0 => (
            make_date(year, 3, 20, offset),
            make_date(year, 6, 20, offset),
        ),
        1 => (
            make_date(year, 6, 21, offset),
            make_date(year, 9, 24, offset),
        ),
        2 => (
            make_date(year, 9, 23, offset),
            make_date(year, 12, 20, offset),
        ),
        3 => (
            make_date(year.saturating_sub(1), 12, 21, offset),
            make_date(year, 3, 19, offset),
        ),
        _ => (make_date(year, 1, 1, offset), make_date(year, 4, 1, offset)),
    }
}

// ============================================================
// Part of day interval
// ============================================================

pub(super) fn pod_interval(
    pod: PartOfDay,
    date: NaiveDate,
    early_late: Option<EarlyLate>,
    offset: FixedOffset,
) -> (DateTime<FixedOffset>, DateTime<FixedOffset>) {
    let (start_h, end_h) = match (pod, early_late) {
        (PartOfDay::Morning, Some(EarlyLate::Early)) => (0, 9),
        (PartOfDay::Morning, _) => (0, 12),
        (PartOfDay::Afternoon, _) => (12, 19),
        (PartOfDay::Evening, Some(EarlyLate::Late)) => (21, 24),
        (PartOfDay::Evening, _) => (18, 24),
        (PartOfDay::Night, Some(EarlyLate::Late)) => (21, 24),
        (PartOfDay::Night, _) => (18, 24),
        (PartOfDay::Lunch, _) => (12, 14),
    };

    let from = fixed_at(offset, date, start_h, 0, 0);
    let to = if end_h >= 24 {
        let next_day = date
            .checked_add_signed(Duration::try_days(1).unwrap_or_default())
            .unwrap_or(date);
        fixed_at(offset, next_day, 0, 0, 0)
    } else {
        fixed_at(offset, date, end_h, 0, 0)
    };
    (from, to)
}

// ============================================================
// Helper functions
// ============================================================

fn fixed(naive: NaiveDateTime, offset: FixedOffset) -> DateTime<FixedOffset> {
    offset.from_local_datetime(&naive).single().unwrap()
}

fn fixed_at(
    offset: FixedOffset,
    date: NaiveDate,
    hour: u32,
    minute: u32,
    second: u32,
) -> DateTime<FixedOffset> {
    fixed(date.and_hms_opt(hour, minute, second).unwrap(), offset)
}

pub(super) fn midnight(dt: DateTime<FixedOffset>) -> DateTime<FixedOffset> {
    fixed_at(*dt.offset(), dt.date_naive(), 0, 0, 0)
}

fn make_date(y: i32, m: u32, d: u32, offset: FixedOffset) -> DateTime<FixedOffset> {
    let date =
        NaiveDate::from_ymd_opt(y, m, d).unwrap_or(NaiveDate::from_ymd_opt(y, 1, 1).unwrap());
    fixed_at(offset, date, 0, 0, 0)
}

fn start_of_week(dt: DateTime<FixedOffset>) -> DateTime<FixedOffset> {
    let dow = dt.weekday().num_days_from_monday();
    midnight(
        Duration::try_days(dow as i64)
            .and_then(|d| dt.checked_sub_signed(d))
            .unwrap_or(dt),
    )
}

fn start_of_month(dt: DateTime<FixedOffset>) -> DateTime<FixedOffset> {
    fixed_at(
        *dt.offset(),
        NaiveDate::from_ymd_opt(dt.year(), dt.month(), 1).unwrap(),
        0,
        0,
        0,
    )
}

fn start_of_year(dt: DateTime<FixedOffset>) -> DateTime<FixedOffset> {
    fixed_at(
        *dt.offset(),
        NaiveDate::from_ymd_opt(dt.year(), 1, 1).unwrap(),
        0,
        0,
        0,
    )
}

fn start_of_quarter(dt: DateTime<FixedOffset>) -> DateTime<FixedOffset> {
    let q = dt.month().saturating_sub(1) / 3;
    let month = q.saturating_mul(3).saturating_add(1);
    fixed_at(
        *dt.offset(),
        NaiveDate::from_ymd_opt(dt.year(), month, 1).unwrap(),
        0,
        0,
        0,
    )
}

pub(super) fn grain_start(dt: DateTime<FixedOffset>, grain: Grain) -> DateTime<FixedOffset> {
    match grain {
        Grain::NoGrain | Grain::Second => dt,
        Grain::Minute => dt
            .date_naive()
            .and_hms_opt(dt.hour(), dt.minute(), 0)
            .map(|naive| fixed(naive, *dt.offset()))
            .unwrap(),
        Grain::Hour => dt
            .date_naive()
            .and_hms_opt(dt.hour(), 0, 0)
            .map(|naive| fixed(naive, *dt.offset()))
            .unwrap(),
        Grain::Day => midnight(dt),
        Grain::Week => start_of_week(dt),
        Grain::Month => start_of_month(dt),
        Grain::Quarter => start_of_quarter(dt),
        Grain::Year => start_of_year(dt),
    }
}

pub(super) fn add_grain(
    dt: DateTime<FixedOffset>,
    grain: Grain,
    n: i64,
) -> Option<DateTime<FixedOffset>> {
    match grain {
        Grain::NoGrain | Grain::Second => {
            Duration::try_seconds(n).and_then(|d| dt.checked_add_signed(d))
        }
        Grain::Minute => Duration::try_minutes(n).and_then(|d| dt.checked_add_signed(d)),
        Grain::Hour => Duration::try_hours(n).and_then(|d| dt.checked_add_signed(d)),
        Grain::Day => Duration::try_days(n).and_then(|d| dt.checked_add_signed(d)),
        Grain::Week => Duration::try_weeks(n).and_then(|d| dt.checked_add_signed(d)),
        Grain::Month => add_months(dt, n),
        Grain::Quarter => add_months(dt, n.checked_mul(3)?),
        Grain::Year => add_years(dt, n),
    }
}

pub(super) fn add_months(dt: DateTime<FixedOffset>, months: i64) -> Option<DateTime<FixedOffset>> {
    let total = i64::from(dt.year())
        .checked_mul(12)?
        .checked_add(i64::from(dt.month()).checked_sub(1)?)?
        .checked_add(months)?;
    let year = i32::try_from(total.div_euclid(12)).ok()?;
    let month = (total.rem_euclid(12).checked_add(1)?) as u32;
    let day = dt.day().min(days_in_month(year, month));
    let date = NaiveDate::from_ymd_opt(year, month, day)?;
    let naive = date.and_hms_opt(dt.hour(), dt.minute(), dt.second())?;
    Some(fixed(naive, *dt.offset()))
}

pub(super) fn add_years(dt: DateTime<FixedOffset>, years: i64) -> Option<DateTime<FixedOffset>> {
    let year_i64 = i64::from(dt.year()).checked_add(years)?;
    let year = i32::try_from(year_i64).ok()?;
    let day = dt.day().min(days_in_month(year, dt.month()));
    let date = NaiveDate::from_ymd_opt(year, dt.month(), day)?;
    let naive = date.and_hms_opt(dt.hour(), dt.minute(), dt.second())?;
    Some(fixed(naive, *dt.offset()))
}

#[allow(clippy::arithmetic_side_effects)]
fn days_in_month(year: i32, month: u32) -> u32 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 => {
            if is_leap_year(year) {
                29
            } else {
                28
            }
        }
        _ => 30,
    }
}

fn is_leap_year(year: i32) -> bool {
    (year % 4 == 0 && year % 100 != 0) || year % 400 == 0
}

// ============================================================
// Holiday resolution
// ============================================================

#[allow(clippy::arithmetic_side_effects)]
fn resolve_holiday_with_direction(
    name: &str,
    ref_time: DateTime<FixedOffset>,
    direction: Option<Direction>,
) -> (DateTime<FixedOffset>, &'static str) {
    let ref_midnight = midnight(ref_time);
    let this_year = ref_time.year();

    // Resolve for this year
    let this_year_date = resolve_holiday(name, this_year).map(|d| {
        d.and_hms_opt(0, 0, 0)
            .unwrap()
            .and_local_timezone(*ref_time.offset())
            .single()
            .unwrap()
    });
    let next_year_date = resolve_holiday(name, this_year + 1).map(|d| {
        d.and_hms_opt(0, 0, 0)
            .unwrap()
            .and_local_timezone(*ref_time.offset())
            .single()
            .unwrap()
    });
    let prev_year_date = resolve_holiday(name, this_year - 1).map(|d| {
        d.and_hms_opt(0, 0, 0)
            .unwrap()
            .and_local_timezone(*ref_time.offset())
            .single()
            .unwrap()
    });

    let result = match direction {
        Some(Direction::Past) => {
            // Pick the most recent occurrence that is strictly before ref_time's date
            if let Some(dt) = this_year_date {
                if dt < ref_midnight {
                    dt
                } else if let Some(prev) = prev_year_date {
                    prev
                } else {
                    dt
                }
            } else if let Some(prev) = prev_year_date {
                prev
            } else {
                ref_midnight
            }
        }
        _ => {
            // Default: future-first, but if holiday is today, use today
            if let Some(dt) = this_year_date {
                if dt >= ref_midnight {
                    dt
                } else if let Some(next) = next_year_date {
                    next
                } else {
                    dt
                }
            } else if let Some(next) = next_year_date {
                next
            } else {
                ref_midnight
            }
        }
    };

    (result, "day")
}

/// Resolve a holiday that is an interval of days. Returns (start_date, end_date_exclusive).
#[allow(clippy::arithmetic_side_effects)]
pub(super) fn resolve_holiday_interval(name: &str, year: i32) -> Option<(NaiveDate, NaiveDate)> {
    let name_lower = name.to_lowercase();
    let name = name_lower.as_str();

    // Rosh Hashanah: 3-day interval
    if name.starts_with("rosh hashann") || name.starts_with("rosh hashan") || name == "yom teruah" {
        if let Some(start) = rosh_hashanah(year) {
            return Some((start, start + Duration::days(3)));
        }
    }

    // Chanukah: 8-day interval
    if name.starts_with("chanukah") || name.starts_with("hanuk") || name.starts_with("hannuk") {
        if let Some(start) = chanukah(year) {
            return Some((start, start + Duration::days(8)));
        }
    }

    // Passover: 9-day interval (start to start+9)
    if name == "passover"
        || name.starts_with("pesac")
        || name.starts_with("pesak")
        || name.starts_with("pesah")
    {
        if let Some(start) = passover(year) {
            return Some((start, start + Duration::days(9)));
        }
    }

    // Shavuot: 3-day interval (passover+50 to passover+53)
    if name == "shavuot"
        || name.starts_with("shavu")
        || name.starts_with("shovuo")
        || name.starts_with("feast of weeks")
    {
        if let Some(p) = passover(year) {
            let start = p + Duration::days(50);
            return Some((start, start + Duration::days(3)));
        }
    }

    // Sukkot: 9-day interval (rosh+14 to rosh+23)
    if name.starts_with("sukkot")
        || name.starts_with("succos")
        || name.starts_with("succot")
        || name.starts_with("sukko")
        || name.starts_with("feast of the ingathering")
        || name.starts_with("feast of booth")
        || name.starts_with("feast of tabernacle")
    {
        if let Some(rosh) = rosh_hashanah(year) {
            let start = rosh + Duration::days(14);
            return Some((start, start + Duration::days(9)));
        }
    }

    // Navaratri: 10-day interval (start to start+10)
    if name == "navaratri"
        || name == "durga puja"
        || name == "durgotsava"
        || name.starts_with("navarat")
    {
        if let Some(start) = navaratri(year) {
            return Some((start, start + Duration::days(10)));
        }
    }

    // Ramadan: from ramadan start to eid al-fitr (exclusive)
    if name == "ramadan"
        || name.starts_with("ramadh")
        || name.starts_with("ramadt")
        || name.starts_with("ramzan")
        || name.starts_with("ramzaan")
    {
        if let Some(start) = ramadan(year) {
            if let Some(end) = eid_al_fitr(year) {
                return Some((start, end));
            }
            // Fallback: ~30 days
            return Some((start, start + Duration::days(30)));
        }
    }

    // Lent: Ash Wednesday to Easter (exclusive)
    if name == "lent" {
        let easter = easter_date(year);
        let start = easter - Duration::days(46); // Ash Wednesday
        return Some((start, easter));
    }

    // Great Fast / Great Lent: Clean Monday to Orthodox Easter - 8 (exclusive)
    if name == "great fast" || name == "great lent" {
        let orthodox = orthodox_easter_date(year);
        let start = orthodox - Duration::days(48); // Clean Monday
        let end = orthodox - Duration::days(8); // Day after Lazarus Saturday (exclusive)
        return Some((start, end));
    }

    // GYSD: 3-day interval
    if name == "gysd"
        || name.starts_with("global youth service")
        || name.starts_with("national youth service")
    {
        if let Some(start) = gysd_date(year) {
            return Some((start, start + Duration::days(3)));
        }
    }

    // EMS week: 3rd Sunday of May to following Sunday
    if name.starts_with("ems week") {
        let start = nth_dow_of_month(year, 5, 6, 3); // 3rd Sunday in May
        return Some((start, start + Duration::days(7)));
    }

    // Royal Hobart Regatta: 3 days ending on 2nd Monday in February
    if name == "royal hobart regatta" {
        let end = nth_dow_of_month(year, 2, 0, 2);
        let start = end - Duration::days(2);
        return Some((start, end + Duration::days(1)));
    }

    // National Arbor Week (ZA): September 1..7 inclusive
    if name == "national arbor week" {
        let start = NaiveDate::from_ymd_opt(year, 9, 1)?;
        return Some((start, start + Duration::days(7)));
    }

    // NAIDOC Week (AU): Sunday before 2nd Friday in July through following Sunday
    if name == "naidoc week" {
        let second_friday = nth_dow_of_month(year, 7, 4, 2); // 4=Friday
        let start = second_friday - Duration::days(5);
        let end_exclusive = second_friday + Duration::days(2);
        return Some((start, end_exclusive));
    }

    // Royal Queensland Show / Ekka (AU): 10-day interval from computed start Friday in August
    if name == "ekka"
        || name == "royal queensland show"
        || name == "royal national agricultural show"
        || name == "rna show"
    {
        if let Some(start) = royal_queensland_show_start(year) {
            return Some((start, start + Duration::days(10)));
        }
    }

    // Labour Day weekend (CA): from previous Friday 18:00 to Tuesday 00:00 after Labour Day Monday
    if name.starts_with("labor day weekend") || name.starts_with("labour day weekend") {
        let labor_day = nth_dow_of_month(year, 9, 0, 1);
        let start = labor_day - Duration::days(3);
        let end_exclusive = labor_day + Duration::days(1);
        return Some((start, end_exclusive));
    }

    None
}

/// Resolve a holiday that is a minute-level interval (e.g., Earth Hour).
#[allow(clippy::arithmetic_side_effects)]
pub(super) fn resolve_holiday_minute_interval(
    name: &str,
    year: i32,
    offset: FixedOffset,
) -> Option<(DateTime<FixedOffset>, DateTime<FixedOffset>)> {
    let name_lower = name.to_lowercase();
    let name = name_lower.as_str();

    if name == "earth hour" {
        // Last Saturday of March, unless it's Holy Saturday, then one week before
        let last_sat = last_dow_of_month(year, 3, 5); // 5=Saturday
        let easter = easter_date(year);
        let holy_saturday = easter - Duration::days(1);
        let date = if last_sat == holy_saturday {
            last_sat - Duration::days(7)
        } else {
            last_sat
        };
        let from = fixed_at(offset, date, 20, 30, 0);
        let to = fixed_at(offset, date, 21, 31, 0); // 60 minutes later, closed interval
        return Some((from, to));
    }

    None
}

#[allow(clippy::arithmetic_side_effects)]
pub(super) fn resolve_holiday(name: &str, year: i32) -> Option<NaiveDate> {
    let name_lower = name.to_lowercase();
    let name = name_lower.as_str();

    // Chinese New Year (must be checked before "new year's day")
    if name.contains("chinese") && name.contains("new year") {
        return chinese_new_year(year);
    }

    // Fixed-date holidays
    match name {
        s if s.starts_with("christmas") || s.starts_with("xmas") => {
            return NaiveDate::from_ymd_opt(year, 12, 25)
        }
        s if s.contains("new year") && s.contains("eve") => {
            return NaiveDate::from_ymd_opt(year, 12, 31)
        }
        s if s.contains("new year")
            && s.contains("day")
            && !s.contains("chinese")
            && !s.contains("lunar") =>
        {
            return NaiveDate::from_ymd_opt(year, 1, 1)
        }
        s if s.contains("valentine") => return NaiveDate::from_ymd_opt(year, 2, 14),
        "halloween" => return NaiveDate::from_ymd_opt(year, 10, 31),
        "independence day" => return NaiveDate::from_ymd_opt(year, 7, 4),
        "canada day" => return NaiveDate::from_ymd_opt(year, 7, 1),
        "dominion day" => return NaiveDate::from_ymd_opt(year, 7, 1),
        "anzac day" => return NaiveDate::from_ymd_opt(year, 4, 25),
        "garifuna settlement day" => return NaiveDate::from_ymd_opt(year, 11, 19),
        "indian arrival day" => return NaiveDate::from_ymd_opt(year, 5, 30),
        "rizal day" => return NaiveDate::from_ymd_opt(year, 12, 30),
        "shivaji jayanti" => return NaiveDate::from_ymd_opt(year, 2, 19),
        "hazarat ali's birthday" => return rajab(year).map(|d| d + Duration::days(12)),
        "reconciliation day" => {
            let base = NaiveDate::from_ymd_opt(year, 5, 26)?;
            let delta = (7_i64 - i64::from(base.weekday().num_days_from_monday())) % 7;
            return Some(base + Duration::days(delta));
        }
        s if s.starts_with("day of") && (s.contains("vow") || s.contains("reconciliation")) => {
            return NaiveDate::from_ymd_opt(year, 12, 16);
        }
        "heritage day" => return NaiveDate::from_ymd_opt(year, 9, 24),
        "vimy ridge day" => return NaiveDate::from_ymd_opt(year, 4, 9),
        "orangemen's day" | "the twelfth" | "the glorious twelfth" => {
            return NaiveDate::from_ymd_opt(year, 7, 12);
        }
        "victoria day" | "sovereign's birthday" => {
            let base = NaiveDate::from_ymd_opt(year, 5, 25)?;
            let back = i64::from(base.weekday().num_days_from_monday());
            return Some(base - Duration::days(back));
        }
        "discovery day" => {
            let base = NaiveDate::from_ymd_opt(year, 6, 24)?;
            return Some(closest_weekday(base));
        }
        "civic day"
        | "civic holiday"
        | "british columbia day"
        | "natal day"
        | "new brunswick day"
        | "saskatchewan day"
        | "terry fox day" => return Some(nth_dow_of_month(year, 8, 0, 1)),
        "family day" | "islander day" | "louis riel day" | "nova scotia heritage day" => {
            return Some(nth_dow_of_month(year, 2, 0, 3));
        }
        "national patriots day" | "national patriot's day" => {
            let base = NaiveDate::from_ymd_opt(year, 5, 25)?;
            let back = i64::from(base.weekday().num_days_from_monday());
            return Some(base - Duration::days(back));
        }
        "royal queensland show day" | "ekka day" | "rna show day" => {
            if let Some(start) = royal_queensland_show_start(year) {
                return Some(start + Duration::days(5));
            }
        }
        s if s.starts_with("veteran") => return NaiveDate::from_ymd_opt(year, 11, 11),
        "law day" | "lei day" | "loyalty day" => return NaiveDate::from_ymd_opt(year, 5, 1),
        s if s.contains("lincoln") => return NaiveDate::from_ymd_opt(year, 2, 12),
        "guy fawkes day" => return NaiveDate::from_ymd_opt(year, 11, 5),
        "groundhog day" | "groundhogs day" => return NaiveDate::from_ymd_opt(year, 2, 2),
        "siblings day" | "national sibling day" => return NaiveDate::from_ymd_opt(year, 4, 10),
        "world vegan day" => return NaiveDate::from_ymd_opt(year, 11, 1),
        s if s.contains("patrick") || s.contains("paddy") => {
            return NaiveDate::from_ymd_opt(year, 3, 17)
        }
        "koningsdag" | "king's day" => {
            // April 27, unless it's a Sunday, then April 26
            let d = NaiveDate::from_ymd_opt(year, 4, 27).unwrap();
            if d.weekday().num_days_from_monday() == 6 {
                // Sunday -> use April 26
                return NaiveDate::from_ymd_opt(year, 4, 26);
            }
            return NaiveDate::from_ymd_opt(year, 4, 27);
        }
        _ => {}
    }

    // Thanksgiving (4th Thursday of November)
    if name.starts_with("thanksgiving") {
        return Some(nth_dow_of_month(year, 11, 3, 4)); // 3=Thursday, 4th occurrence
    }

    // Memorial Day / Decoration Day (last Monday of May)
    if name.starts_with("memorial") || name.starts_with("decoration") {
        return Some(last_dow_of_month(year, 5, 0)); // 0=Monday
    }

    // Labor/Labour Day (default: 1st Monday of September)
    if name.starts_with("labor day") || name.starts_with("labour day") {
        return Some(nth_dow_of_month(year, 9, 0, 1)); // 0=Monday, 1st occurrence
    }

    // UK August Bank Holiday (last Monday of August)
    if name.contains("bank holiday") {
        return Some(last_dow_of_month(year, 8, 0)); // 0=Monday
    }

    // Black Friday (day after Thanksgiving)
    if name == "black friday" {
        let thanksgiving = nth_dow_of_month(year, 11, 3, 4);
        return Some(thanksgiving + Duration::days(1));
    }

    // Boss's Day (closest weekday to Oct 16)
    if name.starts_with("boss") {
        return Some(closest_weekday(
            NaiveDate::from_ymd_opt(year, 10, 16).unwrap(),
        ));
    }

    // MLK Day / Civil Rights Day / Idaho Human Rights Day (3rd Monday of January)
    if name.contains("mlk")
        || name.contains("martin luther king")
        || name.contains("civil rights")
        || name.contains("idaho human")
    {
        return Some(nth_dow_of_month(year, 1, 0, 3)); // 0=Monday, 3rd occurrence
    }

    // Presidents' Day / Washington's Birthday aliases (3rd Monday of February)
    if name.contains("washington")
        || name.contains("president")
        || name.contains("daisy gatson bates")
    {
        return Some(nth_dow_of_month(year, 2, 0, 3)); // 0=Monday, 3rd occurrence
    }

    // Mother's Day (2nd Sunday of May)
    if name.contains("mother") && name.contains("day") {
        return Some(nth_dow_of_month(year, 5, 6, 2)); // 6=Sunday, 2nd occurrence
    }

    // UK Mothering Sunday (3 weeks before Easter)
    if name.contains("mothering sunday") {
        return Some(easter_date(year) - Duration::days(21));
    }

    // Father's Day (3rd Sunday of June)
    if name.contains("father") && name.contains("day") {
        return Some(nth_dow_of_month(year, 6, 6, 3)); // 6=Sunday, 3rd occurrence
    }

    // National Grandparents Day (2nd Sunday in September)
    if name.contains("grandparents") {
        return Some(nth_dow_of_month(year, 9, 6, 2)); // 6=Sunday, 2nd occurrence
    }

    // Military Spouse Day (Friday before Mother's Day)
    if name.contains("military spouse") {
        let mothers_day = nth_dow_of_month(year, 5, 6, 2);
        return Some(mothers_day - Duration::days(2)); // Friday
    }

    // Emancipation Day (DC observed date)
    if name.starts_with("emancipation") {
        return Some(observed_emancipation_day(year));
    }

    // Tax Day (April 15 adjusted for weekend and Emancipation Day)
    if name == "tax day" {
        return Some(compute_tax_day(year));
    }

    // Emergency Medical Services for Children Day (Wednesday in EMS week)
    if name.starts_with("emsc day") {
        let ems_week_start = nth_dow_of_month(year, 5, 6, 3); // 3rd Sunday in May
        return Some(ems_week_start + Duration::days(3));
    }

    // Administrative Professionals' Day (Wednesday of last full week in April)
    if name.contains("administrative")
        || name.contains("secretaries")
        || name.starts_with("admins day")
    {
        return Some(administrative_professionals_day(year));
    }

    // Daylight Saving start/end day (US rules)
    if name.contains("daylight saving") || name.contains("daylight savings") {
        if name.contains("start") {
            return Some(nth_dow_of_month(year, 3, 6, 2)); // 2nd Sunday in March
        }
        if name.contains("end") {
            return Some(nth_dow_of_month(year, 11, 6, 1)); // 1st Sunday in November
        }
    }

    // Super Tuesday family
    if name.contains("super tuesday")
        || name.contains("giga tuesday")
        || name.contains("mega giga tuesday")
        || name.contains("super duper tuesday")
        || name.contains("tsunami tuesday")
    {
        return super_tuesday_date(year);
    }

    // Mini-Tuesday
    if name.contains("mini-tuesday") || name.contains("mini tuesday") {
        return mini_tuesday_date(year);
    }

    // Easter-based holidays
    let easter = easter_date(year);
    match name {
        s if s.starts_with("easter") => {
            if s.contains("mon") {
                return Some(easter + Duration::days(1));
            }
            return Some(easter);
        }
        s if s.starts_with("ascension day")
            || s.starts_with("ascension of jesus")
            || s.starts_with("ascension thursday") =>
        {
            return Some(easter + Duration::days(39))
        }
        s if s.starts_with("corpus christi")
            || s.starts_with("feast of corpus christi")
            || s.starts_with("body and blood of christ") =>
        {
            return Some(easter + Duration::days(60))
        }
        "good friday" => return Some(easter - Duration::days(2)),
        s if s.starts_with("holy saturday") || s.starts_with("black saturday") => {
            return Some(easter - Duration::days(1))
        }
        s if s.starts_with("palm sunday") || s.starts_with("branch sunday") => {
            return Some(easter - Duration::days(7))
        }
        s if s.starts_with("maundy")
            || s.starts_with("covenant thu")
            || s.contains("of mysteries") =>
        {
            return Some(easter - Duration::days(3))
        }
        s if s.starts_with("pentecost") || s.starts_with("white sunday") => {
            return Some(easter + Duration::days(49))
        }
        s if s.starts_with("whit monday") || s.starts_with("monday of the holy spirit") => {
            return Some(easter + Duration::days(50))
        }
        "trinity sunday" => return Some(easter + Duration::days(56)),
        s if s.starts_with("pancake")
            || s.starts_with("mardi gras")
            || s.starts_with("shrove tuesday") =>
        {
            return Some(easter - Duration::days(47))
        }
        s if s.starts_with("ash wednesday") => return Some(easter - Duration::days(46)),
        "lent" => {
            // Lent is an interval — return start date (Ash Wednesday)
            return Some(easter - Duration::days(46));
        }
        _ => {}
    }

    // Orthodox Easter-based holidays
    let orthodox = orthodox_easter_date(year);
    if name.starts_with("orthodox") {
        if name.contains("easter") && name.contains("mon") {
            return Some(orthodox + Duration::days(1));
        }
        if name.contains("easter") {
            return Some(orthodox);
        }
        if name.contains("good friday") || name.contains("great friday") {
            return Some(orthodox - Duration::days(2));
        }
        if name.contains("holy saturday") || name.contains("black saturday") {
            return Some(orthodox - Duration::days(1));
        }
    }
    // Clean Monday and its aliases (Haskell: (orthodox\s+)?(ash|clean|green|pure|shrove)\s+monday)
    if name.contains("monday")
        && (name.contains("clean")
            || name.contains("ash")
            || name.contains("green")
            || name.contains("pure")
            || name.contains("shrove")
            || name.contains("lent"))
    {
        return Some(orthodox - Duration::days(48));
    }
    if name == "lazarus saturday" {
        return Some(orthodox - Duration::days(8));
    }
    if name == "great fast" {
        // Interval: Clean Monday to Lazarus Saturday
        return Some(orthodox - Duration::days(48));
    }

    // Jewish holidays (lookup tables)
    if let Some(d) = resolve_jewish_holiday(name, year) {
        return Some(d);
    }

    // Islamic holidays (lookup tables)
    if let Some(d) = resolve_islamic_holiday(name, year) {
        return Some(d);
    }

    // Hindu holidays (lookup tables)
    if let Some(d) = resolve_hindu_holiday(name, year) {
        return Some(d);
    }

    // GYSD / Global Youth Service Day (varies)
    if name == "gysd" || name.starts_with("global youth service") {
        return gysd_date(year);
    }

    // National Heroes' Day (PH): last Monday of August
    if name == "national heroes' day" || name == "national heroes day" {
        return Some(last_dow_of_month(year, 8, 0));
    }

    // Heroes' Day aliases
    if name == "heroes' day" || name == "heroes day" || name == "kruger day" {
        return NaiveDate::from_ymd_opt(year, 10, 10);
    }

    // Hosay (TT): Day of Ashura
    if name == "hosay" {
        return islamic_new_year(year).map(|d| d + Duration::days(9));
    }

    // Earth Hour (last Saturday of March)
    if name == "earth hour" {
        return Some(last_dow_of_month(year, 3, 5)); // 5=Saturday
    }

    // Parsi New Year / Jamshedi Navroz (every 365 days from Aug 16, 2020)
    if name.contains("parsi") || name.contains("jamshedi") || name.contains("navroz") {
        return parsi_new_year(year);
    }

    // Vesak / Buddha Day
    if name.starts_with("vesak") || name.starts_with("vaisakha") || name.starts_with("buddha") {
        return resolve_vesak(year);
    }

    None
}

// ============================================================
// Easter computation (Anonymous Gregorian algorithm)
// ============================================================

#[allow(clippy::arithmetic_side_effects)]
fn easter_date(year: i32) -> NaiveDate {
    let a = year % 19;
    let b = year / 100;
    let c = year % 100;
    let d = b / 4;
    let e = b % 4;
    let f = (b + 8) / 25;
    let g = (b - f + 1) / 3;
    let h = (19 * a + b - d - g + 15) % 30;
    let i = c / 4;
    let k = c % 4;
    let l = (32 + 2 * e + 2 * i - h - k) % 7;
    let m = (a + 11 * h + 22 * l) / 451;
    let month = (h + l - 7 * m + 114) / 31;
    let day = (h + l - 7 * m + 114) % 31 + 1;
    NaiveDate::from_ymd_opt(year, month as u32, day as u32).unwrap()
}

#[allow(clippy::arithmetic_side_effects)]
fn orthodox_easter_date(year: i32) -> NaiveDate {
    if let Some(d) = orthodox_easter_lookup(year) {
        return d;
    }
    // Fallback: algorithmic computation for years outside table
    let a = year % 4;
    let b = year % 7;
    let c = year % 19;
    let d = (19 * c + 15) % 30;
    let e = (2 * a + 4 * b - d + 34) % 7;
    let month = (d + e + 114) / 31;
    let day = (d + e + 114) % 31 + 1;
    let julian = NaiveDate::from_ymd_opt(year, month as u32, day as u32).unwrap();
    julian + Duration::days(13)
}

fn orthodox_easter_lookup(year: i32) -> Option<NaiveDate> {
    let (m, d) = match year {
        1950 => (4, 9),
        1951 => (4, 29),
        1952 => (4, 20),
        1953 => (4, 5),
        1954 => (4, 25),
        1955 => (4, 17),
        1956 => (5, 6),
        1957 => (4, 21),
        1958 => (4, 13),
        1959 => (5, 3),
        1960 => (4, 17),
        1961 => (4, 9),
        1962 => (4, 29),
        1963 => (4, 14),
        1964 => (5, 3),
        1965 => (4, 25),
        1966 => (4, 10),
        1967 => (4, 30),
        1968 => (4, 21),
        1969 => (4, 13),
        1970 => (4, 26),
        1971 => (4, 18),
        1972 => (4, 9),
        1973 => (4, 29),
        1974 => (4, 14),
        1975 => (5, 4),
        1976 => (4, 25),
        1977 => (4, 10),
        1978 => (4, 30),
        1979 => (4, 22),
        1980 => (4, 6),
        1981 => (4, 26),
        1982 => (4, 18),
        1983 => (5, 8),
        1984 => (4, 22),
        1985 => (4, 14),
        1986 => (5, 4),
        1987 => (4, 19),
        1988 => (4, 10),
        1989 => (4, 30),
        1990 => (4, 15),
        1991 => (4, 7),
        1992 => (4, 26),
        1993 => (4, 18),
        1994 => (5, 1),
        1995 => (4, 23),
        1996 => (4, 14),
        1997 => (4, 27),
        1998 => (4, 19),
        1999 => (4, 11),
        2000 => (4, 30),
        2001 => (4, 15),
        2002 => (5, 5),
        2003 => (4, 27),
        2004 => (4, 11),
        2005 => (5, 1),
        2006 => (4, 23),
        2007 => (4, 8),
        2008 => (4, 27),
        2009 => (4, 19),
        2010 => (4, 4),
        2011 => (4, 24),
        2012 => (4, 15),
        2013 => (5, 5),
        2014 => (4, 20),
        2015 => (4, 12),
        2016 => (5, 1),
        2017 => (4, 16),
        2018 => (4, 8),
        2019 => (4, 28),
        2020 => (4, 19),
        2021 => (5, 2),
        2022 => (4, 24),
        2023 => (4, 16),
        2024 => (5, 5),
        2025 => (4, 20),
        2026 => (4, 12),
        2027 => (5, 2),
        2028 => (4, 16),
        2029 => (4, 8),
        2030 => (4, 28),
        2031 => (4, 13),
        2032 => (5, 2),
        2033 => (4, 24),
        2034 => (4, 9),
        2035 => (4, 29),
        2036 => (4, 20),
        2037 => (4, 5),
        2038 => (4, 25),
        2039 => (4, 17),
        2040 => (5, 6),
        2041 => (4, 21),
        2042 => (4, 13),
        2043 => (5, 3),
        2044 => (4, 24),
        2045 => (4, 9),
        2046 => (4, 29),
        2047 => (4, 21),
        2048 => (4, 5),
        2049 => (4, 25),
        2050 => (4, 17),
        _ => return None,
    };
    NaiveDate::from_ymd_opt(year, m, d)
}

// ============================================================
// Nth DOW of month computation
// ============================================================

/// Find the nth occurrence of a given weekday in a month.
/// dow: 0=Monday, 1=Tuesday, ..., 6=Sunday
/// n: 1-based (1=first, 2=second, etc.)
#[allow(clippy::arithmetic_side_effects)]
fn nth_dow_of_month(year: i32, month: u32, dow: u32, n: u32) -> NaiveDate {
    let first = NaiveDate::from_ymd_opt(year, month, 1).unwrap();
    let first_dow = first.weekday().num_days_from_monday();
    let offset = ((dow as i32 - first_dow as i32) + 7) % 7;
    let day = 1 + offset as u32 + (n - 1) * 7;
    NaiveDate::from_ymd_opt(year, month, day).unwrap_or(first)
}

/// Find the last occurrence of a given weekday in a month.
#[allow(clippy::arithmetic_side_effects)]
fn last_dow_of_month(year: i32, month: u32, dow: u32) -> NaiveDate {
    let last_day = days_in_month(year, month);
    let last = NaiveDate::from_ymd_opt(year, month, last_day).unwrap();
    let last_dow = last.weekday().num_days_from_monday();
    let diff = ((last_dow as i32 - dow as i32) + 7) % 7;
    last - Duration::days(diff as i64)
}

fn observed_emancipation_day(year: i32) -> NaiveDate {
    let d = NaiveDate::from_ymd_opt(year, 4, 16).unwrap();
    match d.weekday().num_days_from_monday() {
        5 => d
            .checked_sub_signed(Duration::try_days(1).unwrap_or_default())
            .unwrap_or(d), // Saturday -> Friday
        6 => d
            .checked_add_signed(Duration::try_days(1).unwrap_or_default())
            .unwrap_or(d), // Sunday -> Monday
        _ => d,
    }
}

fn compute_tax_day(year: i32) -> NaiveDate {
    let mut d = NaiveDate::from_ymd_opt(year, 4, 15).unwrap();
    let emancipation = observed_emancipation_day(year);
    while matches!(d.weekday().num_days_from_monday(), 5 | 6) || d == emancipation {
        d = d
            .checked_add_signed(Duration::try_days(1).unwrap_or_default())
            .unwrap_or(d);
    }
    d
}

#[allow(clippy::arithmetic_side_effects)]
fn administrative_professionals_day(year: i32) -> NaiveDate {
    // Wednesday of the last full week in April (Sunday-start week).
    let mut sunday = NaiveDate::from_ymd_opt(year, 4, 30).unwrap();
    while sunday.weekday().num_days_from_sunday() != 0 {
        sunday = sunday
            .checked_sub_signed(Duration::try_days(1).unwrap_or_default())
            .unwrap_or(sunday);
    }
    if sunday.month() != 4
        || (sunday
            .checked_add_signed(Duration::try_days(6).unwrap_or_default())
            .unwrap_or(sunday))
        .month()
            != 4
    {
        sunday = sunday
            .checked_sub_signed(Duration::try_days(7).unwrap_or_default())
            .unwrap_or(sunday);
    }
    sunday
        .checked_add_signed(Duration::try_days(3).unwrap_or_default())
        .unwrap_or(sunday) // Wednesday
}

fn super_tuesday_date(year: i32) -> Option<NaiveDate> {
    let (m, d) = match year {
        2000 => (3, 7),
        2004 => (3, 2),
        2008 => (2, 5),
        2012 => (3, 6),
        2016 => (3, 1),
        2020 => (3, 3),
        _ => return None,
    };
    NaiveDate::from_ymd_opt(year, m, d)
}

fn mini_tuesday_date(year: i32) -> Option<NaiveDate> {
    let (m, d) = match year {
        2004 => (2, 3),
        _ => return None,
    };
    NaiveDate::from_ymd_opt(year, m, d)
}

/// Find the closest occurrence of a specific day of week to a given date.
/// Given a DOW and a specific date (month/day, optional year), find the nearest occurrence
/// where the date falls on the given DOW. Checks both past and future.
#[allow(clippy::arithmetic_side_effects)]
fn find_dow_date_intersection(
    dow: u32,
    base_dt: DateTime<FixedOffset>,
    month: u32,
    day: u32,
    year: Option<i32>,
) -> DateTime<FixedOffset> {
    let offset = *base_dt.offset();
    if let Some(y) = year {
        // Year is fixed — just return the date regardless of DOW
        let date = NaiveDate::from_ymd_opt(y, month, day).unwrap_or(base_dt.date_naive());
        return fixed_at(offset, date, 0, 0, 0);
    }
    // Search both forward and backward to find the nearest year where DOW matches
    let ref_date = base_dt.date_naive();
    let start_y = base_dt.year();
    let mut best: Option<NaiveDate> = None;
    for delta in -3..=7i32 {
        let y = start_y.saturating_add(delta);
        if let Some(date) = NaiveDate::from_ymd_opt(y, month, day) {
            if date.weekday().num_days_from_monday() == dow {
                match best {
                    None => best = Some(date),
                    Some(prev) => {
                        if date.signed_duration_since(ref_date).num_days().abs()
                            < prev.signed_duration_since(ref_date).num_days().abs()
                        {
                            best = Some(date);
                        }
                    }
                }
            }
        }
    }
    fixed_at(offset, best.unwrap_or(base_dt.date_naive()), 0, 0, 0)
}

/// Find next date where both DOW and day-of-month match.
/// E.g., "Thu 15th" — find next 15th that falls on a Thursday.
fn find_dow_dom_intersection(
    dow: u32,
    day: u32,
    ref_time: DateTime<FixedOffset>,
) -> DateTime<FixedOffset> {
    // Search forward from ref_time up to 12 months
    let mut dt = ref_time;
    for _ in 0..12 {
        let y = dt.year();
        let m = dt.month();
        if let Some(date) = NaiveDate::from_ymd_opt(y, m, day) {
            let candidate = date
                .and_hms_opt(0, 0, 0)
                .unwrap()
                .and_local_timezone(*ref_time.offset())
                .single()
                .unwrap();
            if candidate > ref_time && date.weekday().num_days_from_monday() == dow {
                return candidate;
            }
        }
        // Move to next month
        dt = add_months(
            NaiveDate::from_ymd_opt(y, m, 1)
                .unwrap()
                .and_hms_opt(0, 0, 0)
                .unwrap()
                .and_local_timezone(*ref_time.offset())
                .single()
                .unwrap(),
            1,
        )
        .unwrap_or(dt);
    }
    // Fallback: just return the next occurrence of that day
    midnight(ref_time)
}

/// Find the closest weekday (Mon-Fri) to a given date.
fn closest_weekday(date: NaiveDate) -> NaiveDate {
    let dow = date.weekday().num_days_from_monday();
    match dow {
        5 => date
            .checked_sub_signed(Duration::try_days(1).unwrap_or_default())
            .unwrap_or(date), // Saturday -> Friday
        6 => date
            .checked_add_signed(Duration::try_days(1).unwrap_or_default())
            .unwrap_or(date), // Sunday -> Monday
        _ => date,
    }
}

fn royal_queensland_show_start(year: i32) -> Option<NaiveDate> {
    // First Friday in August, unless that falls before Aug 5, then use second Friday.
    let first_friday = nth_dow_of_month(year, 8, 4, 1);
    if first_friday.day() < 5 {
        Some(nth_dow_of_month(year, 8, 4, 2))
    } else {
        Some(first_friday)
    }
}

// ============================================================
// Holiday lookup tables
// ============================================================

fn chinese_new_year(year: i32) -> Option<NaiveDate> {
    let (m, d) = match year {
        1950 => (2, 17),
        1951 => (2, 6),
        1952 => (1, 27),
        1953 => (2, 14),
        1954 => (2, 3),
        1955 => (1, 24),
        1956 => (2, 12),
        1957 => (1, 31),
        1958 => (2, 18),
        1959 => (2, 8),
        1960 => (1, 28),
        1961 => (2, 15),
        1962 => (2, 5),
        1963 => (1, 25),
        1964 => (2, 13),
        1965 => (2, 2),
        1966 => (1, 21),
        1967 => (2, 9),
        1968 => (1, 30),
        1969 => (2, 17),
        1970 => (2, 6),
        1971 => (1, 27),
        1972 => (2, 15),
        1973 => (2, 3),
        1974 => (1, 23),
        1975 => (2, 11),
        1976 => (1, 31),
        1977 => (2, 18),
        1978 => (2, 7),
        1979 => (1, 28),
        1980 => (2, 16),
        1981 => (2, 5),
        1982 => (1, 25),
        1983 => (2, 13),
        1984 => (2, 2),
        1985 => (2, 20),
        1986 => (2, 9),
        1987 => (1, 29),
        1988 => (2, 17),
        1989 => (2, 6),
        1990 => (1, 27),
        1991 => (2, 15),
        1992 => (2, 4),
        1993 => (1, 23),
        1994 => (2, 10),
        1995 => (1, 31),
        1996 => (2, 19),
        1997 => (2, 7),
        1998 => (1, 28),
        1999 => (2, 16),
        2000 => (2, 5),
        2001 => (1, 24),
        2002 => (2, 12),
        2003 => (2, 1),
        2004 => (1, 22),
        2005 => (2, 9),
        2006 => (1, 29),
        2007 => (2, 18),
        2008 => (2, 7),
        2009 => (1, 26),
        2010 => (2, 14),
        2011 => (2, 3),
        2012 => (1, 23),
        2013 => (2, 10),
        2014 => (1, 31),
        2015 => (2, 19),
        2016 => (2, 8),
        2017 => (1, 28),
        2018 => (2, 16),
        2019 => (2, 5),
        2020 => (1, 25),
        2021 => (2, 12),
        2022 => (2, 1),
        2023 => (1, 22),
        2024 => (2, 10),
        2025 => (1, 29),
        2026 => (2, 17),
        2027 => (2, 6),
        2028 => (1, 26),
        2029 => (2, 13),
        2030 => (2, 3),
        2031 => (1, 23),
        2032 => (2, 11),
        2033 => (1, 31),
        2034 => (2, 19),
        2035 => (2, 8),
        2036 => (1, 28),
        2037 => (2, 15),
        2038 => (2, 4),
        2039 => (1, 24),
        2040 => (2, 12),
        2041 => (2, 1),
        2042 => (1, 22),
        2043 => (2, 10),
        2044 => (1, 30),
        2045 => (2, 17),
        2046 => (2, 6),
        2047 => (1, 26),
        2048 => (2, 14),
        2049 => (2, 2),
        2050 => (1, 23),
        _ => return None,
    };
    NaiveDate::from_ymd_opt(year, m, d)
}

#[allow(clippy::arithmetic_side_effects)]
fn resolve_jewish_holiday(name: &str, year: i32) -> Option<NaiveDate> {
    // Rosh Hashanah
    if name.starts_with("rosh hashann") || name.starts_with("rosh hashan") || name == "yom teruah" {
        return rosh_hashanah(year);
    }
    if name == "yom kippur" {
        return rosh_hashanah(year).map(|d| d + Duration::days(9));
    }
    if name == "shemini atzeret" {
        return rosh_hashanah(year).map(|d| d + Duration::days(21));
    }
    if name == "simchat torah" {
        return rosh_hashanah(year).map(|d| d + Duration::days(22));
    }
    if name == "passover" {
        return passover(year);
    }
    if name == "shavuot" {
        return passover(year).map(|d| d + Duration::days(49));
    }
    if name.starts_with("lag baomer") || name.starts_with("lag b'omer") {
        return passover(year).map(|d| d + Duration::days(33));
    }
    if name.starts_with("chanukah") || name.starts_with("hanuk") || name.starts_with("hannuk") {
        return chanukah(year);
    }
    if name.starts_with("tisha") {
        return tisha_bav(year);
    }
    if name == "yom haatzmaut" {
        return yom_haatzmaut(year);
    }
    if name.starts_with("lag b") || name.starts_with("lag b'") {
        return lag_bomer(year);
    }
    if name == "yom hashoah" || name == "holocaust day" {
        return yom_hashoah(year);
    }
    if name.starts_with("sukkot")
        || name.starts_with("succos")
        || name.starts_with("feast of the ingathering")
    {
        return rosh_hashanah(year).map(|d| d + Duration::days(14));
    }
    if name == "tu bishvat" {
        return tu_bishvat(year);
    }
    if name == "purim" {
        return purim(year);
    }
    if name == "shushan purim" {
        return purim(year).map(|d| d + Duration::days(1));
    }
    None
}

fn rosh_hashanah(year: i32) -> Option<NaiveDate> {
    let (m, d) = match year {
        1950 => (9, 11),
        1951 => (9, 30),
        1952 => (9, 19),
        1953 => (9, 9),
        1954 => (9, 27),
        1955 => (9, 16),
        1956 => (9, 5),
        1957 => (9, 25),
        1958 => (9, 14),
        1959 => (10, 2),
        1960 => (9, 21),
        1961 => (9, 10),
        1962 => (9, 28),
        1963 => (9, 18),
        1964 => (9, 6),
        1965 => (9, 26),
        1966 => (9, 14),
        1967 => (10, 4),
        1968 => (9, 22),
        1969 => (9, 12),
        1970 => (9, 30),
        1971 => (9, 19),
        1972 => (9, 8),
        1973 => (9, 26),
        1974 => (9, 16),
        1975 => (9, 5),
        1976 => (9, 24),
        1977 => (9, 12),
        1978 => (10, 1),
        1979 => (9, 21),
        1980 => (9, 10),
        1981 => (9, 28),
        1982 => (9, 17),
        1983 => (9, 7),
        1984 => (9, 26),
        1985 => (9, 15),
        1986 => (10, 3),
        1987 => (9, 23),
        1988 => (9, 11),
        1989 => (9, 29),
        1990 => (9, 19),
        1991 => (9, 8),
        1992 => (9, 27),
        1993 => (9, 15),
        1994 => (9, 5),
        1995 => (9, 24),
        1996 => (9, 13),
        1997 => (10, 1),
        1998 => (9, 20),
        1999 => (9, 10),
        2000 => (9, 29),
        2001 => (9, 17),
        2002 => (9, 6),
        2003 => (9, 26),
        2004 => (9, 15),
        2005 => (10, 3),
        2006 => (9, 22),
        2007 => (9, 12),
        2008 => (9, 29),
        2009 => (9, 18),
        2010 => (9, 8),
        2011 => (9, 28),
        2012 => (9, 18),
        2013 => (9, 4),
        2014 => (9, 24),
        2015 => (9, 13),
        2016 => (10, 2),
        2017 => (9, 20),
        2018 => (9, 9),
        2019 => (9, 29),
        2020 => (9, 18),
        2021 => (9, 6),
        2022 => (9, 25),
        2023 => (9, 15),
        2024 => (10, 2),
        2025 => (9, 22),
        2026 => (9, 11),
        2027 => (10, 1),
        2028 => (9, 20),
        2029 => (9, 9),
        2030 => (9, 27),
        2031 => (9, 17),
        2032 => (9, 5),
        2033 => (9, 23),
        2034 => (9, 13),
        2035 => (10, 3),
        2036 => (9, 21),
        2037 => (9, 9),
        2038 => (9, 29),
        2039 => (9, 18),
        2040 => (9, 7),
        2041 => (9, 25),
        2042 => (9, 14),
        2043 => (10, 4),
        2044 => (9, 21),
        2045 => (9, 11),
        2046 => (9, 30),
        2047 => (9, 20),
        2048 => (9, 7),
        2049 => (9, 26),
        2050 => (9, 16),
        _ => return None,
    };
    NaiveDate::from_ymd_opt(year, m, d)
}

fn passover(year: i32) -> Option<NaiveDate> {
    let (m, d) = match year {
        1950 => (4, 1),
        1951 => (4, 20),
        1952 => (4, 9),
        1953 => (3, 30),
        1954 => (4, 17),
        1955 => (4, 6),
        1956 => (3, 26),
        1957 => (4, 15),
        1958 => (4, 4),
        1959 => (4, 22),
        1960 => (4, 11),
        1961 => (3, 31),
        1962 => (4, 18),
        1963 => (4, 8),
        1964 => (3, 27),
        1965 => (4, 16),
        1966 => (4, 4),
        1967 => (4, 24),
        1968 => (4, 12),
        1969 => (4, 2),
        1970 => (4, 20),
        1971 => (4, 9),
        1972 => (3, 29),
        1973 => (4, 16),
        1974 => (4, 6),
        1975 => (3, 26),
        1976 => (4, 14),
        1977 => (4, 2),
        1978 => (4, 21),
        1979 => (4, 11),
        1980 => (3, 31),
        1981 => (4, 18),
        1982 => (4, 7),
        1983 => (3, 28),
        1984 => (4, 16),
        1985 => (4, 5),
        1986 => (4, 23),
        1987 => (4, 13),
        1988 => (4, 1),
        1989 => (4, 19),
        1990 => (4, 9),
        1991 => (3, 29),
        1992 => (4, 17),
        1993 => (4, 5),
        1994 => (3, 26),
        1995 => (4, 14),
        1996 => (4, 3),
        1997 => (4, 21),
        1998 => (4, 10),
        1999 => (3, 31),
        2000 => (4, 19),
        2001 => (4, 7),
        2002 => (3, 27),
        2003 => (4, 16),
        2004 => (4, 5),
        2005 => (4, 23),
        2006 => (4, 12),
        2007 => (4, 2),
        2008 => (4, 19),
        2009 => (4, 8),
        2010 => (3, 29),
        2011 => (4, 18),
        2012 => (4, 6),
        2013 => (3, 25),
        2014 => (4, 14),
        2015 => (4, 3),
        2016 => (4, 22),
        2017 => (4, 10),
        2018 => (3, 30),
        2019 => (4, 19),
        2020 => (4, 8),
        2021 => (3, 27),
        2022 => (4, 15),
        2023 => (4, 5),
        2024 => (4, 22),
        2025 => (4, 12),
        2026 => (4, 1),
        2027 => (4, 21),
        2028 => (4, 10),
        2029 => (3, 30),
        2030 => (4, 17),
        2031 => (4, 7),
        2032 => (3, 26),
        2033 => (4, 13),
        2034 => (4, 3),
        2035 => (4, 23),
        2036 => (4, 11),
        2037 => (3, 30),
        2038 => (4, 19),
        2039 => (4, 8),
        2040 => (3, 28),
        2041 => (4, 15),
        2042 => (4, 4),
        2043 => (4, 24),
        2044 => (4, 11),
        2045 => (4, 1),
        2046 => (4, 20),
        2047 => (4, 10),
        2048 => (3, 28),
        2049 => (4, 16),
        2050 => (4, 6),
        _ => return None,
    };
    NaiveDate::from_ymd_opt(year, m, d)
}

fn chanukah(year: i32) -> Option<NaiveDate> {
    let (m, d) = match year {
        1950 => (12, 3),
        1951 => (12, 23),
        1952 => (12, 12),
        1953 => (12, 1),
        1954 => (12, 19),
        1955 => (12, 9),
        1956 => (11, 28),
        1957 => (12, 17),
        1958 => (12, 6),
        1959 => (12, 25),
        1960 => (12, 13),
        1961 => (12, 2),
        1962 => (12, 21),
        1963 => (12, 10),
        1964 => (11, 29),
        1965 => (12, 18),
        1966 => (12, 7),
        1967 => (12, 26),
        1968 => (12, 15),
        1969 => (12, 4),
        1970 => (12, 22),
        1971 => (12, 12),
        1972 => (11, 30),
        1973 => (12, 19),
        1974 => (12, 8),
        1975 => (11, 28),
        1976 => (12, 16),
        1977 => (12, 4),
        1978 => (12, 24),
        1979 => (12, 14),
        1980 => (12, 2),
        1981 => (12, 20),
        1982 => (12, 10),
        1983 => (11, 30),
        1984 => (12, 18),
        1985 => (12, 7),
        1986 => (12, 26),
        1987 => (12, 15),
        1988 => (12, 3),
        1989 => (12, 22),
        1990 => (12, 11),
        1991 => (12, 1),
        1992 => (12, 19),
        1993 => (12, 8),
        1994 => (11, 27),
        1995 => (12, 17),
        1996 => (12, 5),
        1997 => (12, 23),
        1998 => (12, 13),
        1999 => (12, 3),
        2000 => (12, 21),
        2001 => (12, 9),
        2002 => (11, 29),
        2003 => (12, 19),
        2004 => (12, 7),
        2005 => (12, 25),
        2006 => (12, 15),
        2007 => (12, 4),
        2008 => (12, 21),
        2009 => (12, 11),
        2010 => (12, 1),
        2011 => (12, 20),
        2012 => (12, 8),
        2013 => (11, 27),
        2014 => (12, 16),
        2015 => (12, 6),
        2016 => (12, 24),
        2017 => (12, 12),
        2018 => (12, 2),
        2019 => (12, 22),
        2020 => (12, 10),
        2021 => (11, 28),
        2022 => (12, 18),
        2023 => (12, 7),
        2024 => (12, 25),
        2025 => (12, 14),
        2026 => (12, 4),
        2027 => (12, 24),
        2028 => (12, 12),
        2029 => (12, 1),
        2030 => (12, 20),
        2031 => (12, 9),
        2032 => (11, 27),
        2033 => (12, 16),
        2034 => (12, 6),
        2035 => (12, 25),
        2036 => (12, 13),
        2037 => (12, 2),
        2038 => (12, 21),
        2039 => (12, 11),
        2040 => (11, 29),
        2041 => (12, 17),
        2042 => (12, 7),
        2043 => (12, 26),
        2044 => (12, 14),
        2045 => (12, 3),
        2046 => (12, 23),
        2047 => (12, 12),
        2048 => (11, 29),
        2049 => (12, 19),
        2050 => (12, 9),
        _ => return None,
    };
    NaiveDate::from_ymd_opt(year, m, d)
}

fn tisha_bav(year: i32) -> Option<NaiveDate> {
    let (m, d) = match year {
        1950 => (7, 22),
        1951 => (8, 11),
        1952 => (7, 30),
        1953 => (7, 20),
        1954 => (8, 7),
        1955 => (7, 27),
        1956 => (7, 16),
        1957 => (8, 5),
        1958 => (7, 26),
        1959 => (8, 12),
        1960 => (8, 1),
        1961 => (7, 22),
        1962 => (8, 8),
        1963 => (7, 29),
        1964 => (7, 18),
        1965 => (8, 7),
        1966 => (7, 25),
        1967 => (8, 14),
        1968 => (8, 3),
        1969 => (7, 23),
        1970 => (8, 10),
        1971 => (7, 31),
        1972 => (7, 19),
        1973 => (8, 6),
        1974 => (7, 27),
        1975 => (7, 16),
        1976 => (8, 4),
        1977 => (7, 23),
        1978 => (8, 12),
        1979 => (8, 1),
        1980 => (7, 21),
        1981 => (8, 8),
        1982 => (7, 28),
        1983 => (7, 18),
        1984 => (8, 6),
        1985 => (7, 27),
        1986 => (8, 13),
        1987 => (8, 3),
        1988 => (7, 23),
        1989 => (8, 9),
        1990 => (7, 30),
        1991 => (7, 20),
        1992 => (8, 8),
        1993 => (7, 26),
        1994 => (7, 16),
        1995 => (8, 5),
        1996 => (7, 24),
        1997 => (8, 11),
        1998 => (8, 1),
        1999 => (7, 21),
        2000 => (8, 9),
        2001 => (7, 28),
        2002 => (7, 17),
        2003 => (8, 6),
        2004 => (7, 26),
        2005 => (8, 13),
        2006 => (8, 2),
        2007 => (7, 23),
        2008 => (8, 9),
        2009 => (7, 29),
        2010 => (7, 19),
        2011 => (8, 8),
        2012 => (7, 28),
        2013 => (7, 15),
        2014 => (8, 4),
        2015 => (7, 25),
        2016 => (8, 13),
        2017 => (7, 31),
        2018 => (7, 21),
        2019 => (8, 10),
        2020 => (7, 29),
        2021 => (7, 17),
        2022 => (8, 6),
        2023 => (7, 26),
        2024 => (8, 12),
        2025 => (8, 2),
        2026 => (7, 22),
        2027 => (8, 11),
        2028 => (7, 31),
        2029 => (7, 21),
        2030 => (8, 7),
        2031 => (7, 28),
        2032 => (7, 17),
        2033 => (8, 3),
        2034 => (7, 24),
        2035 => (8, 13),
        2036 => (8, 2),
        2037 => (7, 20),
        2038 => (8, 9),
        2039 => (7, 30),
        2040 => (7, 18),
        2041 => (8, 5),
        2042 => (7, 26),
        2043 => (8, 15),
        2044 => (8, 1),
        2045 => (7, 22),
        2046 => (8, 11),
        2047 => (7, 31),
        2048 => (7, 18),
        2049 => (8, 7),
        2050 => (7, 27),
        _ => return None,
    };
    NaiveDate::from_ymd_opt(year, m, d)
}

fn yom_haatzmaut(year: i32) -> Option<NaiveDate> {
    let (m, d) = match year {
        1950 => (4, 19),
        1951 => (5, 9),
        1952 => (4, 29),
        1953 => (4, 19),
        1954 => (5, 5),
        1955 => (4, 26),
        1956 => (4, 15),
        1957 => (5, 5),
        1958 => (4, 23),
        1959 => (5, 12),
        1960 => (5, 1),
        1961 => (4, 19),
        1962 => (5, 8),
        1963 => (4, 28),
        1964 => (4, 15),
        1965 => (5, 5),
        1966 => (4, 24),
        1967 => (5, 14),
        1968 => (5, 1),
        1969 => (4, 22),
        1970 => (5, 10),
        1971 => (4, 28),
        1972 => (4, 18),
        1973 => (5, 6),
        1974 => (4, 24),
        1975 => (4, 15),
        1976 => (5, 4),
        1977 => (4, 20),
        1978 => (5, 10),
        1979 => (5, 1),
        1980 => (4, 20),
        1981 => (5, 6),
        1982 => (4, 27),
        1983 => (4, 17),
        1984 => (5, 6),
        1985 => (4, 24),
        1986 => (5, 13),
        1987 => (5, 3),
        1988 => (4, 20),
        1989 => (5, 9),
        1990 => (4, 29),
        1991 => (4, 17),
        1992 => (5, 6),
        1993 => (4, 25),
        1994 => (4, 13),
        1995 => (5, 3),
        1996 => (4, 23),
        1997 => (5, 11),
        1998 => (4, 29),
        1999 => (4, 20),
        2000 => (5, 9),
        2001 => (4, 25),
        2002 => (4, 16),
        2003 => (5, 6),
        2004 => (4, 26),
        2005 => (5, 11),
        2006 => (5, 2),
        2007 => (4, 23),
        2008 => (5, 7),
        2009 => (4, 28),
        2010 => (4, 19),
        2011 => (5, 9),
        2012 => (4, 25),
        2013 => (4, 15),
        2014 => (5, 5),
        2015 => (4, 22),
        2016 => (5, 11),
        2017 => (5, 1),
        2018 => (4, 18),
        2019 => (5, 8),
        2020 => (4, 28),
        2021 => (4, 14),
        2022 => (5, 4),
        2023 => (4, 25),
        2024 => (5, 13),
        2025 => (4, 30),
        2026 => (4, 21),
        2027 => (5, 11),
        2028 => (5, 1),
        2029 => (4, 18),
        2030 => (5, 7),
        2031 => (4, 28),
        2032 => (4, 14),
        2033 => (5, 3),
        2034 => (4, 24),
        2035 => (5, 14),
        2036 => (4, 30),
        2037 => (4, 20),
        2038 => (5, 10),
        2039 => (4, 27),
        2040 => (4, 17),
        2041 => (5, 6),
        2042 => (4, 23),
        2043 => (5, 13),
        2044 => (5, 2),
        2045 => (4, 19),
        2046 => (5, 9),
        2047 => (4, 30),
        2048 => (4, 15),
        2049 => (5, 5),
        2050 => (4, 26),
        _ => return None,
    };
    NaiveDate::from_ymd_opt(year, m, d)
}

fn lag_bomer(year: i32) -> Option<NaiveDate> {
    // Lag Ba'Omer from Haskell lagBaOmer table
    let (m, d) = match year {
        1950 => (5, 4),
        1951 => (5, 23),
        1952 => (5, 12),
        1953 => (5, 2),
        1954 => (5, 20),
        1955 => (5, 9),
        1956 => (4, 28),
        1957 => (5, 18),
        1958 => (5, 7),
        1959 => (5, 25),
        1960 => (5, 14),
        1961 => (5, 3),
        1962 => (5, 21),
        1963 => (5, 11),
        1964 => (4, 29),
        1965 => (5, 19),
        1966 => (5, 7),
        1967 => (5, 27),
        1968 => (5, 15),
        1969 => (5, 5),
        1970 => (5, 23),
        1971 => (5, 12),
        1972 => (5, 1),
        1973 => (5, 19),
        1974 => (5, 9),
        1975 => (4, 28),
        1976 => (5, 17),
        1977 => (5, 5),
        1978 => (5, 24),
        1979 => (5, 14),
        1980 => (5, 3),
        1981 => (5, 21),
        1982 => (5, 10),
        1983 => (4, 30),
        1984 => (5, 19),
        1985 => (5, 8),
        1986 => (5, 26),
        1987 => (5, 16),
        1988 => (5, 4),
        1989 => (5, 22),
        1990 => (5, 12),
        1991 => (5, 1),
        1992 => (5, 20),
        1993 => (5, 8),
        1994 => (4, 28),
        1995 => (5, 17),
        1996 => (5, 6),
        1997 => (5, 24),
        1998 => (5, 13),
        1999 => (5, 3),
        2000 => (5, 22),
        2001 => (5, 10),
        2002 => (4, 29),
        2003 => (5, 19),
        2004 => (5, 8),
        2005 => (5, 26),
        2006 => (5, 15),
        2007 => (5, 5),
        2008 => (5, 22),
        2009 => (5, 11),
        2010 => (5, 1),
        2011 => (5, 21),
        2012 => (5, 9),
        2013 => (4, 27),
        2014 => (5, 17),
        2015 => (5, 6),
        2016 => (5, 25),
        2017 => (5, 13),
        2018 => (5, 2),
        2019 => (5, 22),
        2020 => (5, 11),
        2021 => (4, 29),
        2022 => (5, 18),
        2023 => (5, 8),
        2024 => (5, 25),
        2025 => (5, 15),
        2026 => (5, 4),
        2027 => (5, 24),
        2028 => (5, 13),
        2029 => (5, 2),
        2030 => (5, 20),
        2031 => (5, 10),
        2032 => (4, 28),
        2033 => (5, 16),
        2034 => (5, 6),
        2035 => (5, 26),
        2036 => (5, 14),
        2037 => (5, 2),
        2038 => (5, 22),
        2039 => (5, 11),
        2040 => (4, 30),
        2041 => (5, 18),
        2042 => (5, 7),
        2043 => (5, 27),
        2044 => (5, 14),
        2045 => (5, 4),
        2046 => (5, 23),
        2047 => (5, 13),
        2048 => (4, 30),
        2049 => (5, 19),
        2050 => (5, 9),
        _ => return None,
    };
    NaiveDate::from_ymd_opt(year, m, d)
}

// Yom HaShoah = Passover + 12 (matching Haskell)
fn yom_hashoah(year: i32) -> Option<NaiveDate> {
    passover(year).and_then(|d| d.checked_add_signed(Duration::try_days(12)?))
}

fn tu_bishvat(year: i32) -> Option<NaiveDate> {
    let (m, d) = match year {
        1950 => (2, 1),
        1951 => (1, 21),
        1952 => (2, 10),
        1953 => (1, 30),
        1954 => (1, 18),
        1955 => (2, 6),
        1956 => (1, 27),
        1957 => (1, 16),
        1958 => (2, 4),
        1959 => (1, 23),
        1960 => (2, 12),
        1961 => (1, 31),
        1962 => (1, 19),
        1963 => (2, 8),
        1964 => (1, 28),
        1965 => (1, 17),
        1966 => (2, 4),
        1967 => (1, 25),
        1968 => (2, 13),
        1969 => (2, 2),
        1970 => (1, 21),
        1971 => (2, 9),
        1972 => (1, 30),
        1973 => (1, 17),
        1974 => (2, 6),
        1975 => (1, 26),
        1976 => (1, 16),
        1977 => (2, 2),
        1978 => (1, 22),
        1979 => (2, 11),
        1980 => (2, 1),
        1981 => (1, 19),
        1982 => (2, 7),
        1983 => (1, 28),
        1984 => (1, 18),
        1985 => (2, 5),
        1986 => (1, 24),
        1987 => (2, 13),
        1988 => (2, 2),
        1989 => (1, 20),
        1990 => (2, 9),
        1991 => (1, 29),
        1992 => (1, 19),
        1993 => (2, 5),
        1994 => (1, 26),
        1995 => (1, 15),
        1996 => (2, 4),
        1997 => (1, 22),
        1998 => (2, 10),
        1999 => (1, 31),
        2000 => (1, 21),
        2001 => (2, 7),
        2002 => (1, 27),
        2003 => (1, 17),
        2004 => (2, 6),
        2005 => (1, 24),
        2006 => (2, 12),
        2007 => (2, 2),
        2008 => (1, 21),
        2009 => (2, 8),
        2010 => (1, 29),
        2011 => (1, 19),
        2012 => (2, 7),
        2013 => (1, 25),
        2014 => (1, 15),
        2015 => (2, 3),
        2016 => (1, 24),
        2017 => (2, 10),
        2018 => (1, 30),
        2019 => (1, 20),
        2020 => (2, 9),
        2021 => (1, 27),
        2022 => (1, 16),
        2023 => (2, 5),
        2024 => (1, 24),
        2025 => (2, 12),
        2026 => (2, 1),
        2027 => (1, 22),
        2028 => (2, 11),
        2029 => (1, 30),
        2030 => (1, 18),
        2031 => (2, 7),
        2032 => (1, 27),
        2033 => (1, 14),
        2034 => (2, 3),
        2035 => (1, 24),
        2036 => (2, 12),
        2037 => (1, 30),
        2038 => (1, 20),
        2039 => (2, 8),
        2040 => (1, 29),
        2041 => (1, 16),
        2042 => (2, 4),
        2043 => (1, 25),
        2044 => (2, 12),
        2045 => (2, 1),
        2046 => (1, 21),
        2047 => (2, 10),
        2048 => (1, 29),
        2049 => (1, 17),
        2050 => (2, 6),
        _ => return None,
    };
    NaiveDate::from_ymd_opt(year, m, d)
}

fn purim(year: i32) -> Option<NaiveDate> {
    let (m, d) = match year {
        1950 => (3, 2),
        1951 => (3, 21),
        1952 => (3, 10),
        1953 => (2, 28),
        1954 => (3, 18),
        1955 => (3, 7),
        1956 => (2, 25),
        1957 => (3, 16),
        1958 => (3, 5),
        1959 => (3, 23),
        1960 => (3, 12),
        1961 => (3, 1),
        1962 => (3, 19),
        1963 => (3, 9),
        1964 => (2, 26),
        1965 => (3, 17),
        1966 => (3, 5),
        1967 => (3, 25),
        1968 => (3, 13),
        1969 => (3, 3),
        1970 => (3, 21),
        1971 => (3, 10),
        1972 => (2, 28),
        1973 => (3, 17),
        1974 => (3, 7),
        1975 => (2, 24),
        1976 => (3, 15),
        1977 => (3, 3),
        1978 => (3, 22),
        1979 => (3, 12),
        1980 => (3, 1),
        1981 => (3, 19),
        1982 => (3, 8),
        1983 => (2, 26),
        1984 => (3, 17),
        1985 => (3, 6),
        1986 => (3, 24),
        1987 => (3, 14),
        1988 => (3, 2),
        1989 => (3, 20),
        1990 => (3, 10),
        1991 => (2, 27),
        1992 => (3, 18),
        1993 => (3, 6),
        1994 => (2, 24),
        1995 => (3, 15),
        1996 => (3, 4),
        1997 => (3, 22),
        1998 => (3, 11),
        1999 => (3, 1),
        2000 => (3, 20),
        2001 => (3, 8),
        2002 => (2, 27),
        2003 => (3, 19),
        2004 => (3, 6),
        2005 => (3, 24),
        2006 => (3, 13),
        2007 => (3, 3),
        2008 => (3, 20),
        2009 => (3, 9),
        2010 => (2, 27),
        2011 => (3, 19),
        2012 => (3, 7),
        2013 => (2, 23),
        2014 => (3, 15),
        2015 => (3, 4),
        2016 => (3, 23),
        2017 => (3, 11),
        2018 => (2, 28),
        2019 => (3, 20),
        2020 => (3, 9),
        2021 => (2, 25),
        2022 => (3, 16),
        2023 => (3, 6),
        2024 => (3, 23),
        2025 => (3, 13),
        2026 => (3, 2),
        2027 => (3, 22),
        2028 => (3, 11),
        2029 => (2, 28),
        2030 => (3, 18),
        2031 => (3, 8),
        2032 => (2, 25),
        2033 => (3, 14),
        2034 => (3, 4),
        2035 => (3, 24),
        2036 => (3, 12),
        2037 => (2, 28),
        2038 => (3, 20),
        2039 => (3, 9),
        2040 => (2, 27),
        2041 => (3, 16),
        2042 => (3, 5),
        2043 => (3, 25),
        2044 => (3, 12),
        2045 => (3, 2),
        2046 => (3, 21),
        2047 => (3, 11),
        2048 => (2, 27),
        2049 => (3, 17),
        2050 => (3, 7),
        _ => return None,
    };
    NaiveDate::from_ymd_opt(year, m, d)
}

#[allow(clippy::arithmetic_side_effects)]
fn resolve_islamic_holiday(name: &str, year: i32) -> Option<NaiveDate> {
    if name.starts_with("mawlid") {
        return mawlid(year);
    }
    if name.starts_with("eid al-fitr") || name.starts_with("eid al fitr") {
        return eid_al_fitr(year);
    }
    if name.starts_with("eid al-adha")
        || name.starts_with("eid al adha")
        || name.starts_with("id ul")
        || name.starts_with("sacrifice feast")
        || name.starts_with("bakr id")
    {
        return eid_al_adha(year);
    }
    if name.starts_with("laylat") || name.starts_with("night of") {
        return laylat_al_qadr(year);
    }
    if name.starts_with("islamic new year") || name.starts_with("amun jadid") {
        return islamic_new_year(year);
    }
    if name.starts_with("ashura") || name.starts_with("day of ashura") {
        return islamic_new_year(year).map(|d| d + Duration::days(9));
    }
    if name == "ramadan" {
        return ramadan(year);
    }
    if name.starts_with("isra")
        || name.starts_with("the prophet")
        || name.starts_with("the night journey")
        || name.starts_with("ascension to heaven")
    {
        return isra_miraj(year);
    }
    if name.starts_with("jumu") || name.starts_with("jamat") {
        // Jamat ul-Vida = last Friday before Eid al-Fitr
        return eid_al_fitr(year).map(|eid| {
            let dow = eid.weekday().num_days_from_monday(); // 0=Mon...4=Fri
            let days_back = ((dow as i64 - 4) % 7 + 7) % 7; // days to go back to Friday
            let days_back = if days_back == 0 { 7 } else { days_back }; // if Eid is Friday, go back 7
            eid - Duration::days(days_back)
        });
    }
    None
}

fn mawlid(year: i32) -> Option<NaiveDate> {
    let (m, d) = match year {
        1950 => (1, 1), // Also 1950-12-22, but take earliest
        1951 => (12, 11),
        1952 => (11, 30),
        1953 => (11, 19),
        1954 => (11, 8),
        1955 => (10, 29),
        1956 => (10, 17),
        1957 => (10, 6),
        1958 => (9, 26),
        1959 => (9, 15),
        1960 => (9, 3),
        1961 => (8, 23),
        1962 => (8, 12),
        1963 => (8, 2),
        1964 => (7, 21),
        1965 => (7, 10),
        1966 => (7, 1),
        1967 => (6, 19),
        1968 => (6, 8),
        1969 => (5, 28),
        1970 => (5, 18),
        1971 => (5, 7),
        1972 => (4, 25),
        1973 => (4, 15),
        1974 => (4, 4),
        1975 => (3, 24),
        1976 => (3, 12),
        1977 => (3, 2),
        1978 => (2, 19),
        1979 => (2, 9),
        1980 => (1, 30),
        1981 => (1, 18),
        1982 => (1, 7),
        1983 => (12, 16),
        1984 => (12, 4),
        1985 => (11, 24),
        1986 => (11, 14),
        1987 => (11, 3),
        1988 => (10, 22),
        1989 => (10, 11),
        1990 => (10, 1),
        1991 => (9, 20),
        1992 => (9, 9),
        1993 => (8, 29),
        1994 => (8, 19),
        1995 => (8, 8),
        1996 => (7, 27),
        1997 => (7, 16),
        1998 => (7, 6),
        1999 => (6, 26),
        2000 => (6, 14),
        2001 => (6, 4),
        2002 => (5, 24),
        2003 => (5, 13),
        2004 => (5, 1),
        2005 => (4, 21),
        2006 => (4, 10),
        2007 => (3, 31),
        2008 => (3, 20),
        2009 => (3, 9),
        2010 => (2, 26),
        2011 => (2, 15),
        2012 => (2, 4),
        2013 => (1, 24),
        2014 => (1, 13),
        2015 => (1, 3),
        2016 => (12, 11),
        2017 => (11, 30),
        2018 => (11, 20),
        2019 => (11, 9),
        2020 => (10, 29),
        2021 => (10, 18),
        2022 => (10, 8),
        2023 => (9, 27),
        2024 => (9, 15),
        2025 => (9, 4),
        2026 => (8, 25),
        2027 => (8, 14),
        2028 => (8, 3),
        _ => return None,
    };
    NaiveDate::from_ymd_opt(year, m, d)
}

fn eid_al_fitr(year: i32) -> Option<NaiveDate> {
    let (m, d) = match year {
        1950 => (7, 16),
        1951 => (7, 6),
        1952 => (6, 23),
        1953 => (6, 13),
        1954 => (6, 2),
        1955 => (5, 23),
        1956 => (5, 11),
        1957 => (5, 1),
        1958 => (4, 20),
        1959 => (4, 10),
        1960 => (3, 28),
        1961 => (3, 18),
        1962 => (3, 7),
        1963 => (2, 24),
        1964 => (2, 14),
        1965 => (2, 2),
        1966 => (1, 22),
        1967 => (1, 12),
        1968 => (1, 1),
        // 1968 also has Dec 21
        1969 => (12, 10),
        1970 => (11, 30),
        1971 => (11, 19),
        1972 => (11, 7),
        1973 => (10, 27),
        1974 => (10, 16),
        1975 => (10, 6),
        1976 => (9, 24),
        1977 => (9, 14),
        1978 => (9, 3),
        1979 => (8, 23),
        1980 => (8, 12),
        1981 => (8, 1),
        1982 => (7, 21),
        1983 => (7, 11),
        1984 => (6, 30),
        1985 => (6, 19),
        1986 => (6, 8),
        1987 => (5, 28),
        1988 => (5, 16),
        1989 => (5, 6),
        1990 => (4, 26),
        1991 => (4, 15),
        1992 => (4, 4),
        1993 => (3, 24),
        1994 => (3, 13),
        1995 => (3, 2),
        1996 => (2, 19),
        1997 => (2, 8),
        1998 => (1, 29),
        1999 => (1, 18),
        2000 => (1, 8),
        // 2000 also has Dec 27
        2001 => (12, 16),
        2002 => (12, 5),
        2003 => (11, 25),
        2004 => (11, 14),
        2005 => (11, 3),
        2006 => (10, 23),
        2007 => (10, 13),
        2008 => (10, 1),
        2009 => (9, 20),
        2010 => (9, 10),
        2011 => (8, 30),
        2012 => (8, 19),
        2013 => (8, 8),
        2014 => (7, 28),
        2015 => (7, 17),
        2016 => (7, 6),
        2017 => (6, 25),
        2018 => (6, 15),
        2019 => (6, 4),
        2020 => (5, 24),
        2021 => (5, 13),
        2022 => (5, 2),
        2023 => (4, 21),
        2024 => (4, 10),
        2025 => (3, 30),
        2026 => (3, 20),
        2027 => (3, 9),
        2028 => (2, 26),
        2029 => (2, 14),
        2030 => (2, 5),
        2031 => (1, 25),
        2032 => (1, 14),
        2033 => (1, 3),
        // 2033 also has Dec 23
        2034 => (12, 12),
        2035 => (12, 2),
        2036 => (11, 20),
        2037 => (11, 10),
        2038 => (10, 30),
        2039 => (10, 19),
        2040 => (10, 8),
        2041 => (9, 27),
        2042 => (9, 16),
        2043 => (9, 6),
        2044 => (8, 25),
        2045 => (8, 15),
        2046 => (8, 4),
        2047 => (7, 24),
        2048 => (7, 13),
        2049 => (7, 2),
        2050 => (6, 21),
        _ => return None,
    };
    NaiveDate::from_ymd_opt(year, m, d)
}

fn eid_al_adha(year: i32) -> Option<NaiveDate> {
    let (m, d) = match year {
        1950 => (9, 23),
        1951 => (9, 12),
        1952 => (8, 31),
        1953 => (8, 20),
        1954 => (8, 9),
        1955 => (7, 30),
        1956 => (7, 19),
        1957 => (7, 8),
        1958 => (6, 27),
        1959 => (6, 17),
        1960 => (6, 4),
        1961 => (5, 25),
        1962 => (5, 14),
        1963 => (5, 3),
        1964 => (4, 22),
        1965 => (4, 11),
        1966 => (4, 1),
        1967 => (3, 21),
        1968 => (3, 9),
        1969 => (2, 27),
        1970 => (2, 16),
        1971 => (2, 6),
        1972 => (1, 26),
        1973 => (1, 14),
        1974 => (1, 3),
        // 1974 also has Dec 24
        1975 => (12, 13),
        1976 => (12, 1),
        1977 => (11, 21),
        1978 => (11, 10),
        1979 => (10, 31),
        1980 => (10, 19),
        1981 => (10, 8),
        1982 => (9, 27),
        1983 => (9, 17),
        1984 => (9, 5),
        1985 => (8, 26),
        1986 => (8, 15),
        1987 => (8, 4),
        1988 => (7, 23),
        1989 => (7, 13),
        1990 => (7, 2),
        1991 => (6, 22),
        1992 => (6, 11),
        1993 => (5, 31),
        1994 => (5, 20),
        1995 => (5, 9),
        1996 => (4, 27),
        1997 => (4, 17),
        1998 => (4, 7),
        1999 => (3, 27),
        2000 => (3, 16),
        2001 => (3, 5),
        2002 => (2, 22),
        2003 => (2, 11),
        2004 => (2, 1),
        2005 => (1, 21),
        2006 => (1, 10),
        // 2006 also has Dec 31
        2007 => (12, 20),
        2008 => (12, 8),
        2009 => (11, 27),
        2011 => (11, 6),
        2012 => (10, 26),
        2013 => (10, 15),
        2014 => (10, 4),
        2015 => (8, 23),
        2016 => (9, 11),
        2017 => (9, 1),
        2018 => (8, 21),
        2019 => (8, 11),
        2020 => (7, 31),
        2021 => (7, 20),
        2022 => (7, 9),
        2023 => (6, 28),
        2024 => (6, 16),
        2025 => (6, 6),
        2026 => (5, 27),
        2027 => (5, 16),
        2028 => (5, 5),
        2029 => (4, 24),
        _ => return None,
    };
    NaiveDate::from_ymd_opt(year, m, d)
}

fn laylat_al_qadr(year: i32) -> Option<NaiveDate> {
    // Laylat al-Qadr = Ramadan + 26 days
    ramadan(year).and_then(|d| d.checked_add_signed(Duration::try_days(26)?))
}

fn islamic_new_year(year: i32) -> Option<NaiveDate> {
    // Muharram / Islamic New Year from Haskell muharram table
    let (m, d) = match year {
        1998 => (4, 27),
        1999 => (4, 17),
        2000 => (4, 6),
        2001 => (3, 26),
        2002 => (3, 15),
        2003 => (4, 4),
        2004 => (2, 21),
        2005 => (2, 10),
        2006 => (1, 31),
        2007 => (1, 20),
        2008 => (1, 10),
        2009 => (12, 18),
        2010 => (12, 7),
        2011 => (11, 26),
        2012 => (11, 15),
        2013 => (11, 4),
        2014 => (10, 25),
        2015 => (10, 14),
        2016 => (10, 2),
        2017 => (9, 21),
        2018 => (9, 11),
        2019 => (8, 31),
        2020 => (8, 20),
        2021 => (8, 9),
        2022 => (7, 30),
        2023 => (7, 19),
        2024 => (7, 7),
        2025 => (6, 26),
        2026 => (6, 16),
        2027 => (6, 6),
        2028 => (5, 25),
        _ => return None,
    };
    NaiveDate::from_ymd_opt(year, m, d)
}

fn ramadan(year: i32) -> Option<NaiveDate> {
    let (m, d) = match year {
        1950 => (6, 17),
        1951 => (6, 6),
        1952 => (5, 25),
        1953 => (5, 14),
        1954 => (5, 4),
        1955 => (4, 24),
        1956 => (4, 12),
        1957 => (4, 1),
        1958 => (3, 21),
        1959 => (3, 11),
        1960 => (2, 28),
        1961 => (2, 16),
        1962 => (2, 5),
        1963 => (1, 26),
        1964 => (1, 15),
        1965 => (1, 3),
        // 1965 also has Dec 23
        1966 => (12, 13),
        1967 => (12, 2),
        1968 => (11, 21),
        1969 => (11, 10),
        1970 => (11, 1),
        1971 => (10, 20),
        1972 => (10, 8),
        1973 => (9, 27),
        1974 => (9, 17),
        1975 => (9, 6),
        1976 => (8, 26),
        1977 => (8, 15),
        1978 => (8, 5),
        1979 => (7, 25),
        1980 => (7, 13),
        1981 => (7, 2),
        1982 => (6, 22),
        1983 => (6, 12),
        1984 => (5, 31),
        1985 => (5, 20),
        1986 => (5, 9),
        1987 => (4, 29),
        1988 => (4, 17),
        1989 => (4, 7),
        1990 => (3, 27),
        1991 => (3, 17),
        1992 => (3, 5),
        1993 => (2, 22),
        1994 => (2, 11),
        1995 => (1, 31),
        1996 => (1, 21),
        1997 => (1, 10),
        // 1997 also has Dec 30
        1998 => (12, 19),
        1999 => (12, 9),
        2000 => (11, 27),
        2001 => (11, 16),
        2002 => (11, 6),
        2003 => (10, 26),
        2004 => (10, 15),
        2005 => (10, 4),
        2006 => (9, 24),
        2007 => (9, 13),
        2008 => (9, 1),
        2009 => (8, 22),
        2010 => (8, 11),
        2011 => (8, 1),
        2012 => (7, 20),
        2013 => (7, 9),
        2014 => (6, 28),
        2015 => (6, 18),
        2016 => (6, 6),
        2017 => (5, 27),
        2018 => (5, 16),
        2019 => (5, 6),
        2020 => (4, 24),
        2021 => (4, 13),
        2022 => (4, 2),
        2023 => (3, 23),
        2024 => (3, 11),
        2025 => (3, 1),
        2026 => (2, 18),
        2027 => (2, 8),
        2028 => (1, 28),
        2029 => (1, 16),
        2030 => (1, 6),
        // 2030 also has Dec 26
        2031 => (12, 15),
        2032 => (12, 4),
        2033 => (11, 23),
        2034 => (11, 12),
        2035 => (11, 2),
        2036 => (10, 21),
        2037 => (10, 11),
        2038 => (9, 30),
        2039 => (9, 19),
        2040 => (9, 8),
        2041 => (8, 28),
        2042 => (8, 17),
        2043 => (8, 7),
        2044 => (7, 26),
        2045 => (7, 16),
        2046 => (7, 5),
        2047 => (6, 24),
        2048 => (6, 13),
        2049 => (6, 2),
        2050 => (5, 22),
        _ => return None,
    };
    NaiveDate::from_ymd_opt(year, m, d)
}

fn rajab(year: i32) -> Option<NaiveDate> {
    // Rajab start dates from Haskell rajab table
    let (m, d) = match year {
        1999 => (10, 10),
        2000 => (9, 28),
        2001 => (9, 18),
        2002 => (9, 8),
        2003 => (8, 29),
        2004 => (8, 17),
        2005 => (8, 6),
        2006 => (7, 26),
        2007 => (7, 15),
        2008 => (7, 4),
        2009 => (6, 24),
        2010 => (6, 13),
        2011 => (6, 3),
        2012 => (5, 22),
        2013 => (5, 11),
        2014 => (4, 30),
        2015 => (4, 20),
        2016 => (4, 8),
        2017 => (3, 29),
        2018 => (3, 18),
        2019 => (3, 8),
        2020 => (2, 25),
        2021 => (2, 13),
        2022 => (2, 2),
        2023 => (1, 23),
        2024 => (1, 13),
        2025 => (1, 1),
        // 2025 also has Dec 21
        2026 => (12, 10),
        2027 => (11, 29),
        2028 => (11, 18),
        _ => return None,
    };
    NaiveDate::from_ymd_opt(year, m, d)
}

fn isra_miraj(year: i32) -> Option<NaiveDate> {
    // Isra and Mi'raj = Rajab + 26 days
    rajab(year).and_then(|d| d.checked_add_signed(Duration::try_days(26)?))
}

#[allow(clippy::arithmetic_side_effects)]
fn resolve_hindu_holiday(name: &str, year: i32) -> Option<NaiveDate> {
    if name == "diwali" || name == "deepavali" {
        return diwali(year);
    }
    if name == "holi" || name == "dhulandi" || name == "phagwah" {
        return holi(year);
    }
    if name == "chhoti holi" || name == "holika dahan" || name == "kamudu pyre" {
        return holi(year).map(|d| d - Duration::days(1));
    }
    if name == "navaratri" || name == "durga puja" {
        return navaratri(year);
    }
    if name.starts_with("dussehra") || name.starts_with("vijayadashami") {
        return navaratri(year).map(|d| d + Duration::days(9));
    }
    if name == "dhanteras" || name == "dhanatrayodashi" {
        return dhanteras(year);
    }
    if name.starts_with("bhai dooj") {
        return diwali(year).map(|d| d + Duration::days(2));
    }
    if name.starts_with("maha shivaratri") {
        return maha_shivaratri(year);
    }
    if name.starts_with("ganesh chaturthi") {
        return ganesh_chaturthi(year);
    }
    if name.starts_with("krishna janmashtami") || name.starts_with("gokulashtami") {
        return janmashtami(year);
    }
    if name.starts_with("rama navami") {
        return rama_navami(year);
    }
    if name.starts_with("rakhi") || name.starts_with("raksha bandhan") {
        return raksha_bandhan(year);
    }
    if name.starts_with("pongal")
        || name.starts_with("makar")
        || name.starts_with("makara")
        || name.starts_with("maghi")
    {
        return thai_pongal(year);
    }
    if name.starts_with("vaisakhi")
        || name.starts_with("baisakhi")
        || name.starts_with("mesadi")
        || name.starts_with("vasakhi")
        || name.starts_with("vaishakhi")
        || name.starts_with("vaisakhadi")
    {
        return vaisakhi(year);
    }
    if name.starts_with("onam") || name.contains("onam") || name.starts_with("thiru") {
        return onam(year);
    }
    if name.starts_with("ugadi")
        || name.starts_with("yugadi")
        || name.starts_with("samvatsaradi")
        || name.starts_with("chaitra suk")
    {
        return ugadi(year);
    }
    if name.starts_with("karva chauth") {
        return karva_chauth(year);
    }
    if name.starts_with("ratha") {
        return rath_yatra(year);
    }
    if name.starts_with("chhath")
        || name.starts_with("dala puja")
        || name.starts_with("dala chhath")
        || name.starts_with("surya shashthi")
    {
        return chhath(year);
    }
    if name.starts_with("vasant panchami") || name.starts_with("basant panchami") {
        return vasant_panchami(year);
    }
    if name.starts_with("saraswati jayanti") {
        return saraswati_jayanti(year);
    }
    if name.starts_with("naraka")
        || name.starts_with("kali chaudas")
        || name.starts_with("roop chaudas")
        || name.starts_with("choti diwali")
    {
        // Kali Chaudas = Dhanteras + 1 (matching Haskell)
        return dhanteras(year).map(|d| d + Duration::days(1));
    }
    if name.starts_with("maha saptami") {
        return navaratri(year).map(|d| d + Duration::days(6));
    }
    if name.starts_with("mahavir jayanti")
        || name.starts_with("mahavir janma")
        || name.starts_with("mahaveer jayanti")
        || name.starts_with("mahaveer janma")
    {
        return mahavir_jayanti(year);
    }
    if name.starts_with("guru gobind") || name.starts_with("guru govind") {
        return guru_gobind_singh(year);
    }
    if name.starts_with("guru ravida") || name.starts_with("guru ravidas") {
        return guru_ravidas(year);
    }
    if name.starts_with("valmiki jayanti")
        || name.starts_with("maharishi valmiki")
        || name.starts_with("pargat diwas")
    {
        return valmiki_jayanti(year);
    }
    if name.starts_with("rabindra jayanti") || name.starts_with("rabindranath jayanti") {
        return rabindra_jayanti(year);
    }
    if name.starts_with("bogi pandigai") {
        return NaiveDate::from_ymd_opt(year, 1, 13);
    }
    if name.starts_with("maattu pongal") {
        return NaiveDate::from_ymd_opt(year, 1, 15);
    }
    if name.starts_with("kaanum pongal") || name.starts_with("kanni pongal") {
        return NaiveDate::from_ymd_opt(year, 1, 16);
    }
    if name.starts_with("lakshmi puja") {
        return diwali(year);
    }
    None
}

// Hindu holiday lookup tables
// Diwali = Dhanteras + 2 (matching Haskell: cycleNthAfter False TG.Day 2 dhanteras)
fn diwali(year: i32) -> Option<NaiveDate> {
    dhanteras(year).and_then(|d| d.checked_add_signed(Duration::try_days(2)?))
}

fn holi(year: i32) -> Option<NaiveDate> {
    let (m, d) = match year {
        2013 => (3, 27),
        2014 => (3, 17),
        2015 => (3, 6),
        2016 => (3, 24),
        2017 => (3, 13),
        2018 => (3, 2),
        2019 => (3, 21),
        2020 => (3, 10),
        2021 => (3, 29),
        2022 => (3, 18),
        2023 => (3, 8),
        _ => return None,
    };
    NaiveDate::from_ymd_opt(year, m, d)
}

fn navaratri(year: i32) -> Option<NaiveDate> {
    let (m, d) = match year {
        2000 => (9, 28),
        2001 => (10, 17),
        2002 => (10, 7),
        2003 => (9, 26),
        2004 => (10, 14),
        2005 => (10, 4),
        2006 => (9, 23),
        2007 => (10, 12),
        2008 => (9, 30),
        2009 => (9, 19),
        2010 => (10, 8),
        2011 => (9, 28),
        2012 => (10, 16),
        2013 => (10, 5),
        2014 => (9, 25),
        2015 => (10, 13),
        2016 => (10, 1),
        2017 => (9, 21),
        2018 => (10, 9),
        2019 => (9, 29),
        2020 => (10, 17),
        2021 => (10, 6),
        2022 => (9, 26),
        2023 => (10, 15),
        2024 => (10, 3),
        2025 => (9, 22),
        2026 => (10, 11),
        2027 => (9, 30),
        2028 => (9, 19),
        2029 => (10, 8),
        2030 => (9, 27),
        _ => return None,
    };
    NaiveDate::from_ymd_opt(year, m, d)
}

fn maha_shivaratri(year: i32) -> Option<NaiveDate> {
    let (m, d) = match year {
        2000 => (3, 4),
        2001 => (2, 21),
        2002 => (3, 12),
        2003 => (3, 1),
        2004 => (2, 18),
        2005 => (3, 8),
        2006 => (2, 26),
        2007 => (2, 16),
        2008 => (3, 6),
        2009 => (2, 23),
        2010 => (2, 12),
        2011 => (3, 2),
        2012 => (2, 20),
        2013 => (3, 10),
        2014 => (2, 27),
        2015 => (2, 17),
        2016 => (3, 7),
        2017 => (2, 24),
        2018 => (2, 13),
        2019 => (3, 4),
        2020 => (2, 21),
        2021 => (3, 11),
        2022 => (3, 1),
        2023 => (2, 18),
        2024 => (3, 8),
        2025 => (2, 26),
        2026 => (2, 15),
        2027 => (3, 6),
        2028 => (2, 23),
        2029 => (2, 11),
        2030 => (3, 2),
        _ => return None,
    };
    NaiveDate::from_ymd_opt(year, m, d)
}

fn ganesh_chaturthi(year: i32) -> Option<NaiveDate> {
    let (m, d) = match year {
        2000 => (9, 1),
        2001 => (8, 22),
        2002 => (9, 10),
        2003 => (8, 31),
        2004 => (9, 18),
        2005 => (9, 7),
        2006 => (8, 27),
        2007 => (9, 15),
        2008 => (9, 3),
        2009 => (8, 23),
        2010 => (9, 11),
        2011 => (9, 1),
        2012 => (9, 19),
        2013 => (9, 9),
        2014 => (8, 29),
        2015 => (9, 17),
        2016 => (9, 5),
        2017 => (8, 25),
        2018 => (9, 13),
        2019 => (9, 2),
        2020 => (8, 22),
        2021 => (9, 9),
        2022 => (8, 30),
        2023 => (9, 18),
        2024 => (9, 6),
        2025 => (8, 26),
        2026 => (9, 14),
        2027 => (9, 3),
        2028 => (8, 23),
        2029 => (9, 11),
        2030 => (9, 1),
        _ => return None,
    };
    NaiveDate::from_ymd_opt(year, m, d)
}

fn janmashtami(year: i32) -> Option<NaiveDate> {
    let (m, d) = match year {
        2000 => (8, 22),
        2001 => (8, 12),
        2002 => (8, 31),
        2003 => (8, 20),
        2004 => (9, 6),
        2005 => (8, 26),
        2006 => (8, 15),
        2007 => (9, 3),
        2008 => (8, 23),
        2009 => (8, 13),
        2010 => (9, 1),
        2011 => (8, 21),
        2012 => (8, 9),
        2013 => (8, 28),
        2014 => (8, 17),
        2015 => (9, 5),
        2016 => (8, 25),
        2017 => (8, 14),
        2018 => (9, 3),
        2019 => (8, 23),
        2020 => (8, 11),
        2021 => (8, 30),
        2022 => (8, 19),
        2023 => (9, 6),
        2024 => (8, 26),
        2025 => (8, 16),
        2026 => (9, 4),
        2027 => (8, 25),
        2028 => (8, 13),
        2029 => (9, 1),
        2030 => (8, 21),
        _ => return None,
    };
    NaiveDate::from_ymd_opt(year, m, d)
}

fn rama_navami(year: i32) -> Option<NaiveDate> {
    let (m, d) = match year {
        2000 => (4, 12),
        2001 => (4, 2),
        2002 => (4, 21),
        2003 => (4, 11),
        2004 => (3, 30),
        2005 => (4, 18),
        2006 => (4, 6),
        2007 => (3, 27),
        2008 => (4, 14),
        2009 => (4, 3),
        2010 => (3, 24),
        2011 => (4, 12),
        2012 => (4, 1),
        2013 => (4, 19),
        2014 => (4, 8),
        2015 => (3, 28),
        2016 => (4, 15),
        2017 => (4, 5),
        2018 => (3, 25),
        2019 => (4, 14),
        2020 => (4, 2),
        2021 => (4, 21),
        2022 => (4, 10),
        2023 => (3, 30),
        2024 => (4, 17),
        2025 => (4, 6),
        2026 => (3, 27),
        2027 => (4, 15),
        2028 => (4, 4),
        2029 => (4, 23),
        2030 => (4, 12),
        _ => return None,
    };
    NaiveDate::from_ymd_opt(year, m, d)
}

fn raksha_bandhan(year: i32) -> Option<NaiveDate> {
    let (m, d) = match year {
        2000 => (8, 15),
        2001 => (8, 4),
        2002 => (8, 22),
        2003 => (8, 12),
        2004 => (8, 29),
        2005 => (8, 19),
        2006 => (8, 9),
        2007 => (8, 28),
        2008 => (8, 16),
        2009 => (8, 5),
        2010 => (8, 24),
        2011 => (8, 13),
        2012 => (8, 2),
        2013 => (8, 20),
        2014 => (8, 10),
        2015 => (8, 29),
        2016 => (8, 18),
        2017 => (8, 7),
        2018 => (8, 26),
        2019 => (8, 15),
        2020 => (8, 3),
        2021 => (8, 22),
        2022 => (8, 11),
        2023 => (8, 30),
        2024 => (8, 19),
        2025 => (8, 9),
        2026 => (8, 28),
        2027 => (8, 17),
        2028 => (8, 5),
        2029 => (8, 23),
        2030 => (8, 13),
        _ => return None,
    };
    NaiveDate::from_ymd_opt(year, m, d)
}

fn onam(year: i32) -> Option<NaiveDate> {
    let (m, d) = match year {
        2000 => (9, 10),
        2001 => (8, 31),
        2002 => (8, 21),
        2003 => (9, 8),
        2004 => (8, 28),
        2005 => (9, 15),
        2006 => (9, 5),
        2007 => (8, 26),
        2008 => (9, 12),
        2009 => (9, 2),
        2010 => (8, 23),
        2011 => (9, 9),
        2012 => (8, 29),
        2013 => (8, 20),
        2014 => (9, 6),
        2015 => (8, 28),
        2016 => (9, 13),
        2017 => (9, 4),
        2018 => (8, 24),
        2019 => (9, 11),
        2020 => (8, 31),
        2021 => (8, 21),
        2022 => (9, 8),
        2023 => (8, 29),
        2024 => (9, 15),
        2025 => (9, 5),
        2026 => (8, 26),
        2027 => (9, 12),
        2028 => (9, 1),
        2029 => (8, 22),
        2030 => (9, 9),
        _ => return None,
    };
    NaiveDate::from_ymd_opt(year, m, d)
}

fn ugadi(year: i32) -> Option<NaiveDate> {
    let (m, d) = match year {
        2013 => (4, 11),
        2014 => (3, 31),
        2015 => (3, 21),
        2016 => (4, 8),
        2017 => (3, 28),
        2018 => (3, 18),
        2019 => (4, 6),
        2020 => (3, 25),
        _ => return None,
    };
    NaiveDate::from_ymd_opt(year, m, d)
}

fn karva_chauth(year: i32) -> Option<NaiveDate> {
    let (m, d) = match year {
        2000 => (10, 16),
        2001 => (10, 4),
        2002 => (10, 24),
        2003 => (10, 13),
        2004 => (10, 31),
        2005 => (10, 20),
        2006 => (10, 9),
        2007 => (10, 28),
        2008 => (10, 17),
        2009 => (10, 7),
        2010 => (10, 26),
        2011 => (10, 15),
        2012 => (10, 2),
        2013 => (10, 22),
        2014 => (10, 11),
        2015 => (10, 30),
        2016 => (10, 18),
        2017 => (10, 8),
        2018 => (10, 27),
        2019 => (10, 17),
        2020 => (10, 3),
        2021 => (10, 23),
        2022 => (10, 12),
        2023 => (10, 31),
        2024 => (10, 20),
        2025 => (10, 9),
        2026 => (10, 28),
        2027 => (10, 18),
        2028 => (10, 6),
        2029 => (10, 25),
        2030 => (10, 14),
        _ => return None,
    };
    NaiveDate::from_ymd_opt(year, m, d)
}

fn rath_yatra(year: i32) -> Option<NaiveDate> {
    let (m, d) = match year {
        2000 => (7, 2),
        2001 => (6, 22),
        2002 => (7, 11),
        2003 => (7, 1),
        2004 => (6, 19),
        2005 => (7, 8),
        2006 => (6, 27),
        2007 => (7, 16),
        2008 => (7, 4),
        2009 => (6, 24),
        2010 => (7, 13),
        2011 => (7, 3),
        2012 => (6, 21),
        2013 => (7, 10),
        2014 => (6, 29),
        2015 => (7, 18),
        2016 => (7, 6),
        2017 => (6, 25),
        2018 => (7, 14),
        2019 => (7, 4),
        2020 => (6, 23),
        2021 => (7, 11),
        2022 => (6, 30),
        2023 => (6, 19),
        2024 => (7, 7),
        2025 => (6, 26),
        2026 => (7, 15),
        2027 => (7, 5),
        2028 => (6, 24),
        2029 => (7, 13),
        2030 => (7, 2),
        _ => return None,
    };
    NaiveDate::from_ymd_opt(year, m, d)
}

fn vasant_panchami(year: i32) -> Option<NaiveDate> {
    let (m, d) = match year {
        2000 => (2, 10),
        2001 => (1, 29),
        2002 => (2, 17),
        2003 => (2, 6),
        2004 => (1, 26),
        2005 => (2, 13),
        2006 => (2, 2),
        2007 => (1, 23),
        2008 => (2, 11),
        2009 => (1, 31),
        2010 => (1, 20),
        2011 => (2, 8),
        2012 => (1, 28),
        2013 => (2, 15),
        2014 => (2, 4),
        2015 => (1, 24),
        2016 => (2, 12),
        2017 => (2, 1),
        2018 => (1, 22),
        2019 => (2, 10),
        2020 => (1, 29),
        2021 => (2, 16),
        2022 => (2, 5),
        2023 => (1, 26),
        2024 => (2, 14),
        2025 => (2, 2),
        2026 => (1, 23),
        2027 => (2, 11),
        2028 => (1, 31),
        2029 => (1, 19),
        2030 => (2, 7),
        _ => return None,
    };
    NaiveDate::from_ymd_opt(year, m, d)
}

fn saraswati_jayanti(year: i32) -> Option<NaiveDate> {
    let (m, d) = match year {
        2000 => (2, 29),
        2001 => (2, 17),
        2002 => (3, 8),
        2003 => (2, 26),
        2004 => (2, 15),
        2005 => (3, 5),
        2006 => (2, 23),
        2007 => (2, 12),
        2008 => (3, 2),
        2009 => (2, 19),
        2010 => (2, 8),
        2011 => (2, 27),
        2012 => (2, 16),
        2013 => (3, 7),
        2014 => (2, 24),
        2015 => (2, 14),
        2016 => (3, 4),
        2017 => (2, 21),
        2018 => (2, 10),
        2019 => (2, 28),
        2020 => (2, 18),
        2021 => (3, 8),
        2022 => (2, 26),
        2023 => (2, 15),
        2024 => (3, 5),
        2025 => (2, 23),
        2026 => (2, 12),
        2027 => (3, 2),
        2028 => (2, 19),
        2029 => (2, 8),
        2030 => (2, 27),
        _ => return None,
    };
    NaiveDate::from_ymd_opt(year, m, d)
}

fn mahavir_jayanti(year: i32) -> Option<NaiveDate> {
    let (m, d) = match year {
        2013 => (4, 24),
        2014 => (4, 13),
        2015 => (4, 2),
        2016 => (4, 20),
        2017 => (4, 9),
        2018 => (3, 29),
        2019 => (4, 17),
        2020 => (4, 6),
        _ => return None,
    };
    NaiveDate::from_ymd_opt(year, m, d)
}

fn guru_gobind_singh(year: i32) -> Option<NaiveDate> {
    let (m, d) = match year {
        2000 => (1, 14),
        2001 => (1, 2),
        2002 => (1, 21),
        2003 => (12, 29), // Actually in Dec of same year (following year's Nanakshahi)
        2004 => (1, 5),
        2005 => (1, 16),
        2006 => (12, 27),
        2007 => (1, 5),
        2008 => (1, 14),
        2009 => (12, 23),
        2010 => (1, 5),
        2011 => (1, 5),
        2012 => (1, 5),
        2013 => (1, 18),
        2014 => (1, 7),
        2015 => (1, 5),
        2016 => (1, 16),
        2017 => (12, 25),
        2018 => (1, 5),
        2019 => (1, 13),
        2020 => (2, 1),
        2021 => (1, 19),
        2022 => (1, 8),
        2023 => (1, 5),
        2024 => (1, 17),
        2025 => (1, 5),
        2026 => (1, 5),
        2027 => (1, 14),
        2028 => (1, 3),
        2029 => (1, 5),
        2030 => (1, 10),
        _ => return None,
    };
    NaiveDate::from_ymd_opt(year, m, d)
}

fn guru_ravidas(year: i32) -> Option<NaiveDate> {
    let (m, d) = match year {
        2000 => (2, 19),
        2001 => (2, 8),
        2002 => (2, 27),
        2003 => (2, 16),
        2004 => (2, 6),
        2005 => (2, 24),
        2006 => (2, 13),
        2007 => (2, 2),
        2008 => (2, 21),
        2009 => (2, 9),
        2010 => (1, 30),
        2011 => (2, 18),
        2012 => (2, 7),
        2013 => (2, 25),
        2014 => (2, 14),
        2015 => (2, 3),
        2016 => (2, 22),
        2017 => (2, 10),
        2018 => (1, 31),
        2019 => (2, 19),
        2020 => (2, 9),
        2021 => (2, 27),
        2022 => (2, 16),
        2023 => (2, 5),
        2024 => (2, 24),
        2025 => (2, 12),
        2026 => (2, 1),
        2027 => (2, 20),
        2028 => (2, 10),
        2029 => (1, 30),
        2030 => (2, 18),
        _ => return None,
    };
    NaiveDate::from_ymd_opt(year, m, d)
}

fn valmiki_jayanti(year: i32) -> Option<NaiveDate> {
    let (m, d) = match year {
        2000 => (10, 12),
        2001 => (10, 31),
        2002 => (10, 20),
        2003 => (10, 9),
        2004 => (10, 27),
        2005 => (10, 16),
        2006 => (10, 6),
        2007 => (10, 25),
        2008 => (10, 14),
        2009 => (10, 3),
        2010 => (10, 22),
        2011 => (10, 11),
        2012 => (10, 29),
        2013 => (10, 18),
        2014 => (10, 7),
        2015 => (10, 26),
        2016 => (10, 15),
        2017 => (10, 5),
        2018 => (10, 24),
        2019 => (10, 13),
        2020 => (10, 30),
        2021 => (10, 19),
        2022 => (10, 9),
        2023 => (10, 28),
        2024 => (10, 16),
        2025 => (10, 6),
        2026 => (10, 25),
        2027 => (10, 14),
        2028 => (10, 2),
        2029 => (10, 21),
        2030 => (10, 10),
        _ => return None,
    };
    NaiveDate::from_ymd_opt(year, m, d)
}

fn vaisakhi(year: i32) -> Option<NaiveDate> {
    let (m, d) = match year {
        2000 => (4, 13),
        2001 => (4, 13),
        2002 => (4, 14),
        2003 => (4, 14),
        2004 => (4, 13),
        2005 => (4, 13),
        2006 => (4, 14),
        2007 => (4, 14),
        2008 => (4, 13),
        2009 => (4, 14),
        2010 => (4, 14),
        2011 => (4, 14),
        2012 => (4, 13),
        2013 => (4, 14),
        2014 => (4, 14),
        2015 => (4, 14),
        2016 => (4, 13),
        2017 => (4, 14),
        2018 => (4, 14),
        2019 => (4, 14),
        2020 => (4, 13),
        2021 => (4, 14),
        2022 => (4, 14),
        2023 => (4, 14),
        2024 => (4, 13),
        2025 => (4, 14),
        2026 => (4, 14),
        2027 => (4, 14),
        2028 => (4, 13),
        2029 => (4, 14),
        2030 => (4, 14),
        _ => return None,
    };
    NaiveDate::from_ymd_opt(year, m, d)
}

fn parsi_new_year(year: i32) -> Option<NaiveDate> {
    // Parsi New Year: every 365 days from anchor Aug 16, 2020
    let anchor = NaiveDate::from_ymd_opt(2020, 8, 16).unwrap();
    // Calculate for the given year by finding offset from anchor
    // Try the date from a precomputed table for accuracy
    let (m, d) = match year {
        2010 => (8, 19),
        2011 => (8, 19),
        2012 => (8, 18),
        2013 => (8, 18),
        2014 => (8, 18),
        2015 => (8, 18),
        2016 => (8, 17),
        2017 => (8, 17),
        2018 => (8, 17),
        2019 => (8, 17),
        2020 => (8, 16),
        2021 => (8, 16),
        2022 => (8, 16),
        2023 => (8, 16),
        2024 => (8, 15),
        2025 => (8, 15),
        2026 => (8, 15),
        2027 => (8, 15),
        2028 => (8, 14),
        2029 => (8, 14),
        2030 => (8, 14),
        _ => {
            // Fallback: compute dynamically
            let diff_years = year.saturating_sub(2020);
            let approx = Duration::try_days(i64::from(diff_years).saturating_mul(365))
                .and_then(|dur| anchor.checked_add_signed(dur))
                .unwrap_or(anchor);
            // Find the closest date in the target year
            if approx.year() == year {
                return Some(approx);
            }
            return Some(approx);
        }
    };
    NaiveDate::from_ymd_opt(year, m, d)
}

fn thai_pongal(year: i32) -> Option<NaiveDate> {
    let (m, d) = match year {
        2000 => (1, 15),
        2001 => (1, 14),
        2002 => (1, 14),
        2003 => (1, 14),
        2004 => (1, 15),
        2005 => (1, 14),
        2006 => (1, 14),
        2007 => (1, 15),
        2008 => (1, 15),
        2009 => (1, 14),
        2010 => (1, 14),
        2011 => (1, 15),
        2012 => (1, 15),
        2013 => (1, 14),
        2014 => (1, 14),
        2015 => (1, 15),
        2016 => (1, 15),
        2017 => (1, 14),
        2018 => (1, 14),
        2019 => (1, 15),
        2020 => (1, 15),
        2021 => (1, 14),
        2022 => (1, 14),
        2023 => (1, 15),
        2024 => (1, 15),
        2025 => (1, 14),
        2026 => (1, 14),
        2027 => (1, 15),
        2028 => (1, 15),
        2029 => (1, 14),
        2030 => (1, 14),
        _ => return None,
    };
    NaiveDate::from_ymd_opt(year, m, d)
}

fn rabindra_jayanti(year: i32) -> Option<NaiveDate> {
    let (m, d) = match year {
        2000 => (5, 8),
        2001 => (5, 9),
        2002 => (5, 9),
        2003 => (5, 9),
        2004 => (5, 8),
        2005 => (5, 9),
        2006 => (5, 9),
        2007 => (5, 9),
        2008 => (5, 8),
        2009 => (5, 9),
        2010 => (5, 9),
        2011 => (5, 9),
        2012 => (5, 8),
        2013 => (5, 9),
        2014 => (5, 9),
        2015 => (5, 9),
        2016 => (5, 8),
        2017 => (5, 9),
        2018 => (5, 9),
        2019 => (5, 9),
        2020 => (5, 8),
        2021 => (5, 9),
        2022 => (5, 9),
        2023 => (5, 9),
        2024 => (5, 8),
        2025 => (5, 9),
        2026 => (5, 9),
        2027 => (5, 9),
        2028 => (5, 9),
        2029 => (5, 9),
        2030 => (5, 9),
        _ => return None,
    };
    NaiveDate::from_ymd_opt(year, m, d)
}

fn gysd_date(year: i32) -> Option<NaiveDate> {
    let (m, d) = match year {
        2013 => (4, 26),
        2014 => (4, 11),
        2015 => (4, 17),
        2016 => (4, 15),
        2017 => (4, 21),
        2018 => (4, 20),
        2019 => (4, 26),
        2020 => (4, 17),
        _ => return None,
    };
    NaiveDate::from_ymd_opt(year, m, d)
}

fn resolve_vesak(year: i32) -> Option<NaiveDate> {
    let (m, d) = match year {
        2000 => (5, 18),
        2001 => (5, 7),
        2002 => (5, 26),
        2003 => (5, 15),
        2004 => (5, 4),
        2005 => (5, 23),
        2006 => (5, 12),
        2007 => (5, 31),
        2008 => (5, 19),
        2009 => (5, 8),
        2010 => (5, 27),
        2011 => (5, 17),
        2012 => (5, 5),
        2013 => (5, 24),
        2014 => (5, 14),
        2015 => (5, 3),
        2016 => (5, 21),
        2017 => (5, 10),
        2018 => (5, 29),
        2019 => (5, 18),
        2020 => (5, 7),
        2021 => (5, 26),
        2022 => (5, 15),
        2023 => (5, 5),
        2024 => (5, 23),
        2025 => (5, 12),
        2026 => (5, 31),
        2027 => (5, 20),
        2028 => (5, 8),
        2029 => (5, 27),
        2030 => (5, 17),
        _ => return None,
    };
    NaiveDate::from_ymd_opt(year, m, d)
}

// Chhath = Dhanteras + 8 (matching Haskell: cycleNthAfter False TG.Day 8 dhanteras)
fn chhath(year: i32) -> Option<NaiveDate> {
    dhanteras(year).and_then(|d| d.checked_add_signed(Duration::try_days(8)?))
}

fn dhanteras(year: i32) -> Option<NaiveDate> {
    let (m, d) = match year {
        2000 => (10, 24),
        2001 => (11, 12),
        2002 => (11, 2),
        2003 => (10, 23),
        2004 => (11, 10),
        2005 => (10, 30),
        2006 => (10, 19),
        2007 => (11, 7),
        2008 => (10, 26),
        2009 => (10, 15),
        2010 => (11, 3),
        2011 => (10, 24),
        2012 => (11, 11),
        2013 => (11, 1),
        2014 => (10, 21),
        2015 => (11, 9),
        2016 => (10, 28),
        2017 => (10, 17),
        2018 => (11, 5),
        2019 => (10, 25),
        2020 => (11, 13),
        2021 => (11, 2),
        2022 => (10, 22),
        2023 => (11, 10),
        2024 => (10, 29),
        2025 => (10, 18),
        2026 => (11, 6),
        2027 => (10, 27),
        2028 => (10, 15),
        2029 => (11, 4),
        2030 => (10, 24),
        _ => return None,
    };
    NaiveDate::from_ymd_opt(year, m, d)
}
