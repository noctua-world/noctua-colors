//! The specification format.
//!
//! Every field has a default, so a working spec can be three lines:
//!
//! ```toml
//! [families.accent]
//! hue = 264
//! ```
//!
//! Everything else — the twelve scale roles, their contrast targets, the
//! neutral ramp, the default theme — comes from [`crate::defaults`].

use indexmap::IndexMap;
use noctua_core::Gamut;
use serde::Deserialize;
use toml::Spanned;

use crate::curve::{CurveSpec, HueSpec};

/// A whole color system.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Spec {
    /// Gamut and naming settings.
    #[serde(default)]
    pub output: Output,

    /// The functional scale: which roles exist and what each one targets.
    #[serde(default)]
    pub scale: Scale,

    /// Color families, keyed by name. Iteration follows spec order.
    #[serde(default)]
    pub families: IndexMap<String, Family>,

    /// The neutral ramp, which is a family with two extra powers.
    #[serde(default)]
    pub neutral: Neutral,

    /// Semantic slot to family, for every theme.
    ///
    /// A slot named after a family maps to it automatically, so this is only
    /// for the names that differ — `rejected = "danger"`. Declared once rather
    /// than per theme: with a grid of thirty-six palettes, repeating two dozen
    /// aliases in each would be the largest and least informative part of the
    /// specification. A `[themes.<name>.semantic]` entry still overrides this
    /// for one palette.
    #[serde(default)]
    pub semantic: IndexMap<String, Spanned<String>>,

    /// The translucency ladder.
    #[serde(default)]
    pub alpha: Alpha,

    /// Content hash of the file this was parsed from.
    ///
    /// Filled in by [`crate::parse`], never read from the file — a spec that
    /// could state its own hash could state the wrong one.
    #[serde(skip)]
    pub source_hash: String,

    /// Ordered and categorical scales, in spec order.
    ///
    /// A scale is a hue path plus a number of stops — the categorical chart,
    /// an ordinal traffic light, anything read off a legend rather than from a
    /// role name.
    #[serde(default)]
    pub scales: Vec<NamedScale>,

    /// Accent hues offered as palettes, keyed by name.
    ///
    /// Combined with [`Spec::saturations`] into one theme per pair. Twelve
    /// accents and three saturations is thirty-six palettes from fifteen
    /// lines, which is the whole reason this is a separate axis rather than
    /// thirty-six hand-written `[themes.*]` blocks.
    #[serde(default)]
    pub accents: IndexMap<String, Accent>,

    /// Chroma multipliers offered as palettes, keyed by name.
    ///
    /// The sober-to-vivid axis, named. Ignored unless [`Spec::accents`] is
    /// also given — on its own it would just be `[themes.*]` spelled shorter.
    #[serde(default)]
    pub saturations: IndexMap<String, f64>,

    /// Named presets applying global transforms over the same families.
    ///
    /// Generated themes are prepended to this in spec order, so a hand-written
    /// theme can still exist alongside the grid.
    #[serde(default)]
    pub themes: IndexMap<String, Theme>,

    /// The categorical scale, for charts and any other set of colors that
    /// must be told apart rather than ordered.
    #[serde(default)]
    pub chart: Chart,

    /// Further categorical scales, each under a name of its own.
    ///
    /// One chart is enough until a page needs two — a second series set beside
    /// the first, or a set wide enough that its legend names every entry. Each
    /// entry here is an ordinary [`Chart`] plus a `name`, and produces
    /// `<name>-1`, `<name>-2` and so on.
    #[serde(default)]
    pub charts: Vec<Chart>,

    /// Export destinations.
    #[serde(default)]
    pub consumers: Vec<Consumer>,
}

/// Gamut and naming settings.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Output {
    /// The gamut relative chroma resolves against, and the one the hex
    /// fallback layer is written in.
    #[serde(default = "default_gamut", deserialize_with = "de_gamut")]
    pub gamut: Gamut,

    /// Wider gamuts to emit as additional layers. The same token is more
    /// saturated in each, because its relative chroma resolves against more
    /// room.
    #[serde(default = "default_upgrades", deserialize_with = "de_gamut_list")]
    pub upgrades: Vec<Gamut>,

    /// Namespace for emitted custom properties, without dashes: `nc` produces
    /// `--nc-accent-solid`.
    #[serde(default = "default_prefix")]
    pub prefix: String,
}

