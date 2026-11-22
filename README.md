# typolish

`typolish` is a Rust crate that typesets plain text for a language. Text typed the easy way, with
straight quotes, hyphens, three full stops and ordinary spaces, becomes typeset text: quotes by
nesting level, apostrophes, dashes, ellipses, no-break spaces, and rules for words, signs and
numbers. The built-in languages are `ru-RU` and `en-US`; other languages are added as data.

## Status

In development; nothing is released. The design is in
[`docs/design/`](https://github.com/dcl-group/typolish/tree/main/docs/design). The engine recognizes
protected and technical fragments and applies the rules of quotes, dashes, ranges and ellipses;
`Typographer::report` lists every edit, protected span and diagnostic. The other rules (no-break
spaces, numbers, symbols) change nothing yet.

## Usage

```rust
use typolish::{Language, Output, RuleSet, Typographer};

static RU_UI: Typographer = Typographer::new(Language::RU_RU, RuleSet::INTERFACE);

let text = RU_UI.typeset("Файл не найден", Output::Unicode);
assert_eq!(text, "Файл не найден");
let text = RU_UI.typeset("Он сказал \"привет\" - и ушёл...", Output::Unicode);
assert_eq!(text, "Он сказал «привет» — и ушёл…");
```

## License

Licensed under either of [MIT](./LICENSE-MIT) or [Apache-2.0](./LICENSE-APACHE) at your option.
