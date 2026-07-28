//! Curve evaluation, contrast-anchored scale solving and palette construction.
//!
//! The stage between a parsed spec and an emitted artifact. Given a
//! [`Spec`](noctua_spec::Spec), [`build`] produces the finished
//! [`Palette`] — every theme, both modes, every gamut, every family, every
//! step — with all color decisions already made and quantized.
//!
//! Nothing here knows about output formats.
#![allow(missing_docs)]

pub mod chart;
pub mod curve;
pub mod error;
pub mod fit;
pub mod neutral;
pub mod ordinal;
pub mod palette;
pub mod solve;

pub use error::EngineError;
pub use fit::{Fit, fit_family};
pub use palette::{
    AlphaScale, BASE_NEUTRAL_RAMP, CHART_SCALE, Identity, Palette, ResolvedColor, ResolvedFamily,
    ResolvedMode, ResolvedScale, ResolvedStep, ResolvedTheme, ScaleKind, build,
};
pub use solve::Mode;
