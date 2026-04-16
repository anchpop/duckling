/// Supported languages.
#[allow(missing_docs)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Lang {
    AF,
    AR,
    BG,
    BN,
    CA,
    CS,
    DA,
    DE,
    EL,
    /// English
    EN,
    ES,
    ET,
    FA,
    FI,
    FR,
    GA,
    HE,
    HI,
    HR,
    HU,
    ID,
    IS,
    IT,
    JA,
    KA,
    KM,
    KN,
    KO,
    LO,
    ML,
    MN,
    MY,
    NB,
    NE,
    NL,
    PL,
    PT,
    RO,
    RU,
    SK,
    SV,
    SW,
    TA,
    TE,
    TH,
    TR,
    UK,
    VI,
    ZH,
}

impl Lang {
    /// Returns the ISO 639-1 language code.
    pub fn code(&self) -> &'static str {
        match self {
            Lang::AF => "af",
            Lang::AR => "ar",
            Lang::BG => "bg",
            Lang::BN => "bn",
            Lang::CA => "ca",
            Lang::CS => "cs",
            Lang::DA => "da",
            Lang::DE => "de",
            Lang::EL => "el",
            Lang::EN => "en",
            Lang::ES => "es",
            Lang::ET => "et",
            Lang::FA => "fa",
            Lang::FI => "fi",
            Lang::FR => "fr",
            Lang::GA => "ga",
            Lang::HE => "he",
            Lang::HI => "hi",
            Lang::HR => "hr",
            Lang::HU => "hu",
            Lang::ID => "id",
            Lang::IS => "is",
            Lang::IT => "it",
            Lang::JA => "ja",
            Lang::KA => "ka",
            Lang::KM => "km",
            Lang::KN => "kn",
            Lang::KO => "ko",
            Lang::LO => "lo",
            Lang::ML => "ml",
            Lang::MN => "mn",
            Lang::MY => "my",
            Lang::NB => "nb",
            Lang::NE => "ne",
            Lang::NL => "nl",
            Lang::PL => "pl",
            Lang::PT => "pt",
            Lang::RO => "ro",
            Lang::RU => "ru",
            Lang::SK => "sk",
            Lang::SV => "sv",
            Lang::SW => "sw",
            Lang::TA => "ta",
            Lang::TE => "te",
            Lang::TH => "th",
            Lang::TR => "tr",
            Lang::UK => "uk",
            Lang::VI => "vi",
            Lang::ZH => "zh",
        }
    }
}

/// Supported regions for locale-specific behavior.
#[allow(missing_docs)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Region {
    AR,
    /// United States
    US,
    /// United Kingdom
    GB,
    /// Australia
    AU,
    BE,
    BZ,
    /// Canada
    CA,
    CL,
    CN,
    CO,
    EG,
    ES,
    HK,
    IE,
    /// India
    IN,
    JM,
    MO,
    MX,
    NZ,
    PE,
    PH,
    TT,
    TW,
    VE,
    ZA,
}

impl Region {
    /// Returns the ISO 3166-1 alpha-2 region code.
    pub fn code(&self) -> &'static str {
        match self {
            Region::AR => "AR",
            Region::US => "US",
            Region::GB => "GB",
            Region::AU => "AU",
            Region::BE => "BE",
            Region::BZ => "BZ",
            Region::CA => "CA",
            Region::CL => "CL",
            Region::CN => "CN",
            Region::CO => "CO",
            Region::EG => "EG",
            Region::ES => "ES",
            Region::HK => "HK",
            Region::IE => "IE",
            Region::IN => "IN",
            Region::JM => "JM",
            Region::MO => "MO",
            Region::MX => "MX",
            Region::NZ => "NZ",
            Region::PE => "PE",
            Region::PH => "PH",
            Region::TT => "TT",
            Region::TW => "TW",
            Region::VE => "VE",
            Region::ZA => "ZA",
        }
    }
}

/// A locale combining a language and optional region.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Locale {
    /// The language.
    pub lang: Lang,
    /// Optional region for locale-specific rules.
    pub region: Option<Region>,
}

impl Locale {
    /// Create a new locale. Unsupported language/region pairs are normalized to `region = None`.
    ///
    /// ```
    /// use duckling::{Locale, Lang, Region};
    ///
    /// let en_gb = Locale::new(Lang::EN, Some(Region::GB));
    /// assert_eq!(en_gb.lang, Lang::EN);
    /// assert_eq!(en_gb.region, Some(Region::GB));
    ///
    /// // Region is dropped when the language has no region-specific rules for it.
    /// let ar_us = Locale::new(Lang::AR, Some(Region::US));
    /// assert_eq!(ar_us.region, None);
    /// ```
    pub fn new(lang: Lang, region: Option<Region>) -> Self {
        Locale {
            lang,
            region: normalize_region(lang, region),
        }
    }
}

impl Default for Locale {
    fn default() -> Self {
        Locale {
            lang: Lang::EN,
            region: Some(Region::US),
        }
    }
}

fn normalize_region(lang: Lang, region: Option<Region>) -> Option<Region> {
    match (lang, region) {
        (_, None) => None,
        (Lang::AR, Some(Region::EG)) => Some(Region::EG),
        (
            Lang::EN,
            Some(
                r @ (Region::AU
                | Region::BZ
                | Region::CA
                | Region::GB
                | Region::IN
                | Region::IE
                | Region::JM
                | Region::NZ
                | Region::PH
                | Region::TT
                | Region::US
                | Region::ZA),
            ),
        ) => Some(r),
        (
            Lang::ES,
            Some(
                r @ (Region::AR
                | Region::CL
                | Region::CO
                | Region::ES
                | Region::MX
                | Region::PE
                | Region::VE),
            ),
        ) => Some(r),
        (Lang::NL, Some(r @ Region::BE)) => Some(r),
        (Lang::ZH, Some(r @ (Region::CN | Region::HK | Region::MO | Region::TW))) => Some(r),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn locale_normalizes_unsupported_region_to_none() {
        let locale = Locale::new(Lang::AR, Some(Region::US));
        assert_eq!(locale.region, None);
    }

    #[test]
    fn locale_keeps_supported_region() {
        let locale = Locale::new(Lang::EN, Some(Region::GB));
        assert_eq!(locale.region, Some(Region::GB));
    }
}
