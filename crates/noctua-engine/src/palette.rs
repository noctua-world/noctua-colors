//! Building a whole palette from a spec.
//!
//! Everything else in this crate answers one question at a time. This module
//! walks the spec — every theme, both modes, every gamut, every family, every
//! role — and produces the finished color set that emitters and gates consume.
//!
//! # Quantization happens exactly once, here
//!
//! Colors are rounded at construction, and the rounded value is what flows on
//! to every gate and every emitter. Nothing downstream re-rounds.
//!
//! That matters more than it sounds. If a gate measured full-precision colors
//! while the emitter wrote four decimal places, the gate would be certifying a
//! palette nobody ships — and the discrepancy would appear exactly at the
//! margins, where a step barely passes. Quantizing once means **what was
//! checked is byte-for-byte what ships**.
//!
//! Chroma is rounded *down* rather than to nearest, so quantization can only
//! move a color further inside its gamut, never out.

use indexmap::IndexMap;
use noctua_core::map::Mapped;
use noctua_core::space::normalize_hue;
use noctua_core::{Gamut, Oklch, Rgb, map_into_gamut, to_hex};
use noctua_spec::{Spec, Spread};

use crate::chart;
use crate::curve::Curve;
use crate::error::EngineError;
use crate::neutral;
use crate::ordinal;
use crate::solve::{Anchor, FamilyCurves, Mode, solve};

/// Decimal places kept for lightness and chroma.
const COORDINATE_PLACES: i32 = 4;

/// Decimal places kept for hue. Degrees are a coarser unit, and two places
/// resolve far finer than any display can show.
const HUE_PLACES: i32 = 2;

/// Lightness range of the dense neutral ramp.
///
/// Stops short of pure black and pure white deliberately: both are harsh as
/// interface surfaces, and a ramp that ends at them has nowhere left to go
/// when a design needs one more step.
///
/// The low end is 0.10 rather than something nearer zero for a concrete
/// reason: Oklab lightness below about 0.09 rounds to `#000000` in eight-bit
/// sRGB, so steps placed there are not merely dark, they are *the same color*.
/// A ramp starting at 0.04 spent its first step on pure black and its next two
/// within one code value of each other.
const NEUTRAL_RANGE: (f64, f64) = (0.1000, 0.9900);

/// A finished color system.
#[derive(Debug, Clone)]
pub struct Palette {
    /// Custom-property namespace, from `output.prefix`.
    pub prefix: String,
    /// Gamuts emitted, primary first.
    pub gamuts: Vec<Gamut>,
    /// Scale role names, in ramp order.
    pub roles: Vec<String>,
    /// Roles that a family's contrast shift applies to, and which are
    /// therefore allowed to sit off the ramp's lightness trajectory.
    ///
    /// The solid steps. A solid is chosen to be recognised as the brand or as
    /// a state, not to occupy a particular rung — real twelve-step scales
    /// break their own ramp there on purpose, and the gates need to know that
    /// so they check drift rather than intent.
    pub shiftable_roles: Vec<String>,
    /// The dense neutral ramps, keyed by token stem — `gray`, and `gray-cool`
    /// and `gray-warm` where the spec asks for them.
    ///
    /// Mode-independent: these are a raw resource that both modes draw from, so
    /// `gray-4` means one color, not two. Every ramp shares the same step
    /// lightnesses — placement depends only on `steps` and `density` — so
    /// `gray-7`, `gray-cool-7` and `gray-warm-7` differ in tint and nothing
    /// else, and one can be swapped for another without moving any contrast.
    pub neutral_ramps: IndexMap<String, Vec<ResolvedStep>>,
    /// Content hash of the specification this was built from.
    ///
    /// Carried so an artifact can name the input it came from. A downstream
    /// repository that pins the axes can then tell a palette that moved
    /// underneath it from one that did not.
    pub spec_hash: String,
    /// What the colour system calls itself, and the version being published.
    ///
    /// Carried on the palette so emitters need no access to the spec, and so
    /// the version they stamp is **the colour system's**, not the compiler's.
    /// Before this it came from `env!("CARGO_PKG_VERSION")`, which is the
    /// compiler's own version and is fixed at compile time — which is also
    /// what made `xtask release` stamp the version it had just replaced.
    pub identity: Identity,
    /// The translucency ladder.
    pub alpha: AlphaScale,
    /// Every theme, in spec order.
    pub themes: Vec<ResolvedTheme>,
}

