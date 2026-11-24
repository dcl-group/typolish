//! The tables of the `nbsp.*` rules (design sections 3.9, 8.2, 8.7 and 13.2,
//! `tests/rules_nbsp.rs`).
//!
//! Every row runs with the one rule it exercises, alone (`RuleSet::EMPTY.with(rule)`), unless it
//! names a configuration. The rows of "inside both sets" run in the full `PROSE` and `INTERFACE`,
//! where other rules add their edits: those expectations are written out for each set. As in
//! design section 8.7, an output shows the edits of the rules that run. A row is written with the
//! markers of `vis` (`⍽` is U+00A0, `⎵` is U+202F).

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::print_stdout,
    clippy::print_stderr,
    clippy::dbg_macro
)]

mod common;
#[path = "common/probe.rs"]
mod probe;

use std::ops::Range;

use common::{assert_changed_by, change_rules, check_exact as check_in, only, show, vis};
use typolish::{
    DashStyle, Language, LanguageData, MonthPosition, NarrowGlue, Output, PercentSpacing, Piece,
    Rule, RuleSet, Slot, Typographer,
};

const ABBR: Rule = Rule::NbspAbbreviations;
const INITIALS: Rule = Rule::NbspInitials;
const DATES: Rule = Rule::NbspDates;
const UNITS: Rule = Rule::NbspUnits;
const SIGNS: Rule = Rule::NbspSigns;
const PHONE: Rule = Rule::NbspPhone;
const SHORT_WORDS: Rule = Rule::NbspShortWords;
const SHORT_LENGTH: Rule = Rule::NbspShortLength;
const PARTICLES: Rule = Rule::NbspParticles;
const DASH: Rule = Rule::NbspDash;

const NBSP_RULES: [Rule; 9] = [
    ABBR,
    INITIALS,
    DATES,
    UNITS,
    SIGNS,
    PHONE,
    SHORT_WORDS,
    SHORT_LENGTH,
    PARTICLES,
];

fn ru() -> Language {
    Language::RU_RU
}

fn en() -> Language {
    Language::EN_US
}

/// Asserts that a row is typeset to itself and borrowed.
#[track_caller]
fn check_unchanged(what: &str, t: &Typographer, text: &str) {
    check_in(what, t, (text, text));
}

/// A typographer with the one rule, alone.
fn alone(language: Language, rule: Rule) -> Typographer {
    only(language, &[rule])
}

/// A built-in set of a language with default settings.
fn set(language: Language, set: RuleSet) -> Typographer {
    Typographer::new(language, set)
}

/// A typographer that runs the given rules and has the settings of `f` applied.
fn with(
    language: Language,
    rules: &[Rule],
    f: impl FnOnce(typolish::TypographerBuilder) -> typolish::TypographerBuilder,
) -> Typographer {
    let set: RuleSet = rules.iter().copied().collect();
    f(Typographer::builder(language, set)).build().unwrap()
}

/// Runs rows with the rule alone.
#[track_caller]
fn alone_rows(language: Language, rule: Rule, rows: &[(&str, &str)]) {
    let t = alone(language.clone(), rule);
    let what = format!("{language} {} alone", rule.id());
    for row in rows {
        check_in(&what, &t, *row);
    }
}

/// Runs rows `(input, Prose, Interface)` in the full sets.
#[track_caller]
fn set_rows(language: Language, rows: &[(&str, &str, &str)]) {
    let prose = set(language.clone(), RuleSet::PROSE);
    let interface = set(language.clone(), RuleSet::INTERFACE);
    for (input, prose_out, interface_out) in rows {
        check_in(&format!("{language} Prose"), &prose, (input, prose_out));
        check_in(
            &format!("{language} Interface"),
            &interface,
            (input, interface_out),
        );
    }
}

/// Rows that read the same in Prose and in Interface.
#[track_caller]
fn both_sets(language: Language, rows: &[(&str, &str)]) {
    let rows: Vec<(&str, &str, &str)> = rows.iter().map(|r| (r.0, r.1, r.1)).collect();
    set_rows(language, &rows);
}

/// The texts of a template.
fn texts(t: &Typographer, pieces: Vec<Piece<'_, &'static str>>) -> Vec<String> {
    t.typeset_template(pieces, Output::Unicode)
        .texts()
        .iter()
        .map(|s| s.to_string())
        .collect()
}

/// Edits of `report` as `(rule, range, replacement)`.
fn edits(t: &Typographer, text: &str) -> Vec<(Rule, Range<usize>, String)> {
    t.report(&vis(text), Output::Unicode)
        .changes()
        .iter()
        .map(|c| (c.rule(), c.range(), c.replacement().to_owned()))
        .collect()
}

// ---------------------------------------------------------------------------------------------
// nbsp.abbreviations
// ---------------------------------------------------------------------------------------------

const RU_ABBR: &[(&str, &str)] = &[
    // Inner entries: the spaces inside are glued, a missing space is inserted.
    ("т.е.", "т.⍽е."),
    ("т. е.", "т.⍽е."),
    ("т.д.", "т.⍽д."),
    ("т.к.", "т.⍽к."),
    ("т.н.", "т.⍽н."),
    ("т.п.", "т.⍽п."),
    ("т.ч.", "т.⍽ч."),
    ("н. э.", "н.⍽э."),
    ("и т. д.", "и⍽т.⍽д."),
    ("и т.д.", "и⍽т.⍽д."),
    ("и т.п.", "и⍽т.⍽п."),
    ("и т. п.", "и⍽т.⍽п."),
    ("до н. э.", "до⍽н.⍽э."),
    ("в т. ч.", "в⍽т.⍽ч."),
    ("в т.ч.", "в⍽т.⍽ч."),
    ("и др.", "и⍽др."),
    ("и пр.", "и⍽пр."),
    ("Яблоки, груши и т.д.", "Яблоки, груши и⍽т.⍽д."),
    // The full stop that ends an entry is the first full stop of a `Dots` token.
    ("и т.д…", "и⍽т.⍽д…"),
    ("и т. д…", "и⍽т.⍽д…"),
    // Typed glue and other spaces: nothing changes.
    ("т.⍽е.", "т.⍽е."),
    ("и⍽т.⍽д.", "и⍽т.⍽д."),
    ("т.  е.", "т.  е."),
    ("т.\tе.", "т.\tе."),
    ("т.\nе.", "т.\nе."),
    // `др.` alone is not an entry.
    ("др. Иванов", "др. Иванов"),
    // `Next` entries glue to the following operand; an entry that starts with a lowercase letter
    // also matches with a capital.
    ("см. рис. 5", "см.⍽рис.⍽5"),
    ("См. рис. 5", "См.⍽рис.⍽5"),
    ("ул. Ленина", "ул.⍽Ленина"),
    ("Ул. Ленина", "Ул.⍽Ленина"),
    ("стр. 5", "стр.⍽5"),
    ("рис. 3", "рис.⍽3"),
    ("табл. 2", "табл.⍽2"),
    ("ст. 5", "ст.⍽5"),
    ("ср. стр. 5", "ср.⍽стр.⍽5"),
    ("пп. 5", "пп.⍽5"),
    ("(см. рис. 5)", "(см.⍽рис.⍽5)"),
    ("ч. 2", "ч.⍽2"),
    // A one-letter entry matches only as written (design section 7.1).
    ("п. 5 устава", "п.⍽5 устава"),
    ("П. 5 устава", "П. 5 устава"),
    ("д. 5", "д.⍽5"),
    ("Д. 5", "Д. 5"),
    (
        "Выберите вариант Д. После этого",
        "Выберите вариант Д. После этого",
    ),
    // `NextCapitalized`: only before a capitalized word.
    ("г. Москва", "г.⍽Москва"),
    ("г. москва", "г. москва"),
    ("в г. Москве", "в г.⍽Москве"),
    // A `Next` or `NextCapitalized` entry directly after a number, a range, a `YearMonth` core,
    // a date core or a `Number` or `Date` slot does not bind forward.
    ("5 ч. Потом", "5 ч. Потом"),
    ("5–10 ч. Потом", "5–10 ч. Потом"),
    ("2026 г. Затем", "2026 г. Затем"),
    ("1990–2000 г. Затем", "1990–2000 г. Затем"),
    ("2010-11 г. Затем", "2010-11 г. Затем"),
    ("21.11.2026 г. Затем", "21.11.2026 г. Затем"),
    ("21.11.26 г. Затем", "21.11.26 г. Затем"),
    (
        "Договор от 21.11.2026 г. Затем",
        "Договор от 21.11.2026 г. Затем",
    ),
    (
        "Договор от 21.11.26 г. Затем",
        "Договор от 21.11.26 г. Затем",
    ),
    // A label number, a label range and a percentage are not numbers here.
    ("Школа №5 г. Москвы", "Школа №5 г.⍽Москвы"),
    ("Школа № 5 г. Москвы", "Школа № 5 г.⍽Москвы"),
    ("§5 ч. 2", "§5 ч.⍽2"),
    ("§§ 5-7 г. Москвы", "§§ 5-7 г.⍽Москвы"),
    ("5% ч. 2", "5% ч.⍽2"),
    // `0.2.27` is not a date core.
    ("0.2.27 г.", "0.2.27 г."),
    // Never before a fragment that starts with `-`; no space is inserted after a `Next` entry.
    ("см. -x", "см. -x"),
    ("ул.Ленина", "ул.Ленина"),
];

#[test]
fn a_format_character_of_section_5_4_is_skipped() {
    // U+061C, U+180E, U+206A to U+206F and U+FFF9 to U+FFFB are format characters of design
    // section 5.4, as U+200E is: the word before them stays a word and the space after it is
    // glued, and nothing is protected.
    let t = set(ru(), RuleSet::PROSE);
    for f in [
        '\u{200e}', '\u{61c}', '\u{180e}', '\u{206a}', '\u{206f}', '\u{fff9}',
    ] {
        let input = format!("в{f} лесу");
        let report = t.report(&input, Output::Unicode);
        assert_eq!(report.result(), &format!("в{f}\u{a0}лесу"), "{f:?}");
        assert!(
            report.protected().is_empty(),
            "{f:?}: {:?}",
            report.protected()
        );
    }
}

#[test]
fn abbreviations_in_russian() {
    alone_rows(ru(), ABBR, RU_ABBR);
    // A `Format` token between the core and its stop is skipped (design section 3.6).
    let prose = set(ru(), RuleSet::PROSE);
    check_in("ru Prose", &prose, ("т.е\u{AD}. да", "т.⍽е\u{AD}. да"));
    check_in("ru Prose", &prose, ("т.е\u{ad}...", "т.⍽е\u{ad}…"));
    let t = alone(ru(), ABBR);
    for text in ["т.е.", "и т. д.", "см. рис. 5", "ул. Ленина"] {
        assert_changed_by("ru abbreviations", &t, text, ABBR);
    }
}

#[test]
fn a_capitalized_first_part_of_an_entry_is_relaxed() {
    // The first part of an entry of several parts is relaxed even when it is a single letter with
    // a full stop (design section 7.1): `Т. е.` at a sentence start matches `т. е.`; a one-letter
    // entry and a later part still match only as written.
    alone_rows(
        ru(),
        ABBR,
        &[
            ("Т. е. так", "Т.⍽е. так"),
            ("Т.е. так", "Т.⍽е. так"),
            ("Т. д. так", "Т.⍽д. так"),
            ("Т. Е. Иванов", "Т. Е. Иванов"),
            ("т. Е. так", "т. Е. так"),
            ("Т. Иванов", "Т. Иванов"),
            ("вариант Д. После", "вариант Д. После"),
            ("Буква Т. е. не", "Буква Т.⍽е. не"),
        ],
    );
    let prose = set(ru(), RuleSet::PROSE);
    for row in [
        ("Т. е. так", "Т.⍽е. так"),
        ("Т.е. так", "Т.⍽е. так"),
        ("Т. д. так", "Т.⍽д. так"),
        ("Т. Е. Иванов", "Т.⍽Е.⍽Иванов"),
        ("Т. Иванов", "Т.⍽Иванов"),
        ("вариант Д. После", "вариант Д.⍽После"),
        ("Буква Т. е. не", "Буква⍽Т.⍽е. не"),
    ] {
        check_in("ru Prose", &prose, row);
    }
}

