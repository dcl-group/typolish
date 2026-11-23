//! The error type of the crate (design section 2.11).

/// A configuration or parsing error.
///
/// Errors arise only while a typographer, a language, a rule set or a number is built or parsed.
/// Applying a typographer never fails.
///
/// ```
/// use typolish::{Error, Rule};
///
/// let err = "nbsp.everything".parse::<Rule>().unwrap_err();
/// assert!(matches!(err, Error::UnknownRule { .. }));
/// assert_eq!(err.to_string(), "unknown rule id `nbsp.everything`");
/// ```
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum Error {
    /// The tag is malformed or names a language that has no built-in data.
    #[error("unknown language tag `{}`", crate::report::Raw(.tag))]
    #[non_exhaustive]
    UnknownLanguage {
        /// The tag as given.
        tag: String,
    },
    /// Language data failed a check of design section 7.6.
    #[error("invalid language data for `{}`: {reason}", crate::report::Raw(.tag))]
    #[non_exhaustive]
    InvalidLanguageData {
        /// The tag of the table.
        tag: String,
        /// The failed check, naming the field.
        reason: &'static str,
    },
    /// No rule has this id.
    #[error("unknown rule id `{}`", crate::report::Raw(.id))]
    #[non_exhaustive]
    UnknownRule {
        /// The id as given.
        id: String,
    },
    /// A rule set string breaks the grammar of design section 2.3.
    #[error("invalid rule set `{}`: {reason}", crate::report::Raw(.input))]
    #[non_exhaustive]
    InvalidRuleSet {
        /// The whole string as given.
        input: String,
        /// What is wrong with it.
        reason: &'static str,
    },
    /// A rule is both enabled and disabled in a builder.
    #[error("rule `{id}` is both enabled and disabled")]
    #[non_exhaustive]
    RuleConflict {
        /// The id of the rule.
        id: &'static str,
    },
    /// A string is not an ASCII decimal number of design section 2.7.
    #[error("invalid number `{}`: {reason}", crate::report::Raw(.input))]
    #[non_exhaustive]
    InvalidNumber {
        /// The string as given.
        input: String,
        /// What is wrong with it.
        reason: &'static str,
    },
    /// A setting of the typographer builder has an invalid value.
    #[error("invalid setting `{setting}`: {reason}")]
    #[non_exhaustive]
    InvalidSetting {
        /// The name of the setting.
        setting: &'static str,
        /// What is wrong, with the offending value.
        reason: String,
    },
}