/// What the colour system calls itself, from the spec's `[system]` table.
#[derive(Debug, Clone)]
pub struct Identity {
    /// The published version. Distinct from the compiler's.
    pub version: String,
    /// The name generated package metadata uses.
    pub name: String,
    /// One line, for the same metadata.
    pub description: String,
}

/// The translucency ladder, carried through so emitters need no spec.
#[derive(Debug, Clone)]
pub struct AlphaScale {
    /// Opacity of each stop, as a percentage.
    pub percentages: Vec<f64>,
    /// The role each stop is a wash of.
    pub role: String,
    /// Families the ladder is emitted for, in palette order.
    ///
    /// The neutrals and the accent, not every family. A wash is either a way to
    /// dim or raise the page — which is what a neutral is — or a way to show
    /// selection and focus, which is what the brand is. A `danger` wash at 6%
    /// would be a worse-specified duplicate of `danger-bg`, which is a solved
    /// step with checked contrast rather than a colour that depends on whatever
    /// it happens to sit on.
    pub families: Vec<String>,
}

impl Palette {
    /// The untinted dense ramp, for the cases that mean specifically that one.
    ///
    /// Never empty in practice; returns `&[]` only for a spec with no neutral
    /// steps at all.
    #[must_use]
    pub fn neutral_ramp(&self) -> &[ResolvedStep] {
        self.neutral_ramps
            .get(BASE_NEUTRAL_RAMP)
            .map_or(&[], Vec::as_slice)
    }
}

/// Token stem of the untinted dense neutral ramp.
pub const BASE_NEUTRAL_RAMP: &str = "gray";

/// Key of the unnamed categorical scale within [`ResolvedMode::scales`].
pub const CHART_SCALE: &str = "chart";

/// What a scale is for, and therefore how it is read and how it is checked.
///
/// Recorded rather than inferred from the name. It used to be `name ==
/// "chart"`, spelled out in the colour-vision gate, the documentation site and
/// the site's script — three copies of one fact, and all three wrong the moment
/// a second categorical set existed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScaleKind {
    /// Hues spread *around the wheel*, told apart from a legend.
    ///
    /// Checked pairwise, every entry against every other, because any two of
    /// them can appear side by side and confusing them loses the meaning.
    Categorical {
        /// Whether the spec declares that this set's legend names every entry.
        ///
        /// Colour alone cannot separate more than about six generated entries
        /// under all three dichromacies. Above that the honest report is the
        /// measured margin as a note, not a warning telling someone to make a
        /// different choice when no different choice exists.
        labelled: bool,
    },
    /// A hue *path*, walked in order.
    ///
    /// Pairwise is the wrong property: confusing `level-2` with `level-7`
    /// loses precision, not meaning. Checked instead on neighbours being
    /// separable, ends being opposed, and simulated lightness staying
    /// monotone.
    Ordered,
}

impl ScaleKind {
    /// Whether this scale is read off a legend rather than in order.
    #[must_use]
    pub fn is_categorical(self) -> bool {
        matches!(self, Self::Categorical { .. })
    }

    /// Whether the spec declares this set's legend names every entry.
    #[must_use]
    pub fn is_labelled(self) -> bool {
        matches!(self, Self::Categorical { labelled: true })
    }

    /// The word for this kind, as emitted and as shown.
    #[must_use]
    pub fn id(self) -> &'static str {
        match self {
            Self::Categorical { .. } => "categorical",
            Self::Ordered => "ordered",
        }
    }
}

/// One scale, resolved: what it is for, and its stops.
#[derive(Debug, Clone)]
pub struct ResolvedScale {
    /// What this scale is for.
    pub kind: ScaleKind,
    /// The stops, in order.
    pub steps: Vec<ResolvedStep>,
}

/// One theme, resolved in both modes.
#[derive(Debug, Clone)]
pub struct ResolvedTheme {
    /// Theme name.
    pub name: String,
    /// Which accent this palette came from, when it came from the accent grid.
    ///
    /// Carried so a consumer can offer the two axes as two controls rather
    /// than splitting the name on a hyphen and hoping.
    pub accent: Option<String>,
    /// Which saturation this palette came from.
    pub saturation: Option<String>,
    /// Light and dark, in that order.
    pub modes: Vec<ResolvedMode>,
}

/// One theme in one mode.
#[derive(Debug, Clone)]
pub struct ResolvedMode {
    /// Which mode this is.
    pub mode: Mode,
    /// Families, keyed by name. `neutral` comes first.
    pub families: IndexMap<String, ResolvedFamily>,
    /// Scales, keyed by token stem, with the categorical `chart` first.
    ///
    /// One map rather than a field per scale, because there is nothing special
    /// about any of them: an emitter loops over whatever is here, so adding a
    /// scale to the spec adds it to seven output formats without touching one
    /// of them. Each carries its own [`ScaleKind`], so the one gate that has to
    /// treat categorical and ordered differently asks the scale rather than
    /// comparing its name.
    pub scales: IndexMap<String, ResolvedScale>,
    /// Semantic slot to the family that fills it.
    pub semantic: IndexMap<String, String>,
}