const EN_ABBR: &[(&str, &str)] = &[
    ("Mr. Smith", "Mr.⍽Smith"),
    ("Mrs. Smith", "Mrs.⍽Smith"),
    ("Ms. Smith", "Ms.⍽Smith"),
    ("Dr. Who", "Dr.⍽Who"),
    ("Prof. Smith", "Prof.⍽Smith"),
    ("St. Louis", "St.⍽Louis"),
    ("Mt. Everest", "Mt.⍽Everest"),
    ("Mr. Smith, Dr. Jones", "Mr.⍽Smith, Dr.⍽Jones"),
    ("Fig. 3", "Fig.⍽3"),
    ("Figs. 3 and 4", "Figs.⍽3 and 4"),
    ("See No. 5", "See No.⍽5"),
    ("Nos. 5", "Nos.⍽5"),
    ("ch. 4", "ch.⍽4"),
    ("sec. 5", "sec.⍽5"),
    ("vol. 2", "vol.⍽2"),
    ("p. 5", "p.⍽5"),
    ("pp. 5-7", "pp.⍽5-7"),
    // An entry that starts with a capital matches only as written; a one-letter entry too.
    ("I said no. Then I left.", "I said no. Then I left."),
    ("no. 5", "no. 5"),
    ("P. 5", "P. 5"),
    (
        "I chose plan P. Then I left.",
        "I chose plan P. Then I left.",
    ),
    // After a number the entry does not bind forward.
    ("5 vol. 2", "5 vol. 2"),
    // Atom exceptions are not abbreviations of the list.
    ("e.g. this", "e.g. this"),
    ("etc. and", "etc. and"),
    // Initials belong to `nbsp.initials`.
    ("J. R. R. Tolkien", "J. R. R. Tolkien"),
    ("Mr.  Smith", "Mr.  Smith"),
    ("Mr.\tSmith", "Mr.\tSmith"),
    ("Mr.⍽Smith", "Mr.⍽Smith"),
];

#[test]
fn abbreviations_in_english() {
    alone_rows(en(), ABBR, EN_ABBR);
    // Russian entries mean nothing in English text and the other way round.
    alone_rows(
        en(),
        ABBR,
        &[("т.е.", "т.е."), ("См. рис. 5", "См. рис. 5")],
    );
    alone_rows(ru(), ABBR, &[("Mr. Smith", "Mr. Smith")]);
}

#[test]
fn abbreviations_with_the_ellipsis_phase() {
    // The ellipsis phase has replaced the run, `nbsp.abbreviations` reads the `…` as the full stop.
    let t = only(ru(), &[Rule::EllipsisDots, ABBR]);
    check_in("ru", &t, ("и т.д...", "и⍽т.⍽д…"));
    check_in("ru", &t, ("и т. д…", "и⍽т.⍽д…"));
    let t = only(en(), &[Rule::EllipsisDots, ABBR]);
    // `etc` with its stop is a `Dictionary` core: no glue, the run becomes `…`.
    check_in("en", &t, ("etc...", "etc…"));
}

#[test]
fn abbreviations_inside_both_sets() {
    both_sets(
        ru(),
        &[
            ("Яблоки, груши и т.д.", "Яблоки, груши и⍽т.⍽д."),
            ("См. рис. 5", "См.⍽рис.⍽5"),
            ("Школа №5 г. Москвы", "Школа №⍽5 г.⍽Москвы"),
            ("§5 ч. 2", "§⍽5 ч.⍽2"),
            ("т.е.", "т.⍽е."),
        ],
    );
    both_sets(
        en(),
        &[("Mr. Smith", "Mr.⍽Smith"), ("See No. 5", "See No.⍽5")],
    );
    // Unchanged in both sets.
    for t in [set(ru(), RuleSet::PROSE), set(ru(), RuleSet::INTERFACE)] {
        check_unchanged("ru set", &t, "т.⍽е.");
        check_unchanged("ru set", &t, "и⍽т.⍽д.");
    }
}

#[test]
fn compact_abbreviations_next_to_a_dash() {
    // A compact abbreviation reads as a word, as the parts it is split into do (design section 8.2,
    // invariant I2 of design section 4.3), so the dash rules match on the first pass and a second
    // pass returns the output.
    set_rows(
        ru(),
        &[
            (
                "Он пришёл - т.е. опоздал",
                "Он⍽пришёл⍽— т.⍽е. опоздал",
                "Он пришёл⍽— т.⍽е. опоздал",
            ),
            ("Слово - т.е.", "Слово⍽— т.⍽е.", "Слово⍽— т.⍽е."),
            (
                "Москва – т.е. столица",
                "Москва⍽– т.⍽е. столица",
                "Москва⍽– т.⍽е. столица",
            ),
            // `--` is a dash only in Prose.
            ("Слово -- т.д. ", "Слово⍽— т.⍽д. ", "Слово -- т.⍽д. "),
            // A single-letter part next to a number is refused in both forms.
            ("5 - т.е.", "5 - т.⍽е.", "5 - т.⍽е."),
            ("т.е. - 5", "т.⍽е. - 5", "т.⍽е. - 5"),
            ("т.е. - слово", "т.⍽е.⍽— слово", "т.⍽е.⍽— слово"),
        ],
    );
    for t in [set(ru(), RuleSet::PROSE), set(ru(), RuleSet::INTERFACE)] {
        for input in [
            "Он пришёл - т.е. опоздал",
            "Слово - т.е.",
            "Москва – т.е. столица",
            "Слово -- т.д. ",
            "5 - т.е.",
            "т.е. - 5",
            "т.е. - слово",
        ] {
            let once = t.typeset(input, Output::Unicode).into_owned();
            check_unchanged("second pass", &t, &show(&once));
        }
    }
}

#[test]
fn abbreviations_are_off_when_disabled() {
    for base in [RuleSet::PROSE, RuleSet::INTERFACE] {
        let t = set(ru(), base.without(ABBR));
        // `т.е.` stays without its inserted space; `См. рис. 5` keeps its plain spaces.
        check_in("disabled", &t, ("т.е.", "т.е."));
        check_in("disabled", &t, ("См. рис. 5", "См. рис. 5"));
    }
}

#[test]
fn abbreviations_with_narrow_glue() {
    let t = with(ru(), &[ABBR], |b| b.narrow_glue(NarrowGlue::ABBREVIATIONS));
    // `Narrow` goes only to the spaces between the parts of an entry (design section 8.7).
    check_in("ru", &t, ("и т. д.", "и⍽т.⎵д."));
    check_in("ru", &t, ("т.е.", "т.⎵е."));
    check_in("ru", &t, ("т. е.", "т.⎵е."));
    // The space after a leading word and the forward glue of `Next` entries are `NoBreak`.
    check_in("ru", &t, ("в т.ч.", "в⍽т.⎵ч."));
    check_in("ru", &t, ("см. рис. 5", "см.⍽рис.⍽5"));
    check_in("ru", &t, ("ул. Ленина", "ул.⍽Ленина"));
    // The other places keep U+00A0.
    for (places, input, expected) in [
        (NarrowGlue::INITIALS, "и т. д.", "и⍽т.⍽д."),
        (NarrowGlue::SIGNS, "и т. д.", "и⍽т.⍽д."),
        (NarrowGlue::NONE, "и т. д.", "и⍽т.⍽д."),
    ] {
        let t = with(ru(), &[ABBR], |b| b.narrow_glue(places));
        check_in("ru", &t, (input, expected));
    }
    // Typed glue is never rewritten.
    let all = with(ru(), &[ABBR], |b| b.narrow_glue(NarrowGlue::ALL));
    check_in("ru", &all, ("и⍽т.⍽д.", "и⍽т.⍽д."));
    check_in("ru", &all, ("т.⎵е.", "т.⎵е."));
    check_in("ru", &all, ("и⍽т.⎵д.", "и⍽т.⎵д."));
}

#[test]
fn abbreviations_report_their_glue() {
    let t = alone(ru(), ABBR);
    // `т.е.`: one insertion after `т.` (3 bytes).
    assert_eq!(
        edits(&t, "т.е."),
        [(ABBR, 3..3, "\u{a0}".to_owned())],
        "an inserted space is an empty range"
    );
    // `и т. д.`: the two spaces are single-character ranges.
    assert_eq!(
        edits(&t, "и т. д."),
        [
            (ABBR, 2..3, "\u{a0}".to_owned()),
            (ABBR, 6..7, "\u{a0}".to_owned())
        ]
    );
    let narrow = with(ru(), &[ABBR], |b| b.narrow_glue(NarrowGlue::ABBREVIATIONS));
    assert_eq!(
        edits(&narrow, "т.е."),
        [(ABBR, 3..3, "\u{202f}".to_owned())]
    );
}

// ---------------------------------------------------------------------------------------------
// nbsp.initials
// ---------------------------------------------------------------------------------------------

const RU_INITIALS: &[(&str, &str)] = &[
    ("А. С. Пушкин", "А.⍽С.⍽Пушкин"),
    ("И. Петров", "И.⍽Петров"),
    ("Пушкин А. С.", "Пушкин⍽А.⍽С."),
    ("Петров И.", "Петров⍽И."),
    ("А. Б. В. Иванов", "А.⍽Б.⍽В.⍽Иванов"),
    // A compact `А.С.` gets no inserted space; the space before the surname is glued.
    ("А.С. Пушкин", "А.С.⍽Пушкин"),
    // A single initial before a surname is glued wherever it stands (settled, design section 15.2,
    // item 11).
    ("Об этом сказал А. Петров.", "Об этом сказал А.⍽Петров."),
    ("Автор: А. Пушкин", "Автор: А.⍽Пушкин"),
    ("сказал А. Петров", "сказал А.⍽Петров"),
    ("от И. Петрова", "от И.⍽Петрова"),
    // Also a capital letter with a full stop that ends a sentence (design section 10.8).
    (
        "Выберите вариант А. После этого",
        "Выберите вариант А.⍽После этого",
    ),
    (
        "Выберите вариант Д. После этого",
        "Выберите вариант Д.⍽После этого",
    ),
    // Four or more initials are not a match anywhere in the run.
    ("А. Б. В. Г. Иванов", "А. Б. В. Г. Иванов"),
    // A surname is a capitalized word of at least two letters.
    ("А. петров", "А. петров"),
    ("А. Я", "А. Я"),
    // The script of the language only.
    ("J. R. R. Tolkien", "J. R. R. Tolkien"),
    ("J. Smith", "J. Smith"),
    // Spaces other than a single U+0020 are not glued.
    ("А.\tПетров", "А.\tПетров"),
    ("А.\nПетров", "А.\nПетров"),
    ("А.⍽Петров", "А.⍽Петров"),
];

#[test]
fn initials_in_russian() {
    alone_rows(ru(), INITIALS, RU_INITIALS);
    let t = alone(ru(), INITIALS);
    for text in ["А. С. Пушкин", "Пушкин А. С.", "вариант А. После"] {
        assert_changed_by("ru initials", &t, text, INITIALS);
    }
}

const EN_INITIALS: &[(&str, &str)] = &[
    ("J. R. R. Tolkien", "J.⍽R.⍽R.⍽Tolkien"),
    ("J. Smith", "J.⍽Smith"),
    ("Smith J.", "Smith⍽J."),
    ("J. R. R. R. Tolkien", "J. R. R. R. Tolkien"),
    (
        "I chose plan B. Then I left.",
        "I chose plan B.⍽Then I left.",
    ),
    (
        "I chose plan P. Then I left.",
        "I chose plan P.⍽Then I left.",
    ),
    // A single initial is glued after any word (design section 10.8).
    (
        "Neither did I. Then we left.",
        "Neither did I.⍽Then we left.",
    ),
    ("J. smith", "J. smith"),
    ("А. С. Пушкин", "А. С. Пушкин"),
];

#[test]
fn initials_in_english() {
    alone_rows(en(), INITIALS, EN_INITIALS);
}

#[test]
fn initials_with_narrow_glue() {
    let t = with(ru(), &[INITIALS], |b| b.narrow_glue(NarrowGlue::INITIALS));
    check_in("ru", &t, ("А. С. Пушкин", "А.⎵С.⎵Пушкин"));
    check_in("ru", &t, ("Пушкин А. С.", "Пушкин⎵А.⎵С."));
    check_in("ru", &t, ("А.⍽С.⍽Пушкин", "А.⍽С.⍽Пушкин"));
    let t = with(ru(), &[INITIALS], |b| {
        b.narrow_glue(NarrowGlue::ABBREVIATIONS)
    });
    check_in("ru", &t, ("А. С. Пушкин", "А.⍽С.⍽Пушкин"));
}

