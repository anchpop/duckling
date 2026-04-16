#![doc = include_str!("../README.md")]
#![warn(missing_docs)]
#![warn(clippy::arithmetic_side_effects)]

#[cfg(not(debug_assertions))]
use std::any::Any;
#[cfg(not(debug_assertions))]
use std::panic::{catch_unwind, AssertUnwindSafe};

pub(crate) mod dimensions;
pub(crate) mod document;
pub(crate) mod engine;
pub(crate) mod lang;
pub(crate) mod locale;
pub(crate) mod pattern;
pub(crate) mod ranking;
pub(crate) mod resolve;
pub(crate) mod stash;
#[cfg(test)]
pub(crate) mod testing;
pub(crate) mod types;

/// Corpus examples for training classifiers.
#[cfg(feature = "train")]
pub mod corpus;

// Re-exports for convenience
pub use dimensions::time_grain::Grain;
pub use locale::{Lang, Locale, Region};
pub use resolve::{Context, Options};
pub use types::{
    DimensionKind, DimensionValue, Entity, IntervalEndpoints, MeasurementPoint, MeasurementValue,
    TimePoint, TimeValue,
};

#[cfg(feature = "train")]
pub use ranking::train::TrainingCorpus;
#[cfg(feature = "train")]
pub use ranking::Classifiers;

/// Train classifiers for a locale from a corpus.
/// This wraps the internal training pipeline, hiding the `Rule` type.
#[cfg(feature = "train")]
pub fn train_classifiers(
    locale: &Locale,
    corpus: &ranking::train::TrainingCorpus,
    dims: &[DimensionKind],
) -> Classifiers {
    let rules = lang::rules_for(*locale, dims);
    ranking::train::make_classifiers(rules, corpus, dims)
}

/// Parse natural language text and return structured entities.
///
/// # Arguments
/// * `text` - The input text to parse
/// * `locale` - The locale (language + optional region)
/// * `dims` - Which dimensions to extract (empty = all)
/// * `context` - Reference time and locale context
/// * `options` - Parsing options (e.g., whether to include latent matches)
///
/// # Example
/// ```
/// use duckling::{parse, Locale, Lang, Context, Options, DimensionKind};
///
/// let context = Context::default();
/// let options = Options::default();
/// let locale = Locale::new(Lang::EN, None);
///
/// let entities = parse("I need 3 degrees celsius", &locale, &[DimensionKind::Temperature], &context, &options);
/// assert!(!entities.is_empty());
/// ```
pub fn parse(
    text: &str,
    locale: &Locale,
    dims: &[DimensionKind],
    context: &Context,
    options: &Options,
) -> Vec<Entity> {
    #[cfg(debug_assertions)]
    {
        parse_inner(text, locale, dims, context, options)
    }

    #[cfg(not(debug_assertions))]
    {
        match catch_unwind(AssertUnwindSafe(|| {
            parse_inner(text, locale, dims, context, options)
        })) {
            Ok(entities) => entities,
            Err(payload) => {
                log::error!(
                    "duckling::parse panicked: {}",
                    panic_payload_message(&payload)
                );
                Vec::new()
            }
        }
    }
}