impl ResolvedMode {
    /// The unnamed categorical chart, for the cases that mean specifically
    /// that one — the hero figure on the documentation site, and the golden.
    #[must_use]
    pub fn chart(&self) -> &[ResolvedStep] {
        self.scales
            .get(CHART_SCALE)
            .map_or(&[], |scale| scale.steps.as_slice())
    }

    /// Every categorical scale, in spec order.
    pub fn categorical(&self) -> impl Iterator<Item = (&String, &ResolvedScale)> {
        self.scales
            .iter()
            .filter(|(_, scale)| scale.kind.is_categorical())
    }

    /// Every ordered scale, in spec order.
    pub fn ordered(&self) -> impl Iterator<Item = (&String, &ResolvedScale)> {
        self.scales
            .iter()
            .filter(|(_, scale)| !scale.kind.is_categorical())
    }
}

/// One family's ramp.
#[derive(Debug, Clone)]
pub struct ResolvedFamily {
    /// Family name.
    pub name: String,
    /// Nominal hue, for reporting.
    pub base_hue: f64,
    /// Steps, in ramp order.
    pub steps: Vec<ResolvedStep>,
}

/// One step of a ramp, rendered in every emitted gamut.
#[derive(Debug, Clone)]
pub struct ResolvedStep {
    /// Role name, or the index as a string for ramps without roles.
    pub role: String,
    /// Position in the ramp, from one.
    pub index: usize,
    /// One rendition per gamut, in the same order as [`Palette::gamuts`].
    pub renditions: Vec<ResolvedColor>,
}

impl ResolvedStep {
    /// The rendition in the primary gamut, which is what the hex fallback and
    /// every gate are measured from.
    #[must_use]
    pub fn primary(&self) -> &ResolvedColor {
        &self.renditions[0]
    }
}

/// One step in one gamut.
#[derive(Debug, Clone)]
pub struct ResolvedColor {
    /// The gamut this rendition resolves against.
    pub gamut: Gamut,
    /// Quantized OKLCH. This is the authoritative value.
    pub oklch: Oklch,
    /// Encoded channels in `gamut`.
    pub rgb: Rgb,
    /// Relative chroma the spec asked for, before the gamut had a say.
    pub requested_relative_chroma: f64,
    /// Relative chroma actually achieved: a shortfall means the gamut ran out.
    pub achieved_relative_chroma: f64,
    /// How much chroma remains between this color and the gamut boundary.
    pub chroma_headroom: f64,
}

impl ResolvedColor {
    /// Channel values as `#rrggbb`.
    ///
    /// Only a valid CSS hex color when [`Self::gamut`] is sRGB; for wider
    /// gamuts these are that gamut's channels and belong in a `color()`
    /// function instead.
    #[must_use]
    pub fn hex(&self) -> String {
        to_hex(self.rgb)
    }
}