#[test]
fn initials_inside_both_sets() {
    both_sets(
        ru(),
        &[
            ("А. С. Пушкин", "А.⍽С.⍽Пушкин"),
            // `Об` is a listed short word.
            ("Об этом сказал А. Петров.", "Об⍽этом сказал А.⍽Петров."),
            (
                "Выберите вариант А. После этого",
                "Выберите вариант А.⍽После этого",
            ),
            (
                "Выберите вариант Д. После этого",
                "Выберите вариант Д.⍽После этого",
            ),
        ],
    );
    both_sets(
        en(),
        &[
            ("J. R. R. Tolkien", "J.⍽R.⍽R.⍽Tolkien"),
            (
                "I chose plan B. Then I left.",
                "I chose plan B.⍽Then I left.",
            ),
            (
                "I chose plan P. Then I left.",
                "I chose plan P.⍽Then I left.",
            ),
            ("J. R. R. R. Tolkien", "J. R. R. R. Tolkien"),
        ],
    );
    // `nbsp.abbreviations` alone does not take the one-letter forms.
    alone_rows(
        ru(),
        ABBR,
        &[(
            "Выберите вариант Д. После этого",
            "Выберите вариант Д. После этого",
        )],
    );
    // Interface: `от` is glued by `nbsp.short-words`, the initial by `nbsp.initials`.
    check_in(
        "ru Interface",
        &set(ru(), RuleSet::INTERFACE),
        ("от И. Петрова", "от⍽И.⍽Петрова"),
    );
}

#[test]
fn initials_with_abbreviations() {
    // Design section 8.7: `Mr. Smith` by abbreviations, `J. R. R. Tolkien` by initials.
    let t = only(en(), &[ABBR, INITIALS]);
    check_in("en", &t, ("Mr. Smith", "Mr.⍽Smith"));
    check_in("en", &t, ("J. R. R. Tolkien", "J.⍽R.⍽R.⍽Tolkien"));
}

#[test]
fn initials_are_inert_without_a_script() {
    let t = alone(Language::COMMON, INITIALS);
    check_unchanged("und", &t, "J. R. R. Tolkien");
    check_unchanged("und", &t, "А. С. Пушкин");
    assert!(t.active_rules().is_empty());
}

// ---------------------------------------------------------------------------------------------
// nbsp.dates
// ---------------------------------------------------------------------------------------------

const RU_DATES: &[(&str, &str)] = &[
    ("21 ноября", "21⍽ноября"),
    ("21 Ноября", "21⍽Ноября"),
    ("1 января", "1⍽января"),
    ("31 мая", "31⍽мая"),
    ("21 янв.", "21⍽янв."),
    ("21 нояб.", "21⍽нояб."),
    ("21 ноября 2026 г.", "21⍽ноября 2026⍽г."),
    // The day is 1 to 31, and `AfterDay` is the only order.
    ("32 мая", "32 мая"),
    ("0 мая", "0 мая"),
    ("ноября 21", "ноября 21"),
    // Year words that end in a full stop.
    ("2026 г.", "2026⍽г."),
    ("2026 гг.", "2026⍽гг."),
    ("1990–2000 гг.", "1990–2000⍽гг."),
    ("в 2010-11 гг.", "в 2010-11⍽гг."),
    ("21.11.2026 г.", "21.11.2026⍽г."),
    ("21.11.26 г.", "21.11.26⍽г."),
    ("01.02.26 г.", "01.02.26⍽г."),
    ("1.2.27 г.", "1.2.27⍽г."),
    ("0.2.27 г.", "0.2.27 г."),
    ("32.11.2026 г.", "32.11.2026 г."),
    (
        "Договор от 21.11.2026 г. Затем",
        "Договор от 21.11.2026⍽г. Затем",
    ),
    // A year word without a full stop is not a trigger of this rule.
    ("2026 году", "2026 году"),
    ("2026 г", "2026 г"),
    ("2026 год", "2026 год"),
    // Century words.
    ("XIX в.", "XIX⍽в."),
    ("XIX–XX вв.", "XIX–XX⍽вв."),
    ("5 в.", "5⍽в."),
    ("XIX век", "XIX век"),
    // Labels and percentages are not numbers here.
    ("Школа №5 г. Москвы", "Школа №5 г. Москвы"),
    ("№ 5 мая", "№ 5 мая"),
    ("§5 г.", "§5 г."),
    ("50% г.", "50% г."),
    // Whitespace other than a single U+0020 is not glued.
    ("21  ноября", "21  ноября"),
    ("21\tноября", "21\tноября"),
    ("21\nноября", "21\nноября"),
    ("21⍽ноября", "21⍽ноября"),
    // Design section 5.4: a soft hyphen or a ZWNJ inside the month word is skipped by the lookup.
    ("21 ноя\u{AD}бря", "21⍽ноя\u{AD}бря"),
    ("21 ноя\u{200C}бря", "21⍽ноя\u{200C}бря"),
    // Design section 3.6: a number split by a `Format` token is one number.
    ("2\u{AD}1 ноября", "2\u{AD}1⍽ноября"),
    ("2\u{AD}1.11.2026 г. Затем", "2\u{AD}1.11.2026⍽г. Затем"),
    ("21.11.20\u{AD}26 г. Затем", "21.11.20\u{AD}26⍽г. Затем"),
];

#[test]
fn dates_in_russian() {
    alone_rows(ru(), DATES, RU_DATES);
    let t = alone(ru(), DATES);
    for text in [
        "21 ноября",
        "2026 г.",
        "XIX в.",
        "21 ноя\u{AD}бря",
        "21 ноя\u{200C}бря",
    ] {
        assert_changed_by("ru dates", &t, text, DATES);
    }
}

#[test]
fn a_date_core_before_a_year_word_is_protected_as_a_version() {
    // Design section 13.2: `Договор от 21.11.2026 г. Затем` gets no glue after `г.`, and the date
    // is a `Technical(Version)` span (check 13 of design section 3.6) in both sets.
    const TEXT: &str = "Договор от 21.11.2026 г. Затем";
    let version = probe::tech(typolish::TechKind::Version);
    for (what, t, expected) in [
        (
            "Prose",
            set(ru(), RuleSet::PROSE),
            "Договор от⍽21.11.2026⍽г. Затем",
        ),
        (
            "Interface",
            set(ru(), RuleSet::INTERFACE),
            "Договор от⍽21.11.2026⍽г. Затем",
        ),
    ] {
        check_in(what, &t, (TEXT, expected));
        assert_eq!(
            probe::span_texts(&t, TEXT),
            [(version, "21.11.2026")],
            "{what}"
        );
    }
}

const EN_DATES: &[(&str, &str)] = &[
    ("November 21", "November⍽21"),
    ("21 November", "21⍽November"),
    ("Nov. 21", "Nov.⍽21"),
    ("Jan. 5", "Jan.⍽5"),
    ("5 Jan.", "5⍽Jan."),
    ("Sept. 5", "Sept.⍽5"),
    ("15 Jan", "15⍽Jan"),
    ("May 5", "May⍽5"),
    ("on 5 May", "on 5⍽May"),
    ("21 November, 2026", "21⍽November, 2026"),
    // `May` matches only as written; the day is 1 to 31.
    ("may 5", "may 5"),
    ("chapter 5 may be", "chapter 5 may be"),
    ("May 32", "May 32"),
    ("32 May", "32 May"),
    ("May 2026", "May 2026"),
    // English has no year or century words.
    ("2026 g.", "2026 g."),
    ("XIX c.", "XIX c."),
];

#[test]
fn dates_in_english() {
    alone_rows(en(), DATES, EN_DATES);
}

static BEFORE_DAY: LanguageData = LanguageData::EN_US
    .with_tag("en-XB")
    .with_month_position(MonthPosition::BeforeDay);
static AFTER_DAY: LanguageData = LanguageData::EN_US
    .with_tag("en-XA")
    .with_month_position(MonthPosition::AfterDay);

#[test]
fn dates_follow_the_month_position() {
    let before = alone(Language::from_data(&BEFORE_DAY).unwrap(), DATES);
    check_in("BeforeDay", &before, ("November 21", "November⍽21"));
    check_in("BeforeDay", &before, ("21 November", "21 November"));
    let after = alone(Language::from_data(&AFTER_DAY).unwrap(), DATES);
    check_in("AfterDay", &after, ("21 November", "21⍽November"));
    check_in("AfterDay", &after, ("November 21", "November 21"));
}

#[test]
fn dates_are_inert_without_months_and_year_words() {
    let t = alone(Language::COMMON, DATES);
    assert!(t.active_rules().is_empty());
    check_unchanged("und", &t, "21 ноября 2026 г.");
}

#[test]
fn dates_inside_both_sets() {
    both_sets(
        ru(),
        &[
            ("21 ноября 2026 г.", "21⍽ноября 2026⍽г."),
            ("Школа №5 г. Москвы", "Школа №⍽5 г.⍽Москвы"),
        ],
    );
    // Design section 8.7: short words, dates and units together.
    both_sets(
        en(),
        &[("On November 21, in 5 MB", "On⍽November⍽21, in⍽5⍽MB")],
    );
    // `on 5 May`: `on⍽5⍽May` in both sets (design section 13.2).
    both_sets(en(), &[("on 5 May", "on⍽5⍽May")]);
    // `2010-11 гг.`: Prose gives the range dash, Interface keeps the hyphen (design section 10.7).
    set_rows(
        ru(),
        &[
            ("2010-11 гг.", "2010–11⍽гг.", "2010-11⍽гг."),
            ("2026-11 гг.", "2026-11⍽гг.", "2026-11⍽гг."),
        ],
    );
}

// ---------------------------------------------------------------------------------------------
// nbsp.units
// ---------------------------------------------------------------------------------------------

const RU_UNITS: &[(&str, &str)] = &[
    ("5 МБ", "5⍽МБ"),
    ("30 с.", "30⍽с."),
    ("12 дн.", "12⍽дн."),
    ("5–10 кг", "5–10⍽кг"),
    ("5-10 кг", "5-10⍽кг"),
    ("5 ГБ/с", "5⍽ГБ/с"),
    ("60 км/ч", "60⍽км/ч"),
    ("Размер 5 МБ.", "Размер 5⍽МБ."),
    ("Привезли 5 т груза", "Привезли 5⍽т груза"),
    ("5 руб.", "5⍽руб."),
    ("5 ₽", "5⍽₽"),
    ("5 $", "5⍽$"),
    ("3 тыс.", "3⍽тыс."),
    ("20 °C", "20⍽°C"),
    ("20 °С", "20⍽°С"),
    ("20 °", "20⍽°"),
    ("5 В", "5⍽В"),
    ("1,5 кг", "1,5⍽кг"),
    // Forms typed without a superscript are units too; the digit is never changed.
    ("5 м2", "5⍽м2"),
    ("5–10 м2", "5–10⍽м2"),
    ("5 м2x", "5 м2x"),
    ("3 км2", "3⍽км2"),
    // An ambiguous unit is glued only before punctuation, a line break or the end.
    ("Ждать 30 с.", "Ждать 30⍽с."),
    ("30 с друзьями", "30 с друзьями"),
    ("30 с", "30⍽с"),
    ("30 с, потом", "30⍽с, потом"),
    ("30 с\nдалее", "30⍽с\nдалее"),
    // Only existing spaces; the list is case-sensitive; a unit is followed by a non-letter and a
    // non-digit.
    ("5МБ", "5МБ"),
    ("5 мб", "5 мб"),
    ("5 МБх", "5 МБх"),
    ("5 МБ1", "5 МБ1"),
    // Whitespace other than a single U+0020 is not glued.
    ("5  МБ", "5  МБ"),
    ("5\tМБ", "5\tМБ"),
    ("5\nМБ", "5\nМБ"),
    ("5⍽МБ", "5⍽МБ"),
    // A label and a percentage are not quantities.
    ("№ 5 МБ", "№ 5 МБ"),
    ("№5 МБ", "№5 МБ"),
    // The word before the number is not touched.
    ("Скидка 100 %.", "Скидка 100⍽%."),
    ("100 %", "100⍽%"),
    ("100%", "100%"),
];

#[test]
fn units_in_russian() {
    alone_rows(ru(), UNITS, RU_UNITS);
    let t = alone(ru(), UNITS);
    for text in ["5 МБ", "30 с.", "12 дн.", "5–10 кг", "100 %"] {
        assert_changed_by("ru units", &t, text, UNITS);
    }
}

const EN_UNITS: &[(&str, &str)] = &[
    ("5 MB", "5⍽MB"),
    ("5 kB", "5⍽kB"),
    ("100 MB/s", "100⍽MB/s"),
    ("27 in.", "27⍽in."),
    ("5 in the box", "5 in the box"),
    ("20 °C", "20⍽°C"),
    ("5 m²", "5⍽m²"),
    ("60 km/h", "60⍽km/h"),
    ("3 hr", "3⍽hr"),
    ("10 min.", "10⍽min."),
    ("5–10 kg", "5–10⍽kg"),
    ("5 mb", "5 mb"),
    ("5MB", "5MB"),
    ("5 MBs", "5 MBs"),
    ("100 %", "100⍽%"),
    ("100%", "100%"),
    ("5 МБ", "5 МБ"),
    ("№ 5 MB", "№ 5 MB"),
];