/// The functional scale.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Scale {
    /// The roles, in ramp order. Position in this list is the `t` a curve is
    /// evaluated at.
    #[serde(default = "crate::defaults::roles")]
    pub roles: Vec<Role>,
}

/// One step of the scale, named by what it is for rather than by number.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Role {
    /// Canonical kebab-case name, such as `bg-subtle` or `text-strong`.
    pub name: Spanned<String>,

    /// What this role targets in light mode.
    pub light: TargetSpec,

    /// What this role targets in dark mode.
    pub dark: TargetSpec,

    /// Whether a family's [`Family::contrast_shift`] applies to this role.
    ///
    /// True for the solid steps, which is where a family carries meaning as a
    /// fill and where two families most need telling apart. False everywhere
    /// else, and deliberately so for text: text contrast is an accessibility
    /// floor, not a place to encode semantics. Danger text must be exactly as
    /// readable as any other text, and shifting it asked for Lc 114 against a
    /// ceiling of 105.
    #[serde(default)]
    pub shift: bool,
}

/// A role's anchor, before validation.
///
/// Exactly one field must be set. Three kinds exist because one metric cannot
/// honestly govern all twelve roles:
///
/// - `apca` for text and solids, where legibility is the question.
/// - `delta_l` for surfaces and borders, where the question is whether two
///   areas read as distinct. APCA is defined for text contrast; asking it
///   whether step 2 is distinguishable from step 1 at Lc 3 is outside its
///   design range and the number means nothing there.
/// - `lightness` to anchor a ramp outright.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TargetSpec {
    /// An explicit lightness in `[0, 1]`.
    pub lightness: Option<Spanned<f64>>,
    /// An APCA contrast target against another role.
    pub apca: Option<ApcaTarget>,
    /// A perceptual lightness separation from another role.
    pub delta_l: Option<DeltaLTarget>,
}

/// An APCA contrast target.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ApcaTarget {
    /// The role to measure against.
    pub against: Spanned<String>,
    /// Target contrast in Lc, as a **magnitude**. Polarity follows the mode:
    /// light mode solves for a darker color, dark mode for a lighter one, so
    /// the same number reads correctly in both.
    pub lc: Spanned<f64>,
}

/// A perceptual lightness separation target.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DeltaLTarget {
    /// The role to measure against.
    pub against: Spanned<String>,
    /// Oklab lightness separation, as a magnitude. Direction follows the mode
    /// exactly as for [`ApcaTarget::lc`].
    pub amount: Spanned<f64>,
}

/// A color family.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Family {
    /// Hue over the ramp, with optional torsion.
    pub hue: HueSpec,

    /// Relative chroma over the ramp: a fraction of what the target gamut can
    /// show at each step's lightness and hue.
    #[serde(default = "crate::defaults::chroma_curve")]
    pub cr: CurveSpec,

    /// Shifts this family's APCA targets, in Lc.
    ///
    /// Exists for colour-vision safety. Every solid is anchored on contrast
    /// against the same background, so without this they all land at
    /// essentially the same lightness and differ only in hue — and hue is
    /// exactly what dichromacy removes. A deuteranope then sees success and
    /// danger as the same color.
    ///
    /// A positive shift pushes a family further from the background (darker in
    /// light mode), a negative shift closer. Give two opposed families
    /// opposite shifts and they separate along the one axis dichromacy leaves
    /// intact.
    ///
    /// Applies only to APCA-anchored roles — the solids and text. Surfaces and
    /// borders are anchored by lightness separation and stay aligned across
    /// families, which is what keeps the ramps looking like a set.
    #[serde(default)]
    pub contrast_shift: f64,

    /// Corrective hue offsets as `[lightness, degrees]` pairs.
    ///
    /// Oklab drifts blues toward purple at high chroma as lightness changes.
    /// This compensates that, per family, without leaving the space. It is
    /// applied *after* torsion and is deliberately a separate field: torsion
    /// is intent, this is a workaround for a known defect.
    #[serde(default)]
    pub hue_correction: Vec<[f64; 2]>,
}