/// Builds a palette from a validated spec.
///
/// # Errors
///
/// Returns the first target the color math cannot satisfy, with what the
/// family could actually reach.
pub fn build(spec: &Spec) -> Result<Palette, EngineError> {
    let gamuts = gamut_list(spec);
    let roles: Vec<String> = spec
        .scale
        .roles
        .iter()
        .map(|r| r.name.get_ref().clone())
        .collect();

    // The dense ramps are shared by every theme, so they take the base hue.
    let neutral_ramps: IndexMap<String, Vec<ResolvedStep>> = neutral_tints(spec, None)
        .into_iter()
        .map(|tint| {
            (
                format!("{BASE_NEUTRAL_RAMP}{}", tint.suffix),
                build_neutral_ramp(spec, &tint, &gamuts),
            )
        })
        .collect();

    // A spec that names no themes still gets one, so that the default path
    // needs no ceremony.
    let theme_names: Vec<String> = if spec.themes.is_empty() {
        vec!["default".to_owned()]
    } else {
        spec.themes.keys().cloned().collect()
    };

    let mut themes = Vec::with_capacity(theme_names.len());
    for name in theme_names {
        let tints = neutral_tints(spec, Some(&name));
        let mut modes = Vec::with_capacity(2);
        for mode in Mode::all() {
            modes.push(build_mode(spec, &name, mode, &gamuts, &tints)?);
        }
        let (accent, saturation) = spec
            .themes
            .get(&name)
            .map_or((None, None), |t| (t.accent.clone(), t.saturation.clone()));
        themes.push(ResolvedTheme {
            name,
            accent,
            saturation,
            modes,
        });
    }

    // The neutrals and the accent. Derived rather than configured: which
    // families want a wash follows from what a wash is for, and a spec that
    // could ask for `danger-a3` would be inviting a token the gates cannot
    // check against any backdrop.
    let alpha_families: Vec<String> = themes
        .first()
        .and_then(|theme| theme.modes.first())
        .map(|mode| {
            mode.families
                .keys()
                .filter(|name| name.starts_with("neutral") || *name == "accent")
                .cloned()
                .collect()
        })
        .unwrap_or_default();

    let alpha_role = spec
        .alpha
        .role
        .clone()
        .or_else(|| roles.last().cloned())
        .unwrap_or_default();

    let shiftable_roles: Vec<String> = spec
        .scale
        .roles
        .iter()
        .filter(|r| r.shift)
        .map(|r| r.name.get_ref().clone())
        .collect();

    Ok(Palette {
        prefix: spec.output.prefix.clone(),
        gamuts,
        roles,
        shiftable_roles,
        neutral_ramps,
        spec_hash: spec.source_hash.clone(),
        identity: Identity {
            version: spec.system.version.clone(),
            name: spec.system.name.clone(),
            description: spec.system.description.clone(),
        },
        alpha: AlphaScale {
            percentages: spec.alpha.percentages.clone(),
            // The end of the ramp unless the spec names something else.
            role: alpha_role,
            families: alpha_families,
        },
        themes,
    })
}

/// The primary gamut followed by any upgrades, without repeats.
fn gamut_list(spec: &Spec) -> Vec<Gamut> {
    let mut gamuts = vec![spec.output.gamut];
    for gamut in &spec.output.upgrades {
        if !gamuts.contains(gamut) {
            gamuts.push(*gamut);
        }
    }
    gamuts
}

/// The hue a neutral is tinted toward.
///
/// Defaults to the accent family's hue, because a gray that leans very
/// slightly toward the brand sits with it far better than a dead neutral —
/// and defaulting saves the author from repeating a number they already wrote.
///
/// Read per theme where a theme is given, so a palette that moves its accent
/// moves its grays with it. Reading the base family for every theme would tint
/// the whole grid toward the first accent, which is the one thing the grid
/// exists to vary.
///
/// `None` asks for the base hue, which is what the **dense** ramp uses: that
/// ramp is emitted once and shared by every theme and both modes, so it cannot
/// follow any one accent. The per-theme `neutral` *family* can and does — the
/// surfaces, borders and text a palette actually paints with lean toward its
/// own accent, while `--nc-gray-*` stays a fixed resource.
fn neutral_tint_hue(spec: &Spec, theme_name: Option<&str>) -> f64 {
    if let Some(explicit) = spec.neutral.tint_hue {
        return explicit;
    }

    let overridden = theme_name
        .and_then(|name| spec.themes.get(name))
        .and_then(|theme| theme.families.get("accent"))
        .and_then(|over| over.hue.as_ref());

    overridden.map_or_else(
        || {
            spec.families
                .get("accent")
                .or_else(|| spec.families.values().next())
                .map_or(0.0, |family| family.hue.base())
        },
        noctua_spec::HueSpec::base,
    )
}

/// One way of tinting the neutral: which direction, and how far.
///
/// The base tint leans toward the accent and carries an empty suffix, so it
/// keeps the names it has always had — `gray-7`, `neutral`. The `cool` and
/// `warm` variants are fixed hues that do not follow the accent: their whole
/// purpose is to be a *different* temperature from the brand-tinted gray, so
/// following it would collapse them onto it.
#[derive(Debug, Clone, Copy)]
struct NeutralTint {
    /// Appended to `gray` and to `neutral`. Empty for the base.
    suffix: &'static str,
    /// Hue leaned toward, in degrees.
    hue: f64,
    /// Relative chroma of the lean.
    strength: f64,
}