#[test]
fn units_in_english() {
    alone_rows(en(), UNITS, EN_UNITS);
}

#[test]
fn units_in_common() {
    let t = alone(Language::COMMON, UNITS);
    for row in [
        ("5 MB", "5⍽MB"),
        ("20 °C", "20⍽°C"),
        ("5 kHz", "5⍽kHz"),
        ("5 мб", "5 мб"),
        ("5 МБ", "5 МБ"),
        ("5 mb", "5 mb"),
        ("5MB", "5MB"),
    ] {
        check_in("und", &t, row);
    }
}

#[test]
fn units_by_setting() {
    let t = with(en(), &[UNITS], |b| b.add_units(["req/s", "vCPU"]));
    check_in("en", &t, ("5 req/s", "5⍽req/s"));
    check_in("en", &t, ("2 vCPU", "2⍽vCPU"));
    check_in("en", &t, ("5 MB", "5⍽MB"));
    let t = with(ru(), &[UNITS], |b| b.add_units(["ед."]));
    check_in("ru", &t, ("5 ед.", "5⍽ед."));
    let t = with(ru(), &[UNITS], |b| b.remove_units(["МБ"]));
    check_in("ru", &t, ("5 МБ", "5 МБ"));
    check_in("ru", &t, ("5 ГБ", "5⍽ГБ"));
}

#[test]
fn units_of_any_length() {
    // The unit list has no length limit (design sections 7.6 and 8.7): a long unit added by the
    // builder or by `LanguageBuilder` is matched whole, as the longest prefix followed by a
    // non-letter.
    let long: String = "абвгдежзийклмнопрстуфхцчшщъыьэюя".repeat(2);
    let short: String = long.chars().take(25).collect();
    for unit in [short.as_str(), long.as_str()] {
        let t = with(ru(), &[UNITS], |b| b.add_units([unit]));
        check_in(
            "ru add_units",
            &t,
            (&format!("5 {unit}"), &format!("5⍽{unit}")),
        );
        check_in(
            "ru add_units",
            &t,
            (&format!("5 {unit}."), &format!("5⍽{unit}.")),
        );
        // Not a prefix of a longer word.
        check_unchanged("ru add_units", &t, &format!("5 {unit}а"));
        let language = typolish::LanguageBuilder::from_language(&ru(), "ru-KZ")
            .units([unit])
            .ambiguous_units(Vec::<&str>::new())
            .build()
            .unwrap();
        let t = alone(language, UNITS);
        check_in(
            "LanguageBuilder",
            &t,
            (&format!("5 {unit}"), &format!("5⍽{unit}")),
        );
    }
    // The longest unit wins over a shorter one that is its prefix.
    let t = with(ru(), &[UNITS], |b| b.add_units([short.as_str(), "абв"]));
    check_in("ru", &t, (&format!("5 {short}"), &format!("5⍽{short}")));
    check_in("ru", &t, ("5 абв", "5⍽абв"));
    check_unchanged("ru", &t, "5 абвг");
}

/// Rows of `PercentSpacing` for one language: `(input, expected)` per setting.
type Percent = &'static [(&'static str, &'static str)];

#[test]
fn percent_spacing_in_russian() {
    use typolish::PercentSpacing::{AsWritten, Attached, Language as Lang, Spaced};
    let table: &[(typolish::PercentSpacing, Percent)] = &[
        (
            AsWritten,
            &[
                ("100 %", "100⍽%"),
                ("100%", "100%"),
                ("100⍽%", "100⍽%"),
                ("100  %", "100  %"),
                ("100‰", "100‰"),
                ("100 ‰", "100⍽‰"),
            ],
        ),
        (
            Lang,
            &[
                ("100%", "100⍽%"),
                ("100 %", "100⍽%"),
                ("100⍽%", "100⍽%"),
                ("100‰", "100⍽‰"),
                ("100 ‰", "100⍽‰"),
            ],
        ),
        (
            Attached,
            &[
                ("100 %", "100%"),
                ("100%", "100%"),
                // Only a single U+0020 is removed: typed glue, a tab and a run of spaces stay.
                ("100⍽%", "100⍽%"),
                ("100⎵%", "100⎵%"),
                ("100  %", "100  %"),
                ("100\t%", "100\t%"),
                ("100 ‰", "100‰"),
            ],
        ),
        (
            Spaced,
            &[
                ("100%", "100⍽%"),
                ("100 %", "100⍽%"),
                ("100⍽%", "100⍽%"),
                ("100‰", "100⍽‰"),
                ("Скидка 100%.", "Скидка 100⍽%."),
            ],
        ),
    ];
    for (spacing, rows) in table {
        let t = with(ru(), &[UNITS], |b| b.percent(*spacing));
        for row in *rows {
            check_in(&format!("ru {spacing:?}"), &t, *row);
        }
    }
}

#[test]
fn percent_spacing_in_english() {
    use typolish::PercentSpacing::{AsWritten, Attached, Language as Lang, Spaced};
    let table: &[(typolish::PercentSpacing, Percent)] = &[
        (
            AsWritten,
            &[("100 %", "100⍽%"), ("100%", "100%"), ("100‰", "100‰")],
        ),
        (
            // The CLDR pattern of English has no space; only a single U+0020 is removed.
            Lang,
            &[
                ("100 %", "100%"),
                ("100%", "100%"),
                ("100⍽%", "100⍽%"),
                ("100  %", "100  %"),
                ("100 ‰", "100‰"),
            ],
        ),
        (
            Attached,
            &[("100 %", "100%"), ("100⍽%", "100⍽%"), ("100%", "100%")],
        ),
        (
            Spaced,
            &[("100%", "100⍽%"), ("100 %", "100⍽%"), ("100‰", "100⍽‰")],
        ),
    ];
    for (spacing, rows) in table {
        let t = with(en(), &[UNITS], |b| b.percent(*spacing));
        for row in *rows {
            check_in(&format!("en {spacing:?}"), &t, *row);
        }
    }
}

#[test]
fn percent_spacing_inserts_and_removes_through_report() {
    let t = with(ru(), &[UNITS], |b| {
        b.percent(typolish::PercentSpacing::Spaced)
    });
    // `100%`: an insertion after the `0`.
    assert_eq!(
        edits(&t, "100%"),
        [(UNITS, 3..3, "\u{a0}".to_owned())],
        "an inserted space"
    );
    let t = with(en(), &[UNITS], |b| {
        b.percent(typolish::PercentSpacing::Attached)
    });
    // `100 %`: a removal of the space at 3..4.
    assert_eq!(edits(&t, "100 %"), [(UNITS, 3..4, String::new())]);
}

#[test]
fn percent_spacing_of_a_range() {
    // A range is a quantity (design section 8.2): the percent setting inserts the space before an
    // attached sign as it glues or removes a typed one (design section 8.7), and the output is
    // stable.
    use typolish::PercentSpacing::{Attached, Language as Lang, Spaced};
    let rows: &[(Language, typolish::PercentSpacing, &str, &str)] = &[
        (ru(), Spaced, "5–10%", "5–10⍽%"),
        (ru(), Spaced, "5–10 %", "5–10⍽%"),
        (ru(), Spaced, "5–10‰", "5–10⍽‰"),
        (ru(), Lang, "5–10%", "5–10⍽%"),
        (ru(), Lang, "Рост на 5–10%.", "Рост на 5–10⍽%."),
        (ru(), Attached, "5–10 %", "5–10%"),
        (ru(), Attached, "5–10%", "5–10%"),
        (en(), Spaced, "5–10%", "5–10⍽%"),
        (en(), Lang, "5–10%", "5–10%"),
        (en(), Lang, "5–10 %", "5–10%"),
    ];
    for (language, spacing, input, expected) in rows {
        let t = with(language.clone(), &[UNITS], |b| b.percent(*spacing));
        let what = format!("{language} {spacing:?}");
        check_in(&what, &t, (input, expected));
        check_unchanged(&format!("{what}, second pass"), &t, expected);
    }
    // A template range of two `Number` slots: the sign is in the last text.
    let pieces = || {
        vec![
            Piece::Slot("a", Slot::NUMBER),
            Piece::Text("–"),
            Piece::Slot("b", Slot::NUMBER),
            Piece::Text("%"),
        ]
    };
    for (language, spacing, last) in [
        (ru(), Spaced, "\u{a0}%"),
        (ru(), Lang, "\u{a0}%"),
        (en(), Lang, "%"),
    ] {
        let t = with(language.clone(), &[UNITS], |b| b.percent(spacing));
        assert_eq!(
            texts(&t, pieces()),
            ["", "–", last],
            "{language} {spacing:?}"
        );
    }
}

#[test]
fn percent_spacing_and_the_dash_rules() {
    // A percentage is one in both forms (design section 8.2, invariant I2 of design section 4.3):
    // the dash rules read a percent sign after spaces with its number, so the space that the
    // setting inserts or removes changes nothing for them, and a second pass returns the output.
    use typolish::PercentSpacing::{AsWritten, Attached, Spaced};
    let rows: &[(typolish::PercentSpacing, &str, &str)] = &[
        (AsWritten, "100 % - норма", "100⍽%⍽— норма"),
        (AsWritten, "100% - норма", "100%⍽— норма"),
        (Attached, "100 % - норма", "100%⍽— норма"),
        (Spaced, "100% - норма", "100⍽%⍽— норма"),
        (Spaced, "5–10% - норма", "5–10⍽%⍽— норма"),
        (Attached, "5–10 % - норма", "5–10%⍽— норма"),
        (Spaced, "Рост - 5–10% за год", "Рост⍽— 5–10⍽% за⍽год"),
        (AsWritten, "Рост - 5–10% за год", "Рост⍽— 5–10% за⍽год"),
        // Numbers on both sides are refused in both forms.
        (Attached, "5 % - 10 %", "5% - 10%"),
        (Spaced, "5% - 10%", "5⍽% - 10⍽%"),
        (Spaced, "5 - 5–10%", "5 - 5–10⍽%"),
    ];
    for (spacing, input, expected) in rows {
        let t = Typographer::builder(ru(), RuleSet::PROSE)
            .percent(*spacing)
            .build()
            .unwrap();
        let what = format!("ru Prose {spacing:?}");
        check_in(&what, &t, (input, expected));
        check_unchanged(&format!("{what}, second pass"), &t, expected);
    }
}

#[test]
fn percent_spacing_with_a_label_number() {
    // Design sections 8.2 and 8.7: a label number counts as a number for the percent handling of
    // `nbsp.units`, as it does for the dash rules; `№5%` and `§5%` are frozen by check 24 of
    // design section 3.6 and stay in every setting.
    use typolish::PercentSpacing::{AsWritten, Attached, Language as Lang, Spaced};
    let rows: &[(Language, typolish::PercentSpacing, &str, &str)] = &[
        (ru(), AsWritten, "№ 5 %", "№⍽5⍽%"),
        (ru(), AsWritten, "№5 %", "№⍽5⍽%"),
        (ru(), AsWritten, "§ 5 %", "§⍽5⍽%"),
        (ru(), AsWritten, "№ 5%", "№⍽5%"),
        (ru(), AsWritten, "№5 % - норма", "№⍽5⍽%⍽— норма"),
        (ru(), Attached, "№ 5 %", "№⍽5%"),
        (ru(), Attached, "№5 %", "№⍽5%"),
        (ru(), Spaced, "№ 5%", "№⍽5⍽%"),
        (ru(), Lang, "№ 5%", "№⍽5⍽%"),
        (en(), Lang, "№ 5 %", "№⍽5%"),
        (ru(), AsWritten, "№ 5 МБ", "№⍽5 МБ"),
    ];
    for (language, spacing, input, expected) in rows {
        let t = Typographer::builder(language.clone(), RuleSet::PROSE)
            .percent(*spacing)
            .build()
            .unwrap();
        let what = format!("{language} Prose {spacing:?}");
        check_in(&what, &t, (input, expected));
        check_unchanged(&format!("{what}, second pass"), &t, expected);
    }
    for spacing in [AsWritten, Attached, Spaced, Lang] {
        for language in [ru(), en()] {
            let t = Typographer::builder(language.clone(), RuleSet::PROSE)
                .percent(spacing)
                .build()
                .unwrap();
            for text in ["№5%", "§5%"] {
                check_unchanged(&format!("{language} Prose {spacing:?}"), &t, text);
            }
        }
    }
    let t = Typographer::builder(ru(), RuleSet::PROSE)
        .percent(AsWritten)
        .disable(Rule::NbspSigns)
        .build()
        .unwrap();
    check_in("ru Prose without nbsp.signs", &t, ("№5 %", "№5⍽%"));
}

