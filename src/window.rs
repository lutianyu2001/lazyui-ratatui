//! Windows and the arrows between them.

use ratatui::buffer::Buffer;
use ratatui::layout::{Position, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::widgets::{Block, BorderType, Widget};

use crate::text::width;

/// A window with rounded corners, named in its top right corner (`╭──[1] Files─╮`) and
/// with an optional note in its bottom right one (`╰──[2 marked]─╯`). Both are drawn in
/// the border's style, bold, when the window is wide enough for them.
#[derive(Debug, Clone, Default)]
pub struct Window<'a> {
    key: &'a str,
    name: &'a str,
    note: &'a str,
    border: Style,
}

impl<'a> Window<'a> {
    pub fn new() -> Self {
        Self::default()
    }

    /// The name in the top right corner, after the key that focuses the window
    /// (`[1] Files`); an empty key leaves just the name.
    pub fn name(mut self, key: &'a str, name: &'a str) -> Self {
        self.key = key;
        self.name = name;
        self
    }

    /// Text for the bottom right corner.
    pub fn note(mut self, note: &'a str) -> Self {
        self.note = note;
        self
    }

    /// Style of the border and the texts on it; a focused window usually gets a color.
    pub fn border_style(mut self, style: Style) -> Self {
        self.border = style;
        self
    }

    /// The area inside the border.
    pub fn inner(&self, area: Rect) -> Rect {
        Block::bordered().inner(area)
    }

    fn label(&self) -> String {
        match (self.key, self.name) {
            (_, "") => String::new(),
            ("", name) => name.to_owned(),
            (key, name) => format!("[{key}] {name}"),
        }
    }
}

impl Widget for Window<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        // Only the part inside the buffer, which is where `Block` draws the border.
        let area = area.intersection(buf.area);
        if area.is_empty() {
            return;
        }
        Block::bordered()
            .border_type(BorderType::Rounded)
            .border_style(self.border)
            .render(area, buf);
        let style = self.border.add_modifier(Modifier::BOLD);
        corner(buf, area, area.y, &self.label(), style);
        if area.height >= 2 {
            corner(buf, area, area.bottom().saturating_sub(1), self.note, style);
        }
    }
}

/// `text` at the right end of border row `y`, one `─` away from the corner.
fn corner(buf: &mut Buffer, area: Rect, y: u16, text: &str, style: Style) {
    if let Some((x, w)) = corner_place(area, width(text)) {
        buf.set_stringn(x, y, text, usize::from(w), style);
    }
}

/// Where a text `width` cells wide starts on a border of `area`, and its width, if there is
/// a text and it leaves the corners and a `─` beside each.
pub(crate) fn corner_place(area: Rect, width: usize) -> Option<(u16, u16)> {
    let w = u16::try_from(width).ok().filter(|&w| w > 0 && w.saturating_add(4) <= area.width)?;
    Some((area.right().saturating_sub(w.saturating_add(2)), w))
}

/// Joins `upper` to the window `lower` under it with an arrow at column `x`: `┬` on the
/// upper window's bottom border, `│` across any rows between, `▼` on the lower one's
/// top border. Does nothing if `x` isn't on both borders, `lower` isn't below, or the
/// arrow would leave the buffer.
pub fn arrow_down(buf: &mut Buffer, x: u16, upper: Rect, lower: Rect, style: Style) {
    let Some((border, to)) = arrow_ends(x, upper, lower, buf.area) else { return };
    buf.set_string(x, border, "┬", style);
    for y in border.saturating_add(1)..to {
        buf.set_string(x, y, "│", style);
    }
    buf.set_string(x, to, "▼", style);
}

