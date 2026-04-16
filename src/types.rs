use std::fmt;
use std::rc::Rc;

use crate::dimensions::amount_of_money::AmountOfMoneyData;
use crate::dimensions::credit_card_number::CreditCardNumberData;
use crate::dimensions::distance::DistanceData;
use crate::dimensions::duration::DurationData;
use crate::dimensions::email::EmailData;
use crate::dimensions::numeral::NumeralData;
use crate::dimensions::ordinal::OrdinalData;
use crate::dimensions::phone_number::PhoneNumberData;
use crate::dimensions::quantity::QuantityData;
use crate::dimensions::temperature::TemperatureData;
use crate::dimensions::time::TimeData;
use crate::dimensions::time_grain::Grain;
use crate::dimensions::url::UrlData;
use crate::dimensions::volume::VolumeData;
use chrono::{DateTime, FixedOffset, NaiveDateTime};

/// The kind of dimension to extract from text.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DimensionKind {
    /// Numbers: "forty-two", "3.5", "100K"
    Numeral,
    /// Ordinals: "first", "3rd", "22nd"
    Ordinal,
    /// Temperatures: "80 degrees fahrenheit", "3°C"
    Temperature,
    /// Distances: "5 miles", "3 kilometers"
    Distance,
    /// Volumes: "2 gallons", "500ml"
    Volume,
    /// Quantities with product: "5 pounds of sugar"
    Quantity,
    /// Money: "$42.50", "3 euros"
    AmountOfMoney,
    /// Email addresses: "user@example.com"
    Email,
    /// Phone numbers: "(555) 123-4567"
    PhoneNumber,
    /// URLs: `"https://example.com"`
    Url,
    /// Credit card numbers
    CreditCardNumber,
    /// Time grains: "day", "week", "month"
    TimeGrain,
    /// Durations: "3 days", "2 hours"
    Duration,
    /// Times and dates: "tomorrow at 3pm", "in 2 hours"
    Time,
}

impl fmt::Display for DimensionKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            DimensionKind::Numeral => "number",
            DimensionKind::Ordinal => "ordinal",
            DimensionKind::Temperature => "temperature",
            DimensionKind::Distance => "distance",
            DimensionKind::Volume => "volume",
            DimensionKind::Quantity => "quantity",
            DimensionKind::AmountOfMoney => "amount-of-money",
            DimensionKind::Email => "email",
            DimensionKind::PhoneNumber => "phone-number",
            DimensionKind::Url => "url",
            DimensionKind::CreditCardNumber => "credit-card-number",
            DimensionKind::TimeGrain => "time-grain",
            DimensionKind::Duration => "duration",
            DimensionKind::Time => "time",
        };
        write!(f, "{}", s)
    }
}

/// A measurement with a numeric value and unit. Used by Temperature, Distance,
/// Volume, Quantity, and AmountOfMoney dimensions.
///
/// ```
/// use duckling::{parse_en, DimensionKind, DimensionValue, MeasurementValue};
///
/// let results = parse_en("$42.50", &[DimensionKind::AmountOfMoney]);
/// assert_eq!(results[0].value, DimensionValue::AmountOfMoney(MeasurementValue::Value {
///     value: 42.5, unit: "USD".into(),
/// }));
/// ```
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub enum MeasurementValue {
    /// An exact measurement.
    Value {
        /// The numeric value.
        value: f64,
        /// The unit (e.g. "fahrenheit", "mile", "USD").
        unit: String,
    },
    /// A range of measurements (e.g. "between 3 and 5 dollars").
    Interval {
        /// The lower bound, if any.
        from: Option<MeasurementPoint>,
        /// The upper bound, if any.
        to: Option<MeasurementPoint>,
    },
}

/// A single endpoint in a [`MeasurementValue::Interval`].
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct MeasurementPoint {
    /// The numeric value.
    pub value: f64,
    /// The unit.
    pub unit: String,
}

