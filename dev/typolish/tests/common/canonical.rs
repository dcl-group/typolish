//! Generators of canonical plain strings and templates for property 13 (design sections 12.1,
//! 12.2 and 13.5), and of plain strings for P2.
//!
//! A canonical string is written in the plain form of the typographer under test: no typographic
//! character, a dash between words written `-` with spaces on both sides, ranges `a-b`, `x`,
//! `(c)`, `(r)`, `(tm)`, abbreviations with a space (`т. е.`), `№ 5`, double straight quotes at
//! every level, `'` only for apostrophes and primes, `...`, ru `?..` and the form of a percentage
//! that the `PercentSpacing` of the setup keeps. The generator avoids only the pattern of
//! `NestedPossessive`: two `s'` at the same depth of one nested pair.
//!
//! The strings are built from atoms, each of which is canonical alone and starts and ends with a
//! letter, a digit, a quote or one full stop, and from joiners that put the atoms together. Where
//! the join would put two full stops or a full stop and a punctuation mark together, the joiner
//! becomes a space.

#![allow(dead_code, clippy::unwrap_used, clippy::indexing_slicing)]

use proptest::prelude::*;
use typolish::{Language, PercentSpacing, Rule, RuleSet, SingleQuotes, Typographer};

use super::strategies::{RISKY, rule_set};

/// The settings a canonical string is generated for.
#[derive(Clone, Debug)]
pub struct Setup {
    /// 0 for ru-RU, 1 for en-US.
    pub lang: usize,
    /// The enabled rules.
    pub rules: RuleSet,
    /// The single quote mode.
    pub single: SingleQuotes,
    /// The percent setting.
    pub percent: PercentSpacing,
}

impl Setup {
    /// The language of the setup.
    pub fn language(&self) -> Language {
        if self.lang == 0 {
            Language::RU_RU
        } else {
            Language::EN_US
        }
    }

    /// The typographer of the setup.
    pub fn typographer(&self) -> Typographer {
        Typographer::builder(self.language(), self.rules)
            .single_quotes(self.single)
            .percent(self.percent)
            .build()
            .unwrap()
    }

    /// The setup written for a failure message.
    pub fn describe(&self) -> String {
        format!(
            "{} rules {:?} {:?} {:?}",
            self.language(),
            self.rules.iter().map(|r| r.id()).collect::<Vec<_>>(),
            self.single,
            self.percent
        )
    }
}

/// A setup: both languages, the built-in sets, each rule alone and random subsets, the three
/// single quote modes and the four percent settings.
pub fn setup() -> impl Strategy<Value = Setup> {
    let rules = prop_oneof![
        3 => Just(RuleSet::PROSE),
        3 => Just(RuleSet::INTERFACE),
        3 => (0..Rule::ALL.len()).prop_map(|i| RuleSet::EMPTY.with(Rule::ALL[i])),
        3 => rule_set(),
    ];
    (0usize..2, rules, 0u8..3, 0u8..4).prop_map(|(lang, rules, single, percent)| Setup {
        lang,
        rules,
        single: match single {
            0 => SingleQuotes::NestedOnly,
            1 => SingleQuotes::Everywhere,
            _ => SingleQuotes::Never,
        },
        percent: match percent {
            0 => PercentSpacing::AsWritten,
            1 => PercentSpacing::Language,
            2 => PercentSpacing::Attached,
            _ => PercentSpacing::Spaced,
        },
    })
}

/// Whether the form `5 %` (with a space) is canonical for the setup, and whether `5%` is.
fn percent_forms(lang: usize, percent: PercentSpacing) -> (bool, bool) {
    match percent {
        PercentSpacing::Language => {
            if lang == 0 {
                (true, false)
            } else {
                (false, true)
            }
        }
        PercentSpacing::Attached => (false, true),
        PercentSpacing::Spaced => (true, false),
        _ => (true, true),
    }
}