fn parse_inner(
    text: &str,
    locale: &Locale,
    dims: &[DimensionKind],
    context: &Context,
    options: &Options,
) -> Vec<Entity> {
    use dimensions::time::series::{Budget, DEFAULT_WORK_BUDGET};
    use types::ResolvedToken;

    // Bound cumulative time-resolution work across this parse so one input
    // cannot blow up from hundreds of compose-heavy candidates (see series.rs).
    // The budget is threaded explicitly through resolve rather than stored in
    // thread-local state.
    let mut budget = Budget::new(DEFAULT_WORK_BUDGET);

    let rules = lang::rules_for(*locale, dims);
    let stash = engine::parse_string(text, rules);

    // Resolve all nodes first, then rank — matching Haskell's
    // parseAndResolve → rank pipeline from Api.hs/Engine.hs.
    let resolved_tokens: Vec<ResolvedToken> = stash
        .all_nodes()
        .filter(|node| {
            node.token_data
                .dimension_kind()
                .map(|dk| dims.is_empty() || dims.contains(&dk))
                .unwrap_or(false)
        })
        .filter_map(|node| {
            let entity = resolve::resolve(node, context, options, text, &mut budget)?;
            Some(ResolvedToken {
                node: node.clone(),
                entity,
            })
        })
        .collect();

    let ranked = ranking::rank_resolved(resolved_tokens, locale, dims);
    let entities: Vec<Entity> = ranked.into_iter().map(|rt| rt.entity).collect();
    ranking::remove_overlapping(entities)
}

#[cfg(not(debug_assertions))]
fn panic_payload_message(payload: &Box<dyn Any + Send>) -> String {
    if let Some(message) = payload.downcast_ref::<&str>() {
        return (*message).to_string();
    }
    if let Some(message) = payload.downcast_ref::<String>() {
        return message.clone();
    }
    "non-string panic payload".to_string()
}

/// Convenience function to parse text with default settings for English.
///
/// ```
/// use duckling::{parse_en, Entity, DimensionKind, DimensionValue};
///
/// assert_eq!(parse_en("forty-two", &[DimensionKind::Numeral]), vec![Entity {
///     body: "forty-two".into(), start: 0, end: 9, latent: Some(false),
///     value: DimensionValue::Numeral(42.0),
/// }]);
/// ```
pub fn parse_en(text: &str, dims: &[DimensionKind]) -> Vec<Entity> {
    let locale = Locale::new(Lang::EN, None);
    let context = Context::default();
    let options = Options::default();
    parse(text, &locale, dims, &context, &options)
}

#[cfg(test)]
mod integration_tests {
    use super::*;

    #[test]
    fn test_parse_numeral() {
        let entities = parse_en("thirty three", &[DimensionKind::Numeral]);
        let found = entities
            .iter()
            .any(|e| matches!(&e.value, DimensionValue::Numeral(v) if (*v - 33.0).abs() < 0.01));
        assert!(found, "Expected 33, got: {:?}", entities);
    }

    #[test]
    fn test_parse_100k() {
        let entities = parse_en("100K", &[DimensionKind::Numeral]);
        let found = entities.iter().any(
            |e| matches!(&e.value, DimensionValue::Numeral(v) if (*v - 100_000.0).abs() < 0.01),
        );
        assert!(found, "Expected 100000, got: {:?}", entities);
    }

    #[test]
    fn test_parse_temperature() {
        let entities = parse_en("80 degrees fahrenheit", &[DimensionKind::Temperature]);
        let found = entities.iter().any(|e| {
            matches!(&e.value, DimensionValue::Temperature(MeasurementValue::Value { value, unit })
                if (*value - 80.0).abs() < 0.01 && unit == "fahrenheit")
        });
        assert!(found, "Expected 80F, got: {:?}", entities);
    }

    #[test]
    fn test_parse_email() {
        let entities = parse_en("user@example.com", &[DimensionKind::Email]);
        let found = entities
            .iter()
            .any(|e| matches!(&e.value, DimensionValue::Email(v) if v == "user@example.com"));
        assert!(found, "Expected email, got: {:?}", entities);
    }

    #[test]
    fn test_parse_mixed_numeral_and_temperature() {
        let entities = parse_en(
            "it's 3 degrees outside",
            &[DimensionKind::Numeral, DimensionKind::Temperature],
        );

        let has_numeral = entities
            .iter()
            .any(|e| matches!(&e.value, DimensionValue::Numeral(v) if (*v - 3.0).abs() < 0.01));

        let has_temp = entities.iter().any(|e| {
            matches!(&e.value, DimensionValue::Temperature(MeasurementValue::Value { value, .. })
                if (*value - 3.0).abs() < 0.01)
        });

        assert!(
            has_numeral || has_temp,
            "Expected numeral(3) and/or temperature(3), got: {:?}",
            entities
        );
    }