/// The neutral tints a palette emits, base first.
///
/// `theme_name` picks whose accent the base leans toward; `None` asks for the
/// base hue, which is what the shared dense ramps use.
fn neutral_tints(spec: &Spec, theme_name: Option<&str>) -> Vec<NeutralTint> {
    let base = if spec.neutral.achromatic {
        0.0
    } else {
        spec.neutral.tint_strength
    };

    let mut tints = vec![NeutralTint {
        suffix: "",
        hue: neutral_tint_hue(spec, theme_name),
        strength: base,
    }];

    // An achromatic ramp has no temperature to vary — three ramps at zero
    // chroma would be three copies of the same grays under different names.
    if spec.neutral.achromatic {
        return tints;
    }

    // Naming a hue is what asks for the variant. There is no default hue,
    // because there is no answer that is right for every brand — a cool gray
    // beside a blue accent has to lean somewhere other than the accent, and
    // only the spec knows where that is.
    if let Some(hue) = spec.neutral.cool_hue {
        tints.push(NeutralTint {
            suffix: "-cool",
            hue,
            strength: spec.neutral.cool_tint_strength.unwrap_or(base),
        });
    }
    if let Some(hue) = spec.neutral.warm_hue {
        tints.push(NeutralTint {
            suffix: "-warm",
            hue,
            strength: spec.neutral.warm_tint_strength.unwrap_or(base),
        });
    }

    tints
}

/// Curves for a synthesized neutral family.
fn neutral_curves(tint: &NeutralTint, multiplier: f64) -> FamilyCurves {
    FamilyCurves {
        hue: Curve::hue([[0.0, tint.hue], [1.0, tint.hue]]),
        chroma: Curve::constant(tint.strength),
        correction: Curve::constant(0.0),
        multiplier,
    }
}

fn build_neutral_ramp(spec: &Spec, tint: &NeutralTint, gamuts: &[Gamut]) -> Vec<ResolvedStep> {
    let curves = neutral_curves(tint, 1.0);
    let lightnesses = neutral::place(
        spec.neutral.steps,
        NEUTRAL_RANGE.0,
        NEUTRAL_RANGE.1,
        &spec.neutral.density,
    );

    lightnesses
        .iter()
        .enumerate()
        .map(|(i, &lightness)| {
            let t = if lightnesses.len() > 1 {
                i as f64 / (lightnesses.len() - 1) as f64
            } else {
                0.0
            };
            ResolvedStep {
                role: (i + 1).to_string(),
                index: i + 1,
                renditions: gamuts
                    .iter()
                    .map(|&gamut| render(&curves, t, lightness, gamut))
                    .collect(),
            }
        })
        .collect()
}

fn build_mode(
    spec: &Spec,
    theme_name: &str,
    mode: Mode,
    gamuts: &[Gamut],
    tints: &[NeutralTint],
) -> Result<ResolvedMode, EngineError> {
    let theme = spec.themes.get(theme_name);
    let theme_chroma = theme.map_or(1.0, |t| t.chroma);

    let mut families = IndexMap::new();

    // The neutrals go first: they are the ground everything else sits on.
    if !spec.families.contains_key("neutral") {
        for tint in tints {
            let name = format!("neutral{}", tint.suffix);
            let curves = neutral_curves(tint, theme_chroma);
            families.insert(
                name.clone(),
                build_family(
                    spec,
                    &FamilyContext {
                        name: &name,
                        theme: theme_name,
                        base_hue: tint.hue,
                        mode,
                        contrast_shift: 0.0,
                    },
                    &curves,
                    gamuts,
                )?,
            );
        }
    }

    for (name, family) in &spec.families {
        let override_for = theme.and_then(|t| t.families.get(name));

        let hue_spec = override_for
            .and_then(|o| o.hue.as_ref())
            .unwrap_or(&family.hue);
        let chroma_spec = override_for
            .and_then(|o| o.cr.as_ref())
            .unwrap_or(&family.cr);
        let multiplier = theme_chroma * override_for.and_then(|o| o.chroma).unwrap_or(1.0);

        let curves = FamilyCurves {
            hue: Curve::hue(hue_spec.knots()),
            chroma: Curve::new(chroma_spec.knots()),
            // A theme that moves a hue may name its own correction. The term
            // compensates a defect that depends on where the hue sits, so
            // carrying the base family's onto a hue it was never measured for
            // is worse than carrying none.
            correction: {
                let correction = override_for
                    .and_then(|o| o.hue_correction.as_deref())
                    .unwrap_or(&family.hue_correction);
                if correction.is_empty() {
                    Curve::constant(0.0)
                } else {
                    Curve::new(correction.iter().copied())
                }
            },
            multiplier,
        };

        families.insert(
            name.clone(),
            build_family(
                spec,
                &FamilyContext {
                    name,
                    theme: theme_name,
                    base_hue: hue_spec.base(),
                    mode,
                    contrast_shift: family.contrast_shift,
                },
                &curves,
                gamuts,
            )?,
        );
    }

    let semantic = resolve_semantic(spec, theme_name, &families)?;
    let base_hue = tints.first().map_or(0.0, |tint| tint.hue);
    let scales = build_scales(spec, mode, gamuts, base_hue, theme_chroma);

    Ok(ResolvedMode {
        mode,
        families,
        scales,
        semantic,
    })
}

