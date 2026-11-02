//! One-line text fields.

use ratatui::buffer::Buffer;
use ratatui::layout::{Position, Rect};
use ratatui::style::{Modifier, Style};

use crate::text::{grapheme_width, take_width, width};
use unicode_segmentation::UnicodeSegmentation;

/// A one-line text field with a cursor. Text too long for it scrolls so the cursor stays
/// visible; a highlighted `<` in the first cell or `>` in the last says text is cut off
/// on that side, as readline's horizontal-scroll-mode and nano do.
#[derive(Debug, Clone)]
pub struct Field<'a> {
    text: &'a str,
    cursor: usize,
    style: Style,
}

/// What a field shows in `w` cells: `shown` starts after the `<` cell when `left` is
/// set, and `cursor` is the cursor's column counted from the field's first cell.
///
/// `shown` is a piece of the text as it came, control characters included: ratatui leaves
/// those out of a [`Buffer`], but a program that prints `shown` itself has to do so too.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FieldView<'a> {
    pub left: bool,
    pub shown: &'a str,
    pub right: bool,
    pub cursor: usize,
}

impl<'a> Field<'a> {
    /// A field holding `text`, with the cursor before char `cursor`.
    pub fn new(text: &'a str, cursor: usize) -> Self {
        Self { text, cursor, style: Style::new() }
    }

    /// Style for the whole field (underlined, say); the markers add reverse video.
    pub fn style(mut self, style: Style) -> Self {
        self.style = style;
        self
    }

    /// Draws the field over `area`'s first row and returns where the cursor goes. Only
    /// the part of that row inside `buf` is drawn on; if none is, the cursor goes to
    /// `area`'s top left corner.
    pub fn render(self, area: Rect, buf: &mut Buffer) -> Position {
        let corner = area.as_position();
        let area = Rect { height: area.height.min(1), ..area }.intersection(buf.area);
        if area.is_empty() {
            return corner;
        }
        buf.set_style(area, self.style);
        let view = Self::view(self.text, self.cursor, usize::from(area.width));
        let marker = self.style.add_modifier(Modifier::REVERSED);
        let x = area.x.saturating_add(u16::from(view.left));
        if view.left {
            buf.set_string(area.x, area.y, "<", marker);
        }
        buf.set_stringn(x, area.y, view.shown, width(view.shown), self.style);
        if view.right {
            buf.set_string(area.right().saturating_sub(1), area.y, ">", marker);
        }
        cursor_cell(area, view.cursor)
    }

    /// The part of `text` a field `w` cells wide shows with the cursor (a char index)
    /// visible: from the start while the cursor fits before the last cell, else scrolled
    /// so it sits in the cell before the `>` (or the last cell, at the end of the text).
    /// In a field too narrow for a marker beside the cursor's cell, the marker goes.
    pub fn view(text: &str, cursor: usize, w: usize) -> FieldView<'_> {
        let total = width(text);
        // Capped: a char can take more cells alone than in the grapheme it ends up in, as
        // U+231A does followed by U+FE0E, and then the chars before the cursor would take
        // more cells than the whole text.
        let before = width(&text.chars().take(cursor).collect::<String>()).min(total);
        if w == 0 {
            return FieldView { left: false, shown: "", right: false, cursor: 0 };
        }
        // Everything fits, cursor cell included.
        if total < w {
            return FieldView { left: false, shown: text, right: false, cursor: before };
        }
        if before.saturating_add(2) <= w {
            let shown = take_width(text, w.saturating_sub(1));
            return FieldView { left: false, shown, right: true, cursor: before };
        }
        let Scroll { left, right_cell, skip, fits } = scroll(before, total, w);
        let mut rest = text;
        let mut skipped: usize = 0;
        for g in text.graphemes(true) {
            if skipped >= skip {
                break;
            }
            skipped = skipped.saturating_add(grapheme_width(g));
            rest = &rest[g.len()..];
        }
        let shown = take_width(rest, fits);
        let right = right_cell && skipped.saturating_add(width(shown)) < total;
        // Saturating for real: with the cursor inside a grapheme, `skipped` can pass `before`.
        let cursor = before.saturating_add(usize::from(left)).saturating_sub(skipped);
        FieldView { left, shown, right, cursor }
    }
}

/// Where the cursor goes for column `col` of a field drawn on `area`: on `area`, whatever
/// the column.
pub(crate) fn cursor_cell(area: Rect, col: usize) -> Position {
    let col = u16::try_from(col).unwrap_or(0);
    Position::new(area.x.saturating_add(col.min(area.width.saturating_sub(1))), area.y)
}

/// How a scrolled field divides its cells.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Scroll {
    /// Whether the first cell holds `<`.
    pub(crate) left: bool,
    /// Whether the last cell is kept from the text, for `>`.
    pub(crate) right_cell: bool,
    /// The cells of text to leave out on the left, at least.
    pub(crate) skip: usize,
    /// The cells left for text.
    pub(crate) fits: usize,
}

