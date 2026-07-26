//! What can go wrong while building a palette.
//!
//! The spec layer has already rejected everything it can check by reading the
//! file. What is left are the failures that only appear once the color math
//! runs — and nearly all of them are one thing: **a target the gamut cannot
//! reach.**
//!
//! Those are reported, never quietly clamped. Silently settling for Lc 71 when
//! the spec asked for 96 would ship unreadable text and pass every gate,
//! because the gate would be measuring what was produced rather than what was
//! asked for.

use thiserror::Error;

/// A failure while constructing a palette.
#[derive(Debug, Error)]
pub enum EngineError {
    /// A contrast or separation target cannot be met at any lightness.
    ///
    /// Boxed because it is by far the largest variant and every successful
    /// build carries the `Result` around without ever constructing one.
    #[error(transparent)]
    UnreachableTarget(#[from] Box<Unreachable>),

    /// A role referenced another role that had not been resolved yet.
    ///
    /// The spec validator rejects forward and circular references, so reaching
    /// this means the two layers disagree about resolution order.
    #[error("role `{role}` refers to `{against}`, which has not been resolved")]
    UnresolvedReference {
        /// The role being resolved.
        role: String,
        /// The reference that could not be satisfied.
        against: String,
    },

    /// A theme mapped a semantic slot onto a family that does not exist.
    #[error("theme `{theme}` maps `{slot}` to family `{family}`, which does not exist")]
    UnknownFamily {
        /// Theme being built.
        theme: String,
        /// Semantic slot.
        slot: String,
        /// The missing family.
        family: String,
    },
}

/// Details of a target the color math could not satisfy.
#[derive(Debug, Error)]
#[error(
    "{theme}/{mode}: family `{family}` cannot reach {requested:.1} {units} for role \
     `{role}` against `{against}` in {gamut}; the most it can reach is {achievable:.1}"
)]
pub struct Unreachable {
    /// Theme being built.
    pub theme: String,
    /// Mode being built.
    pub mode: &'static str,
    /// Family being built.
    pub family: String,
    /// Role whose target failed.
    pub role: String,
    /// Role it is anchored to.
    pub against: String,
    /// What the spec asked for.
    pub requested: f64,
    /// The best the family can actually do.
    pub achievable: f64,
    /// Units of the two numbers above, for the message.
    pub units: &'static str,
    /// Gamut being resolved against.
    pub gamut: &'static str,
}

impl EngineError {
    /// A sentence telling the developer what to change.
    ///
    /// Kept beside the error rather than at the printing site so that every
    /// caller — CLI, tests, the docs site — gives the same advice.
    #[must_use]
    pub fn fix(&self) -> String {
        match self {
            Self::UnreachableTarget(details) => {
                let Unreachable {
                    role,
                    requested,
                    achievable,
                    family,
                    units,
                    ..
                } = &**details;
                format!(
                    "Lower the target for `{role}` to {achievable:.0} {units} or less, or give \
                     family `{family}` more room: reduce its relative chroma near this step, or \
                     widen `output.gamut`. Very light hues such as yellow cannot reach high \
                     contrast against a light background at full chroma — that is a real \
                     constraint, not a bug, and it is why this fails loudly. Requested \
                     {requested:.0}."
                )
            }
            Self::UnresolvedReference { .. } => {
                "This is an internal inconsistency between spec validation and the engine; \
                 please report it."
                    .to_owned()
            }
            Self::UnknownFamily { family, .. } => {
                format!("Define `[families.{family}]`, or point the slot at a family that exists.")
            }
        }
    }
}