/// A resolved time point — either an absolute fixed-offset instant or a naive wall-clock time.
///
/// ```
/// use duckling::{parse, Locale, Lang, Context, Options, DimensionKind,
///                DimensionValue, TimeValue, TimePoint, Grain};
/// use chrono::{FixedOffset, NaiveDate, TimeZone};
///
/// let locale = Locale::new(Lang::EN, None);
/// let context = Context::new(
///     FixedOffset::west_opt(2 * 3600).unwrap()
///         .with_ymd_and_hms(2013, 2, 12, 4, 30, 0).unwrap(),
///     locale,
/// );
/// let options = Options::default();
///
/// // Wall-clock times are Naive (no timezone baked in)
/// let results = parse("tomorrow at 3pm", &locale, &[DimensionKind::Time], &context, &options);
/// if let DimensionValue::Time(TimeValue::Single { value: TimePoint::Naive { value, grain }, .. }) = &results[0].value {
///     assert_eq!(*value, NaiveDate::from_ymd_opt(2013, 2, 13).unwrap().and_hms_opt(15, 0, 0).unwrap());
///     assert_eq!(*grain, Grain::Hour);
/// } else { panic!("expected Naive time point"); }
///
/// // Relative offsets from now are Instant (pinned to an absolute offset-aware moment)
/// let results = parse("in one hour", &locale, &[DimensionKind::Time], &context, &options);
/// if let DimensionValue::Time(TimeValue::Single { value: TimePoint::Instant { value, grain }, .. }) = &results[0].value {
///     let expected = FixedOffset::west_opt(2 * 3600).unwrap()
///         .with_ymd_and_hms(2013, 2, 12, 5, 30, 0).unwrap();
///     assert_eq!(*value, expected);
///     assert_eq!(*grain, Grain::Minute);
/// } else { panic!("expected Instant time point"); }
/// ```
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub enum TimePoint {
    /// An absolute fixed-offset moment (e.g. "now", "in 2 hours", "5pm EST").
    Instant {
        /// The offset-aware datetime.
        value: DateTime<FixedOffset>,
        /// The precision grain.
        grain: Grain,
    },
    /// A wall-clock/calendar time with no timezone assumption (e.g. "5pm", "tomorrow", "March 15th").
    Naive {
        /// The naive datetime.
        value: NaiveDateTime,
        /// The precision grain.
        grain: Grain,
    },
}

impl TimePoint {
    /// Returns the precision grain of this time point.
    pub fn grain(&self) -> Grain {
        match self {
            TimePoint::Instant { grain, .. } | TimePoint::Naive { grain, .. } => *grain,
        }
    }
}

/// A pair of interval endpoints, used in the `values` array for intervals.
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct IntervalEndpoints {
    /// The start of the interval, if bounded.
    pub from: Option<TimePoint>,
    /// The end of the interval, if bounded.
    pub to: Option<TimePoint>,
}

/// A resolved time value — either a single point or an interval.
///
/// Includes a `values` array of up to 3 next occurrences, matching Haskell's
/// `TimeValue SingleTimeValue [SingleTimeValue]`.
///
/// ```
/// use duckling::{parse, Locale, Lang, Context, Options, DimensionKind,
///                DimensionValue, TimeValue, TimePoint, Grain};
/// use chrono::{FixedOffset, NaiveDate, TimeZone};
///
/// let locale = Locale::new(Lang::EN, None);
/// let context = Context::new(
///     FixedOffset::west_opt(2 * 3600).unwrap()
///         .with_ymd_and_hms(2013, 2, 12, 4, 30, 0).unwrap(),
///     locale,
/// );
/// let options = Options::default();
///
/// // Single time point
/// let results = parse("tomorrow", &locale, &[DimensionKind::Time], &context, &options);
/// if let DimensionValue::Time(TimeValue::Single { value: TimePoint::Naive { value, grain }, .. }) = &results[0].value {
///     assert_eq!(*value, NaiveDate::from_ymd_opt(2013, 2, 13).unwrap().and_hms_opt(0, 0, 0).unwrap());
///     assert_eq!(*grain, Grain::Day);
/// } else { panic!("expected Naive time point"); }
///
/// // Time interval
/// let results = parse("from 3pm to 5pm", &locale, &[DimensionKind::Time], &context, &options);
/// if let DimensionValue::Time(TimeValue::Interval { from: Some(f), to: Some(t), .. }) = &results[0].value {
///     assert!(matches!(f, TimePoint::Naive { value, grain: Grain::Hour }
///         if *value == NaiveDate::from_ymd_opt(2013, 2, 12).unwrap().and_hms_opt(15, 0, 0).unwrap()));
///     assert!(matches!(t, TimePoint::Naive { value, grain: Grain::Hour }
///         if *value == NaiveDate::from_ymd_opt(2013, 2, 12).unwrap().and_hms_opt(18, 0, 0).unwrap()));
/// } else { panic!("expected Interval time value"); }
/// ```
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub enum TimeValue {
    /// A single time point with additional future occurrences.
    /// Matches Haskell's `TimeValue (SimpleValue v) [v1, v2, v3] holiday`.
    Single {
        /// The primary resolved time point.
        value: TimePoint,
        /// Up to 3 next occurrences (including the primary as first element).
        values: Vec<TimePoint>,
        /// Matches Haskell's `holiday :: Maybe Text` in TimeValue.
        /// Serialized as `"holidayBeta"` when present.
        #[serde(skip_serializing_if = "Option::is_none", rename = "holidayBeta")]
        holiday: Option<String>,
    },
    /// A time interval with additional future occurrences.
    Interval {
        /// The start of the interval, if bounded.
        from: Option<TimePoint>,
        /// The end of the interval, if bounded.
        to: Option<TimePoint>,
        /// Up to 3 next interval occurrences.
        values: Vec<IntervalEndpoints>,
        /// Matches Haskell's `holiday :: Maybe Text` in TimeValue.
        /// Serialized as `"holidayBeta"` when present.
        #[serde(skip_serializing_if = "Option::is_none", rename = "holidayBeta")]
        holiday: Option<String>,
    },
}

