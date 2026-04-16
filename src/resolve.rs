use crate::dimensions;
use crate::locale::Locale;
use crate::types::{DimensionValue, Entity, Node, TokenData};
use chrono::{DateTime, FixedOffset, Utc};
#[cfg(not(debug_assertions))]
use std::panic::{catch_unwind, AssertUnwindSafe};

/// Context for resolving parsed tokens into structured values.
#[derive(Debug, Clone)]
pub struct Context {
    /// The reference time for resolving relative expressions like "tomorrow" or "in 2 hours".
    pub reference_time: DateTime<FixedOffset>,
    /// The locale used during resolution.
    pub locale: Locale,
}

impl Context {
    /// Build a context from a fixed-offset reference time.
    ///
    /// ```
    /// use duckling::{parse, Context, Locale, Lang, Options, DimensionKind, DimensionValue,
    ///                TimeValue, TimePoint, Grain};
    /// use chrono::{FixedOffset, NaiveDate, TimeZone};
    ///
    /// let locale = Locale::new(Lang::EN, None);
    /// let context = Context::new(
    ///     FixedOffset::east_opt(0).unwrap()
    ///         .with_ymd_and_hms(2013, 2, 12, 4, 30, 0).unwrap(),
    ///     locale,
    /// );
    ///
    /// let results = parse("tomorrow", &locale, &[DimensionKind::Time], &context, &Options::default());
    /// if let DimensionValue::Time(TimeValue::Single { value: TimePoint::Naive { value, grain }, .. }) = &results[0].value {
    ///     assert_eq!(*value, NaiveDate::from_ymd_opt(2013, 2, 13).unwrap().and_hms_opt(0, 0, 0).unwrap());
    ///     assert_eq!(*grain, Grain::Day);
    /// } else { panic!("expected Naive time point"); }
    /// ```
    pub fn new(reference_time: DateTime<FixedOffset>, locale: Locale) -> Self {
        Self {
            reference_time,
            locale,
        }
    }

    /// Return the fixed offset carried by the reference time.
    pub fn timezone(&self) -> FixedOffset {
        *self.reference_time.offset()
    }
}

impl Default for Context {
    fn default() -> Self {
        Self::new(Utc::now().fixed_offset(), Locale::default())
    }
}

/// Options for controlling parsing behavior.
///
/// ```
/// use duckling::{parse, Context, Locale, Lang, Options, DimensionKind, DimensionValue};
/// use chrono::{FixedOffset, TimeZone};
///
/// let locale = Locale::new(Lang::EN, None);
/// let context = Context::new(
///     FixedOffset::east_opt(0).unwrap()
///         .with_ymd_and_hms(2013, 2, 12, 4, 30, 0).unwrap(),
///     locale,
/// );
///
/// // "morning" is a latent time — it only matches when with_latent is true.
/// let opts = Options { with_latent: true };
/// let results = parse("morning", &locale, &[DimensionKind::Time], &context, &opts);
/// assert!(results.iter().any(|e| matches!(&e.value, DimensionValue::Time(_))
///     && e.latent == Some(true)));
///
/// let results = parse("morning", &locale, &[DimensionKind::Time], &context, &Options::default());
/// assert!(results.is_empty());
/// ```
#[derive(Debug, Clone, Default)]
pub struct Options {
    /// Whether to include latent (ambiguous) matches in results.
    pub with_latent: bool,
}

/// Resolve a node into a structured entity.
pub fn resolve(
    node: &Node,
    context: &Context,
    options: &Options,
    text: &str,
    budget: &mut dimensions::time::series::Budget,
) -> Option<Entity> {
    let body = text[node.range.start..node.range.end].to_string();
    let latent = node.token_data.is_latent();
    let resolved = resolve_token(&node.token_data, context, options, budget)?;

    Some(Entity {
        body,
        start: node.range.start,
        end: node.range.end,
        value: resolved,
        latent: Some(latent),
    })
}

fn resolve_token(
    token: &TokenData,
    context: &Context,
    options: &Options,
    budget: &mut dimensions::time::series::Budget,
) -> Option<DimensionValue> {
    #[cfg(debug_assertions)]
    {
        match token {
            TokenData::Numeral(data) => Some(dimensions::numeral::resolve(data)),
            TokenData::Ordinal(data) => Some(dimensions::ordinal::resolve(data)),
            TokenData::Temperature(data) => dimensions::temperature::resolve(data),
            TokenData::Distance(data) => dimensions::distance::resolve(data),
            TokenData::Volume(data) => dimensions::volume::resolve(data),
            TokenData::Quantity(data) => dimensions::quantity::resolve(data),
            TokenData::AmountOfMoney(data) => {
                dimensions::amount_of_money::resolve(data, options.with_latent)
            }
            TokenData::Email(data) => Some(dimensions::email::resolve(data)),
            TokenData::PhoneNumber(data) => Some(dimensions::phone_number::resolve(data)),
            TokenData::Url(data) => Some(dimensions::url::resolve(data)),
            TokenData::CreditCardNumber(data) => {
                Some(dimensions::credit_card_number::resolve(data))
            }
            TokenData::TimeGrain(grain) => Some(dimensions::time_grain::resolve(grain)),
            TokenData::Duration(data) => Some(dimensions::duration::resolve(data)),
            TokenData::Time(data) => {
                dimensions::time::resolve(data, context, options.with_latent, budget)
            }
            TokenData::RegexMatch(_) => None,
        }
    }

    #[cfg(not(debug_assertions))]
    {
        catch_unwind(AssertUnwindSafe(|| match token {
            TokenData::Numeral(data) => Some(dimensions::numeral::resolve(data)),
            TokenData::Ordinal(data) => Some(dimensions::ordinal::resolve(data)),
            TokenData::Temperature(data) => dimensions::temperature::resolve(data),
            TokenData::Distance(data) => dimensions::distance::resolve(data),
            TokenData::Volume(data) => dimensions::volume::resolve(data),
            TokenData::Quantity(data) => dimensions::quantity::resolve(data),
            TokenData::AmountOfMoney(data) => {
                dimensions::amount_of_money::resolve(data, options.with_latent)
            }
            TokenData::Email(data) => Some(dimensions::email::resolve(data)),
            TokenData::PhoneNumber(data) => Some(dimensions::phone_number::resolve(data)),
            TokenData::Url(data) => Some(dimensions::url::resolve(data)),
            TokenData::CreditCardNumber(data) => {
                Some(dimensions::credit_card_number::resolve(data))
            }
            TokenData::TimeGrain(grain) => Some(dimensions::time_grain::resolve(grain)),
            TokenData::Duration(data) => Some(dimensions::duration::resolve(data)),
            TokenData::Time(data) => {
                dimensions::time::resolve(data, context, options.with_latent, budget)
            }
            TokenData::RegexMatch(_) => None,
        }))
        .ok()
        .flatten()
    }
}
