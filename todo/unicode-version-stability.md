# Output stability across versions of `unicode-segmentation`

## Problem

Design section 5.6 says: "A newer dependency or compiler can change the classification of characters
assigned after these versions only." The manifest asks for `unicode-segmentation = "1.13.3"`, so any
later 1.x is accepted. The audit of 2026-11-24 states that the grapheme cluster rules of Unicode
have changed for long-assigned characters too (rule GB9c, Unicode 15.1), so a newer dependency could
change where clusters break, and with it the output of the built-in sets, which design section 15.1
holds stable. The statement was not checked by a probe.

## Goal

Check the statement with a fixture of grapheme boundaries (the GB9c sequences and the other rules
changed since the version the crate was built against) run against two versions of the dependency.

## Acceptance

Either the text of design section 5.6 is confirmed by the fixture and the fixture becomes a test, or
the maintainer is asked one question about the policy of design section 15.1: an exception for
Unicode updates recorded in 15.1, or a requirement `~1.13` with its reason next to the version in
`Cargo.toml`.