#[test]
fn percent_spacing_in_the_sets() {
    // With a non-default setting `nbsp.units` also inserts and removes in Interface (design section
    // 9.1).
    for set_ in [RuleSet::PROSE, RuleSet::INTERFACE] {
        let t = Typographer::builder(ru(), set_)
            .percent(typolish::PercentSpacing::Spaced)
            .build()
            .unwrap();
        check_in("ru", &t, ("Скидка 100%.", "Скидка 100⍽%."));
        let t = Typographer::builder(en(), set_)
            .percent(typolish::PercentSpacing::Attached)
            .build()
            .unwrap();
        check_in("en", &t, ("Discount 100 %.", "Discount 100%."));
    }
}

#[test]
fn units_inside_both_sets() {
    both_sets(
        ru(),
        &[
            ("Размер 5 МБ.", "Размер 5⍽МБ."),
            ("Ждать 30 с.", "Ждать 30⍽с."),
            ("60 км/ч", "60⍽км/ч"),
            ("Скидка 100 %.", "Скидка 100⍽%."),
            ("5 м2", "5⍽м2"),
            // `с` is also a listed short word, so `nbsp.short-words` glues it forward.
            ("30 с друзьями", "30 с⍽друзьями"),
        ],
    );
    both_sets(
        en(),
        &[
            ("5 MB", "5⍽MB"),
            ("27 in.", "27⍽in."),
            ("5 in the box", "5 in⍽the⍽box"),
        ],
    );
}

#[test]
fn units_are_off_when_disabled() {
    for base in [RuleSet::PROSE, RuleSet::INTERFACE] {
        let t = set(ru(), base.without(UNITS));
        check_in("ru", &t, ("Размер 5 МБ.", "Размер 5 МБ."));
    }
}

// ---------------------------------------------------------------------------------------------
// nbsp.signs
// ---------------------------------------------------------------------------------------------

const SIGN_ROWS: &[(&str, &str)] = &[
    ("Документ № 5", "Документ №⍽5"),
    ("№5", "№⍽5"),
    ("Документ №5", "Документ №⍽5"),
    ("Номер № 5.", "Номер №⍽5."),
    ("§5", "§⍽5"),
    ("§ 12", "§⍽12"),
    ("¶5", "¶⍽5"),
    ("¶ 5", "¶⍽5"),
    ("¶IV", "¶⍽IV"),
    ("¶XIX", "¶⍽XIX"),
    ("¶ IV", "¶⍽IV"),
    ("¶VV", "¶VV"),
    ("¶ VV", "¶ VV"),
    // A `Format` token inside a Roman numeral is skipped (design section 3.6).
    ("§ I\u{AD}V", "§⍽I\u{AD}V"),
    ("¶ X\u{AD}IX", "¶⍽X\u{AD}IX"),
    // Doubled signs count as one.
    ("№№ 5", "№№⍽5"),
    ("§§ 3", "§§⍽3"),
    ("№ 5 МБ", "№⍽5 МБ"),
    // Not before a word, not alone, not across other whitespace.
    ("№ абв", "№ абв"),
    ("№", "№"),
    ("№\t5", "№\t5"),
    ("№  5", "№  5"),
    ("№\n5", "№\n5"),
    ("№⍽5", "№⍽5"),
    ("№⎵5", "№⎵5"),
    // Never before a fragment that starts with `-` (design section 8.2).
    ("№ -5", "№ -5"),
    ("§ -1", "§ -1"),
];

#[test]
fn signs_in_both_built_in_languages_and_common() {
    for language in [ru(), en(), Language::COMMON] {
        alone_rows(language.clone(), SIGN, SIGN_ROWS);
        let t = alone(language, SIGN);
        assert_changed_by("signs", &t, "№ 5", SIGN);
        assert_changed_by("signs", &t, "¶IV", SIGN);
    }
    alone_rows(en(), SIGN, &[("See § 5", "See §⍽5"), ("¶ 5", "¶⍽5")]);
}

const SIGN: Rule = SIGNS;

#[test]
fn signs_are_not_borrowed_when_a_space_is_inserted() {
    let t = alone(ru(), SIGN);
    for text in ["¶IV", "¶XIX", "№5"] {
        let out = t.typeset(text, Output::Unicode);
        assert!(matches!(out, std::borrow::Cow::Owned(_)), "{text}");
    }
    assert_eq!(edits(&t, "№5"), [(SIGN, 3..3, "\u{a0}".to_owned())]);
    assert_eq!(edits(&t, "№ 5"), [(SIGN, 3..4, "\u{a0}".to_owned())]);
}

#[test]
fn signs_with_narrow_glue() {
    let t = with(ru(), &[SIGN], |b| b.narrow_glue(NarrowGlue::SIGNS));
    check_in("ru", &t, ("№ 5", "№⎵5"));
    check_in("ru", &t, ("№5", "№⎵5"));
    check_in("ru", &t, ("§ 12", "§⎵12"));
    check_in("ru", &t, ("¶IV", "¶⎵IV"));
    check_in("ru", &t, ("№⍽5", "№⍽5"));
    let t = with(ru(), &[SIGN], |b| b.narrow_glue(NarrowGlue::INITIALS));
    check_in("ru", &t, ("№ 5", "№⍽5"));
    assert_eq!(
        edits(
            &with(ru(), &[SIGN], |b| b.narrow_glue(NarrowGlue::ALL)),
            "№5"
        ),
        [(SIGN, 3..3, "\u{202f}".to_owned())]
    );
}

#[test]
fn signs_inside_both_sets() {
    both_sets(
        ru(),
        &[
            ("Документ № 5", "Документ №⍽5"),
            ("№5", "№⍽5"),
            ("№\t5", "№\t5"),
        ],
    );
    // No glue before a fragment that starts with `-` (design section 8.2) in either set; in Prose
    // `number.minus` turns the hyphen into the minus sign (design section 8.8), and the glue stays
    // off.
    set_rows(
        ru(),
        &[
            ("№ -5", "№ \u{2212}5", "№ -5"),
            ("§ -1", "§ \u{2212}1", "§ -1"),
        ],
    );
    // A short word before a sign (design section 8.7, `nbsp.short-words`): the sign is a next
    // token.
    both_sets(
        ru(),
        &[
            ("в §5", "в⍽§⍽5"),
            ("в ¶5", "в⍽¶⍽5"),
            ("и ¶ 5", "и⍽¶⍽5"),
            ("¶IV", "¶⍽IV"),
            ("¶VV", "¶VV"),
        ],
    );
    both_sets(en(), &[("§ 5", "§⍽5")]);
}

// ---------------------------------------------------------------------------------------------
// nbsp.phone
// ---------------------------------------------------------------------------------------------

const PHONE_ROWS: &[(&str, &str)] = &[
    ("+7 495 123-45-67", "+7⍽495⍽123-45-67"),
    ("+7 495 123-4567", "+7⍽495⍽123-4567"),
    ("8 (800) 555-35-35", "8⍽(800)⍽555-35-35"),
    ("(495) 123-45-67", "(495)⍽123-45-67"),
    ("(495) 123 4567", "(495)⍽123⍽4567"),
    ("+1 300 200 100 50", "+1⍽300⍽200⍽100⍽50"),
    ("+7 495 123 45 67", "+7⍽495⍽123⍽45⍽67"),
    ("+123 4567 8901 2345", "+123⍽4567⍽8901⍽2345"),
    (
        "Звоните +7 495 123-45-67 сегодня",
        "Звоните +7⍽495⍽123-45-67 сегодня",
    ),
    ("Телефон: +7 495 123-45-67.", "Телефон: +7⍽495⍽123-45-67."),
    // A space the author already glued is read as done; the other spaces are glued.
    ("+7⍽495 123-45-67", "+7⍽495⍽123-45-67"),
    // Too few digits (below 10), too many (above 15), too many fragments (more than four after
    // the first), a group of more than four digits.
    ("+7 495 12", "+7 495 12"),
    ("+7 495", "+7 495"),
    ("(495) 123 456", "(495) 123 456"),
    ("+7 495 123 45 67 89", "+7 495 123 45 67 89"),
    ("+7 4951 1234 5678 9012", "+7 4951 1234 5678 9012"),
    ("+7 49500 12345", "+7 49500 12345"),
    // Only single spaces.
    ("+7  495 123-45-67", "+7  495 123-45-67"),
    ("+7\t495 123-45-67", "+7\t495 123-45-67"),
    ("+7 495\n123-45-67", "+7 495\n123-45-67"),
    // Not a telephone number.
    ("5 мая 2026", "5 мая 2026"),
    ("1 2 3", "1 2 3"),
    // A percentage is not a group, in both of its forms (design section 8.2).
    ("+7 495 123-45-67 100%", "+7⍽495⍽123-45-67 100%"),
    ("+7 495 123-45-67 100 %", "+7⍽495⍽123-45-67 100 %"),
    ("+7 495 123-45-67 100⍽%", "+7⍽495⍽123-45-67 100⍽%"),
    // A number split by a `Format` token is one number (design section 3.6).
    ("+7 (80\u{AD}0) 123-45-67", "+7⍽(80\u{AD}0)⍽123-45-67"),
    ("+7 800 12\u{AD}3-45-67", "+7⍽800⍽12\u{AD}3-45-67"),
    ("+3\u{AD}8 044 123-45-67", "+3\u{AD}8⍽044⍽123-45-67"),
];

#[test]
fn phone_numbers_in_every_built_in_language() {
    for language in [ru(), en(), Language::COMMON] {
        alone_rows(language.clone(), PHONE, PHONE_ROWS);
        let t = alone(language, PHONE);
        assert_changed_by("phone", &t, "+7 495 123-45-67", PHONE);
    }
}

#[test]
fn phone_numbers_in_the_sets() {
    // Prose has the rule and `range.digits`, Interface neither (design section 8.7, 10.7).
    for language in [ru(), en()] {
        set_rows(
            language.clone(),
            &[
                ("+7 495 123-45-67", "+7⍽495⍽123-45-67", "+7 495 123-45-67"),
                ("+7 495 123-4567", "+7⍽495⍽123–4567", "+7 495 123-4567"),
                ("123-4567", "123–4567", "123-4567"),
                (
                    "+1 300 200 100 50",
                    "+1⍽300⍽200⍽100⍽50",
                    "+1 300 200 100 50",
                ),
                (
                    "8 (800) 555-35-35",
                    "8⍽(800)⍽555-35-35",
                    "8 (800) 555-35-35",
                ),
            ],
        );
        // A number typed with U+00A0 forms a nowrap group in every set: nothing to change.
        for t in [
            set(language.clone(), RuleSet::PROSE),
            set(language.clone(), RuleSet::INTERFACE),
        ] {
            check_unchanged("typed", &t, "+7⍽495⍽123-45-67");
        }
        assert!(
            !set(language.clone(), RuleSet::INTERFACE)
                .rules()
                .contains(PHONE)
        );
    }
}

#[test]
fn phone_reads_a_percentage_alike_in_both_forms() {
    // The percent setting of ru glues `100%` into `100⍽%`; the number stays outside the telephone
    // number in both forms, so a second pass changes nothing (design section 8.2, invariant I2).
    let t = Typographer::builder(ru(), RuleSet::PROSE)
        .percent(PercentSpacing::Language)
        .build()
        .unwrap();
    for text in ["+7 495 123-45-67 100%", "+7 495 123-45-67 100 %"] {
        check_in("ru Prose", &t, (text, "+7⍽495⍽123-45-67 100⍽%"));
        let once = t.typeset(text, Output::Unicode).into_owned();
        assert_eq!(
            t.typeset(&once, Output::Unicode),
            once,
            "idempotence: {text}"
        );
    }
}

#[test]
fn phone_is_off_when_disabled() {
    let t = set(ru(), RuleSet::PROSE.without(PHONE));
    check_in("ru", &t, ("+7 495 123-45-67", "+7 495 123-45-67"));
}

// ---------------------------------------------------------------------------------------------
// nbsp.short-words
// ---------------------------------------------------------------------------------------------

