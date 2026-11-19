//! Values changed with the arrows beside them.

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::widgets::Widget;

use crate::dropdown::saturate;
use crate::text::{sanitize, take_width, width};

const BACK: &str = "◂";
const FORWARD: &str = "▸";
/// Cells of the two arrows and the space between each and the value.
const ARROWS: u16 = 4;
/// Cells between the right arrow and the note.
const GAP: u16 = 2;

/// One of a few values, between the arrows that change it to the one before and the next
/// (`◂ English ▸  en`), with an optional dim note after them. Given a width for the value,
/// the value goes in its middle: controls under one another, given the width of the longest
/// value any of them takes, have their arrows in two columns, and none moves when a value
/// changes.
///
/// It takes one row. A value longer than its width or the area is cut; the note is drawn
/// whole or not at all. [`Choice::arrows`] says where the arrows are, for clicks. A
/// [`Dropdown`](crate::Dropdown) with a head, given the same area as its control, puts its
/// own arrow where the left one is.
#[derive(Debug, Clone)]
pub struct Choice<'a> {
    name: &'a str,
    note: &'a str,
    cells: Option<usize>,
    style: Style,
    note_style: Style,
}

impl<'a> Choice<'a> {
    /// The value `name`, its arrows close beside it.
    pub fn new(name: &'a str) -> Self {
        Self {
            name,
            note: "",
            cells: None,
            style: Style::new(),
            note_style: Style::new().add_modifier(Modifier::DIM),
        }
    }

    /// Text after the right arrow, such as the value as a file writes it.
    pub fn note(mut self, note: &'a str) -> Self {
        self.note = note;
        self
    }

    /// The cells between the arrows, the value in their middle; the value's own unless set.
    pub fn width(mut self, cells: usize) -> Self {
        self.cells = Some(cells);
        self
    }

    /// Style of the arrows, the value and the cells between them.
    pub fn style(mut self, style: Style) -> Self {
        self.style = style;
        self
    }

    /// Style of the note and the cells before it; dim unless set.
    pub fn note_style(mut self, style: Style) -> Self {
        self.note_style = style;
        self
    }

    /// Where a control on `area` has its arrows, the one back and the one forward, each
    /// with the space between it and the value, for clicks; none in an area without room
    /// for them. `area` is the part of the control on the screen.
    pub fn arrows(&self, area: Rect) -> Option<(Rect, Rect)> {
        let between = between(area.width, self.cells()).filter(|_| area.height > 0)?;
        let arrow = Rect { width: 2, height: 1, ..area };
        Some((arrow, Rect { x: forward(area.x, between), ..arrow }))
    }

    fn cells(&self) -> usize {
        self.cells.unwrap_or_else(|| width(self.name))
    }
}

impl Widget for Choice<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        // Its row is the area's first, on the screen or not.
        let area = Rect { height: area.height.min(1), ..area }.intersection(buf.area);
        let Some(between) = between(area.width, self.cells()).filter(|_| area.height > 0) else {
            return;
        };
        let (x, y) = (area.x, area.y);
        let all = usize::from(between.saturating_add(ARROWS));
        buf.set_stringn(x, y, " ".repeat(all), all, self.style);
        buf.set_stringn(x, y, BACK, 1, self.style);
        let name = take_width(self.name, usize::from(between));
        let before = between.saturating_sub(saturate(width(name))) / 2;
        let at = x.saturating_add(2).saturating_add(before);
        buf.set_stringn(at, y, sanitize(name), width(name), self.style);
        let after = forward(x, between);
        buf.set_stringn(after.saturating_add(1), y, FORWARD, 1, self.style);
        // Whole or not at all: half a note reads as another one.
        let end = after.saturating_add(2);
        let room = usize::from(area.right().saturating_sub(end).saturating_sub(GAP));
        let cells = width(self.note);
        if cells > 0 && cells <= room {
            let gap = usize::from(GAP);
            buf.set_stringn(end, y, " ".repeat(gap), gap, self.note_style);
            let at = end.saturating_add(GAP);
            buf.set_stringn(at, y, sanitize(self.note), cells, self.note_style);
        }
    }
}

/// The cells between the arrows of a control `room` cells wide that wants `wanted`, if the
/// arrows fit.
pub(crate) fn between(room: u16, wanted: usize) -> Option<u16> {
    room.checked_sub(ARROWS).map(|room| room.min(saturate(wanted)))
}