/// Atoms that start with a letter or an opening quote (a sentence may start with them, and a
/// dialogue line may start with them) and the other atoms, for a setup.
pub fn atoms(setup: &Setup) -> (Vec<String>, Vec<String>) {
    let ru = setup.lang == 0;
    let mut first: Vec<&str> = Vec::new();
    let mut other: Vec<&str> = Vec::new();
    if ru {
        first.extend([
            "дом",
            "Москва",
            "река",
            "быстро",
            "очень красивый",
            "кто-то",
            "что-то",
            "учёный-географ",
            "tls-server",
            "kv_lang",
            "в доме",
            "на столе",
            "не знаю",
            "Знаешь ли",
            "сказал бы",
            "это же так",
            "и так",
            "Он пошёл",
            "Д'Артаньян",
            "из Москвы",
            "А. С. Пушкин",
            "Пушкин А. С.",
            "и т. д.",
            "т. е.",
            "и т. п.",
            "Brand(tm)",
            "Brand(r)",
            "\"слово\"",
            "\"два слова\"",
            "\"слово в слове\"",
            "\"слово \"внутри\" слова\"",
            "\"и \"кто-то\" ещё\"",
            "\"a b\"",
            "\"a \"\" b\"",
            "\"a \"-\" b\"",
            "\"a \"?\" b\"",
            "\"слово \"\", слово\"",
            "в 2026 году",
            "21 ноября",
            "ls -la",
            "--force",
            "host:8080",
            "https://example.com/docs",
        ]);
        other.extend([
            "5",
            "10",
            "2026",
            "1,5",
            "100",
            "5 МБ",
            "30 с",
            "12 дн.",
            "60 км/ч",
            "5 кг",
            "10 руб.",
            "21 ноября 2026 г.",
            "2026 г.",
            "XIX в.",
            "5-10 кг",
            "1941-1945 гг.",
            "10-20",
            "20-10",
            "10:00-12:30",
            "стр. 9-12",
            "1,5-2,5 кг",
            "№ 5",
            "§ 12",
            "-5",
            "+30",
            "1920x1080",
            "10 x 5",
            "(c) 2026",
            "2026 (c)",
            "(c) 2020-2026",
            "+-5",
            "a +- b",
            "x -> y",
            "a <-> b",
            "a >= b",
            "x != y",
            "+7 495 123-45-67",
            "2026-11-21",
            "0.2.27",
            "-x",
        ]);
    } else {
        first.extend([
            "house",
            "Moscow",
            "the river",
            "quickly",
            "very beautiful",
            "mother-in-law",
            "tls-server",
            "kv_lang",
            "in the house",
            "on the table",
            "I don't know",
            "it's fine",
            "rock'n'roll",
            "O'Brien",
            "the 90's",
            "the '90s",
            "'Tis the season",
            "Mr. Smith",
            "J. R. R. Tolkien",
            "I chose plan B",
            "Brand(tm)",
            "Brand(r)",
            "\"word\"",
            "\"two words\"",
            "\"a word in a word\"",
            "\"a word \"inside\" a word\"",
            "\"don't \"stop\" it's\"",
            "\"and \"mother-in-law\" too\"",
            "\"a b\"",
            "\"a \"\" b\"",
            "\"a \"-\" b\"",
            "\"a \"?\" b\"",
            "\"a word \"\", a word\"",
            "in 2026",
            "November 21",
            "ls -la",
            "--force",
            "host:8080",
            "https://example.com/docs",
        ]);
        other.extend([
            "5",
            "10",
            "2026",
            "1.5",
            "100",
            "5 MB",
            "30 s",
            "12 min.",
            "60 km/h",
            "5 kg",
            "10 USD",
            "November 21, 2026",
            "XIX c.",
            "5-10 kg",
            "1941-1945",
            "10-20",
            "20-10",
            "10:00-12:30",
            "p. 9-12",
            "1.5-2.5 kg",
            "No. 5",
            "§ 12",
            "-5",
            "+30",
            "1920x1080",
            "10 x 5",
            "(c) 2026",
            "2026 (c)",
            "(c) 2020-2026",
            "+-5",
            "a +- b",
            "x -> y",
            "a <-> b",
            "a >= b",
            "x != y",
            "+7 495 123-45-67",
            "2026-11-21",
            "0.2.27",
            "-x",
            "5'10\"",
            "He is 5'10\" tall",
            "etc.",
            "i.e.",
        ]);
        if setup.single != SingleQuotes::Everywhere {
            first.push("the dogs' bowl");
            first.push("\"the dogs' bowl\"");
            first.push("James's coat");
            // An `s'` at depth two, and at depths two and three.
            first.push("\"a \"the dogs' bowl\" b\"");
            first.push("\"\"a dogs' b \"c dogs' d\" e\" f\"");
        }
    }
    let (spaced, attached) = percent_forms(setup.lang, setup.percent);
    if spaced {
        other.push("5 %");
        other.push("100 %");
    }
    if attached {
        other.push("5%");
        other.push("100%");
    }
    let to_strings = |v: Vec<&str>| v.into_iter().map(str::to_owned).collect();
    (to_strings(first), to_strings(other))
}

