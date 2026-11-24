# `to_plain` and `lint` are quadratic in one long paragraph

## Problem

Every pass of the reverse transformation that changes something turns at least one non-ASCII
character into ASCII text, and the pass repeats until nothing changes (design section 12.1). On a
long run of ambiguous characters in one paragraph (`’`, nested quotes) a pass may settle only a few
readings, so the number of passes grows with the run and the time is quadratic in the length of the
paragraph (design section 12.1).

Measured in a release build with `Language::RU_RU` and `RuleSet::PROSE`:

| Input                                     | `to_plain`     | `typeset`   |
| ----------------------------------------- | -------------- | ----------- |
| `’` repeated, 24 000 bytes, one paragraph | 0.27 to 0.44 s | 1.2 to 4 ms |
| `’` repeated, 48 000 bytes, one paragraph | 1.07 to 1.62 s |             |
| `’` repeated, 96 000 bytes, one paragraph | 4.3 s          |             |
| the same 99 KB split into paragraphs      | 14.8 ms        |             |

Ordinary text is linear.

## Goal

A number of passes that does not grow with the length of such a run, while `to_plain` stays stable
(`to_plain(to_plain(x)) == to_plain(x)`) and `lint` keeps reporting a `TypographicChar` exactly
where `to_plain` changes the text.

## Acceptance

An `#[ignore]` test in `tests/load.rs`, after the pattern of `load_is_linear`, for `’` repeated n
times and for `«„` repeated n times, then `x`, then `“»` repeated n times: the time per byte at 512
KiB is at most twice that at 64 KiB. The test is added together with the change of the code.
`to_plain_is_linear_on_prose` covers ordinary prose and passes; the runs of `’` remain.