/// The neutral ramp.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Neutral {
    /// Hue the neutral is tinted toward. Defaults to the accent family's hue,
    /// which is almost always what a brand-tinted gray wants.
    pub tint_hue: Option<f64>,

    /// Relative chroma of the tint. Low but **not zero**: a faintly tinted
    /// gray sits with a brand far better than a dead neutral.
    #[serde(default = "default_tint_strength")]
    pub tint_strength: f64,

    /// Opt out of tinting entirely for a strictly achromatic ramp.
    #[serde(default)]
    pub achromatic: bool,

    /// Hue for the `cool` neutral variant. `None` disables the variant.
    pub cool_hue: Option<f64>,

    /// Hue for the `warm` neutral variant. `None` disables the variant.
    pub warm_hue: Option<f64>,

    /// Tint strength for the variants, defaulting to [`Neutral::tint_strength`].
    ///
    /// Separate because the two are not symmetric: a blue at low chroma reads
    /// as neutral more readily than an orange does, so a cool gray needs more
    /// chroma than a warm one to read as tinted at all.
    pub cool_tint_strength: Option<f64>,
    /// As above, for the warm variant.
    pub warm_tint_strength: Option<f64>,

    /// How many steps the neutral ramp has. Independent of the scale roles,
    /// because interfaces need far finer gray resolution than twelve steps.
    #[serde(default = "default_neutral_steps")]
    pub steps: usize,

    /// Where to concentrate those steps.
    ///
    /// Given as bands over lightness with a relative weight. The default puts
    /// extra resolution where interface surfaces actually live — just below
    /// white for light mode, just above black for dark mode — and leaves the
    /// middle sparser.
    #[serde(default = "crate::defaults::density")]
    pub density: Vec<DensityBand>,
}

/// A lightness band with a sampling weight.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DensityBand {
    /// Inclusive lightness range as `[low, high]`.
    pub range: [f64; 2],
    /// Relative density inside the range. `1.0` is the baseline elsewhere.
    pub weight: f64,
}

/// The translucency ladder: one wash of a family's strongest step per stop.
///
/// Deliberately *real* alpha rather than a hex solved to composite to a target
/// over a fixed backdrop. Solving bakes in one backdrop, which is the opposite
/// of what translucency is for: an overlay has to work over the page, over a
/// card, and over an image.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Alpha {
    /// Opacity of each stop, as a percentage from 0 to 100.
    #[serde(default = "crate::defaults::alpha_percentages")]
    pub percentages: Vec<f64>,

    /// The role each stop is a wash of.
    ///
    /// `None` means the **last** role in the scale, which is what a ladder
    /// wants: the end of the ramp is the family's strongest step, so mixed with
    /// transparent at 2% it is the faintest wash the family can produce, and it
    /// flips with the mode for free — a dark wash on a light page, a light wash
    /// on a dark one, from one token.
    ///
    /// Named "the last role" rather than defaulted to `text-strong`, because a
    /// spec is free to rename its roles and the ladder should not stop working
    /// when it does.
    pub role: Option<String>,
}

impl Default for Alpha {
    fn default() -> Self {
        Self {
            percentages: crate::defaults::alpha_percentages(),
            role: None,
        }
    }
}

/// A named scale: a hue path, and the stops along it.
///
/// Distinct from [`Chart`], which predates it and stays for compatibility: a
/// chart spreads hues *around the wheel* to be told apart, while a scale walks
/// a hue *path* to be read in order.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NamedScale {
    /// Token stem. `level` produces `level-0`, `level-1`, and so on.
    pub name: String,

    /// The stops: a count for numbered stops, or explicit names.
    pub stops: Stops,

    /// Hue over the scale. A traffic light is
    /// `{ knots = [[0.0, 144.0], [0.5, 90.0], [1.0, 30.0]] }`.
    pub hue: HueSpec,

    /// Relative chroma over the scale.
    #[serde(default = "crate::defaults::chroma_curve")]
    pub cr: CurveSpec,

    /// Lightness at the middle of the scale, per mode.
    #[serde(default = "default_chart_lightness_light")]
    pub lightness_light: f64,
    /// As above, for dark mode.
    #[serde(default = "default_chart_lightness_dark")]
    pub lightness_dark: f64,

    /// How much lightness the scale spans.
    ///
    /// Load-bearing for an ordinal scale rather than a tiebreaker: hue is the
    /// axis dichromacy removes, so a monotone lightness ramp is what makes the
    /// scale still read as ordered without it.
    #[serde(default = "default_chart_lightness_spread")]
    pub lightness_spread: f64,

    /// Whether the stops are perceptually spaced along the path.
    ///
    /// `true` walks the path measuring delta-E and places stops at equal
    /// perceptual intervals; `false` spaces them evenly in `t`.
    #[serde(default = "default_true")]
    pub even: bool,
}

