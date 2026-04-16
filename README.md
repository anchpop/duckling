# Duckling (Rust)

A Rust port of Facebook's [Duckling](https://github.com/facebook/duckling) — a library for parsing natural language into structured data.

Given a string like `"tomorrow at 3pm"` or `"42 degrees fahrenheit"`, Duckling extracts "dimensions" such as URLs and times.

## Supported dimensions

Time, Numeral, Ordinal, Temperature, Distance, Volume, Quantity, AmountOfMoney, Duration, Email, PhoneNumber, Url, CreditCardNumber.

## Usage

```rust
use duckling::{parse, Entity, Locale, Lang, Context, Options, DimensionKind, DimensionValue,
               MeasurementValue, TimeValue, TimePoint, Grain};
use chrono::{FixedOffset, NaiveDate, TimeZone};

let locale = Locale::new(Lang::EN, None);
let context = Context::new(
    FixedOffset::east_opt(0).unwrap()
        .with_ymd_and_hms(2013, 2, 12, 4, 30, 0).unwrap(),
    locale,
);
let options = Options::default();

// Time — "tomorrow at 3pm" parses as a naive (wall-clock) time
let results = parse("tomorrow at 3pm", &locale, &[DimensionKind::Time], &context, &options);
if let DimensionValue::Time(TimeValue::Single { value: TimePoint::Naive { value, grain }, .. }) = &results[0].value {
    assert_eq!(*value, NaiveDate::from_ymd_opt(2013, 2, 13).unwrap().and_hms_opt(15, 0, 0).unwrap());
    assert_eq!(*grain, Grain::Hour);
} else { panic!("expected Naive time point"); }

// Temperature
let results = parse("80 degrees fahrenheit", &locale, &[DimensionKind::Temperature], &context, &options);
assert_eq!(results, vec![Entity {
    body: "80 degrees fahrenheit".into(),
    start: 0, end: 21, latent: Some(false),
    value: DimensionValue::Temperature(MeasurementValue::Value {
        value: 80.0, unit: "fahrenheit".into(),
    }),
}]);

// Numerals
let results = parse("forty-two", &locale, &[DimensionKind::Numeral], &context, &options);
assert_eq!(results, vec![Entity {
    body: "forty-two".into(),
    start: 0, end: 9, latent: Some(false),
    value: DimensionValue::Numeral(42.0),
}]);
```

## Time: instant vs naive

Time values distinguish between absolute instants and wall-clock/calendar times:

- **Naive** — `"5 pm"`, `"tomorrow"`, `"March 15th"` — wall-clock times with no timezone assumption
- **Instant** — `"now"`, `"in 2 hours"`, `"5 pm EST"` — pinned to a specific offset-aware moment

```text
// Naive: no timezone baked in
TimeValue::Single { value: TimePoint::Naive { value: NaiveDateTime, grain }, values: Vec<TimePoint> }

// Instant: absolute fixed-offset moment
TimeValue::Single { value: TimePoint::Instant { value: DateTime<FixedOffset>, grain }, values: Vec<TimePoint> }
```

An explicit timezone (e.g. `"3pm CET"`) promotes any naive time to an instant.

## Acknowledgements

This is a Rust rewrite of [facebook/duckling](https://github.com/facebook/duckling), originally written in Haskell.
