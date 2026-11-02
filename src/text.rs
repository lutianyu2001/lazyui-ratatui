//! Cell widths, measured the way ratatui draws text (per grapheme).

use ratatui::buffer::CellWidth;
use unicode_segmentation::UnicodeSegmentation;

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
}