/// How a scale's stops are named.
#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum Stops {
    /// A count, producing `0` through `count - 1`.
    Count(usize),
    /// Explicit names, used verbatim as the token stem.
    Named(Vec<String>),
}

impl Stops {
    /// How many stops there are.
    #[must_use]
    pub fn len(&self) -> usize {
        match self {
            Self::Count(n) => *n,
            Self::Named(names) => names.len(),
        }
    }

    /// Whether there are no stops at all.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// The token stem for stop `index`, without the scale name.
    ///
    /// A counted scale yields the index; a named scale yields its name. Both
    /// land in `ResolvedStep::role`, which is a free-form string.
    #[must_use]
    pub fn label(&self, index: usize) -> String {
        match self {
            Self::Count(_) => index.to_string(),
            Self::Named(names) => names
                .get(index)
                .cloned()
                .unwrap_or_else(|| index.to_string()),
        }
    }
}

fn default_true() -> bool {
    true
}

/// One accent hue offered as a palette.
///
/// Everything here is applied to the `accent` family. It carries
/// `hue_correction` as well as `hue` because Oklab drifts blues toward purple
/// as they darken, and an accent grid that spans the wheel needs that
/// correction on some of its members and not on others.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Accent {
    /// Hue over the ramp, replacing the accent family's own.
    pub hue: HueSpec,

    /// Relative chroma, replacing the accent family's own. Rarely needed —
    /// the default curve suits most hues.
    pub cr: Option<CurveSpec>,

    /// Corrective hue offsets as `[lightness, degrees]` pairs, as on a family.
    #[serde(default)]
    pub hue_correction: Vec<[f64; 2]>,
}

/// A named preset over the same families.
///
/// A theme is mostly a handful of multipliers, which is what makes "sober",
/// "vivid" and "gray-heavy" three lines each rather than three palettes.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Theme {
    /// Global multiplier on every family's relative chroma. This is the whole
    /// sober-to-vivid axis.
    #[serde(default = "default_chroma_multiplier")]
    pub chroma: f64,

    /// Which accent this theme came from, when it was generated from the
    /// accent grid. `None` for a hand-written theme.
    ///
    /// Carried through to the emitted JSON so the documentation site can offer
    /// the two axes as two controls instead of parsing them back out of a
    /// name.
    #[serde(skip)]
    pub accent: Option<String>,

    /// Which saturation this theme came from. `None` for a hand-written theme.
    #[serde(skip)]
    pub saturation: Option<String>,

    /// Which family fills which semantic role.
    #[serde(default)]
    pub semantic: IndexMap<String, Spanned<String>>,

    /// Per-family overrides applied on top of the family definition.
    #[serde(default)]
    pub families: IndexMap<String, FamilyOverride>,
}

/// Per-theme adjustments to one family.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FamilyOverride {
    /// Replacement hue.
    pub hue: Option<HueSpec>,
    /// Replacement relative-chroma curve.
    pub cr: Option<CurveSpec>,
    /// Multiplier applied to this family's relative chroma, on top of the
    /// theme-wide one.
    pub chroma: Option<f64>,
    /// Replacement corrective hue offsets.
    ///
    /// A theme that moves a family's hue usually needs a different correction
    /// with it: the term compensates a defect that depends on where the hue
    /// sits, so carrying the base family's correction onto a hue it was never
    /// measured for is worse than carrying none.
    pub hue_correction: Option<Vec<[f64; 2]>>,
}

