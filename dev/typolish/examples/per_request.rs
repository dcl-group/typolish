//! Per-request use (design sections 2.4 and 2.15): true statics with the defaults of a language,
//! chosen by the language of the request, and a `LazyLock` typographer built once with custom
//! settings.
//!
//! Run with `cargo run --example per_request`.

#![allow(
    clippy::print_stdout,
    clippy::print_stderr,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic
)]

use std::borrow::Cow;
use std::sync::LazyLock;

use typolish::{DashStyle, HtmlStyle, Language, Output, Rule, RuleSet, Typographer};

// Defaults: true statics with no initialization cost (section 2.15).
static RU: Typographer = Typographer::new(Language::RU_RU, RuleSet::PROSE);
static EN: Typographer = Typographer::new(Language::EN_US, RuleSet::PROSE);

// Custom settings: built once, on first use (section 2.4).
static EN_DOCS: LazyLock<Typographer> = LazyLock::new(|| {
    Typographer::builder(Language::EN_US, RuleSet::INTERFACE)
        .enable(Rule::NumberTimes)
        .disable(Rule::NbspShortWords)
        .dash_style(DashStyle::Spaced)
        .add_units(["req/s", "vCPU"])
        .protect_between("{{", "}}")
        .build()
        .expect("settings are valid")
});

fn typographer(tag: &str) -> &'static Typographer {
    if tag.starts_with("ru") { &RU } else { &EN }
}

/// A comment of a site, typeset for an HTML page in the language of the request.
fn render_comment<'a>(tag: &str, body: &'a str) -> Cow<'a, str> {
    typographer(tag).typeset(body, Output::Html(HtmlStyle::NAMED))
}

fn main() {
    let requests = [
        ("ru-RU", "Он сказал \"привет\" - и ушёл..."),
        ("en-US", "She said \"hello\" -- and left..."),
        ("ru-RU", "Сохранить"),
        ("en-US", "Use <b> & \"quotes\""),
    ];
    for (tag, body) in requests {
        let out = render_comment(tag, body);
        let kind = match out {
            Cow::Borrowed(_) => "borrowed",
            Cow::Owned(_) => "owned",
        };
        println!("{tag} {kind:<8} {out}");
    }

    // The configured typographer: `number.times` on, short words off, the spaced dash, own
    // units, and `{{ … }}` protected.
    for text in [
        "Screen 1920x1080 at 60 req/s",
        "Two vCPU - enough for {{ a - b }}",
    ] {
        println!("docs  {}", EN_DOCS.typeset(text, Output::Unicode));
    }
}