const RU_SHORT: &[(&str, &str)] = &[
    ("в 2026 году", "в⍽2026 году"),
    ("и в Москве", "и⍽в⍽Москве"),
    ("В Москве", "В⍽Москве"),
    ("(в 2026)", "(в⍽2026)"),
    ("«в Москве»", "«в⍽Москве»"),
    ("Он вызвал ls на сервере", "Он вызвал ls на⍽сервере"),
    ("за «Москву»", "за⍽«Москву»"),
    ("на (Москве)", "на⍽(Москве)"),
    ("в \"Москве\"", "в⍽\"Москве\""),
    ("в `x`", "в⍽`x`"),
    ("в tls-server", "в⍽tls-server"),
    ("и don't", "и⍽don't"),
    ("в host:8080", "в⍽host:8080"),
    ("в C:\\Users\\x", "в⍽C:\\Users\\x"),
    ("в 192.168.1.1", "в⍽192.168.1.1"),
    ("в §5", "в⍽§5"),
    ("и ¶ 5", "и⍽¶ 5"),
    // The next fragment must start with a letter, a digit, a slot, a protected span, an opening
    // quote or bracket, or a sign.
    ("в /etc/hosts", "в /etc/hosts"),
    ("в ~/.config", "в ~/.config"),
    ("в ::1", "в ::1"),
    ("в …", "в …"),
    ("в -v", "в -v"),
    ("в --force", "в --force"),
    // A word followed by closing punctuation triggers nothing.
    ("и, потом", "и, потом"),
    ("в. 5", "в. 5"),
    // The trigger is a whole core; script of the language only.
    ("to ls", "to ls"),
    ("a b", "a b"),
    // Other whitespace is not glued.
    ("в\tМоскве", "в\tМоскве"),
    ("в  Москве", "в  Москве"),
    ("в\nМоскве", "в\nМоскве"),
    ("в⍽Москве", "в⍽Москве"),
    ("в⎵Москве", "в⎵Москве"),
];

#[test]
fn short_words_in_russian() {
    alone_rows(ru(), SHORT_WORDS, RU_SHORT);
    let t = alone(ru(), SHORT_WORDS);
    assert_changed_by("ru short words", &t, "в Москве", SHORT_WORDS);
    // The list: every listed word glues, a few unlisted ones do not.
    for word in [
        "а", "без", "близ", "в", "вне", "во", "да", "для", "до", "если", "за", "и", "из", "изо",
        "или", "к", "как", "ко", "на", "над", "не", "ни", "но", "о", "об", "обо", "от", "ото",
        "по", "под", "подо", "при", "про", "с", "со", "то", "у",
    ] {
        check_in(
            "ru list",
            &t,
            (&format!("{word} дом"), &format!("{word}⍽дом")),
        );
    }
    for word in ["это", "вот", "уж", "мы", "он"] {
        check_unchanged("ru not listed", &t, &format!("{word} дом"));
    }
}

const EN_SHORT: &[(&str, &str)] = &[
    ("the 90's", "the⍽90's"),
    ("of don't", "of⍽don't"),
    ("He went to the shop", "He went to⍽the⍽shop"),
    ("The Cat", "The⍽Cat"),
    ("in 5 min", "in⍽5 min"),
    ("at the end.", "at⍽the⍽end."),
    ("a \"cat\"", "a⍽\"cat\""),
    ("on 5 May", "on⍽5 May"),
    ("Run X now.", "Run X now."),
    ("и вот", "и вот"),
    ("if a<b and c>d", "if⍽a<b and⍽c>d"),
    ("the -v", "the -v"),
    ("the  cat", "the  cat"),
    ("the, cat", "the, cat"),
];

#[test]
fn short_words_in_english() {
    alone_rows(en(), SHORT_WORDS, EN_SHORT);
    let t = alone(en(), SHORT_WORDS);
    for word in [
        "a", "an", "and", "as", "at", "but", "by", "for", "if", "in", "nor", "not", "of", "off",
        "on", "or", "per", "so", "the", "to", "up", "via", "yet",
    ] {
        check_in(
            "en list",
            &t,
            (&format!("{word} dog"), &format!("{word}⍽dog")),
        );
    }
    for word in ["I", "we", "is", "he", "it"] {
        check_unchanged("en not listed", &t, &format!("{word} dog"));
    }
    // An apostrophe of step 1 is no quote, plain or typed (design sections 6.2 and 8.2).
    check_unchanged("en apostrophe", &t, "the 'tis x");
    check_unchanged("en apostrophe", &t, "the ’tis x");
}

#[test]
fn short_words_by_setting() {
    let t = with(ru(), &[SHORT_WORDS], |b| b.add_short_words(["через"]));
    check_in("ru", &t, ("через лес", "через⍽лес"));
    check_in("ru", &t, ("в лес", "в⍽лес"));
    let t = with(ru(), &[SHORT_WORDS], |b| b.remove_short_words(["в"]));
    check_in("ru", &t, ("в Москве", "в Москве"));
    check_in("ru", &t, ("и в Москве", "и⍽в Москве"));
    // A word in the wrong script is skipped unless `short_words_any_script(true)`.
    let t = with(ru(), &[SHORT_WORDS], |b| b.add_short_words(["to"]));
    check_in("ru", &t, ("to ls", "to ls"));
    let t = with(ru(), &[SHORT_WORDS], |b| {
        b.add_short_words(["to"]).short_words_any_script(true)
    });
    check_in("ru", &t, ("to ls", "to⍽ls"));
    let t = with(en(), &[SHORT_WORDS], |b| b.add_short_words(["и"]));
    check_in("en", &t, ("и вот", "и вот"));
    let t = with(en(), &[SHORT_WORDS], |b| {
        b.add_short_words(["и"]).short_words_any_script(true)
    });
    check_in("en", &t, ("и вот", "и⍽вот"));
    // The default list of the language still works with the setting on.
    check_in("en", &t, ("the cat", "the⍽cat"));
}

#[test]
fn a_word_in_nfd_does_not_match_a_list() {
    // Design section 5.5: the crate does not normalize; `и` + U+0306 is not `й`, so the listed
    // word `май` written in NFD gets no glue and the text comes back unchanged.
    let t = with(ru(), &[SHORT_WORDS], |b| b.add_short_words(["май"]));
    check_in("ru", &t, ("май слово", "май⍽слово"));
    check_unchanged("ru", &t, "ма\u{438}\u{306} слово");
    check_unchanged("ru", &t, "Ма\u{438}\u{306} слово");
}

#[test]
fn a_unit_in_nfd_does_not_match_the_list_and_short_length_reads_no_list() {
    // Design section 5.5: a list entry in NFD does not match; a rule without a list applies.
    let p = set(ru(), RuleSet::PROSE);
    check_in("ru prose", &p, ("5 дюйм.", "5⍽дюйм."));
    check_in("ru prose", &p, ("5 дюи\u{306}м.", "5 дюи\u{306}м."));
    check_in("ru prose", &p, ("и\u{306} слово", "и\u{306}⍽слово"));
    check_in("ru prose", &p, ("й слово", "й⍽слово"));
}

#[test]
fn short_words_never_glue_before_a_flag() {
    // `ls -la` stays whatever the lists say (design section 8.2).
    let t = with(ru(), &[SHORT_WORDS], |b| {
        b.add_short_words(["ls"]).short_words_any_script(true)
    });
    check_in("ru", &t, ("ls -la", "ls -la"));
    check_in("ru", &t, ("ls --all", "ls --all"));
    check_in("ru", &t, ("ls dir", "ls⍽dir"));
}

#[test]
fn short_words_are_not_a_trigger_inside_a_joined_core() {
    // `Moscow—the capital` is one `Joined` fragment: `the` triggers nothing (design section 4.3,
    // I2).
    check_in(
        "en",
        &alone(en(), SHORT_WORDS),
        ("Moscow—the capital", "Moscow—the capital"),
    );
}

#[test]
fn short_words_inside_both_sets() {
    both_sets(
        ru(),
        &[
            ("в 2026 году", "в⍽2026 году"),
            ("и в Москве", "и⍽в⍽Москве"),
            ("(в 2026)", "(в⍽2026)"),
            ("в \"Москве\"", "в⍽«Москве»"),
            ("в /etc/hosts", "в /etc/hosts"),
        ],
    );
    both_sets(
        en(),
        &[
            ("He went to the shop", "He went to⍽the⍽shop"),
            ("Run X now.", "Run X now."),
        ],
    );
    // Design section 8.7: Interface has no `nbsp.short-length`.
    set_rows(
        ru(),
        &[(
            "Он вызвал ls на сервере",
            "Он⍽вызвал ls на⍽сервере",
            "Он вызвал ls на⍽сервере",
        )],
    );
}

#[test]
fn short_words_are_off_when_disabled() {
    // In Prose `nbsp.short-length` still glues the one-letter `в` (design section 8.7); a listed
    // word of three letters shows that `nbsp.short-words` is off.
    for (base, glued) in [
        (RuleSet::PROSE, "в⍽2026 году"),
        (RuleSet::INTERFACE, "в 2026 году"),
    ] {
        let t = set(ru(), base.without(SHORT_WORDS));
        check_in("ru", &t, ("в 2026 году", glued));
        check_in("ru", &t, ("для дома", "для дома"));
    }
}

#[test]
fn short_words_report_their_glue() {
    let t = alone(ru(), SHORT_WORDS);
    // `в` is two bytes, the space is at 2..3.
    assert_eq!(
        edits(&t, "в Москве"),
        [(SHORT_WORDS, 2..3, "\u{a0}".to_owned())]
    );
    // Narrow glue never reaches this rule.
    let all = with(ru(), &[SHORT_WORDS], |b| b.narrow_glue(NarrowGlue::ALL));
    check_in("ru", &all, ("в Москве", "в⍽Москве"));
}

// ---------------------------------------------------------------------------------------------
// nbsp.short-length
// ---------------------------------------------------------------------------------------------

const RU_LENGTH: &[(&str, &str)] = &[
    ("Он пошёл", "Он⍽пошёл"),
    ("Мы и они", "Мы⍽и⍽они"),
    ("Я там", "Я⍽там"),
    ("Ёж бежит", "Ёж⍽бежит"),
    ("Он 5 раз", "Он⍽5 раз"),
    ("в Москве", "в⍽Москве"),
    // Three letters are too long by default.
    ("Они пошли", "Они пошли"),
    // Latin words do not count (script of the language).
    ("Он вызвал ls на сервере", "Он⍽вызвал ls на⍽сервере"),
    ("go там", "go там"),
    ("db там", "db там"),
    // The usual conditions.
    ("Он, пошёл", "Он, пошёл"),
    ("Он -x", "Он -x"),
    ("Он\tпошёл", "Он\tпошёл"),
    ("Он⍽пошёл", "Он⍽пошёл"),
];

#[test]
fn short_length_in_russian() {
    alone_rows(ru(), SHORT_LENGTH, RU_LENGTH);
    let t = alone(ru(), SHORT_LENGTH);
    assert_changed_by("ru short length", &t, "Он пошёл", SHORT_LENGTH);
}

#[test]
fn short_length_by_setting() {
    let t = with(ru(), &[SHORT_LENGTH], |b| b.short_word_length(3));
    check_in("ru 3", &t, ("Они пошли", "Они⍽пошли"));
    check_in("ru 3", &t, ("Они дружно", "Они⍽дружно"));
    let t = with(ru(), &[SHORT_LENGTH], |b| b.short_word_length(1));
    check_in("ru 1", &t, ("Я пошёл", "Я⍽пошёл"));
    check_in("ru 1", &t, ("Он пошёл", "Он пошёл"));
    // 0 makes the rule inert.
    let t = with(ru(), &[SHORT_LENGTH], |b| b.short_word_length(0));
    check_in("ru 0", &t, ("Он пошёл", "Он пошёл"));
    assert!(!t.active_rules().contains(SHORT_LENGTH));
    // English has the rule inert by default and active with a length.
    let t = alone(en(), SHORT_LENGTH);
    assert!(!t.active_rules().contains(SHORT_LENGTH));
    check_in("en default", &t, ("He is here", "He is here"));
    let t = with(en(), &[SHORT_LENGTH], |b| b.short_word_length(2));
    assert!(t.active_rules().contains(SHORT_LENGTH));
    check_in("en 2", &t, ("He is here", "He⍽is⍽here"));
    check_in("en 2", &t, ("Run now", "Run now"));
    // Any script.
    let t = with(ru(), &[SHORT_LENGTH], |b| b.short_words_any_script(true));
    check_in(
        "ru any script",
        &t,
        ("Он вызвал ls на сервере", "Он⍽вызвал ls⍽на⍽сервере"),
    );
}