/// The categorical scale.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Chart {
    /// Token stem, for an entry in `[[charts]]`.
    ///
    /// `None` on the unnamed `[chart]`, which is always emitted as `chart-*`.
    /// One struct rather than a `Chart` and a `NamedChart` that repeat ten
    /// fields between them: `#[serde(flatten)]` would have been the other way
    /// to share them, and it silently disables `deny_unknown_fields`, which
    /// this format relies on. The two shapes that would be wrong — a name on
    /// `[chart]`, no name in `[[charts]]` — are caught by
    /// [`crate::validate`], with a span and a fix.
    #[serde(default)]
    pub name: Option<String>,

    /// Whether this set is documented as needing a labelled legend.
    ///
    /// Six generated colors can be kept apart under all three dichromacies;
    /// twelve cannot, and no arrangement of hue and lightness changes that.
    /// Setting this says the limit is understood and the legend names every
    /// entry, so the colour-vision gate reports the measured margins as notes
    /// rather than as warnings — the difference being that a warning means *a
    /// different choice would fix this* and a note means *this is the measured
    /// limit*. It never silences a finding, and it never lowers the floor:
    /// two entries that are literally the same colour still fail.
    #[serde(default)]
    pub labelled: bool,

    /// How many colors to generate.
    #[serde(default = "default_chart_count")]
    pub count: usize,

    /// How to distribute hues around the wheel.
    #[serde(default)]
    pub spread: Spread,

    /// Relative chroma shared by every entry.
    #[serde(default = "default_chart_cr")]
    pub cr: f64,

    /// Lightness shared by every entry in light mode.
    #[serde(default = "default_chart_lightness_light")]
    pub lightness_light: f64,

    /// Lightness shared by every entry in dark mode.
    #[serde(default = "default_chart_lightness_dark")]
    pub lightness_dark: f64,

    /// Hue of the first entry. Defaults to the accent family's hue, so a
    /// chart leads with the brand.
    pub hue_start: Option<f64>,

    /// Total lightness range the entries span.
    ///
    /// Also for colour-vision safety, and for the same reason as
    /// [`Family::contrast_shift`]. Categorical colors at one lightness are
    /// distinguished by hue alone; a dichromat has at most two dimensions to
    /// tell them apart with, and one of them is lightness. Spreading the
    /// entries across a lightness range is what makes a chart readable to
    /// everyone rather than to most people.
    ///
    /// Zero reproduces the older behavior of one shared lightness, and fails
    /// the colour-vision gate for any interesting number of entries.
    #[serde(default = "default_chart_lightness_spread")]
    pub lightness_spread: f64,
}

/// How categorical hues are distributed.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Spread {
    /// Equalize the perceptual difference between consecutive entries.
    ///
    /// The default, and not the same as equal hue angles: a fixed rotation
    /// through yellow-green changes appearance far less than the same
    /// rotation through blue, so equal angles produce visibly uneven sets.
    #[default]
    EvenDeltaE,
    /// Equal hue angles. Simple and predictable, perceptually uneven.
    EvenHue,
    /// Golden-angle steps, which separate neighbours well when a chart uses
    /// only the first few entries.
    Golden,
}

/// An export destination.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Consumer {
    /// Name, used in reporting.
    pub name: String,
    /// Where to write, relative to the repository root.
    pub path: String,
    /// Which emitters this consumer wants.
    pub targets: Vec<String>,
}

// --- Defaults -------------------------------------------------------------

fn default_gamut() -> Gamut {
    Gamut::Srgb
}

fn default_upgrades() -> Vec<Gamut> {
    vec![Gamut::DisplayP3]
}

fn default_prefix() -> String {
    "nc".to_owned()
}

fn default_tint_strength() -> f64 {
    0.035
}

fn default_neutral_steps() -> usize {
    12
}

fn default_chroma_multiplier() -> f64 {
    1.0
}

fn default_chart_count() -> usize {
    // Six, not eight. Measured: with an optimal lightness spread, eight
    // generated colors bottom out at 0.0416 separation under protanopia and
    // six reach 0.0724. Eight *can* be made safe — Okabe and Ito's palette
    // does it — but only by choosing each colour by hand, which is not
    // something a formula does. Ask for more and the gate says so.
    6
}

fn default_chart_cr() -> f64 {
    0.85
}

fn default_chart_lightness_light() -> f64 {
    0.62
}

fn default_chart_lightness_dark() -> f64 {
    0.70
}

fn default_chart_lightness_spread() -> f64 {
    0.60
}

impl Default for Output {
    fn default() -> Self {
        Self {
            gamut: default_gamut(),
            upgrades: default_upgrades(),
            prefix: default_prefix(),
        }
    }
}

impl Default for Scale {
    fn default() -> Self {
        Self {
            roles: crate::defaults::roles(),
        }
    }
}

impl Default for Neutral {
    fn default() -> Self {
        Self {
            tint_hue: None,
            cool_hue: None,
            warm_hue: None,
            cool_tint_strength: None,
            warm_tint_strength: None,
            tint_strength: default_tint_strength(),
            achromatic: false,
            steps: default_neutral_steps(),
            density: crate::defaults::density(),
        }
    }
}

impl Default for Theme {
    fn default() -> Self {
        Self {
            chroma: default_chroma_multiplier(),
            accent: None,
            saturation: None,
            semantic: IndexMap::new(),
            families: IndexMap::new(),
        }
    }
}