/// Joiners between two atoms.
fn joiners(ru: bool) -> Vec<&'static str> {
    let mut joiners = vec![
        " ", " ", " ", " ", ", ", ". ", "! ", "? ", "... ", " - ", ": ", "; ", " - ",
    ];
    if ru {
        joiners.push("?.. ");
    } else {
        joiners.push("?... ");
    }
    joiners
}

fn ends_punct(text: &str) -> bool {
    text.ends_with('.')
}

fn assemble(first: &str, rest: &[(&'static str, String)], end: &str) -> String {
    let mut out = first.to_owned();
    for (joiner, atom) in rest {
        let punct_joiner = joiner.starts_with(['.', ',', '!', '?', ';', ':']);
        let joiner = if ends_punct(&out) && punct_joiner {
            " "
        } else {
            joiner
        };
        out.push_str(joiner);
        out.push_str(atom);
    }
    if !ends_punct(&out) {
        out.push_str(end);
    }
    out
}

/// A canonical sentence for the setup.
pub fn sentence(setup: &Setup) -> BoxedStrategy<String> {
    let (first, other) = atoms(setup);
    let mut all = first.clone();
    all.extend(other);
    let ends: Vec<&'static str> = if setup.lang == 0 {
        vec!["", ".", "!", "?", "...", "?.."]
    } else {
        vec!["", ".", "!", "?", "...", "?..."]
    };
    (
        prop::sample::select(first),
        prop::collection::vec(
            (
                prop::sample::select(joiners(setup.lang == 0)),
                prop::sample::select(all),
            ),
            0..6,
        ),
        prop::sample::select(ends),
    )
        .prop_map(|(first, rest, end)| assemble(&first, &rest, end))
        .boxed()
}

/// A canonical text of one to four lines for the setup; in ru a line may be a line of dialogue.
pub fn canonical_text(setup: &Setup) -> BoxedStrategy<String> {
    let ru = setup.lang == 0;
    let line = (
        sentence(setup),
        prop::bool::weighted(if ru { 0.25 } else { 0.0 }),
    )
        .prop_map(|(sentence, dialogue)| {
            if dialogue {
                format!("- {sentence}")
            } else {
                sentence
            }
        });
    prop::collection::vec(line, 1..4)
        .prop_map(|lines| lines.join("\n"))
        .boxed()
}

/// Template atoms of a setup, written in the syntax of `parse_tmpl`: text with `{name:kind}`.
pub fn template_atoms(setup: &Setup) -> Vec<String> {
    let ru = setup.lang == 0;
    let mut atoms: Vec<&str> = if ru {
        vec![
            "{n:number} дн.",
            "{n:number} МБ",
            "{n:number} шт.",
            "\"{name:word}\"",
            "\"{x}\"",
            "{a:number}-{b:number}",
            "{a:date}-{b:date}",
            "№ {n:number}",
            "{name:word} - администратор",
            "Привет, {name:word}!",
            "в {d:date}",
            "{d:date}",
            "{a:number} - {b:number}",
            "Файл \"{name:word}\" не найден",
            "Он ответил: \"Читай \"{name:word}\" до конца\"",
            "{x}",
            "{name:word}",
            "{n:number}",
            "{n:number} с.",
            "{y:number} г.",
            "кто-то",
            "и т. д.",
            "5 МБ",
            "\"{name:word} \"\" и \"-\"\"",
        ]
    } else {
        vec![
            "{n:number} days",
            "{n:number} MB",
            "{n:number} pcs.",
            "\"{name:word}\"",
            "\"{x}\"",
            "{a:number}-{b:number}",
            "{a:date}-{b:date}",
            "No. {n:number}",
            "{name:word} - administrator",
            "Hello, {name:word}!",
            "on {d:date}",
            "{d:date}",
            "{a:number} - {b:number}",
            "File \"{name:word}\" was not found",
            "He replied: \"Read \"{name:word}\" to the end\"",
            "{x}",
            "{name:word}",
            "{n:number}",
            "{n:number} s.",
            "{name:word}'s profile",
            "don't",
            "etc.",
            "5 MB",
            "\"{name:word} \"\" and \"-\"\"",
        ]
    };
    if !ru && setup.single != SingleQuotes::Everywhere {
        atoms.push("\"{name:word} \"the dogs' bowl\" here\"");
    }
    let (spaced, attached) = percent_forms(setup.lang, setup.percent);
    if spaced {
        atoms.push("{n:number} %");
    }
    if attached {
        atoms.push("{n:number}%");
    }
    atoms.into_iter().map(str::to_owned).collect()
}

/// A canonical template of one to six atoms for the setup, as the text after `id = ` of a `.tmpl`
/// line; a name is used at most once, which `parse_tmpl` does not need but a reader does.
pub fn canonical_template(setup: &Setup) -> BoxedStrategy<String> {
    let atoms = template_atoms(setup);
    let seps: Vec<&'static str> = vec![" ", " ", ", ", ". ", " - ", ": ", "... "];
    (
        prop::sample::select(atoms.clone()),
        prop::collection::vec(
            (prop::sample::select(seps), prop::sample::select(atoms)),
            0..5,
        ),
    )
        .prop_map(|(first, rest)| {
            let mut out = first;
            for (sep, atom) in rest {
                // No second full stop after a full stop, and no ellipsis after `!` or `?`: in
                // Russian `!...` and `?...` are not canonical (`!..`, `?..`, design section 12.2).
                let punct = sep.starts_with(['.', ',', ';', ':']);
                let sep = if (out.ends_with('.') && punct)
                    || (sep.starts_with("...") && out.ends_with(['!', '?']))
                {
                    " "
                } else {
                    sep
                };
                out.push_str(sep);
                out.push_str(&atom);
            }
            out
        })
        .boxed()
}

/// The characters that `to_plain` changes, or that a typed author uses as typography; a plain
/// string for P2 holds none of them (design section 12.2: P2 is for plain `x`).
const TYPOGRAPHIC_CHARS: &[char] = &[
    '«', '»', '„', '“', '”', '‘', '’', '″', '′', '—', '–', '…', '×', '−', '©', '®', '™', '→', '←',
    '↔', '≥', '≤', '≠', '±', '\u{a0}', '\u{202f}',
];

/// Risky fragments of `strategies::RISKY` that hold no typographic character.
pub fn plain_risky() -> Vec<&'static str> {
    RISKY
        .iter()
        .copied()
        .filter(|piece| !piece.chars().any(|c| TYPOGRAPHIC_CHARS.contains(&c)))
        .collect()
}

/// A plain string: risky fragments without typographic characters, mixed with canonical
/// sentences of both languages.
pub fn plain_text() -> impl Strategy<Value = String> {
    let sentences = (0usize..2).prop_flat_map(|lang| {
        let setup = Setup {
            lang,
            rules: RuleSet::PROSE,
            single: SingleQuotes::NestedOnly,
            percent: PercentSpacing::AsWritten,
        };
        sentence(&setup)
    });
    prop::collection::vec(
        prop_oneof![
            3 => prop::sample::select(plain_risky()).prop_map(str::to_owned),
            2 => sentences,
            1 => prop::sample::select(vec![" ", "\n", ".", ","]).prop_map(str::to_owned),
        ],
        0..10,
    )
    .prop_map(|v| v.concat())
}

/// A plain string and a setup: any rule set of `setup()`.
pub fn plain_with_setup() -> impl Strategy<Value = (Setup, String)> {
    (setup(), plain_text())
}
