//! Cell widths, measured the way ratatui draws text (per grapheme), and text made safe to
//! measure that way.

use std::borrow::Cow;

use ratatui::buffer::CellWidth;
use unicode_segmentation::UnicodeSegmentation;

use crate::unsteady::unsteady;

/// What stands in for a character that can't be kept.
const REPLACEMENT: char = '\u{fffd}';

/// Cells a grapheme takes when ratatui draws it: ratatui's width for it, or 0 with
/// control characters, which ratatui skips.
pub fn grapheme_width(g: &str) -> usize {
    if g.contains(char::is_control) { 0 } else { usize::from(g.cell_width()) }
}

/// Cells `s` takes on screen.
pub fn width(s: &str) -> usize {
    s.graphemes(true).map(grapheme_width).sum()
}

/// The longest prefix of `s` that fits in `w` cells. It is cut from `s` as it came, control
/// characters included, which only a [`Buffer`](ratatui::buffer::Buffer) leaves out.
pub fn take_width(s: &str, w: usize) -> &str {
    let mut used: usize = 0;
    for (i, g) in s.grapheme_indices(true) {
        used = used.saturating_add(grapheme_width(g));
        if used > w {
            return &s[..i];
        }
    }
    s
}

/// `s` as a terminal can be trusted to draw it in the cells [`width`] says, which is how
/// [`Field`](crate::Field) and [`Window`](crate::Window) draw text. Pass it to other widgets
/// on the same screen, or text from elsewhere can push what follows it out of place.
///
/// `width(sanitize(s))` is `width(s)`, whatever `s` is, so that layouts hold. Control
/// characters and others that take no cell go. A grapheme that terminals may measure otherwise
/// than Ratatui keeps its first character when that can be trusted and fits, with spaces for
/// the cells left, and is `U+FFFD` in each cell otherwise. So joined emoji show as their
/// first emoji, a flag as `U+FFFD` twice, and plain text, CJK text and single emoji stay.
///
/// What holds was measured on tmux 3.4. Terminals with older tables than that may disagree on
/// emoji newer than theirs, and terminals set to give ambiguous characters two cells disagree
/// on `─` too.
pub fn sanitize(s: &str) -> Cow<'_, str> {
    if s.graphemes(true).all(|g| grapheme_width(g) > 0 && steady(g)) {
        return Cow::Borrowed(s);
    }
    let mut out = String::with_capacity(s.len());
    // Where the last grapheme written starts, and whether what came between it and the next,
    // or a part of it, is gone, so that the two may now join into one.
    let (mut tail, mut altered) = (0, false);
    for g in s.graphemes(true) {
        let cells = grapheme_width(g);
        if cells == 0 {
            altered = true;
            continue;
        }
        let start = out.len();
        if altered && joins(&out[tail..], g) {
            out.extend(std::iter::repeat_n(REPLACEMENT, cells));
            altered = true;
        } else if steady(g) {
            out.push_str(g);
            altered = false;
        } else {
            stand_in(g, cells, &mut out);
            altered = true;
        }
        tail = start;
    }
    Cow::Owned(out)
}

/// Whether terminals agree with Ratatui on the width of `g`. They count in one of two ways, by
/// grapheme or adding up its characters, so the two have to come to the same.
fn steady(g: &str) -> bool {
    let cells = grapheme_width(g);
    let mut sum: usize = 0;
    g.chars().all(|c| {
        sum = sum.saturating_add(char_width(c));
        !unsteady(c) && sum <= cells
    }) && sum == cells
}

fn char_width(c: char) -> usize {
    grapheme_width(c.encode_utf8(&mut [0; 4]))
}

/// Whether `next` would become part of the last grapheme of `text`.
fn joins(text: &str, next: &str) -> bool {
    let together = format!("{text}{next}");
    together.graphemes(true).count() <= text.graphemes(true).count()
}