/// The resolved value of a parsed entity.
///
/// ```
/// use duckling::{parse_en, DimensionKind, DimensionValue, Grain};
///
/// assert_eq!(parse_en("thirty three", &[DimensionKind::Numeral])[0].value,
///     DimensionValue::Numeral(33.0));
///
/// assert_eq!(parse_en("the 3rd", &[DimensionKind::Ordinal])[0].value,
///     DimensionValue::Ordinal(3));
///
/// assert_eq!(parse_en("3 days", &[DimensionKind::Duration])[0].value,
///     DimensionValue::Duration { value: 3, grain: Grain::Day, normalized_seconds: 259200 });
///
/// assert_eq!(parse_en("user@example.com", &[DimensionKind::Email])[0].value,
///     DimensionValue::Email("user@example.com".into()));
/// ```
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub enum DimensionValue {
    /// A numeric value.
    Numeral(f64),
    /// An ordinal value.
    Ordinal(i64),
    /// A temperature measurement.
    Temperature(MeasurementValue),
    /// A distance measurement.
    Distance(MeasurementValue),
    /// A volume measurement.
    Volume(MeasurementValue),
    /// A quantity with an optional product name.
    Quantity {
        /// The measurement value.
        measurement: MeasurementValue,
        /// The product (e.g. "sugar" in "5 pounds of sugar").
        product: Option<String>,
    },
    /// An amount of money.
    AmountOfMoney(MeasurementValue),
    /// An email address.
    Email(String),
    /// A phone number.
    PhoneNumber(String),
    /// A URL.
    Url {
        /// The full URL.
        value: String,
        /// The domain.
        domain: String,
    },
    /// A credit card number.
    CreditCardNumber {
        /// The card number.
        value: String,
        /// The card issuer (e.g. "visa", "mastercard").
        issuer: String,
    },
    /// A time grain.
    TimeGrain(Grain),
    /// A duration.
    Duration {
        /// The count (e.g. 3 in "3 days").
        value: i64,
        /// The grain (e.g. Day in "3 days").
        grain: Grain,
        /// The duration normalized to seconds.
        normalized_seconds: i64,
    },
    /// A time or date.
    Time(TimeValue),
}