/// Everything `build_family` needs that is not the curves themselves.
struct FamilyContext<'a> {
    /// Family name, for diagnostics and the result.
    name: &'a str,
    /// Theme being built, for diagnostics.
    theme: &'a str,
    /// Nominal hue, carried through for reporting.
    base_hue: f64,
    /// Mode being built.
    mode: Mode,
    /// Shift applied to this family's APCA targets.
    contrast_shift: f64,
}

fn build_family(
    spec: &Spec,
    context: &FamilyContext<'_>,
    curves: &FamilyCurves,
    gamuts: &[Gamut],
) -> Result<ResolvedFamily, EngineError> {
    let FamilyContext {
        name,
        theme: theme_name,
        base_hue,
        mode,
        contrast_shift,
    } = *context;
    let role_count = spec.scale.roles.len();
    let mut steps = Vec::with_capacity(role_count);

    // Resolution is per gamut, because a role anchored on contrast lands at a
    // different lightness once its chroma changes — which is exactly the
    // point of relative chroma.
    let mut resolved: Vec<IndexMap<String, Mapped>> =
        gamuts.iter().map(|_| IndexMap::new()).collect();

    for (index, role) in spec.scale.roles.iter().enumerate() {
        let t = if role_count > 1 {
            index as f64 / (role_count - 1) as f64
        } else {
            0.0
        };
        let role_name = role.name.get_ref();
        let target = if mode == Mode::Light {
            &role.light
        } else {
            &role.dark
        };

        let mut renditions = Vec::with_capacity(gamuts.len());
        for (slot, &gamut) in gamuts.iter().enumerate() {
            let reference_name = target
                .apca
                .as_ref()
                .map(|a| a.against.get_ref())
                .or_else(|| target.delta_l.as_ref().map(|d| d.against.get_ref()));

            let reference = match reference_name {
                Some(reference_name) => {
                    Some(resolved[slot].get(reference_name).ok_or_else(|| {
                        EngineError::UnresolvedReference {
                            role: role_name.clone(),
                            against: reference_name.clone(),
                        }
                    })?)
                }
                None => None,
            };

            let (anchor, requested, units) =
                match (&target.lightness, &target.apca, &target.delta_l) {
                    (Some(value), _, _) => (Anchor::Fixed(*value.get_ref()), 0.0, "Lc"),
                    (_, Some(apca), _) => {
                        // The family's colour-vision shift applies here and
                        // only here. Surfaces and borders are anchored by
                        // lightness separation and stay aligned across
                        // families, which is what keeps the ramps a set;
                        // solids and text are where semantic meaning lives,
                        // and where two families must be told apart.
                        let shift = if role.shift { contrast_shift } else { 0.0 };
                        let lc = (*apca.lc.get_ref() + shift).max(0.0);
                        (
                            Anchor::Apca {
                                reference: reference.expect("checked above"),
                                lc,
                            },
                            lc,
                            "Lc",
                        )
                    }
                    (_, _, Some(delta)) => {
                        let amount = *delta.amount.get_ref();
                        (
                            Anchor::DeltaL {
                                reference: reference.expect("checked above"),
                                amount,
                            },
                            amount,
                            "lightness",
                        )
                    }
                    _ => unreachable!("the spec validator requires exactly one target"),
                };

            let solved = solve(curves, t, anchor, mode, gamut);
            if let Some(achievable) = solved.shortfall {
                return Err(EngineError::UnreachableTarget(Box::new(
                    crate::error::Unreachable {
                        theme: theme_name.to_owned(),
                        mode: mode.id(),
                        family: name.to_owned(),
                        role: role_name.clone(),
                        against: reference_name.cloned().unwrap_or_default(),
                        requested,
                        achievable,
                        units,
                        gamut: gamut.id(),
                    },
                )));
            }

            let color = render(curves, t, solved.lightness, gamut);
            resolved[slot].insert(role_name.clone(), map_into_gamut(color.oklch, gamut));
            renditions.push(color);
        }

        steps.push(ResolvedStep {
            role: role_name.clone(),
            index: index + 1,
            renditions,
        });
    }

    Ok(ResolvedFamily {
        name: name.to_owned(),
        base_hue,
        steps,
    })
}