/// `<` in the first cell, and the cursor's cell before `>` or at the end, for a cursor
/// `before` cells into `total` cells of text.
pub(crate) fn scroll(before: usize, total: usize, w: usize) -> Scroll {
    let at_end = before == total;
    let right_cell = !at_end && w >= 2;
    // None of these saturate: `w` is at least 1, and at least 2 with `right_cell`.
    let col = w.saturating_sub(1).saturating_sub(usize::from(right_cell));
    let left = col > 0;
    let skip = before.saturating_sub(col.saturating_sub(usize::from(left)));
    let fits = w.saturating_sub(usize::from(left)).saturating_sub(usize::from(right_cell));
    Scroll { left, right_cell, skip, fits }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn line(buf: &Buffer) -> String {
        (0..buf.area.width).map(|x| buf[(x, 0)].symbol()).collect()
    }

    #[test]
    fn short_text_has_no_markers() {
        let v = Field::view("abc", 3, 10);
        assert_eq!(v, FieldView { left: false, shown: "abc", right: false, cursor: 3 });
    }

    #[test]
    fn markers_show_what_is_cut_off() {
        let text = "0123456789abcdef";
        // Cursor near the start: the end is cut off.
        let v = Field::view(text, 2, 8);
        assert_eq!((v.left, v.shown, v.right, v.cursor), (false, "0123456", true, 2));
        // At the end: the start is cut off; the cursor takes the last cell.
        let v = Field::view(text, 16, 8);
        assert_eq!((v.left, v.shown, v.right, v.cursor), (true, "abcdef", false, 7));
        // In the middle: both.
        let v = Field::view(text, 10, 8);
        assert_eq!((v.left, v.shown, v.right, v.cursor), (true, "56789a", true, 6));
    }

    #[test]
    fn render_draws_highlighted_markers() {
        let mut buf = Buffer::empty(Rect::new(0, 0, 8, 1));
        let at = Field::new("0123456789abcdef", 10).render(buf.area, &mut buf);
        assert_eq!(line(&buf), "<56789a>");
        assert_eq!(at, Position::new(6, 0));
        assert!(buf[(0, 0)].modifier.contains(Modifier::REVERSED));
        assert!(buf[(7, 0)].modifier.contains(Modifier::REVERSED));
    }

    #[test]
    fn narrow_fields_leave_out_markers_without_a_cell() {
        let text = "0123456789";
        // At the end: `<` and the cursor's cell.
        let v = Field::view(text, 10, 2);
        assert_eq!((v.left, v.shown, v.right, v.cursor), (true, "", false, 1));
        // In the middle: the cursor's cell and `>`.
        let v = Field::view(text, 5, 2);
        assert_eq!((v.left, v.shown, v.right, v.cursor), (false, "5", true, 0));
        // One cell: the cursor's.
        let v = Field::view(text, 5, 1);
        assert_eq!((v.left, v.shown, v.right, v.cursor), (false, "5", false, 0));
        let v = Field::view(text, 5, 0);
        assert_eq!(v, FieldView { left: false, shown: "", right: false, cursor: 0 });
    }

    #[test]
    fn the_cursor_stays_in_the_field() {
        for text in ["0123456789", "文件名很长的中文"] {
            for w in 0..8 {
                for cursor in 0..=text.chars().count() {
                    let v = Field::view(text, cursor, w);
                    let what = format!("{text:?}, cursor {cursor}, {w} cells: {v:?}");
                    let (left, shown, right) = (usize::from(v.left), width(v.shown), v.right);
                    assert!(left + shown + usize::from(right) <= w, "{what}");
                    if w == 0 {
                        continue;
                    }
                    // Off the markers, on the text shown or just after it.
                    assert!(v.cursor < w - usize::from(right), "{what}");
                    assert!(left <= v.cursor && v.cursor <= left + shown, "{what}");
                    // A marker means text is cut off on its side, and from 3 cells on,
                    // text cut off on a side gets one.
                    let start = v.shown.as_ptr().addr() - text.as_ptr().addr();
                    let cut = (start > 0, start + v.shown.len() < text.len());
                    assert!((!v.left || cut.0) && (!right || cut.1), "{what}");
                    if w >= 3 {
                        assert_eq!((v.left, right), cut, "{what}");
                    }
                }
            }
        }
    }

    #[test]
    fn a_cursor_inside_a_grapheme_stays_in_the_field() {
        // Two cells alone, one with U+FE0E: the char before the cursor is wider than the text.
        let text = "\u{231a}\u{fe0e}";
        assert_eq!((width("\u{231a}"), width(text)), (2, 1));
        for w in 2..4 {
            assert_eq!(Field::view(text, 1, w).cursor, 1, "{w} cells");
        }
        assert_eq!(Field::view("a\u{231a}\u{fe0e}", 2, 3).cursor, 2);
    }

    #[test]
    fn a_grapheme_too_wide_to_show_leaves_the_cursor_in_the_field() {
        // A run of Hangul leading consonants is one grapheme, two cells for each.
        let text = format!("ab{}", "\u{1100}".repeat(8));
        let v = Field::view(&text, 10, 6);
        assert_eq!((v.left, v.shown, v.right, v.cursor), (true, "", false, 1));
        // With the cursor inside it, the cursor is past what is shown, but in the field.
        let v = Field::view(&text, 3, 6);
        assert_eq!((v.left, v.shown, v.right), (false, "ab", true));
        assert!(v.cursor < 5, "{v:?}");
    }

    #[test]
    fn nothing_is_drawn_outside_the_area_or_the_buffer() {
        let blank = Buffer::empty(Rect::new(0, 0, 4, 2));
        for area in [
            Rect::new(0, 0, 0, 1),
            Rect::new(2, 0, 0, 1),
            Rect::new(0, 1, 4, 0),
            Rect::new(0, 2, 4, 1),
            Rect::new(6, 0, 4, 1),
        ] {
            let mut buf = blank.clone();
            let at = Field::new("0123456789", 10).render(area, &mut buf);
            assert_eq!((&buf, at), (&blank, area.as_position()), "{area:?}");
        }
    }
}