impl Default for Chart {
    fn default() -> Self {
        Self {
            name: None,
            labelled: false,
            count: default_chart_count(),
            spread: Spread::default(),
            cr: default_chart_cr(),
            lightness_light: default_chart_lightness_light(),
            lightness_dark: default_chart_lightness_dark(),
            hue_start: None,
            lightness_spread: default_chart_lightness_spread(),
        }
    }
}

// --- Gamut deserialization ------------------------------------------------
//
// `noctua-core` deliberately has no serde dependency, so the mapping lives
// here rather than as an impl over there.

fn de_gamut<'de, D>(deserializer: D) -> Result<Gamut, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let id = String::deserialize(deserializer)?;
    Gamut::from_id(&id).ok_or_else(|| {
        serde::de::Error::custom(format!(
            "unknown gamut `{id}`; expected one of: {}",
            Gamut::all().map(Gamut::id).join(", ")
        ))
    })
}

fn de_gamut_list<'de, D>(deserializer: D) -> Result<Vec<Gamut>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let ids = Vec::<String>::deserialize(deserializer)?;
    ids.into_iter()
        .map(|id| {
            Gamut::from_id(&id).ok_or_else(|| {
                serde::de::Error::custom(format!(
                    "unknown gamut `{id}`; expected one of: {}",
                    Gamut::all().map(Gamut::id).join(", ")
                ))
            })
        })
        .collect()
}

#[cfg(test)]
// Comparisons here are against literal values the code returns verbatim.
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;

    #[test]
    fn a_three_line_spec_is_enough() {
        let spec: Spec = toml::from_str("[families.accent]\nhue = 264").expect("parses");
        assert_eq!(spec.output.gamut, Gamut::Srgb);
        assert_eq!(spec.output.prefix, "nc");
        assert_eq!(spec.scale.roles.len(), 12);
        assert_eq!(spec.neutral.steps, 12);
        assert!(!spec.neutral.achromatic);
        assert!(spec.families.contains_key("accent"));
    }

    #[test]
    fn family_order_follows_the_spec_rather_than_the_alphabet() {
        let spec: Spec = toml::from_str(
            "[families.zebra]\nhue = 10\n[families.alpha]\nhue = 20\n[families.mid]\nhue = 30",
        )
        .expect("parses");
        let order: Vec<&str> = spec.families.keys().map(String::as_str).collect();
        assert_eq!(
            order,
            ["zebra", "alpha", "mid"],
            "iteration must be deterministic"
        );
    }

    #[test]
    fn an_unknown_gamut_is_rejected_with_the_valid_options() {
        let error =
            toml::from_str::<Spec>("[output]\ngamut = \"cmyk\"").expect_err("should reject");
        let text = error.to_string();
        assert!(text.contains("cmyk"), "{text}");
        assert!(
            text.contains("display-p3"),
            "should list the alternatives: {text}"
        );
    }

    #[test]
    fn a_misspelled_key_is_rejected_rather_than_silently_ignored() {
        // The whole point of `deny_unknown_fields`: a typo that silently does
        // nothing is far worse than a build failure.
        let error =
            toml::from_str::<Spec>("[output]\nprefixx = \"nc\"").expect_err("should reject");
        assert!(error.to_string().contains("prefixx"), "{error}");
    }

    #[test]
    fn overrides_replace_only_what_they_name() {
        let spec: Spec = toml::from_str(
            r"
            [families.accent]
            hue = 264
            [themes.vivid]
            chroma = 1.2
            [themes.vivid.families.accent]
            chroma = 1.1
            ",
        )
        .expect("parses");
        let theme = &spec.themes["vivid"];
        assert!((theme.chroma - 1.2).abs() < 1e-12);
        let over = &theme.families["accent"];
        assert!(over.hue.is_none() && over.cr.is_none());
        assert!((over.chroma.expect("set") - 1.1).abs() < 1e-12);
    }

    #[test]
    fn every_spread_name_parses() {
        for (text, expected) in [
            ("even-delta-e", Spread::EvenDeltaE),
            ("even-hue", Spread::EvenHue),
            ("golden", Spread::Golden),
        ] {
            let spec: Spec = toml::from_str(&format!("[chart]\nspread = \"{text}\"")).expect(text);
            assert_eq!(spec.chart.spread, expected);
        }
    }
}