impl DimensionValue {
    /// Returns the [`DimensionKind`] for this value.
    pub fn dim_kind(&self) -> DimensionKind {
        match self {
            DimensionValue::Numeral(_) => DimensionKind::Numeral,
            DimensionValue::Ordinal(_) => DimensionKind::Ordinal,
            DimensionValue::Temperature(_) => DimensionKind::Temperature,
            DimensionValue::Distance(_) => DimensionKind::Distance,
            DimensionValue::Volume(_) => DimensionKind::Volume,
            DimensionValue::Quantity { .. } => DimensionKind::Quantity,
            DimensionValue::AmountOfMoney(_) => DimensionKind::AmountOfMoney,
            DimensionValue::Email(_) => DimensionKind::Email,
            DimensionValue::PhoneNumber(_) => DimensionKind::PhoneNumber,
            DimensionValue::Url { .. } => DimensionKind::Url,
            DimensionValue::CreditCardNumber { .. } => DimensionKind::CreditCardNumber,
            DimensionValue::TimeGrain(_) => DimensionKind::TimeGrain,
            DimensionValue::Duration { .. } => DimensionKind::Duration,
            DimensionValue::Time(_) => DimensionKind::Time,
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) enum TokenData {
    Numeral(NumeralData),
    Ordinal(OrdinalData),
    Temperature(TemperatureData),
    Distance(DistanceData),
    Volume(VolumeData),
    Quantity(QuantityData),
    AmountOfMoney(AmountOfMoneyData),
    Email(EmailData),
    PhoneNumber(PhoneNumberData),
    Url(UrlData),
    CreditCardNumber(CreditCardNumberData),
    TimeGrain(Grain),
    Duration(DurationData),
    Time(TimeData),
    RegexMatch(RegexMatchData),
}

impl TokenData {
    pub(crate) fn dimension_kind(&self) -> Option<DimensionKind> {
        match self {
            TokenData::Numeral(_) => Some(DimensionKind::Numeral),
            TokenData::Ordinal(_) => Some(DimensionKind::Ordinal),
            TokenData::Temperature(_) => Some(DimensionKind::Temperature),
            TokenData::Distance(_) => Some(DimensionKind::Distance),
            TokenData::Volume(_) => Some(DimensionKind::Volume),
            TokenData::Quantity(_) => Some(DimensionKind::Quantity),
            TokenData::AmountOfMoney(_) => Some(DimensionKind::AmountOfMoney),
            TokenData::Email(_) => Some(DimensionKind::Email),
            TokenData::PhoneNumber(_) => Some(DimensionKind::PhoneNumber),
            TokenData::Url(_) => Some(DimensionKind::Url),
            TokenData::CreditCardNumber(_) => Some(DimensionKind::CreditCardNumber),
            TokenData::TimeGrain(_) => Some(DimensionKind::TimeGrain),
            TokenData::Duration(_) => Some(DimensionKind::Duration),
            TokenData::Time(_) => Some(DimensionKind::Time),
            TokenData::RegexMatch(_) => None,
        }
    }

    pub(crate) fn is_latent(&self) -> bool {
        match self {
            TokenData::AmountOfMoney(data) => data.latent,
            TokenData::Time(data) => data.latent,
            _ => false,
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct RegexMatchData {
    pub(crate) groups: Vec<Option<String>>,
}

impl RegexMatchData {
    pub(crate) fn group(&self, idx: usize) -> Option<&str> {
        self.groups.get(idx).and_then(|g| g.as_deref())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct Range {
    pub(crate) start: usize,
    pub(crate) end: usize,
}

impl Range {
    pub(crate) fn new(start: usize, end: usize) -> Self {
        Range { start, end }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct Node {
    pub(crate) range: Range,
    pub(crate) token_data: TokenData,
    pub(crate) children: Vec<Rc<Node>>,
    pub(crate) rule_name: Option<String>,
}

impl Node {
    pub(crate) fn new(range: Range, token_data: TokenData) -> Self {
        Node {
            range,
            token_data,
            children: Vec::new(),
            rule_name: None,
        }
    }
}

pub(crate) type Predicate = Box<dyn Fn(&TokenData) -> bool + Send + Sync>;
pub(crate) type Production = Box<dyn Fn(&[&Node]) -> Option<TokenData> + Send + Sync>;

pub(crate) enum PatternItem {
    Regex(regex::Regex),
    Dimension(DimensionKind),
    Predicate(Predicate),
}

impl fmt::Debug for PatternItem {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PatternItem::Regex(r) => write!(f, "Regex({})", r.as_str()),
            PatternItem::Dimension(d) => write!(f, "Dimension({:?})", d),
            PatternItem::Predicate(_) => write!(f, "Predicate(...)"),
        }
    }
}

pub(crate) struct Rule {
    pub(crate) name: String,
    pub(crate) pattern: Vec<PatternItem>,
    pub(crate) production: Production,
}

impl fmt::Debug for Rule {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Rule")
            .field("name", &self.name)
            .field("pattern", &self.pattern)
            .finish()
    }
}

/// Internal resolved token pairing a parse Node with its resolved Entity.
/// Matches Haskell's `Resolved { range, node, rval, isLatent }`.
#[derive(Debug, Clone)]
pub(crate) struct ResolvedToken {
    pub(crate) node: Node,
    pub(crate) entity: Entity,
}

/// A parsed entity extracted from text, with its position, matched text, and resolved value.
///
/// ```
/// use duckling::{parse_en, Entity, DimensionKind, DimensionValue};
///
/// assert_eq!(parse_en("I need 42 widgets", &[DimensionKind::Numeral]), vec![Entity {
///     body: "42".into(), start: 7, end: 9, latent: Some(false),
///     value: DimensionValue::Numeral(42.0),
/// }]);
/// ```
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct Entity {
    /// The matched text.
    pub body: String,
    /// Byte offset of the match start.
    pub start: usize,
    /// Byte offset of the match end.
    pub end: usize,
    /// The resolved structured value.
    pub value: DimensionValue,
    /// Whether this is a latent (ambiguous) match.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub latent: Option<bool>,
}