#[test]
fn short_length_inside_the_sets() {
    // Prose only (design section 9.2).
    set_rows(
        ru(),
        &[
            ("Он пошёл домой", "Он⍽пошёл домой", "Он пошёл домой"),
            // `и` is also a short word: Interface glues it forward.
            ("Мы и они", "Мы⍽и⍽они", "Мы и⍽они"),
            ("Они пошли", "Они пошли", "Они пошли"),
        ],
    );
    assert!(!set(ru(), RuleSet::INTERFACE).rules().contains(SHORT_LENGTH));
    let t = set(ru(), RuleSet::PROSE.without(SHORT_LENGTH));
    check_in("ru", &t, ("Он пошёл домой", "Он пошёл домой"));
}

// ---------------------------------------------------------------------------------------------
// nbsp.particles
// ---------------------------------------------------------------------------------------------

const RU_PARTICLES: &[(&str, &str)] = &[
    ("сказал бы", "сказал⍽бы"),
    ("Знаешь ли", "Знаешь⍽ли"),
    ("Пойдёшь ли", "Пойдёшь⍽ли"),
    ("кто-то же", "кто-то⍽же"),
    ("Сказал Бы", "Сказал⍽Бы"),
    ("Как бы то ни было", "Как⍽бы то ни было"),
    ("он ж", "он⍽ж"),
    ("я б", "я⍽б"),
    ("ждал ль", "ждал⍽ль"),
    // Closing punctuation after the particle does not matter (design section 15.2, item 19).
    ("Знаешь ли, что", "Знаешь⍽ли, что"),
    ("Что ж,", "Что⍽ж,"),
    ("Что ж, ладно.", "Что⍽ж, ладно."),
    ("Что ж?", "Что⍽ж?"),
    ("Знаешь ли...", "Знаешь⍽ли..."),
    ("«Знаешь ли» ты", "«Знаешь⍽ли» ты"),
    ("(сказал бы) и", "(сказал⍽бы) и"),
    // At the end of a window, before a dash, between two particles.
    ("Что ж", "Что⍽ж"),
    ("Что ж\nдалее", "Что⍽ж\nдалее"),
    ("Что ж — ладно", "Что⍽ж — ладно"),
    ("что же бы ты", "что⍽же⍽бы ты"),
    // A capital letter with a full stop is an initial, not a particle (design section 8.7); a small
    // particle letter with a full stop is a particle.
    ("Иван Б. Петров", "Иван Б. Петров"),
    ("писатель Ж. Верн", "писатель Ж. Верн"),
    ("Ну что ж. Ладно", "Ну что⍽ж. Ладно"),
    ("Пошёл бы.", "Пошёл⍽бы."),
    // An opening bracket before the particle: no whole core after the space.
    ("сказал (бы)", "сказал (бы)"),
    // No left word.
    ("Бы пошёл", "Бы пошёл"),
    ("Так, ли ты", "Так, ли ты"),
    ("сказал\tбы", "сказал\tбы"),
    ("сказал  бы", "сказал  бы"),
    ("сказал⍽бы", "сказал⍽бы"),
];

#[test]
fn particles_in_russian() {
    alone_rows(ru(), PARTICLES, RU_PARTICLES);
    let t = alone(ru(), PARTICLES);
    assert_changed_by("ru particles", &t, "сказал бы", PARTICLES);
}

#[test]
fn particles_are_inert_in_english() {
    let t = alone(en(), PARTICLES);
    assert!(t.active_rules().is_empty());
    check_unchanged("en", &t, "сказал бы");
    // A caller can give English particles.
    let t = with(en(), &[PARTICLES], |b| b.add_particles(["though"]));
    assert!(t.active_rules().contains(PARTICLES));
    check_in("en", &t, ("said though", "said⍽though"));
}

#[test]
fn particles_by_setting() {
    let t = with(ru(), &[PARTICLES], |b| b.add_particles(["таки"]));
    check_in("ru", &t, ("Он таки пошёл", "Он⍽таки пошёл"));
    check_in("ru", &t, ("сказал бы", "сказал⍽бы"));
    let t = with(ru(), &[PARTICLES], |b| b.remove_particles(["бы"]));
    check_in("ru", &t, ("сказал бы", "сказал бы"));
    check_in("ru", &t, ("знаешь ли", "знаешь⍽ли"));
}

#[test]
fn particles_inside_the_sets() {
    both_sets(
        ru(),
        &[("сказал бы", "сказал⍽бы"), ("Знаешь ли", "Знаешь⍽ли")],
    );
    // Design section 8.7: particles and short words. The two-letter `бы` belongs to `Как`, so
    // `nbsp.short-length` does not glue it to what follows in Prose (design section 15.2, item 18,
    // read for every particle, design section 15.3, C34).
    both_sets(ru(), &[("Как бы то ни было", "Как⍽бы то⍽ни⍽было")]);
    let t = set(ru(), RuleSet::INTERFACE.without(PARTICLES));
    check_in("ru", &t, ("сказал бы", "сказал бы"));
}

/// Design section 15.2, item 18: a particle is glued to the word before it, and a particle so
/// placed glues nothing after itself; item 19: also when punctuation follows the particle.
#[test]
fn particles_bind_to_the_word_before() {
    set_rows(
        ru(),
        &[
            ("Знаешь ли ты", "Знаешь⍽ли ты", "Знаешь⍽ли ты"),
            (
                "Знаешь ли ты это?",
                "Знаешь⍽ли ты⍽это?",
                "Знаешь⍽ли ты это?",
            ),
            ("Он же пришёл.", "Он⍽же пришёл.", "Он⍽же пришёл."),
            ("Пришёл бы он!", "Пришёл⍽бы он!", "Пришёл⍽бы он!"),
            ("Я б пошёл.", "Я⍽б пошёл.", "Я⍽б пошёл."),
            ("Что ж,", "Что⍽ж,", "Что⍽ж,"),
            ("Что ж, ладно.", "Что⍽ж, ладно.", "Что⍽ж, ладно."),
            ("Знаешь ли...", "Знаешь⍽ли…", "Знаешь⍽ли…"),
            ("«Знаешь ли» ты", "«Знаешь⍽ли» ты", "«Знаешь⍽ли» ты"),
            // A dash glues itself to the particle (`nbsp.dash`); that is no glue after it.
            ("Что ж - ладно", "Что⍽ж⍽— ладно", "Что⍽ж⍽— ладно"),
            // Two particles: each belongs to the word before it, the run glues nothing after
            // (design section 15.3, C33).
            ("что же бы ты", "что⍽же⍽бы ты", "что⍽же⍽бы ты"),
            // No word before the particle: it is an ordinary short word.
            ("Ли ты знал", "Ли⍽ты⍽знал", "Ли ты знал"),
            // Glue typed by the author stays where it is.
            ("Знаешь⍽ли ты", "Знаешь⍽ли ты", "Знаешь⍽ли ты"),
            ("Знаешь ли⍽ты", "Знаешь⍽ли⍽ты", "Знаешь⍽ли⍽ты"),
        ],
    );
    // An initial whose letter is on the particle list is an initial: the forward form of
    // `nbsp.initials` takes it and the word before stays apart, as for any other letter
    // (design section 8.7).
    set_rows(
        ru(),
        &[
            ("Иван Б. Петров", "Иван Б.⍽Петров", "Иван Б.⍽Петров"),
            ("Иван А. Петров", "Иван А.⍽Петров", "Иван А.⍽Петров"),
            (
                "Пушкин Б. С. Иванов",
                "Пушкин Б.⍽С.⍽Иванов",
                "Пушкин Б.⍽С.⍽Иванов",
            ),
            ("роман Б. Акунина", "роман Б.⍽Акунина", "роман Б.⍽Акунина"),
            ("писатель Ж. Верн", "писатель Ж.⍽Верн", "писатель Ж.⍽Верн"),
            ("вариант Б. После", "вариант Б.⍽После", "вариант Б.⍽После"),
            ("Ну что ж. Ладно", "Ну⍽что⍽ж. Ладно", "Ну что⍽ж. Ладно"),
        ],
    );
    let prose = set(ru(), RuleSet::PROSE);
    check_unchanged("ru Prose", &prose, "Знаешь⍽ли ты");
    assert_changed_by("ru Prose", &prose, "Что ж,", PARTICLES);
    // `to_plain` keeps the glue that no rule sets: none is set after a bound particle (design
    // section 12.1).
    assert_eq!(
        prose.to_plain(&vis("Знаешь⍽ли⍽ты")),
        vis("Знаешь ли⍽ты"),
        "to_plain"
    );
    // After a slot (not `Other`) the particle belongs to the slot; after an `Other` slot nothing
    // glues to it, so the particle is an ordinary word (a short one in Prose).
    for (name, t, other) in [
        ("Prose", set(ru(), RuleSet::PROSE), " же⍽ты"),
        ("Interface", set(ru(), RuleSet::INTERFACE), " же ты"),
    ] {
        for (kind, expected) in [
            (Slot::WORD, "⍽же ты"),
            (Slot::NUMBER, "⍽же ты"),
            (Slot::OTHER, other),
        ] {
            let got = texts(&t, vec![Piece::Slot("x", kind), Piece::Text(" же ты")]);
            assert_eq!(got, ["".to_owned(), vis(expected)], "{name} slot {kind:?}");
        }
    }
}

/// The particle list decides which word belongs to the word before it, whichever rules are
/// enabled (design section 15.3, C33).
#[test]
fn a_bound_particle_glues_nothing_after_itself() {
    // `nbsp.short-length` alone: `ли` is a particle after a word, `ты` is not.
    let t = alone(ru(), SHORT_LENGTH);
    check_unchanged("ru short-length", &t, "Знаешь ли ты");
    check_in("ru short-length", &t, ("Я б пошёл.", "Я⍽б пошёл."));
    check_in("ru short-length", &t, ("ли ты", "ли⍽ты"));
    // A particle that is also a short word.
    let t = with(ru(), &[SHORT_WORDS], |b| b.add_short_words(["ли"]));
    check_unchanged("ru short-words", &t, "Знаешь ли ты");
    check_in("ru short-words", &t, ("ли ты", "ли⍽ты"));
    let t = with(ru(), &[SHORT_WORDS, PARTICLES], |b| {
        b.add_short_words(["ли"])
    });
    check_in("ru short-words", &t, ("Знаешь ли ты", "Знаешь⍽ли ты"));
    // A word removed from the particles is an ordinary short word again.
    let t = Typographer::builder(ru(), RuleSet::PROSE)
        .remove_particles(["ли"])
        .build()
        .unwrap();
    check_in("ru Prose", &t, ("Знаешь ли ты", "Знаешь ли⍽ты"));
}

// ---------------------------------------------------------------------------------------------
// nbsp.dash
// ---------------------------------------------------------------------------------------------

const RU_DASH: &[(&str, &str)] = &[
    ("Москва — столица", "Москва⍽— столица"),
    ("Москва – столица", "Москва⍽– столица"),
    ("10 – 20", "10⍽– 20"),
    ("Привет, — сказал он.", "Привет,⍽— сказал он."),
    ("«да» — нет", "«да»⍽— нет"),
    ("Москва — «столица»", "Москва⍽— «столица»"),
    ("Москва — (столица)", "Москва⍽— (столица)"),
    // The refusals of `dash.spaced-hyphen` do not apply to a typed dash.
    ("cat — file", "cat⍽— file"),
    ("cat – file", "cat⍽– file"),
    // A plain hyphen is not a dash; neither is a run of hyphens.
    ("Москва - столица", "Москва - столица"),
    ("Москва -- столица", "Москва -- столица"),
    ("5 - x", "5 - x"),
    ("Москва—столица", "Москва—столица"),
    // A `Dimensions` or `Joined` core is not an operand.
    ("1920x1080 — монитор", "1920x1080 — монитор"),
    ("Moscow—the — capital", "Moscow—the — capital"),
    // Pattern spaces only.
    ("Москва  — столица", "Москва  — столица"),
    ("Москва\t— столица", "Москва\t— столица"),
    ("Москва —\nстолица", "Москва —\nстолица"),
    ("Москва⍽— столица", "Москва⍽— столица"),
    ("Москва⎵— столица", "Москва⎵— столица"),
    // An operand on each side.
    ("— столица", "— столица"),
    ("Москва —", "Москва —"),
];

#[test]
fn dash_glue_in_russian() {
    alone_rows(ru(), DASH, RU_DASH);
    let t = alone(ru(), DASH);
    assert_changed_by("ru dash", &t, "Москва — столица", DASH);
    // The glue is the space before the dash: `Москва` is 12 bytes.
    assert_eq!(
        edits(&t, "Москва — столица"),
        [(DASH, 12..13, "\u{a0}".to_owned())]
    );
}