    #[test]
    fn test_parse_url() {
        let entities = parse_en("visit https://www.example.com/path", &[DimensionKind::Url]);
        let found = entities
            .iter()
            .any(|e| matches!(&e.value, DimensionValue::Url { .. }));
        assert!(found, "Expected URL, got: {:?}", entities);
    }

    #[test]
    fn test_parse_money() {
        let entities = parse_en("$42.50", &[DimensionKind::AmountOfMoney]);
        let found = entities.iter().any(|e| {
            matches!(&e.value, DimensionValue::AmountOfMoney(MeasurementValue::Value { value, unit })
                if (*value - 42.5).abs() < 0.01 && unit == "USD")
        });
        assert!(found, "Expected $42.50, got: {:?}", entities);
    }

    #[test]
    fn test_parse_ordinal() {
        let entities = parse_en("the 3rd", &[DimensionKind::Ordinal]);
        let found = entities
            .iter()
            .any(|e| matches!(&e.value, DimensionValue::Ordinal(3)));
        assert!(found, "Expected 3rd, got: {:?}", entities);
    }

    #[test]
    fn test_parse_duration() {
        let entities = parse_en("3 days", &[DimensionKind::Duration]);
        let found = entities.iter().any(|e| {
            matches!(
                &e.value,
                DimensionValue::Duration {
                    value: 3,
                    grain: Grain::Day,
                    ..
                }
            )
        });
        assert!(found, "Expected 3 days, got: {:?}", entities);
    }

    #[test]
    fn test_parse_time_today() {
        let entities = parse_en("today", &[DimensionKind::Time]);
        let found = entities
            .iter()
            .any(|e| matches!(&e.value, DimensionValue::Time(_)));
        assert!(found, "Expected time for 'today', got: {:?}", entities);
    }

    #[test]
    fn test_parse_distance() {
        let entities = parse_en("5 miles", &[DimensionKind::Distance]);
        let found = entities.iter().any(|e| {
            matches!(&e.value, DimensionValue::Distance(MeasurementValue::Value { value, unit })
                if (*value - 5.0).abs() < 0.01 && unit == "mile")
        });
        assert!(found, "Expected 5 miles, got: {:?}", entities);
    }

    #[test]
    fn test_parse_volume() {
        let entities = parse_en("2 gallons", &[DimensionKind::Volume]);
        let found = entities.iter().any(|e| {
            matches!(&e.value, DimensionValue::Volume(MeasurementValue::Value { value, unit })
                if (*value - 2.0).abs() < 0.01 && unit == "gallon")
        });
        assert!(found, "Expected 2 gallons, got: {:?}", entities);
    }

    #[test]
    fn test_parse_quantity() {
        let entities = parse_en("5 pounds", &[DimensionKind::Quantity]);
        let found = entities.iter().any(|e| {
            matches!(&e.value, DimensionValue::Quantity { measurement: MeasurementValue::Value { value, .. }, .. }
                if (*value - 5.0).abs() < 0.01)
        });
        assert!(found, "Expected 5 pounds, got: {:?}", entities);
    }

    #[test]
    fn test_all_dimensions_at_once() {
        // Should handle parsing with all dimensions enabled
        let entities = parse_en("tomorrow at 3pm for $50", &[]);
        assert!(!entities.is_empty(), "Expected some entities, got none");
    }

    #[test]
    fn test_entity_non_latent_flag_is_set() {
        let entities = parse_en("forty-two", &[DimensionKind::Numeral]);
        let found = entities.iter().any(|e| {
            matches!(&e.value, DimensionValue::Numeral(v) if (*v - 42.0).abs() < 0.01)
                && e.latent == Some(false)
        });
        assert!(
            found,
            "Expected numeral entity with latent=Some(false), got: {:?}",
            entities
        );
    }