/// The rows of an arrow's two ends at column `x`, if it can be drawn on `screen`.
pub(crate) fn arrow_ends(x: u16, upper: Rect, lower: Rect, screen: Rect) -> Option<(u16, u16)> {
    let inside = |r: Rect| x > r.x && x.saturating_add(1) < r.right();
    let (from, to) = (upper.bottom(), lower.y);
    // The upper window's bottom border, once its height is known not to be 0.
    let border = from.saturating_sub(1);
    let on_screen = |y| screen.contains(Position::new(x, y));
    let drawn = upper.height != 0
        && lower.height != 0
        && from <= to
        && inside(upper)
        && inside(lower)
        && on_screen(border)
        && on_screen(to);
    drawn.then_some((border, to))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rows(buf: &Buffer) -> Vec<String> {
        (0..buf.area.height)
            .map(|y| (0..buf.area.width).map(|x| buf[(x, y)].symbol()).collect())
            .collect()
    }

    #[test]
    fn names_and_notes_sit_in_the_corners() {
        let mut buf = Buffer::empty(Rect::new(0, 0, 20, 3));
        Window::new().name("1", "Files").note("[2 marked]").render(buf.area, &mut buf);
        assert_eq!(
            rows(&buf),
            ["╭────────[1] Files─╮", "│                  │", "╰───────[2 marked]─╯"]
        );
        assert_eq!(Window::new().inner(buf.area), Rect::new(1, 1, 18, 1));
    }

    #[test]
    fn a_name_too_wide_is_left_out() {
        let mut buf = Buffer::empty(Rect::new(0, 0, 10, 2));
        Window::new().name("1", "Transfers").render(buf.area, &mut buf);
        assert_eq!(rows(&buf), ["╭────────╮", "╰────────╯"]);
    }

    #[test]
    fn arrows_join_stacked_windows() {
        let mut buf = Buffer::empty(Rect::new(0, 0, 12, 6));
        let (upper, lower) = (Rect::new(4, 0, 8, 3), Rect::new(0, 3, 12, 3));
        Window::new().render(upper, &mut buf);
        Window::new().render(lower, &mut buf);
        arrow_down(&mut buf, 6, upper, lower, Style::new());
        assert_eq!(&rows(&buf)[2..4], ["    ╰─┬────╯", "╭─────▼────╮"]);
    }

    /// One test for each condition of the arrow: the same arrow is drawn, and with that
    /// condition alone changed it is not.
    #[test]
    fn each_condition_of_an_arrow_counts_by_itself() {
        let screen = Rect::new(0, 0, 12, 6);
        let (upper, lower) = (Rect::new(0, 0, 12, 3), Rect::new(0, 3, 12, 3));
        assert_eq!(arrow_ends(6, upper, lower, screen), Some((2, 3)));
        for (x, upper, lower, screen, why) in [
            (6, Rect { height: 0, ..upper }, lower, screen, "the upper window has no height"),
            (6, upper, Rect { height: 0, ..lower }, screen, "the lower window has no height"),
            (6, Rect { y: 1, ..upper }, lower, screen, "the windows overlap"),
            (6, Rect { width: 7, ..upper }, lower, screen, "on the upper window's right corner"),
            (6, Rect { x: 6, width: 6, ..upper }, lower, screen, "on its left corner"),
            (6, upper, Rect { width: 7, ..lower }, screen, "on the lower window's right corner"),
            (6, upper, Rect { x: 6, width: 6, ..lower }, screen, "on its left corner"),
            (
                6,
                upper,
                lower,
                Rect { y: 3, height: 3, ..screen },
                "the upper end is off the screen",
            ),
            (6, upper, lower, Rect { height: 3, ..screen }, "the lower end is off the screen"),
            (6, upper, lower, Rect { width: 6, ..screen }, "the column is off the screen"),
        ] {
            assert_eq!(arrow_ends(x, upper, lower, screen), None, "{why}");
        }
        // Rows between the windows are the arrow's too.
        let lower = Rect::new(0, 5, 12, 1);
        assert_eq!(arrow_ends(6, upper, lower, screen), Some((2, 5)));
        let mut buf = Buffer::empty(screen);
        arrow_down(&mut buf, 6, upper, lower, Style::new());
        let column: Vec<&str> = (0..6).map(|y| buf[(6, y)].symbol()).collect();
        assert_eq!(column, [" ", " ", "┬", "│", "│", "▼"]);
    }

    #[test]
    fn a_text_on_the_border_needs_a_cell_beside_each_corner() {
        let area = Rect::new(3, 0, 10, 2);
        assert_eq!(corner_place(area, 6), Some((5, 6)));
        assert_eq!(corner_place(area, 7), None);
        assert_eq!(corner_place(area, 0), None);
        assert_eq!(corner_place(area, 1 << 16), None);
    }

    #[test]
    fn a_window_one_row_high_has_no_note() {
        let mut buf = Buffer::empty(Rect::new(0, 0, 12, 1));
        Window::new().name("1", "a").note("b").render(buf.area, &mut buf);
        assert_eq!(rows(&buf), ["╭────[1] a─╮"]);
    }

    #[test]
    fn nothing_is_drawn_outside_the_buffer() {
        let blank = Buffer::empty(Rect::new(0, 0, 20, 3));
        let mut buf = blank.clone();
        let files = Window::new().name("1", "Files").note("[2 marked]");
        files.clone().render(Rect::new(0, 3, 20, 3), &mut buf);
        arrow_down(&mut buf, 6, Rect::new(0, 0, 12, 3), Rect::new(0, 4, 12, 3), Style::new());
        assert_eq!(buf, blank);
        // A window cut off by the buffer's edge is drawn on the part inside it, as a block.
        files.render(Rect::new(0, 1, 20, 3), &mut buf);
        assert_eq!(rows(&buf)[1..], ["╭────────[1] Files─╮", "╰───────[2 marked]─╯"]);
    }
}
