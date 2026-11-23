//! Build-time use with Fluent (design section 2.14): a generator that typesets a few hard-coded
//! messages, already mapped from Fluent syntax to pieces, with `report_template` and
//! `Output::Escaped(EscapeStyle::RUST)`. It turns warnings into `cargo::warning` lines and writes
//! each message as a Rust closure whose texts are bodies of string literals.
//!
//! Run with `cargo run --example fluent_generator`.

#![allow(
    clippy::print_stdout,
    clippy::print_stderr,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic
)]

use typolish::{
    EscapeStyle, Language, Output, Piece, RuleSet, Severity, Slot, TemplatePart, Typographer,
};

/// The typographer of a language, shared by the generator and the application, so that the
/// template and the numbers formatted at run time fit together.
pub fn typographer(tag: &str) -> Result<Typographer, typolish::Error> {
    Typographer::builder(Language::from_tag(tag)?, RuleSet::INTERFACE)
        .add_short_words(["через"])
        .add_units(["ед."])
        .build()
}

/// Typesets one message and writes it as Rust code; warnings become `cargo::warning` lines.
fn emit(id: &str, pieces: Vec<Piece<'_, usize>>, t: &Typographer) -> String {
    let r = t.report_template(pieces, Output::Escaped(EscapeStyle::RUST));
    for d in r.diagnostics() {
        if d.severity() >= Severity::Warning {
            println!("cargo::warning={id}: {d}");
        }
    }
    let mut code = String::from("|args: &[&str]| { let mut s = String::new(); ");
    for part in r.result().iter() {
        match part {
            TemplatePart::Text(t) => code.push_str(&format!("s.push_str(\"{t}\"); ")),
            TemplatePart::Slot(i, _) => code.push_str(&format!("s.push_str(args[{i}]); ")),
            // build-time code: failing loudly beats losing text
            _ => panic!("typolish: a TemplatePart variant this generator does not know; update it"),
        }
    }
    code.push_str("s }");
    code
}

fn main() {
    let ru = typographer("ru-RU").expect("the settings are valid");

    // The messages as the generator maps them (design section 2.14): text elements are `Text`,
    // `{ NUMBER($n) }` is a `Number` slot, `{ $name }` a slot of the kind from the schema.
    let messages: Vec<(&str, Vec<Piece<'_, usize>>)> = vec![
        // files-found = Найдено { NUMBER($n) } файлов в "{ $folder }" - проверьте их
        (
            "files-found",
            vec![
                Piece::Text("Найдено "),
                Piece::Slot(0, Slot::NUMBER),
                Piece::Text(" файлов в \""),
                Piece::Slot(1, Slot::WORD),
                Piece::Text("\" - проверьте их"),
            ],
        ),
        // quota = Осталось { NUMBER($n) } ед. через { NUMBER($days) } дн.
        (
            "quota",
            vec![
                Piece::Text("Осталось "),
                Piece::Slot(0, Slot::NUMBER),
                Piece::Text(" ед. через "),
                Piece::Slot(1, Slot::NUMBER),
                Piece::Text(" дн."),
            ],
        ),
        // pages = Страницы { NUMBER($a) }-{ NUMBER($b) }
        (
            "pages",
            vec![
                Piece::Text("Страницы "),
                Piece::Slot(0, Slot::NUMBER),
                Piece::Text("-"),
                Piece::Slot(1, Slot::NUMBER),
            ],
        ),
        // reset = Выполните `git checkout -- { $branch }`, чтобы сбросить
        (
            "reset",
            vec![
                Piece::Text("Выполните `git checkout -- "),
                Piece::Slot(0, Slot::OTHER),
                Piece::Text("`, чтобы сбросить"),
            ],
        ),
        // greeting = Он сказал "привет.  (an unpaired quote: the generator warns)
        ("greeting", vec![Piece::Text("Он сказал \"привет.")]),
    ];

    for (id, pieces) in messages {
        let code = emit(id, pieces, &ru);
        println!("{id} = {code}");
    }

    // At run time the application formats numbers with the same typographer and Unicode output.
    let n = ru.format_number(1000, Output::Unicode).to_string();
    println!("format_number(1000) = {n:?}");
}
