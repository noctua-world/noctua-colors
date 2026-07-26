//! Color math for the `noctua-colors` compiler.
//!
//! This crate knows nothing about the spec format and nothing about the output
//! targets. It converts between color spaces, answers questions about gamut
//! boundaries, measures contrast and perceptual difference, and simulates
//! color vision deficiency. Everything above it is somebody else's crate.
//!
//! # The shape of the problem
//!
//! Oklab is the working space. sRGB, Display P3 and Rec.2020 are *encodings*,
//! not sources of truth. That inversion is what lets the compiler define a
//! color once and render it correctly everywhere: chroma is stored as a
//! fraction of what a gamut can actually show, so the same definition is more
//! saturated on a wide-gamut display without being redefined.
//!
//! # Invariants this crate upholds
//!
//! - **Gamut mapping never clips per channel.** Clipping a channel shifts hue
//!   silently. [`map_into_gamut`] reduces chroma while holding lightness and
//!   hue, and clips only at the very end to absorb floating-point error.
//! - **APCA is the contrast metric.** WCAG 2.x is computed
//!   ([`contrast::wcag21`]) for compliance reporting only, never as a design
//!   criterion.
//! - **No color constant is written down.** The only hardcoded numbers here
//!   are published chromaticity coordinates and transfer-function constants;
//!   every matrix is derived from them at compile time.

#![doc(test(attr(deny(warnings))))]

pub mod contrast;
mod cubic;
pub mod cvd;
pub mod diff;
pub mod gamut;
pub mod map;
mod matrix;
pub mod space;

pub use contrast::{apca, wcag21};
pub use cvd::{Cvd, simulate};
pub use diff::{JND, delta_e_ok};
pub use gamut::Gamut;
pub use map::{map_into_gamut, to_hex};
pub use space::{LinearRgb, Oklab, Oklch, Rgb, UniformSpace, Xyz};