/// The column of the space before the right arrow of a control at column `x` with
/// `between` cells between its arrows.
pub(crate) fn forward(x: u16, between: u16) -> u16 {
    x.saturating_add(2).saturating_add(between)
}

#[cfg(test)]
mod tests {
    use ratatui::layout::Position;

    use super::*;

    fn row(buf: &Buffer) -> String {
        let mut skip = false;
        (buf.area.left()..buf.area.right())
            .filter_map(|x| {
                // The cell after a wide character holds a space of its own.
                let kept = !std::mem::take(&mut skip);
                let symbol = buf[(x, buf.area.y)].symbol();
                skip = kept && width(symbol) > 1;
                kept.then_some(symbol)
            })
            .collect()
    }

    fn drawn(choice: Choice, w: u16) -> String {
        let mut buf = Buffer::empty(Rect::new(0, 0, w, 1));
        buf.set_string(0, 0, "x".repeat(usize::from(w)), Style::new());
        choice.render(buf.area, &mut buf);
        row(&buf)
    }

    #[test]
    fn the_arrows_are_close_beside_a_value_without_a_width() {
        assert_eq!(drawn(Choice::new("On").note("true"), 16), "◂ On ▸  truexxxx");
        assert_eq!(drawn(Choice::new("简体中文"), 14), "◂ 简体中文 ▸xx");
    }

    #[test]
    fn a_value_goes_in_the_middle_of_its_width() {
        assert_eq!(drawn(Choice::new("On").width(8).note("true"), 20), "◂    On    ▸  truexx");
        assert_eq!(drawn(Choice::new("Off").width(8).note("false"), 20), "◂   Off    ▸  falsex");
        assert_eq!(drawn(Choice::new("简体中文").width(8), 12), "◂ 简体中文 ▸");
    }

    #[test]
    fn a_value_is_cut_and_a_note_left_out_where_they_do_not_fit() {
        assert_eq!(drawn(Choice::new("English").note("en"), 9), "◂ Engli ▸");
        assert_eq!(drawn(Choice::new("English").width(3).note("en"), 11), "◂ Eng ▸  en");
        // One cell short of the whole note.
        assert_eq!(drawn(Choice::new("On").note("true"), 11), "◂ On ▸xxxxx");
        // A wide character that would be cut in half is left out.
        assert_eq!(drawn(Choice::new("中文"), 7), "◂ 中  ▸");
        // Without room for the arrows, nothing is drawn.
        assert_eq!(drawn(Choice::new("On"), 3), "xxx");
    }

    #[test]
    fn the_arrows_take_clicks_with_the_space_beside_them() {
        let choice = Choice::new("On").width(6);
        let area = Rect::new(3, 2, 20, 1);
        let (back, forward) = choice.arrows(area).expect("room for the arrows");
        assert_eq!((back, forward), (Rect::new(3, 2, 2, 1), Rect::new(11, 2, 2, 1)));
        let mut buf = Buffer::empty(Rect::new(0, 0, 30, 4));
        choice.clone().render(area, &mut buf);
        assert_eq!((buf[(3, 2)].symbol(), buf[(12, 2)].symbol()), ("◂", "▸"));
        assert!(!back.contains(Position::new(5, 2)) && !forward.contains(Position::new(10, 2)));
        assert_eq!(choice.arrows(Rect::new(3, 2, 3, 1)), None);
        assert_eq!(choice.arrows(Rect::new(3, 2, 20, 0)), None);
    }

    #[test]
    fn styles_are_those_given() {
        let bold = Style::new().add_modifier(Modifier::BOLD);
        let mut buf = Buffer::empty(Rect::new(0, 0, 16, 1));
        Choice::new("On").width(4).note("true").style(bold).render(buf.area, &mut buf);
        for x in 0..8 {
            assert!(buf[(x, 0)].modifier.contains(Modifier::BOLD), "{x}");
        }
        assert_eq!(buf[(10, 0)].modifier, Modifier::DIM);
    }

    #[test]
    fn text_from_elsewhere_is_made_safe() {
        assert_eq!(drawn(Choice::new("a\u{1b}b").note("c\nd"), 14), "◂ ab ▸  cdxxxx");
    }
}
