//! Language data: the format (`data.rs`), the built-in tables and the registry (design section
//! 7).

pub(crate) mod common;
pub(crate) mod data;
pub(crate) mod en_us;
pub(crate) mod ru_ru;

use data::LanguageData;

/// The built-in languages in tag order, `COMMON` excluded. A tag resolves here only by exact
/// match; a bare language subtag resolves through `DEFAULT_REGIONS` of `language.rs` (design
/// section 2.2).
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

    #[test]
    fn the_fast_folding_is_that_of_the_standard_library() {
        // `fold_char` skips the tables for ASCII and basic Cyrillic; the signatures of word lists
        // rely on folding being idempotent (design section 7.1).
        for c in (0..=0x10_ffff).filter_map(char::from_u32) {
            let folded = c.to_lowercase().next().unwrap_or(c);
            assert_eq!(data::fold_char(c), folded, "{}", c.escape_unicode());
            assert_eq!(data::fold_char(folded), folded, "{}", c.escape_unicode());
        }
    }

    #[test]
    fn the_signatures_hold_every_entry() {
        use data::{Signatures, WordList};
        for data in BUILTIN.iter().copied().chain([&common::COMMON]) {
            let table = data.table();
            let mut set = Signatures::NONE;
            set.add_table(&table);
            let lists = [
                (WordList::Units, table.units),
                (WordList::Months, table.months),
                (WordList::Exceptions, table.atom_exceptions),
                (WordList::Exceptions, table.elisions),
                (WordList::ShortWords, table.short_words),
                (WordList::Particles, table.particles),
            ];
            for (list, entries) in lists {
                let dictionary = !matches!(list, WordList::ShortWords | WordList::Particles);
                for entry in entries.iter() {
                    assert!(set.may_hold(list, entry.chars()), "{entry}");
                    assert!(
                        !dictionary || set.may_hold(WordList::Dictionary, entry.chars()),
                        "{entry}"
                    );
                    // With the first letter in uppercase, as `cased_in` reads it; units are
                    // compared with case alone.
                    let mut chars = entry.chars();
                    let upper = chars.next().into_iter().flat_map(char::to_uppercase);
                    if list != WordList::Units {
                        assert!(set.may_hold(list, upper.chain(chars)), "{entry}");
                    }
                }
            }
            for (parts, _) in table.abbreviations.iter() {
                let mut runs: Vec<String> = parts.iter().map(str::to_owned).collect();
                if parts.iter().all(|p| p.ends_with('.')) {
                    runs.push(parts.iter().collect());
                }
                for run in runs {
                    for list in [WordList::Dictionary, WordList::Abbreviations] {
                        assert!(set.may_hold(list, run.chars()), "{run}");
                    }
                }
            }
        }
        // Common words are rejected without a lookup.
        let mut set = Signatures::NONE;
        set.add_table(&ru_ru::RU_RU.table());
        assert!(!set.may_hold(WordList::Dictionary, "изменения".chars()));
        assert!(!set.may_hold(WordList::ShortWords, "Сохранить".chars()));
        // A key longer than every entry of a list is not in it.
        assert_eq!(set.longest(WordList::ShortWords), 4);
        assert!(set.may_hold(WordList::ShortWords, "если".chars()));
        assert!(!set.may_hold(WordList::ShortWords, "еслии".chars()));
        // Units keep their case: `в` is a short word, `В` (volt) a unit.
        assert!(set.may_hold(WordList::Units, "В".chars()));
        assert!(!set.may_hold(WordList::Units, "в".chars()));
    }

    #[test]
    fn a_shape_with_a_character_added_is_that_of_the_longer_key() {
        use data::Shape;
        for key in ["", "a", "т.е", "tis"] {
            let dotted = format!("{key}.");
            let elided = format!("'{key}");
            assert_eq!(Shape::of(key.chars()).then('.'), Shape::of(dotted.chars()));
            assert_eq!(
                Shape::of(key.chars()).after('\''),
                Shape::of(elided.chars())
            );
            let len = key.chars().count() as u64;
            assert_eq!(
                Shape::of_at_most(key.chars(), len),
                Some(Shape::of(key.chars()))
            );
            assert_eq!(
                Shape::of_at_most(key.chars(), len + 1),
                Some(Shape::of(key.chars()))
            );
            if len > 0 {
                assert_eq!(Shape::of_at_most(key.chars(), len - 1), None);
            }
        }
    }

    #[test]
    fn abbreviations_with_a_prefix_are_one_range() {
        for data in BUILTIN {
            let list = data.table().abbreviations;
            for prefix in [
                "т",
                "Т",
                "т.",
                "и",
                "г",
                "г-",
                "г-ж",
                "гл",
                "D",
                "Dr",
                "Dr.",
                "p",
                "pp",
                "a",
                "я",
                ".",
                "",
                "просп",
                "просто",
                "No",
                "Nos.",
                "Nose",
            ] {
                let range = list.with_prefix(prefix.chars());
                let expected: Vec<usize> = (0..list.len())
                    .filter(|&k| {
                        list.get(k)
                            .and_then(|(parts, _)| parts.get(0))
                            .is_some_and(|part| part.starts_with(prefix))
                    })
                    .collect();
                let found: Vec<usize> = range.collect();
                assert_eq!(found, expected, "{} {prefix}", data.tag);
            }
        }
    }
}
