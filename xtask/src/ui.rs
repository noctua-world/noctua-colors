//! Terminal output.
//!
//! Kept in one place so every verb sounds like the same program, and so the
//! decision about what deserves emphasis is made once. Colour goes through
//! `anstream`, which strips it when output is not a terminal — a log file
//! full of escape codes helps nobody.

use anstyle::{AnsiColor, Style};

const HEADING: Style = Style::new().bold();
const GOOD: Style = Style::new().fg_color(Some(anstyle::Color::Ansi(AnsiColor::Green)));
const BAD: Style = Style::new()
    .fg_color(Some(anstyle::Color::Ansi(AnsiColor::Red)))
    .bold();
const WARN: Style = Style::new().fg_color(Some(anstyle::Color::Ansi(AnsiColor::Yellow)));
const DIM: Style = Style::new().dimmed();

/// A section heading.
pub(crate) fn heading(text: &str) {
    anstream::println!("{HEADING}{text}{HEADING:#}");
}

/// A step that succeeded.
pub(crate) fn ok(text: &str) {
    anstream::println!("  {GOOD}ok{GOOD:#}    {text}");
}

/// A step that failed.
pub(crate) fn failure(text: &str) {
    anstream::eprintln!("  {BAD}error{BAD:#} {text}");
}

/// Something worth reading that does not stop the build.
pub(crate) fn warn(text: &str) {
    anstream::println!("  {WARN}warn{WARN:#}  {text}");
}

/// A measured limit, published rather than actionable.
///
/// Dimmed and not yellow, deliberately: yellow says *look at this, you could
/// fix it*, and these cannot be fixed. They are here so the number is on the
/// record and so a real warning has an empty field to stand out in.
pub(crate) fn note(text: &str) {
    anstream::println!("  {DIM}note{DIM:#}  {text}");
}

/// Secondary detail.
pub(crate) fn detail(text: &str) {
    anstream::println!("        {DIM}{text}{DIM:#}");
}

/// A blank line, for grouping.
pub(crate) fn gap() {
    anstream::println!();
}
