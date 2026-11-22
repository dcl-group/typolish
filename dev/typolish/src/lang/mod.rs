//! Language data: the format (`data.rs`), the built-in tables and the registry (design section
//! 7).

pub(crate) mod common;
pub(crate) mod data;
pub(crate) mod en_us;
pub(crate) mod ru_ru;

use data::LanguageData;

/// The built-in languages in tag order, `COMMON` excluded; a bare language subtag resolves to
/// the entry whose tag starts with it (design section 2.2).
pub(crate) const BUILTIN: [&LanguageData; 2] = [&en_us::EN_US, &ru_ru::RU_RU];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn built_in_tables_pass_validation() {
        for data in BUILTIN.iter().copied().chain([&common::COMMON]) {
            assert_eq!(data.validate(), Ok(()), "{}", data.tag);
        }
        assert_eq!(LanguageData::new("xx").validate(), Ok(()));
    }

    #[test]
    fn validation_names_the_field() {
        let bad = LanguageData::EN_US.with_short_words(&["the", "a"]);
        assert!(bad.validate().unwrap_err().starts_with("short_words:"));
        let bad = LanguageData::EN_US.with_tag("EN-gb");
        assert!(bad.validate().unwrap_err().starts_with("tag:"));
        let bad = LanguageData::RU_RU.with_ambiguous_units(&[]);
        assert!(bad.validate().unwrap_err().starts_with("ambiguous_units:"));
        let bad =
            LanguageData::RU_RU.with_quotes(Some(data::QuotePairs::new(('「', '」'), ('«', '»'))));
        assert!(bad.validate().unwrap_err().starts_with("quotes:"));
        for tag in ["ru", "ru-RU", "es-419", "und"] {
            assert_eq!(LanguageData::new(tag).validate(), Ok(()), "{tag}");
        }
        for tag in [
            "", "r", "RU", "ru-ru", "ru_RU", "ru-RUS", "es-41", "UND", "rus",
        ] {
            assert!(LanguageData::new(tag).validate().is_err(), "{tag}");
        }
    }
}