/// Every scale this mode emits: the categorical sets, then the ordered ones.
///
/// All of them in one map keyed by stem, each carrying its own kind, so an
/// emitter iterates and a gate asks. The unnamed chart keeps [`CHART_SCALE`]
/// and comes first, so the default set is the one a reader meets.
fn build_scales(
    spec: &Spec,
    mode: Mode,
    gamuts: &[Gamut],
    fallback_hue: f64,
    theme_chroma: f64,
) -> IndexMap<String, ResolvedScale> {
    let mut scales = IndexMap::new();

    for chart in std::iter::once(&spec.chart).chain(&spec.charts) {
        let name = chart.name.clone().unwrap_or_else(|| CHART_SCALE.to_owned());
        scales.insert(
            name,
            ResolvedScale {
                kind: ScaleKind::Categorical {
                    labelled: chart.labelled,
                },
                steps: build_chart(chart, mode, gamuts, fallback_hue, theme_chroma),
            },
        );
    }

    for scale in &spec.scales {
        scales.insert(
            scale.name.clone(),
            ResolvedScale {
                kind: ScaleKind::Ordered,
                steps: build_scale(scale, mode, gamuts, theme_chroma),
            },
        );
    }

    scales
}

/// One ordered scale: stops along a hue path.
///
/// Direction is the spec's, not this function's — a traffic light that ascends
/// into red and one that ascends into green differ only in the order of the hue
/// knots, and `lightness_spread` may be negative to descend.
fn build_scale(
    scale: &noctua_spec::NamedScale,
    mode: Mode,
    gamuts: &[Gamut],
    theme_chroma: f64,
) -> Vec<ResolvedStep> {
    let count = scale.stops.len();
    if count == 0 {
        return Vec::new();
    }

    let middle = if mode == Mode::Light {
        scale.lightness_light
    } else {
        scale.lightness_dark
    };

    let curves = FamilyCurves {
        hue: Curve::hue(scale.hue.knots()),
        chroma: Curve::new(scale.cr.knots()),
        correction: Curve::constant(0.0),
        multiplier: theme_chroma,
    };

    // Lightness carries the order, and it is spaced by **stop index** rather
    // than by position along the path. Hue is the axis dichromacy removes, so
    // an even lightness step is the one thing that keeps the scale readable as
    // ordered without it — and index is the only parameter that guarantees the
    // step is even. Spacing it by `t` instead lets it bunch wherever the hue
    // path moves quickly, which measured 0.0192 apart under protanopia, inside
    // the just-noticeable difference.
    let lightness_of = |i: usize| {
        let t = if count > 1 {
            i as f64 / (count - 1) as f64
        } else {
            0.5
        };
        (middle - scale.lightness_spread / 2.0 + scale.lightness_spread * t).clamp(0.08, 0.95)
    };

    // Hue and chroma are placed along the path instead, measured on the scale's
    // middle lightness so the two axes cannot trade against each other.
    let positions = if scale.even {
        ordinal::place(count, &curves, middle, gamuts[0])
    } else {
        (0..count)
            .map(|i| {
                if count > 1 {
                    i as f64 / (count - 1) as f64
                } else {
                    0.0
                }
            })
            .collect()
    };

    positions
        .iter()
        .enumerate()
        .map(|(i, &t)| ResolvedStep {
            role: scale.stops.label(i),
            index: i + 1,
            renditions: gamuts
                .iter()
                .map(|&gamut| render(&curves, t, lightness_of(i), gamut))
                .collect(),
        })
        .collect()
}

