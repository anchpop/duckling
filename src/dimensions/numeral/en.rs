use crate::pattern::{dim, predicate, regex};
use crate::types::{DimensionKind, Rule, TokenData};

use super::helpers::*;
use super::NumeralData;

/// Predicate: value >= 0 (matches Haskell's isPositive).
fn is_positive(td: &TokenData) -> bool {
    matches!(td, TokenData::Numeral(d) if d.value >= 0.0)
}

/// Predicate: has a grain set.
fn has_grain(td: &TokenData) -> bool {
    matches!(td, TokenData::Numeral(d) if d.grain.is_some())
}

pub fn rules() -> Vec<Rule> {
    vec![
        // === Integer words (0..9) ===
        Rule {
            name: "integer (0..9)".to_string(),
            pattern: vec![regex(
                r#"(zero|naught|nought|nil|none|zilch|one|single|two|three|four|five|six|seven|eight|nine)"#,
            )],
            production: Box::new(|nodes| {
                let text = match &nodes[0].token_data {
                    TokenData::RegexMatch(m) => m.group(1)?,
                    _ => return None,
                };
                let low = text.to_lowercase();
                let val = match low.as_str() {
                    "zero" | "naught" | "nought" | "nil" | "none" | "zilch" => 0.0,
                    "one" | "single" => 1.0,
                    "two" => 2.0,
                    "three" => 3.0,
                    "four" => 4.0,
                    "five" => 5.0,
                    "six" => 6.0,
                    "seven" => 7.0,
                    "eight" => 8.0,
                    "nine" => 9.0,
                    _ => return None,
                };
                let mut data = NumeralData::new(val);
                if low == "single" {
                    data = data.not_ok_for_any_time();
                }
                Some(TokenData::Numeral(data))
            }),
        },
        // === Integer words (10..19) — longer alternatives first ===
        Rule {
            name: "integer (10..19)".to_string(),
            pattern: vec![regex(
                r#"(thirteen|fourteen|fifteen|sixteen|seventeen|eighteen|nineteen|ten|eleven|twelve)"#,
            )],
            production: Box::new(|nodes| {
                let text = match &nodes[0].token_data {
                    TokenData::RegexMatch(m) => m.group(1)?,
                    _ => return None,
                };
                let val = match text.to_lowercase().as_str() {
                    "ten" => 10.0,
                    "eleven" => 11.0,
                    "twelve" => 12.0,
                    "thirteen" => 13.0,
                    "fourteen" => 14.0,
                    "fifteen" => 15.0,
                    "sixteen" => 16.0,
                    "seventeen" => 17.0,
                    "eighteen" => 18.0,
                    "nineteen" => 19.0,
                    _ => return None,
                };
                Some(TokenData::Numeral(NumeralData::new(val)))
            }),
        },
        // === Integer words (20..90 tens) ===
        Rule {
            name: "integer (20..90)".to_string(),
            pattern: vec![regex(
                r#"(twenty|thirty|fou?rty|fifty|sixty|seventy|eighty|ninety)"#,
            )],
            production: Box::new(|nodes| {
                let text = match &nodes[0].token_data {
                    TokenData::RegexMatch(m) => m.group(1)?,
                    _ => return None,
                };
                let val = match text.to_lowercase().as_str() {
                    "twenty" => 20.0,
                    "thirty" => 30.0,
                    "forty" | "fourty" => 40.0,
                    "fifty" => 50.0,
                    "sixty" => 60.0,
                    "seventy" => 70.0,
                    "eighty" => 80.0,
                    "ninety" => 90.0,
                    _ => return None,
                };
                Some(TokenData::Numeral(NumeralData::new(val)))
            }),
        },
        // a few / few → 3
        Rule {
            name: "a few".to_string(),
            pattern: vec![regex(r#"(a )?few"#)],
            production: Box::new(|_| {
                Some(TokenData::Numeral(
                    NumeralData::new(3.0).not_ok_for_any_time(),
                ))
            }),
        },
        // Compose tens and units: twenty one, thirty-two
        Rule {
            name: "integer (21..99)".to_string(),
            pattern: vec![
                predicate(|t| {
                    if let TokenData::Numeral(d) = t {
                        let v = d.value;
                        (20.0..=90.0).contains(&v) && v % 10.0 == 0.0
                    } else {
                        false
                    }
                }),
                regex(r#"[\s\-]+"#),
                predicate(|t| {
                    if let TokenData::Numeral(d) = t {
                        d.value >= 1.0 && d.value <= 9.0
                    } else {
                        false
                    }
                }),
            ],
            production: Box::new(|nodes| {
                let tens = numeral_data(&nodes[0].token_data)?.value;
                let units = numeral_data(&nodes[2].token_data)?.value;
                Some(TokenData::Numeral(NumeralData::new(tens + units)))
            }),
        },
        // Compose tens and units without separator: "twenty one" (space)
        Rule {
            name: "integer compose (tens + units)".to_string(),
            pattern: vec![
                predicate(|t| {
                    if let TokenData::Numeral(d) = t {
                        let v = d.value;
                        (20.0..=90.0).contains(&v) && v % 10.0 == 0.0
                    } else {
                        false
                    }
                }),
                predicate(|t| {
                    if let TokenData::Numeral(d) = t {
                        d.value >= 1.0 && d.value <= 9.0
                    } else {
                        false
                    }
                }),
            ],
            production: Box::new(|nodes| {
                let tens = numeral_data(&nodes[0].token_data)?.value;
                let units = numeral_data(&nodes[1].token_data)?.value;
                Some(TokenData::Numeral(NumeralData::new(tens + units)))
            }),
        },
        // Skip hundreds 1: "one eleven" → 111, "two twenty" → 220
        Rule {
            name: "one eleven (skip hundreds)".to_string(),
            pattern: vec![
                regex(r"(one|two|three|four|five|six|seven|eight|nine)"),
                regex(
                    r"(ten|eleven|twelve|thirteen|fourteen|fifteen|sixteen|seventeen|eighteen|nineteen|twenty|thirty|fou?rty|fifty|sixty|seventy|eighty|ninety)",
                ),
            ],
            production: Box::new(|nodes| {
                let m1 = match &nodes[0].token_data {
                    TokenData::RegexMatch(m) => m.group(1)?,
                    _ => return None,
                };
                let m2 = match &nodes[1].token_data {
                    TokenData::RegexMatch(m) => m.group(1)?,
                    _ => return None,
                };
                let hundreds = word_to_number(m1)?;
                let rest = word_to_number(m2)?;
                Some(TokenData::Numeral(NumeralData::new(
                    hundreds * 100.0 + rest,
                )))
            }),
        },
        // Skip hundreds 2: "one twenty two" → 122
        Rule {
            name: "one twenty two (skip hundreds)".to_string(),
            pattern: vec![
                regex(r"(one|two|three|four|five|six|seven|eight|nine)"),
                regex(r"(twenty|thirty|fou?rty|fifty|sixty|seventy|eighty|ninety)"),
                regex(r"(one|two|three|four|five|six|seven|eight|nine)"),
            ],
            production: Box::new(|nodes| {
                let m1 = match &nodes[0].token_data {
                    TokenData::RegexMatch(m) => m.group(1)?,
                    _ => return None,
                };
                let m2 = match &nodes[1].token_data {
                    TokenData::RegexMatch(m) => m.group(1)?,
                    _ => return None,
                };
                let m3 = match &nodes[2].token_data {
                    TokenData::RegexMatch(m) => m.group(1)?,
                    _ => return None,
                };
                let hundreds = word_to_number(m1)?;
                let tens = word_to_number(m2)?;
                let units = word_to_number(m3)?;
                Some(TokenData::Numeral(NumeralData::new(
                    hundreds * 100.0 + tens + units,
                )))
            }),
        },
        // Numeric digits
        Rule {
            name: "integer (numeric)".to_string(),
            pattern: vec![regex(r#"(\d{1,18})"#)],
            production: Box::new(|nodes| {
                let text = match &nodes[0].token_data {
                    TokenData::RegexMatch(m) => m.group(1)?,
                    _ => return None,
                };
                let val: f64 = text.parse().ok()?;
                Some(TokenData::Numeral(NumeralData::new(val)))
            }),
        },
        // Decimal numbers: 1.5, 3.14, .5
        Rule {
            name: "decimal number".to_string(),
            pattern: vec![regex(r#"(\d*\.\d+)"#)],
            production: Box::new(|nodes| {
                let text = match &nodes[0].token_data {
                    TokenData::RegexMatch(m) => m.group(1)?,
                    _ => return None,
                };
                let val: f64 = text.parse().ok()?;
                Some(TokenData::Numeral(NumeralData::new(val)))
            }),
        },
        // Fractional numbers: 1/2, 3/4
        Rule {
            name: "fractional number".to_string(),
            pattern: vec![regex(r#"(\d+)/(\d+)"#)],
            production: Box::new(|nodes| {
                let m = match &nodes[0].token_data {
                    TokenData::RegexMatch(m) => m,
                    _ => return None,
                };
                let num: f64 = m.group(1)?.parse().ok()?;
                let den: f64 = m.group(2)?.parse().ok()?;
                if den == 0.0 {
                    return None;
                }
                Some(TokenData::Numeral(NumeralData::new(num / den)))
            }),
        },
        // Comma-separated numbers (with optional decimal): 1,000 or 1,000,000.5
        Rule {
            name: "number with commas".to_string(),
            pattern: vec![regex(r#"(\d{1,3}(?:,\d{3})+(?:\.\d+)?)"#)],
            production: Box::new(|nodes| {
                let text = match &nodes[0].token_data {
                    TokenData::RegexMatch(m) => m.group(1)?,
                    _ => return None,
                };
                let cleaned = text.replace(',', "");
                let val: f64 = cleaned.parse().ok()?;
                Some(TokenData::Numeral(NumeralData::new(val)))
            }),
        },
        // Number suffixes: 100K, 1.2M, .0012G
        Rule {
            name: "number suffixes (K, M, G)".to_string(),
            pattern: vec![dim(DimensionKind::Numeral), regex(r"(k|m|g|b)\b")],
            production: Box::new(|nodes| {
                let v = numeral_data(&nodes[0].token_data)?.value;
                let s = match &nodes[1].token_data {
                    TokenData::RegexMatch(m) => m.group(1)?.to_lowercase(),
                    _ => return None,
                };
                let mult = match s.as_str() {
                    "k" => 1_000.0,
                    "m" => 1_000_000.0,
                    "g" | "b" => 1_000_000_000.0,
                    _ => return None,
                };
                Some(TokenData::Numeral(NumeralData::new(v * mult)))
            }),
        },
        // Negative numbers
        Rule {
            name: "negative number".to_string(),
            pattern: vec![regex(r#"(-|minus|negative)\s?"#), predicate(is_positive)],
            production: Box::new(|nodes| {
                let data = numeral_data(&nodes[1].token_data)?;
                Some(TokenData::Numeral(NumeralData::new(-data.value)))
            }),
        },
        // === Powers of ten (unified): hundred, thousand, lakh, million, crore, billion, trillion ===
        Rule {
            name: "powers of tens".to_string(),
            pattern: vec![regex(
                r#"(hundred|thousand|l(ac|(a?kh)?)|million|((k|c)r(ore)?|koti)|billion|trillion)s?"#,
            )],
            production: Box::new(|nodes| {
                let text = match &nodes[0].token_data {
                    TokenData::RegexMatch(m) => m.group(1)?,
                    _ => return None,
                };
                let grain = match text.to_lowercase().as_str() {
                    "hundred" => 2,
                    "thousand" => 3,
                    "lakh" | "lkh" | "l" | "lac" => 5,
                    "million" => 6,
                    "cr" | "crore" | "krore" | "kr" | "koti" => 7,
                    "billion" => 9,
                    "trillion" => 12,
                    _ => return None,
                };
                let value = 10.0_f64.powi(grain);
                Some(TokenData::Numeral(
                    NumeralData::new(value)
                        .with_grain(grain as u8)
                        .with_multipliable(true),
                ))
            }),
        },
        // a pair / a couple
        Rule {
            name: "a pair / a couple".to_string(),
            pattern: vec![regex(r#"(a\s+)?(pair|couple)s?(\s+of)?"#)],
            production: Box::new(|_nodes| {
                Some(TokenData::Numeral(
                    NumeralData::new(2.0).not_ok_for_any_time(),
                ))
            }),
        },
        // a dozen (multipliable)
        Rule {
            name: "a dozen of".to_string(),
            pattern: vec![regex(r#"(a )?dozens?( of)?"#)],
            production: Box::new(|_nodes| {
                Some(TokenData::Numeral(
                    NumeralData::new(12.0)
                        .with_multipliable(true)
                        .not_ok_for_any_time(),
                ))
            }),
        },
        // "one point 2" / "three point five" (spelled out decimal)
        Rule {
            name: "one point 2".to_string(),
            pattern: vec![
                dim(DimensionKind::Numeral),
                regex(r"point|dot"),
                predicate(|td| matches!(td, TokenData::Numeral(d) if d.grain.is_none())),
            ],
            production: Box::new(|nodes| {
                let n1 = numeral_data(&nodes[0].token_data)?.value;
                let n2 = numeral_data(&nodes[2].token_data)?.value;
                let decimal = decimals_to_double(n2);
                Some(TokenData::Numeral(NumeralData::new(n1 + decimal)))
            }),
        },
        // "point 77" (leading dot)
        Rule {
            name: "point 77".to_string(),
            pattern: vec![
                regex(r"point|dot"),
                predicate(|td| matches!(td, TokenData::Numeral(d) if d.grain.is_none())),
            ],
            production: Box::new(|nodes| {
                let n = numeral_data(&nodes[1].token_data)?.value;
                Some(TokenData::Numeral(NumeralData::new(decimals_to_double(n))))
            }),
        },
        // === Compose by multiplication: "five hundred", "thirty lakh", "200 dozens" ===
        Rule {
            name: "compose by multiplication".to_string(),
            pattern: vec![predicate(is_positive), predicate(is_multipliable)],
            production: Box::new(|nodes| {
                let n1 = numeral_data(&nodes[0].token_data)?;
                let n2 = numeral_data(&nodes[1].token_data)?;
                let v1 = n1.value;
                let v2 = n2.value;
                match n2.grain {
                    None => {
                        // No grain (e.g., dozen): simple multiply
                        Some(TokenData::Numeral(NumeralData::new(v1 * v2)))
                    }
                    Some(grain) => {
                        if v2 > v1 {
                            Some(TokenData::Numeral(
                                NumeralData::new(v1 * v2).with_grain(grain),
                            ))
                        } else {
                            None
                        }
                    }
                }
            }),
        },
        // === Intersect 2 numbers (sum): "five hundred four", "twenty-one thousand eleven" ===
        Rule {
            name: "intersect 2 numbers".to_string(),
            pattern: vec![
                predicate(|td| has_grain(td) && is_positive(td)),
                predicate(|td| !is_multipliable(td) && is_positive(td)),
            ],
            production: Box::new(|nodes| {
                let n1 = numeral_data(&nodes[0].token_data)?;
                let n2 = numeral_data(&nodes[1].token_data)?;
                let grain = n1.grain? as i32;
                let grain_val = 10.0_f64.powi(grain);
                if grain_val > n2.value {
                    Some(TokenData::Numeral(NumeralData::new(n1.value + n2.value)))
                } else {
                    None
                }
            }),
        },
        // === Intersect 2 numbers with "and": "five hundred and four" ===
        Rule {
            name: "intersect 2 numbers (with and)".to_string(),
            pattern: vec![
                predicate(|td| has_grain(td) && is_positive(td)),
                regex(r"and"),
                predicate(|td| !is_multipliable(td) && is_positive(td)),
            ],
            production: Box::new(|nodes| {
                let n1 = numeral_data(&nodes[0].token_data)?;
                let n2 = numeral_data(&nodes[2].token_data)?;
                let grain = n1.grain? as i32;
                let grain_val = 10.0_f64.powi(grain);
                if grain_val > n2.value {
                    Some(TokenData::Numeral(NumeralData::new(n1.value + n2.value)))
                } else {
                    None
                }
            }),
        },
        // === Legal parentheses: "forty-five (45)", "45 (forty five)" ===
        Rule {
            name: "<integer> '('<integer>')'".to_string(),
            pattern: vec![
                predicate(is_natural),
                regex(r"\("),
                predicate(is_natural),
                regex(r"\)"),
            ],
            production: Box::new(|nodes| {
                let n1 = numeral_data(&nodes[0].token_data)?.value;
                let n2 = numeral_data(&nodes[2].token_data)?.value;
                if (n1 - n2).abs() < 0.001 {
                    Some(TokenData::Numeral(NumeralData::new(n1)))
                } else {
                    None
                }
            }),
        },
    ]
}

fn is_common_rule_name(name: &str) -> bool {
    matches!(name, "integer (numeric)" | "fractional number")
}

pub fn common_rules() -> Vec<Rule> {
    rules()
        .into_iter()
        .filter(|r| is_common_rule_name(&r.name))
        .collect()
}

pub fn lang_rules() -> Vec<Rule> {
    rules()
        .into_iter()
        .filter(|r| !is_common_rule_name(&r.name))
        .collect()
}

/// Helper to convert number words to their values.
fn word_to_number(word: &str) -> Option<f64> {
    match word.to_lowercase().as_str() {
        "one" => Some(1.0),
        "two" => Some(2.0),
        "three" => Some(3.0),
        "four" => Some(4.0),
        "five" => Some(5.0),
        "six" => Some(6.0),
        "seven" => Some(7.0),
        "eight" => Some(8.0),
        "nine" => Some(9.0),
        "ten" => Some(10.0),
        "eleven" => Some(11.0),
        "twelve" => Some(12.0),
        "thirteen" => Some(13.0),
        "fourteen" => Some(14.0),
        "fifteen" => Some(15.0),
        "sixteen" => Some(16.0),
        "seventeen" => Some(17.0),
        "eighteen" => Some(18.0),
        "nineteen" => Some(19.0),
        "twenty" => Some(20.0),
        "thirty" => Some(30.0),
        "forty" | "fourty" => Some(40.0),
        "fifty" => Some(50.0),
        "sixty" => Some(60.0),
        "seventy" => Some(70.0),
        "eighty" => Some(80.0),
        "ninety" => Some(90.0),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine;
    use crate::resolve::{Context, Options};
    use crate::testing::{check_corpus, Corpus};
    use crate::types::{DimensionKind, DimensionValue};
    use chrono::Utc;

    fn build_rules() -> Vec<Rule> {
        rules()
    }

    #[test]
    fn test_numeral_corpus() {
        let context = Context::new(Utc::now().fixed_offset(), crate::locale::Locale::default());
        let mut corpus = Corpus::new(context);

        corpus.add(
            vec!["zero", "Zero", "ZERO"],
            |e| matches!(&e.value, DimensionValue::Numeral(v) if *v == 0.0),
        );

        corpus.add(
            vec!["one", "One"],
            |e| matches!(&e.value, DimensionValue::Numeral(v) if *v == 1.0),
        );

        corpus.add(
            vec!["fifteen", "Fifteen"],
            |e| matches!(&e.value, DimensionValue::Numeral(v) if *v == 15.0),
        );

        let rules = build_rules();
        let failures = check_corpus(&corpus, &rules, &[DimensionKind::Numeral]);
        assert!(failures.is_empty(), "Failures: {:?}", failures);
    }

    #[test]
    fn test_thirty_three() {
        let rules = build_rules();
        let options = Options { with_latent: false };
        let context = Context::default();
        let entities = engine::parse_and_resolve(
            "thirty three",
            &rules,
            &context,
            &options,
            &[DimensionKind::Numeral],
        );
        let found = entities
            .iter()
            .any(|e| matches!(&e.value, DimensionValue::Numeral(v) if (*v - 33.0).abs() < 0.01));
        assert!(found, "Expected to find 33, got: {:?}", entities);
    }

    #[test]
    fn test_numeric_integers() {
        let rules = build_rules();
        let options = Options { with_latent: false };
        let context = Context::default();

        let entities =
            engine::parse_and_resolve("42", &rules, &context, &options, &[DimensionKind::Numeral]);
        let found = entities
            .iter()
            .any(|e| matches!(&e.value, DimensionValue::Numeral(v) if (*v - 42.0).abs() < 0.01));
        assert!(found, "Expected 42, got: {:?}", entities);
    }

    #[test]
    fn test_100k() {
        let rules = build_rules();
        let options = Options { with_latent: false };
        let context = Context::default();

        let entities = engine::parse_and_resolve(
            "100K",
            &rules,
            &context,
            &options,
            &[DimensionKind::Numeral],
        );
        let found = entities.iter().any(
            |e| matches!(&e.value, DimensionValue::Numeral(v) if (*v - 100_000.0).abs() < 0.01),
        );
        assert!(found, "Expected 100000, got: {:?}", entities);
    }

    #[test]
    fn test_five_hundred() {
        let rules = build_rules();
        let options = Options { with_latent: false };
        let context = Context::default();

        let entities = engine::parse_and_resolve(
            "five hundred",
            &rules,
            &context,
            &options,
            &[DimensionKind::Numeral],
        );
        let found = entities
            .iter()
            .any(|e| matches!(&e.value, DimensionValue::Numeral(v) if (*v - 500.0).abs() < 0.01));
        assert!(found, "Expected 500, got: {:?}", entities);
    }
}