/// Writes what takes the `cells` of `g` in its place.
fn stand_in(g: &str, cells: usize, out: &mut String) {
    let first = g.chars().next().filter(|&c| !unsteady(c));
    let (fill, left) = match first.map(|c| (c, char_width(c))) {
        Some((c, kept)) if (1..=cells).contains(&kept) => {
            out.push(c);
            (' ', cells.saturating_sub(kept))
        }
        _ => (REPLACEMENT, cells),
    };
    out.extend(std::iter::repeat_n(fill, left));
}

#[cfg(test)]
mod tests {
    use ratatui::buffer::Buffer;
    use ratatui::layout::Rect;
    use ratatui::style::Style;

    use super::*;

    #[test]
    fn widths_match_what_ratatui_draws() {
        // Wide characters, emoji, a combining accent, halfwidth katakana with voiced
        // sound marks, and control characters, which are skipped.
        for s in ["abc", "文件", "❤️ x", "e\u{301}", "ｶﾞｲﾄﾞ", "a\tb\u{1b}"] {
            let mut buf = Buffer::empty(Rect::new(0, 0, 20, 1));
            let (end, _) = buf.set_stringn(0, 0, s, usize::MAX, Style::new());
            assert_eq!(width(s), usize::from(end), "{s:?}");
        }
        assert_eq!(take_width("文件名", 5), "文件");
        // A grapheme that doesn't fit is left out whole.
        assert_eq!(take_width("❤️x", 1), "");
    }

    #[test]
    fn sanitize_leaves_plain_text_as_it_is() {
        for s in ["", "abc", "文件名", "e\u{301}", "\u{1f600}", "\u{1100}\u{1161}\u{11a8}"] {
            assert!(matches!(sanitize(s), Cow::Borrowed(_)), "{s:?}");
        }
    }

    #[test]
    fn sanitize_keeps_what_it_can_of_a_grapheme() {
        for (s, clean) in [
            // Tmux takes the heart with U+FE0F as one cell, Ratatui as two.
            ("\u{2764}\u{fe0f}", "\u{2764} "),
            // Two cells alone and one with U+FE0E: the watch doesn't fit in its place.
            ("\u{231a}\u{fe0e}", "\u{fffd}"),
            ("x\u{fe0f}", "x"),
            // Joined emoji, and emoji with a skin tone.
            ("\u{1f469}\u{200d}\u{1f4bb}", "\u{1f469}"),
            ("\u{1f44d}\u{1f3fb}", "\u{1f44d}"),
            // A flag, and a lone regional indicator.
            ("\u{1f1fa}\u{1f1f8}", "\u{fffd}\u{fffd}"),
            ("\u{1f1e6}", "\u{fffd}"),
            ("\u{fff9}", "\u{fffd}"),
            // What takes no cell goes, reordering controls too.
            ("a\u{1b}b\u{202e}c\u{200b}", "abc"),
        ] {
            assert_eq!(sanitize(s), clean, "{s:?}");
            assert_eq!(width(clean), width(s), "{s:?}");
        }
    }

    #[test]
    fn sanitize_holds_for_graphemes_of_any_length() {
        // Ratatui's width of each wraps at 65,536 cells.
        for s in ["\u{1100}".repeat(32_769), format!("a{}", "\u{ff9e}".repeat(65_536))] {
            let clean = sanitize(&s);
            assert_eq!(width(&clean), width(&s));
            assert!(clean.len() < 16, "{}", clean.len());
        }
    }

    #[test]
    fn sanitize_keeps_graphemes_from_joining_when_something_goes() {
        // Two Hangul leading consonants, kept apart by a control character alone.
        let s = "\u{1100}\u{7}\u{1100}";
        let clean = sanitize(s);
        assert_eq!(clean, "\u{1100}\u{fffd}\u{fffd}");
        assert_eq!(width(&clean), width(s));
        assert_eq!(sanitize(&clean), clean);
    }
}