fn build_chart(
    chart: &noctua_spec::Chart,
    mode: Mode,
    gamuts: &[Gamut],
    fallback_hue: f64,
    theme_chroma: f64,
) -> Vec<ResolvedStep> {
    let lightness = if mode == Mode::Light {
        chart.lightness_light
    } else {
        chart.lightness_dark
    };
    let start = chart.hue_start.unwrap_or(fallback_hue);
    let primary = gamuts[0];

    // A theme's chroma multiplier applies here too: a "sober" theme with
    // fully-saturated charts would be a system at odds with itself. Hue
    // placement uses the same adjusted chroma, since perceptual spacing
    // depends on how saturated the colors actually are.
    let effective_cr = (chart.cr * theme_chroma).clamp(0.0, 1.0);
    let hues = chart::hues(
        chart.count,
        start,
        chart.spread,
        lightness,
        effective_cr,
        primary,
    );

    // Entries ramp across a lightness range rather than sharing one.
    //
    // Hue alone cannot separate a categorical scale for a dichromat: hue is
    // the axis their vision is missing. Eight colors at one lightness are, to
    // a deuteranope, eight shades of the same thing — measured at 0.004 apart
    // for one pair, against a 0.02 just-noticeable difference. Lightness is
    // the axis that survives, so the scale has to use it.
    let lightness_of = |i: usize| -> f64 {
        if chart.count < 2 {
            return lightness;
        }
        let t = i as f64 / (chart.count - 1) as f64;
        (lightness - chart.lightness_spread / 2.0 + chart.lightness_spread * t).clamp(0.08, 0.95)
    };

    hues.iter()
        .enumerate()
        .map(|(i, &hue)| {
            let lightness = lightness_of(i);
            let curves = FamilyCurves {
                hue: Curve::hue([[0.0, hue], [1.0, hue]]),
                chroma: Curve::constant(chart.cr),
                correction: Curve::constant(0.0),
                multiplier: theme_chroma,
            };
            ResolvedStep {
                role: (i + 1).to_string(),
                index: i + 1,
                renditions: gamuts
                    .iter()
                    .map(|&gamut| render(&curves, 0.0, lightness, gamut))
                    .collect(),
            }
        })
        .collect()
}

/// Which family fills which semantic slot.
///
/// Three layers, each overriding the one before: every family is a slot of its
/// own name, then the spec's global `[semantic]` aliases, then the theme's own.
///
/// A family being its own slot is what keeps forty contexts affordable. Adding
/// a family costs a full ramp in every theme, mode and gamut — megabytes — so
/// the great majority of contexts are aliases onto a family that already
/// exists, and only the ones that must be *told apart* get a hue of their own.
fn resolve_semantic(
    spec: &Spec,
    theme_name: &str,
    families: &IndexMap<String, ResolvedFamily>,
) -> Result<IndexMap<String, String>, EngineError> {
    let mut semantic: IndexMap<String, String> = families
        .keys()
        .map(|name| (name.clone(), name.clone()))
        .collect();

    let theme_aliases = spec.themes.get(theme_name).map(|theme| &theme.semantic);
    for (slot, family) in spec
        .semantic
        .iter()
        .chain(theme_aliases.into_iter().flatten())
    {
        let family = family.get_ref();
        if !families.contains_key(family) {
            return Err(EngineError::UnknownFamily {
                theme: theme_name.to_owned(),
                slot: slot.clone(),
                family: family.clone(),
            });
        }
        semantic.insert(slot.clone(), family.clone());
    }

    Ok(semantic)
}

/// Produces a step's color and quantizes it.
fn render(curves: &FamilyCurves, t: f64, lightness: f64, gamut: Gamut) -> ResolvedColor {
    let mapped = curves.color_at(t, lightness, gamut);
    let quantized = quantize(mapped.oklch, gamut);
    let final_mapped = map_into_gamut(quantized, gamut);

    let boundary = gamut.max_chroma(quantized.l, quantized.h);
    let achieved = if boundary > 0.0 {
        quantized.c / boundary
    } else {
        0.0
    };

    ResolvedColor {
        gamut,
        oklch: quantized,
        rgb: final_mapped.rgb,
        requested_relative_chroma: curves.relative_chroma_at(t),
        achieved_relative_chroma: achieved,
        chroma_headroom: (boundary - quantized.c).max(0.0),
    }
}

/// Rounds a color to the precision that will be emitted.
///
/// Lightness and hue round to nearest; chroma rounds **down**, so that
/// quantization can only move a color further inside its gamut. Rounding
/// chroma to nearest could push it a fraction past the boundary and produce an
/// out-of-gamut token that every earlier check had approved.
fn quantize(color: Oklch, gamut: Gamut) -> Oklch {
    let lightness = round_to(color.l, COORDINATE_PLACES).clamp(0.0, 1.0);
    let hue = round_to(normalize_hue(color.h), HUE_PLACES);

    let boundary = gamut.max_chroma(lightness, hue);
    let chroma = floor_to(color.c.min(boundary), COORDINATE_PLACES).max(0.0);

    Oklch {
        l: lightness,
        c: chroma,
        h: hue,
    }
}

fn round_to(value: f64, places: i32) -> f64 {
    let scale = 10f64.powi(places);
    (value * scale).round() / scale
}

fn floor_to(value: f64, places: i32) -> f64 {
    let scale = 10f64.powi(places);
    (value * scale).floor() / scale
}

/// How many entries the chart scale has, matching [`Spread`] expectations.
#[must_use]
pub fn chart_spread_of(spec: &Spec) -> Spread {
    spec.chart.spread
}