    #[test]
    fn test_entity_latent_flag_is_set_when_enabled() {
        let locale = Locale::new(Lang::EN, None);
        let context = Context::default();
        let options = Options { with_latent: true };
        let entities = parse(
            "morning",
            &locale,
            &[DimensionKind::Time],
            &context,
            &options,
        );
        let found = entities
            .iter()
            .any(|e| matches!(&e.value, DimensionValue::Time(_)) && e.latent == Some(true));
        assert!(
            found,
            "Expected latent time entity with latent=Some(true), got: {:?}",
            entities
        );
    }

    #[test]
    fn test_parse_money_grand() {
        let entities = parse_en("a grand", &[DimensionKind::AmountOfMoney]);
        let found = entities.iter().any(|e| {
            matches!(
                &e.value,
                DimensionValue::AmountOfMoney(MeasurementValue::Value { value, unit })
                    if (*value - 1000.0).abs() < 0.01 && unit == "USD"
            )
        });
        assert!(
            found,
            "Expected amount-of-money for 'a grand', got: {:?}",
            entities
        );
    }

    #[test]
    fn test_parse_money_symbol_non_en_common_rule() {
        let locale = Locale::new(Lang::ES, None);
        let context = Context {
            locale,
            ..Context::default()
        };
        let entities = parse(
            "$10",
            &locale,
            &[DimensionKind::AmountOfMoney],
            &context,
            &Options::default(),
        );
        let found = entities.iter().any(|e| {
            matches!(
                &e.value,
                DimensionValue::AmountOfMoney(MeasurementValue::Value { value, .. })
                    if (*value - 10.0).abs() < 0.01
            )
        });
        assert!(
            found,
            "Expected '$10' in non-EN locale, got: {:?}",
            entities
        );
    }

    #[test]
    fn test_parse_time_dmy_slash_stays_naive() {
        use chrono::{FixedOffset, TimeZone};
        let locale = Locale::new(Lang::EN, Some(Region::GB));
        let context = Context::new(
            FixedOffset::west_opt(2 * 3600)
                .unwrap()
                .with_ymd_and_hms(2013, 2, 12, 4, 30, 0)
                .unwrap(),
            locale,
        );
        let options = Options::default();
        let entities = parse("15/2", &locale, &[DimensionKind::Time], &context, &options);
        let found = entities.iter().any(|e| {
            matches!(
                &e.value,
                DimensionValue::Time(TimeValue::Single { value: TimePoint::Naive { value, .. }, .. })
                    if value.date() == chrono::NaiveDate::from_ymd_opt(2013, 2, 15).unwrap()
            )
        });
        assert!(
            found,
            "Expected naive time entity for '15/2', got: {:?}",
            entities
        );
    }

    #[test]
    fn test_parse_time_mdy_space_stays_naive() {
        use chrono::{FixedOffset, TimeZone};
        let locale = Locale::new(Lang::EN, Some(Region::US));
        let context = Context::new(
            FixedOffset::west_opt(2 * 3600)
                .unwrap()
                .with_ymd_and_hms(2013, 2, 12, 4, 30, 0)
                .unwrap(),
            locale,
        );
        let options = Options::default();
        let entities = parse(
            "10 31 1974",
            &locale,
            &[DimensionKind::Time],
            &context,
            &options,
        );
        let found = entities.iter().any(|e| {
            matches!(
                &e.value,
                DimensionValue::Time(TimeValue::Single { value: TimePoint::Naive { value, .. }, .. })
                    if value.date() == chrono::NaiveDate::from_ymd_opt(1974, 10, 31).unwrap()
            )
        });
        assert!(
            found,
            "Expected naive time entity for '10 31 1974', got: {:?}",
            entities
        );
    }
}