#[test]
fn dash_glue_follows_the_style() {
    // Inert except under `SpacedGlued` (design section 8.1).
    let t = with(ru(), &[DASH], |b| b.dash_style(DashStyle::Spaced));
    assert!(!t.active_rules().contains(DASH));
    check_in("Spaced", &t, ("Москва — столица", "Москва — столица"));
    let t = with(ru(), &[DASH], |b| b.dash_style(DashStyle::Closed));
    assert!(!t.active_rules().contains(DASH));
    check_in("Closed", &t, ("Москва — столица", "Москва — столица"));
    let t = with(en(), &[DASH], |b| b.dash_style(DashStyle::SpacedGlued));
    assert!(t.active_rules().contains(DASH));
    check_in(
        "en SpacedGlued",
        &t,
        ("Moscow — the capital", "Moscow⍽— the capital"),
    );
    check_in(
        "en SpacedGlued",
        &t,
        ("Moscow – the capital", "Moscow⍽– the capital"),
    );
    // English by default has the closed style.
    let t = alone(en(), DASH);
    assert!(t.active_rules().is_empty());
    check_unchanged("en", &t, "Moscow — the capital");
    // `COMMON` has no dash data.
    assert!(alone(Language::COMMON, DASH).active_rules().is_empty());
}

#[test]
fn dash_glue_inside_both_sets() {
    both_sets(
        ru(),
        &[
            ("Москва - столица", "Москва⍽— столица"),
            ("Москва — столица", "Москва⍽— столица"),
            ("Москва – столица", "Москва⍽– столица"),
            ("10 – 20", "10⍽– 20"),
            ("1990–2000 — годы", "1990–2000⍽— годы"),
            ("Глава 5 - введение", "Глава 5⍽— введение"),
            ("Привет, - сказал он.", "Привет,⍽— сказал он."),
        ],
    );
    let t = set(ru(), RuleSet::INTERFACE.without(DASH));
    check_in("ru", &t, ("Москва - столица", "Москва — столица"));
    // Report: glue first, then the dash, as in the example of design section 2.9.
    let t = set(ru(), RuleSet::INTERFACE);
    assert_eq!(
        edits(&t, "Цена - 5 руб."),
        [
            (DASH, 8..9, "\u{a0}".to_owned()),
            (Rule::DashSpacedHyphen, 9..10, "—".to_owned()),
            (UNITS, 12..13, "\u{a0}".to_owned()),
        ]
    );
}

// ---------------------------------------------------------------------------------------------
// Common conditions (design section 8.2) and glue marks (design section 3.9)
// ---------------------------------------------------------------------------------------------

#[test]
fn no_glue_sticks_to_an_other_slot() {
    let t = alone(ru(), SHORT_WORDS);
    let out = texts(&t, vec![Piece::Text("в "), Piece::Slot("x", Slot::OTHER)]);
    assert_eq!(out, ["в ", ""]);
    let out = texts(
        &t,
        vec![
            Piece::Text("в "),
            Piece::Slot("x", Slot::NUMBER),
            Piece::Text(" рублей"),
        ],
    );
    assert_eq!(out, ["в\u{a0}", " рублей"]);
    let t = alone(ru(), PARTICLES);
    let out = texts(&t, vec![Piece::Slot("x", Slot::OTHER), Piece::Text(" же")]);
    assert_eq!(out, ["", " же"]);
    let out = texts(&t, vec![Piece::Slot("w", Slot::WORD), Piece::Text(" же")]);
    assert_eq!(out, ["", "\u{a0}же"]);
}

#[test]
fn a_glued_space_needs_a_next_fragment_that_starts_with_an_operand_character() {
    let t = alone(ru(), SHORT_WORDS);
    for (input, expected) in [
        ("в Москве", "в⍽Москве"),
        ("в 5", "в⍽5"),
        ("в «х»", "в⍽«х»"),
        ("в (х)", "в⍽(х)"),
        ("в [х]", "в [х]"),
        ("в {х}", "в {х}"),
        ("в — х", "в — х"),
        ("в ? х", "в ? х"),
        ("в . х", "в . х"),
    ] {
        check_in("next fragment", &t, (input, expected));
    }
}

#[test]
fn a_trigger_word_is_a_whole_core_and_its_wrapper_does_not_hide_it() {
    let t = alone(ru(), SHORT_WORDS);
    check_in("wrapper", &t, ("(в 2026)", "(в⍽2026)"));
    check_in("wrapper", &t, ("\"в 2026", "\"в⍽2026"));
    check_in("closing punctuation", &t, ("(в) 2026", "(в) 2026"));
    check_in("closing punctuation", &t, ("в; 2026", "в; 2026"));
}

#[test]
fn typed_glue_is_never_rewritten() {
    // Whatever `narrow_glue` says (design section 4.5).
    for rule in NBSP_RULES {
        let t = with(ru(), &[rule], |b| b.narrow_glue(NarrowGlue::ALL));
        check_in("typed", &t, ("в⍽Москве", "в⍽Москве"));
        check_in("typed", &t, ("в⎵Москве", "в⎵Москве"));
        check_in("typed", &t, ("5⍽МБ", "5⍽МБ"));
        check_in("typed", &t, ("№⎵5", "№⎵5"));
    }
}

#[test]
fn a_run_of_whitespace_that_contains_glue_is_not_glued_again() {
    // A longer run of horizontal whitespace has `Glue::None` even when it contains U+00A0.
    let t = alone(ru(), SHORT_WORDS);
    check_in("run", &t, ("в\u{a0} Москве", "в\u{a0} Москве"));
    check_in("run", &t, ("в \u{a0}Москве", "в \u{a0}Москве"));
}

#[test]
fn the_first_rule_in_the_order_table_takes_the_space() {
    // `и т. д.`: both spaces could be taken by `nbsp.short-words` (the first) or
    // `nbsp.abbreviations`; abbreviations come first in the order table (design section 3.9).
    let t = set(ru(), RuleSet::INTERFACE);
    let rules = change_rules(&t, "и т. д.");
    assert_eq!(rules, [ABBR, ABBR], "{rules:?}");
    // `в 5 МБ`: different spaces, different rules, in the order of position.
    let rules = change_rules(&t, "в 5 МБ");
    assert_eq!(rules, [SHORT_WORDS, UNITS]);
}

#[test]
fn glue_is_a_single_character_change() {
    // Design section 3.9: the glue is the mark of a `Space` token; `report` shows it as a
    // replacement of the space by U+00A0.
    let t = set(ru(), RuleSet::INTERFACE);
    let source = vis("Он вызвал ls на сервере");
    let r = t.report(&source, Output::Unicode);
    let changes: Vec<_> = r
        .changes()
        .iter()
        .map(|c| (c.rule(), c.range(), c.replacement().to_owned()))
        .collect();
    // Only the space before `сервере`: `Он` is not a short word in Interface.
    assert_eq!(changes.len(), 1, "{changes:?}");
    assert_eq!(changes[0].0, SHORT_WORDS);
    assert_eq!(changes[0].2, "\u{a0}");
    assert_eq!(changes[0].1.len(), 1);
    assert_eq!(&source[changes[0].1.clone()], " ");
}

#[test]
fn rules_off_in_the_empty_set_change_nothing() {
    let t = set(ru(), RuleSet::EMPTY);
    for text in [
        "в Москве",
        "5 МБ",
        "№ 5",
        "и т. д.",
        "А. С. Пушкин",
        "21 ноября",
    ] {
        check_unchanged("empty", &t, text);
    }
}

#[test]
fn every_row_of_the_design_table_of_section_8_7() {
    // Rule alone (the `Rule` column names the mechanisms): the rows that name no set.
    check_in(
        "ru short-words",
        &alone(ru(), SHORT_WORDS),
        ("в 2026 году", "в⍽2026 году"),
    );
    let particles_and_short = only(ru(), &[PARTICLES, SHORT_WORDS]);
    check_in(
        "particles, short-words",
        &particles_and_short,
        ("Как бы то ни было", "Как⍽бы то⍽ни⍽было"),
    );
    let abbr = alone(ru(), ABBR);
    check_in("abbr", &abbr, ("и т. д.", "и⍽т.⍽д."));
    check_in("abbr", &abbr, ("и т.д.", "и⍽т.⍽д."));
    let signs = alone(ru(), SIGNS);
    check_in("signs", &signs, ("Документ № 5", "Документ №⍽5"));
    check_in("signs", &signs, ("№5", "№⍽5"));
    let both = only(ru(), &[SIGNS, ABBR]);
    check_in(
        "signs, abbreviations",
        &both,
        ("Школа №5 г. Москвы", "Школа №⍽5 г.⍽Москвы"),
    );
    check_in("signs, abbreviations", &both, ("§5 ч. 2", "§⍽5 ч.⍽2"));
    let units = alone(ru(), UNITS);
    check_in("units", &units, ("5 МБ", "5⍽МБ"));
    check_in("units", &units, ("30 с.", "30⍽с."));
    check_in("units", &units, ("12 дн.", "12⍽дн."));
    check_in("units", &units, ("100 %", "100⍽%"));
    check_in("units", &units, ("100%", "100%"));
    check_in("units", &units, ("60 км/ч", "60⍽км/ч"));
    let initials = alone(ru(), INITIALS);
    check_in("initials", &initials, ("А. С. Пушкин", "А.⍽С.⍽Пушкин"));
    check_in(
        "initials",
        &initials,
        ("Об этом сказал А. Петров.", "Об этом сказал А.⍽Петров."),
    );
    let dates = alone(ru(), DATES);
    check_in("dates", &dates, ("21 ноября 2026 г.", "21⍽ноября 2026⍽г."));
    let dash = alone(ru(), DASH);
    check_in("dash", &dash, ("Москва – столица", "Москва⍽– столица"));
    check_in("dash", &dash, ("10 – 20", "10⍽– 20"));
    let en_both = only(en(), &[ABBR, INITIALS]);
    check_in("en", &en_both, ("Mr. Smith", "Mr.⍽Smith"));
    check_in("en", &en_both, ("J. R. R. Tolkien", "J.⍽R.⍽R.⍽Tolkien"));
    let en_three = only(en(), &[SHORT_WORDS, DATES, UNITS]);
    check_in(
        "en",
        &en_three,
        ("On November 21, in 5 MB", "On⍽November⍽21, in⍽5⍽MB"),
    );
}

#[test]
fn the_two_rows_of_section_8_7_for_the_full_sets() {
    // Prose and Interface rows: `Он вызвал ls на сервере` (Interface) and the initials row.
    check_in(
        "ru Interface",
        &set(ru(), RuleSet::INTERFACE),
        ("Он вызвал ls на сервере", "Он вызвал ls на⍽сервере"),
    );
    check_in(
        "ru Prose",
        &set(ru(), RuleSet::PROSE),
        ("Об этом сказал А. Петров.", "Об⍽этом сказал А.⍽Петров."),
    );
    // The rows in the table of design section 8.7 that name no rule set also hold in both sets.
    set_rows(
        ru(),
        &[
            ("Документ № 5", "Документ №⍽5", "Документ №⍽5"),
            ("№5", "№⍽5", "№⍽5"),
            ("Скидка 100 %.", "Скидка 100⍽%.", "Скидка 100⍽%."),
            ("Скидка 100%.", "Скидка 100%.", "Скидка 100%."),
        ],
    );
}

#[test]
fn the_examples_of_sections_2_5_and_2_8() {
    // The rustdoc examples of the design, in Unicode: a protected span is one word for its
    // neighbors (`в⍽`--config``), and the space before a dash and before a unit is glued.
    let ui = set(ru(), RuleSet::INTERFACE);
    check_in(
        "design section 2.5",
        &ui,
        (
            "Файл \"report.pdf\" не найден - проверьте путь в `--config`.",
            "Файл «report.pdf» не⍽найден⍽— проверьте путь в⍽`--config`.",
        ),
    );
    check_in(
        "design section 2.8",
        &ui,
        ("Цена - 5 руб.", "Цена⍽— 5⍽руб."),
    );
    // Technical text is neither changed nor copied.
    check_unchanged("design section 2.5", &ui, "host:8080");
    // Already typeset text comes back borrowed (idempotence).
    check_unchanged(
        "design section 2.5",
        &ui,
        "Файл «report.pdf» не⍽найден⍽— проверьте путь в⍽`--config`.",
    );
}

#[test]
fn the_text_of_a_diagnostic_free_input_has_no_visible_marker() {
    // A self-check of the table helpers: markers never reach the typographer.
    assert_eq!(show(&vis("5⍽МБ⎵")), "5⍽МБ⎵");
}
