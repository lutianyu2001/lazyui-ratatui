//! Drop-down lists.

use ratatui::buffer::Buffer;
use ratatui::layout::{Position, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::widgets::{Clear, Widget};

use crate::Window;
use crate::text::{sanitize, take_width, width};
use crate::window::Side;

/// The mark before the item in effect, and before the others.
const CHOSEN: &str = "(*)";
const OTHER: &str = "( )";
/// Cells of a row beside its item: the borders, a space inside each, the mark and the space
/// after it.
const BESIDE: usize = 8;
/// Cells between an item's name and its note.
const GAP: usize = 2;
/// Cells from a list's left edge to the arrow of its head.
const ARROW: u16 = 3;

/// A list of choices that opens at a control, as lazygit's menus look: a rounded window, one
/// choice a row, `(*)` before the one in effect and `( )` before the others, the row under the
/// cursor highlighted, and each choice's note dim after its name. With more choices than
/// rows, it scrolls to keep the cursor visible and says where the cursor is in the bottom
/// right corner (`4/9`).
///
/// It is drawn over what the buffer holds: the cells of its area are cleared first, and a
/// wide character that reaches into the area from the cell before it is blanked there.
///
/// With a [`Dropdown::head`], the list is joined to its control as a combo box is: the
/// border nearest the control lies on the control's row and shows the value there.
///
/// The program holding the list keeps the cursor and moves it; [`Dropdown::area`] says where
/// the list goes for a control, and [`Dropdown::item_at`] which choice a click is on.
#[derive(Debug, Clone)]
pub struct Dropdown<'a> {
    items: &'a [(&'a str, &'a str)],
    chosen: Option<usize>,
    cursor: usize,
    head: Option<(&'a str, &'a str, u16)>,
    border: Style,
    highlight: Style,
}

impl<'a> Dropdown<'a> {
    /// A list of `items`, each a name and a note (which may be empty), with the cursor on
    /// the first.
    pub fn new(items: &'a [(&'a str, &'a str)]) -> Self {
        Self {
            items,
            chosen: None,
            cursor: 0,
            head: None,
            border: Style::new(),
            highlight: Style::new().add_modifier(Modifier::REVERSED),
        }
    }

    /// The item in effect, marked `(*)`.
    pub fn chosen(mut self, item: usize) -> Self {
        self.chosen = Some(item);
        self
    }

    /// The item under the cursor; past the last one, the last.
    pub fn cursor(mut self, item: usize) -> Self {
        self.cursor = item;
        self
    }

    /// Joins the list to its control on screen row `row`: the list starts on that row, where
    /// its top border, between the corners, shows `name` and `note` over those of the
    /// choices, after a `▾`. A list that opens above its control ends on that row instead,
    /// with them in its bottom border after a `▴`, and says where its cursor is at the top.
    pub fn head(mut self, name: &'a str, note: &'a str, row: u16) -> Self {
        self.head = Some((name, note, row));
        self
    }

    /// Style of the border and the text on it.
    pub fn border_style(mut self, style: Style) -> Self {
        self.border = style;
        self
    }

    /// Style of the row under the cursor, over the list's own; reverse video unless set.
    pub fn highlight_style(mut self, style: Style) -> Self {
        self.highlight = style;
        self
    }

    /// Where the list goes for a control at `anchor` on `screen`: under the control, its left
    /// edge at the control's, as wide and as high as its items need. Where the screen ends
    /// before that, it moves left, and goes above the control when there are more rows there
    /// than under it; what still doesn't fit scrolls or is cut off. It never covers a row of
    /// the control.
    ///
    /// A list with a [`Dropdown::head`] starts on the head's row instead, or ends on it
    /// where it opens above, and has the head's arrow in the control's first column, where a
    /// [`Choice`](crate::Choice) has its left arrow. It covers no other row of the
    /// control's, and is nowhere when the head's row is off the screen.
    pub fn area(&self, anchor: Rect, screen: Rect) -> Rect {
        let (name, note) = self.columns();
        let cells = saturate(BESIDE.saturating_add(name).saturating_add(note));
        let rows = saturate(self.items.len().saturating_add(2));
        match self.head {
            Some((_, _, row)) => {
                place_joined(anchor.x.saturating_sub(ARROW), row, screen, cells, rows)
            }
            None => place(anchor, screen, cells, rows),
        }
    }

    /// The item drawn at `at` by a list on `area`, for a click there; none on the border.
    /// `area` is the part of the list on the screen, as [`Dropdown::area`] gives it.
    pub fn item_at(&self, area: Rect, at: Position) -> Option<usize> {
        let inner = Window::new().inner(area);
        if !inner.contains(at) {
            return None;
        }
        let row = usize::from(at.y.saturating_sub(inner.y));
        let item = self.offset(inner.height).saturating_add(row);
        (item < self.items.len()).then_some(item)
    }

    /// The columns the names and the notes of a list on `area` start in, for a control that
    /// lines its own text up with them. Without a note, the second is where one would start;
    /// in an area narrower than its items, they may lie past its end.
    pub fn text_columns(&self, area: Rect) -> (u16, u16) {
        let (name, _) = self.columns();
        let names = area.x.saturating_add(saturate(BESIDE.saturating_sub(2)));
        (names, names.saturating_add(saturate(name.saturating_add(GAP))))
    }

    /// The first item shown in `rows` rows.
    fn offset(&self, rows: u16) -> usize {
        offset(self.cursor, self.items.len(), usize::from(rows))
    }

    /// Cells of the widest name, and of the widest note with the gap before it, the head's
    /// among them.
    fn columns(&self) -> (usize, usize) {
        let head = self.head.map(|(name, note, _)| (name, note));
        let name = self.items.iter().chain(&head).map(|(name, _)| width(name)).max();
        let note = self.items.iter().chain(&head).map(|(_, note)| width(note)).max();
        let note = note.unwrap_or(0);
        (name.unwrap_or(0), if note == 0 { 0 } else { note.saturating_add(GAP) })
    }

    /// Draws the head on border row `y` of a list on `area`, between the corners.
    fn draw_head(&self, area: Rect, y: u16, arrow: &str, buf: &mut Buffer) {
        let Some((name, note, _)) = self.head else { return };
        let row =
            Rect { x: area.x.saturating_add(1), y, width: area.width.saturating_sub(2), height: 1 };
        Clear.render(row, buf);
        let (name_cells, _) = self.columns();
        let bold = Style::new().add_modifier(Modifier::BOLD);
        let mut line = Line { buf, row, x: row.x.saturating_add(2) };
        line.put(arrow, 3, bold);
        line.put(name, name_cells.saturating_add(GAP), bold);
        line.put(note, width(note), Style::new().add_modifier(Modifier::DIM));
    }
}

impl Widget for Dropdown<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let area = area.intersection(buf.area);
        if area.is_empty() {
            return;
        }
        uncover(area, buf);
        Clear.render(area, buf);
        let window = Window::new().border_style(self.border);
        let inner = window.inner(area);
        let cursor = self.cursor.min(self.items.len().saturating_sub(1));
        let note = if self.items.len() > usize::from(inner.height) {
            format!("{}/{}", cursor.saturating_add(1), self.items.len())
        } else {
            String::new()
        };
        // The head takes the border on its row; above its control, the bottom one.
        let last = area.bottom().saturating_sub(1);
        let head = self.head.map(|(_, _, row)| row).filter(|_| area.width >= 2);
        let upward = area.height >= 2 && head == Some(last);
        if upward {
            window.name("", &note).name_side(Side::Right).render(area, buf);
            self.draw_head(area, last, "▴", buf);
        } else {
            window.note(&note).render(area, buf);
            if head == Some(area.y) {
                self.draw_head(area, area.y, "▾", buf);
            }
        }
        let (name_cells, _) = self.columns();
        let first = self.offset(inner.height);
        let shown = self.items.iter().enumerate().skip(first);
        for ((item, (name, note)), y) in shown.zip(inner.y..inner.bottom()) {
            let row = Rect { y, height: 1, ..inner };
            let style = if item == cursor { self.highlight } else { Style::new() };
            buf.set_style(row, style);
            let mark = if self.chosen == Some(item) { CHOSEN } else { OTHER };
            let mut line = Line { buf, row, x: row.x.saturating_add(1) };
            line.put(mark, width(mark).saturating_add(1), style);
            line.put(name, name_cells.saturating_add(GAP), style);
            line.put(note, width(note), style.add_modifier(Modifier::DIM));
        }
    }
}

/// Blanks the wide characters that reach into `area` from the cell before it: a terminal
/// draws such a character over the list's border, and Ratatui leaves the cell after it
/// undrawn. These cells, in the column before the list, are the only ones a list changes
/// outside its area.
pub(crate) fn uncover(area: Rect, buf: &mut Buffer) {
    let Some(x) = area.x.checked_sub(1).filter(|&x| x >= buf.area.x) else { return };
    for y in area.y..area.bottom() {
        if let Some(cell) = buf.cell_mut((x, y)).filter(|cell| width(cell.symbol()) > 1) {
            cell.set_symbol(" ");
        }
    }
}

/// A row being written from left to right, a cell kept free before its right end.
struct Line<'a> {
    buf: &'a mut Buffer,
    row: Rect,
    x: u16,
}

impl Line<'_> {
    /// Writes `text`, cut where the row ends, and moves on by `cells`.
    fn put(&mut self, text: &str, cells: usize, style: Style) {
        let room = usize::from(self.row.right().saturating_sub(1).saturating_sub(self.x));
        let text = take_width(text, room);
        self.buf.set_stringn(self.x, self.row.y, sanitize(text), width(text), style);
        self.x = self.x.saturating_add(saturate(cells)).min(self.row.right());
    }
}

pub(crate) fn saturate(n: usize) -> u16 {
    u16::try_from(n).unwrap_or(u16::MAX)
}

/// The first of `len` items shown in `rows` rows with the one at `cursor` among them: the
/// first item, until the cursor passes the last row.
pub(crate) fn offset(cursor: usize, len: usize, rows: usize) -> usize {
    let cursor = cursor.min(len.saturating_sub(1));
    cursor.saturating_add(1).saturating_sub(rows).min(len.saturating_sub(rows))
}

/// Where a list `w` cells wide and `h` rows high goes on `screen` for a control at `anchor`
/// (see [`Dropdown::area`]).
pub(crate) fn place(anchor: Rect, screen: Rect, w: u16, h: u16) -> Rect {
    // A screen written out to reach past `u16::MAX` ends there.
    let screen = Rect::new(screen.x, screen.y, screen.width, screen.height);
    // The rows under the control and above it, as far as the screen has them.
    let under = anchor.bottom().clamp(screen.y, screen.bottom());
    let over = anchor.y.clamp(screen.y, screen.bottom());
    let (below, above) = (screen.bottom().saturating_sub(under), over.saturating_sub(screen.y));
    let (y, height) = if h <= below || below >= above {
        (under, h.min(below))
    } else {
        let height = h.min(above);
        (over.saturating_sub(height), height)
    };
    let width = w.min(screen.width);
    let x = anchor.x.min(screen.right().saturating_sub(width)).max(screen.x);
    Rect { x, y, width, height }
}

/// Where a list `w` cells wide and `h` rows high, joined to a control on row `row` whose
/// left edge is at `x`, goes on `screen`: from that row down, or, with too few rows there
/// and more above, up to that row (see [`Dropdown::area`]).
pub(crate) fn place_joined(x: u16, row: u16, screen: Rect, w: u16, h: u16) -> Rect {
    let screen = Rect::new(screen.x, screen.y, screen.width, screen.height);
    let width = w.min(screen.width);
    let x = x.min(screen.right().saturating_sub(width)).max(screen.x);
    if row < screen.y || row >= screen.bottom() {
        return Rect { x, y: screen.y, width, height: 0 };
    }
    // The rows from the control's down, and those up to it.
    let below = screen.bottom().saturating_sub(row);
    let above = row.saturating_sub(screen.y).saturating_add(1);
    if h <= below || below >= above {
        Rect { x, y: row, width, height: h.min(below) }
    } else {
        let height = h.min(above);
        Rect { x, y: row.saturating_add(1).saturating_sub(height), width, height }
    }
}

#[cfg(test)]
mod tests {
    use ratatui::style::Color;

    use super::*;

    const LANGUAGES: &[(&str, &str)] =
        &[("System", "auto"), ("English", "en"), ("简体中文", "zh-CN")];

    fn rows(buf: &Buffer) -> Vec<String> {
        (buf.area.top()..buf.area.bottom())
            .map(|y| {
                // The cell after a wide character holds a space of its own.
                let mut skip = 0;
                (buf.area.left()..buf.area.right())
                    .map(|x| buf[(x, y)].symbol())
                    .filter(|symbol| {
                        let kept = skip == 0;
                        skip = if kept {
                            width(symbol).saturating_sub(1)
                        } else {
                            skip.saturating_sub(1)
                        };
                        kept
                    })
                    .collect()
            })
            .collect()
    }

    #[test]
    fn items_are_marked_and_their_notes_line_up() {
        let list = Dropdown::new(LANGUAGES).chosen(1).cursor(2);
        let screen = Rect::new(0, 0, 30, 8);
        let area = list.area(Rect::new(2, 1, 10, 1), screen);
        assert_eq!(area, Rect::new(2, 2, 23, 5));
        let mut buf = Buffer::empty(screen);
        list.render(area, &mut buf);
        assert_eq!(
            &rows(&buf)[2..7],
            [
                "  ╭─────────────────────╮     ",
                "  │ ( ) System    auto  │     ",
                "  │ (*) English   en    │     ",
                "  │ ( ) 简体中文  zh-CN │     ",
                "  ╰─────────────────────╯     ",
            ]
        );
    }

    #[test]
    fn a_list_with_a_head_is_joined_to_its_control() {
        let list = Dropdown::new(LANGUAGES).chosen(1).cursor(1).head("English", "en", 1);
        let screen = Rect::new(0, 0, 30, 8);
        let area = list.area(Rect::new(5, 1, 10, 1), screen);
        assert_eq!(area, Rect::new(2, 1, 23, 5));
        let mut buf = Buffer::empty(screen);
        buf.set_string(0, 1, "x".repeat(30), Style::new());
        list.clone().render(area, &mut buf);
        assert_eq!(
            &rows(&buf)[1..6],
            [
                "xx╭  ▾  English   en    ╮xxxxx",
                "  │ ( ) System    auto  │     ",
                "  │ (*) English   en    │     ",
                "  │ ( ) 简体中文  zh-CN │     ",
                "  ╰─────────────────────╯     ",
            ]
        );
        // The head is over the columns of the list, and no choice.
        let (names, notes) = list.text_columns(area);
        assert_eq!((buf[(names, 1)].symbol(), buf[(notes, 1)].symbol()), ("E", "e"));
        assert_eq!(list.item_at(area, Position::new(8, 1)), None);
        assert_eq!(list.item_at(area, Position::new(8, 2)), Some(0));
    }

    #[test]
    fn a_list_with_a_head_opens_upward_where_the_screen_ends() {
        let items: Vec<(&str, &str)> = ["a", "b", "c", "d", "e"].map(|name| (name, "")).into();
        let list = Dropdown::new(&items).cursor(4).head("e", "", 4);
        let screen = Rect::new(0, 0, 12, 6);
        let area = list.area(Rect::new(4, 4, 5, 1), screen);
        // Two rows from the control's down, five up to it.
        assert_eq!(area, Rect::new(1, 0, 9, 5));
        let mut buf = Buffer::empty(screen);
        list.clone().render(area, &mut buf);
        assert_eq!(
            &rows(&buf)[..5],
            [" ╭───5/5─╮  ", " │ ( ) c │  ", " │ ( ) d │  ", " │ ( ) e │  ", " ╰  ▴  e ╯  "]
        );
        assert_eq!(list.item_at(area, Position::new(3, 3)), Some(4));
        assert_eq!(list.item_at(area, Position::new(3, 4)), None);
        // A head on another row than the list's first or last is not drawn.
        let mut buf = Buffer::empty(screen);
        list.render(Rect::new(1, 1, 9, 5), &mut buf);
        assert_eq!(rows(&buf)[1], " ╭───────╮  ");
    }

    #[test]
    fn a_joined_list_keeps_to_its_controls_row_and_the_screen() {
        let screen = Rect::new(2, 2, 20, 10);
        assert_eq!(place_joined(4, 3, screen, 8, 5), Rect::new(4, 3, 8, 5));
        // Fewer rows down than up: up to the control's row.
        assert_eq!(place_joined(4, 10, screen, 8, 5), Rect::new(4, 6, 8, 5));
        // Cut off where neither side holds it, on the side with more rows.
        assert_eq!(place_joined(4, 6, screen, 8, 9), Rect::new(4, 6, 8, 6));
        assert_eq!(place_joined(4, 8, screen, 8, 9), Rect::new(4, 2, 8, 7));
        // Moved left to stay on the screen; nowhere for a row off it.
        assert_eq!(place_joined(20, 3, screen, 8, 3), Rect::new(14, 3, 8, 3));
        assert!(place_joined(4, 1, screen, 8, 5).is_empty());
        assert!(place_joined(4, 12, screen, 8, 5).is_empty());
    }

    #[test]
    fn a_control_can_line_its_text_up_with_the_list() {
        let list = Dropdown::new(LANGUAGES);
        let area = Rect::new(2, 2, 23, 5);
        let (names, notes) = list.text_columns(area);
        let mut buf = Buffer::empty(Rect::new(0, 0, 30, 8));
        list.render(area, &mut buf);
        assert_eq!((names, notes), (8, 18));
        assert_eq!((buf[(names, 3)].symbol(), buf[(notes, 3)].symbol()), ("S", "a"));
        assert_eq!(Dropdown::new(&[]).text_columns(Rect::new(u16::MAX, 0, 9, 1)).1, u16::MAX);
    }

    #[test]
    fn the_cursor_row_is_highlighted_and_notes_are_dim() {
        let highlight = Style::new().bg(Color::Blue);
        let list = Dropdown::new(LANGUAGES).cursor(1).highlight_style(highlight);
        let mut buf = Buffer::empty(Rect::new(0, 0, 23, 5));
        list.render(buf.area, &mut buf);
        for x in 1..22 {
            assert_eq!(buf[(x, 2)].bg, Color::Blue, "{x}");
            assert_eq!(buf[(x, 1)].bg, Color::Reset, "{x}");
        }
        // The border beside the row is the window's.
        assert_eq!(buf[(0, 2)].bg, Color::Reset);
        assert!(buf[(16, 2)].modifier.contains(Modifier::DIM));
        assert!(!buf[(6, 2)].modifier.contains(Modifier::DIM));
        // Without a style of its own, the cursor's row is in reverse video.
        let mut buf = Buffer::empty(Rect::new(0, 0, 23, 5));
        Dropdown::new(LANGUAGES).render(buf.area, &mut buf);
        assert!(buf[(6, 1)].modifier.contains(Modifier::REVERSED));
        assert!(!buf[(6, 2)].modifier.contains(Modifier::REVERSED));
    }

    #[test]
    fn what_was_drawn_under_the_list_is_gone() {
        let mut buf = Buffer::empty(Rect::new(0, 0, 23, 5));
        buf.set_string(0, 1, "x".repeat(23), Style::new().fg(Color::Red));
        Dropdown::new(LANGUAGES).cursor(1).render(buf.area, &mut buf);
        assert_eq!(rows(&buf)[1], "│ ( ) System    auto  │");
        assert_eq!(buf[(21, 1)].fg, Color::Reset);
    }

    #[test]
    fn a_wide_character_cut_by_the_left_edge_is_blanked() {
        let mut buf = Buffer::empty(Rect::new(0, 0, 14, 4));
        for y in 0..4 {
            buf.set_string(0, y, "文件文件", Style::new().fg(Color::Red));
        }
        // The list starts on the second half of a character on each of its rows.
        Dropdown::new(&[("a", "")]).render(Rect::new(3, 1, 9, 3), &mut buf);
        assert_eq!(
            rows(&buf),
            ["文件文件      ", "文 ╭───────╮  ", "文 │ ( ) a │  ", "文 ╰───────╯  "]
        );
        assert_eq!(buf[(2, 1)].fg, Color::Red);
        // One that ends before the list stays.
        let mut buf = Buffer::empty(Rect::new(0, 0, 14, 1));
        buf.set_string(0, 0, "文件", Style::new());
        Dropdown::new(&[]).render(Rect::new(4, 0, 9, 1), &mut buf);
        assert_eq!(rows(&buf), ["文件╭───────╮ "]);
        // At the buffer's own edge there is no cell before the list.
        let mut buf = Buffer::empty(Rect::new(5, 0, 9, 1));
        Dropdown::new(&[]).render(buf.area, &mut buf);
        assert_eq!(rows(&buf), ["╭───────╮"]);
    }

    #[test]
    fn a_list_too_long_scrolls_with_its_cursor_and_says_where_it_is() {
        let items: Vec<(&str, &str)> =
            ["a", "b", "c", "d", "e"].iter().map(|name| (*name, "")).collect();
        let draw = |cursor| {
            let mut buf = Buffer::empty(Rect::new(0, 0, 9, 4));
            Dropdown::new(&items).cursor(cursor).render(buf.area, &mut buf);
            rows(&buf)
        };
        assert_eq!(draw(0), ["╭───────╮", "│ ( ) a │", "│ ( ) b │", "╰───1/5─╯"]);
        assert_eq!(draw(3), ["╭───────╮", "│ ( ) c │", "│ ( ) d │", "╰───4/5─╯"]);
        // A cursor past the last item is on the last.
        assert_eq!(draw(9), ["╭───────╮", "│ ( ) d │", "│ ( ) e │", "╰───5/5─╯"]);
        assert_eq!(offset(0, 5, 2), 0);
        assert_eq!(offset(1, 5, 2), 0);
        assert_eq!(offset(2, 5, 2), 1);
        assert_eq!(offset(4, 5, 2), 3);
        assert_eq!(offset(4, 5, 9), 0);
        assert_eq!(offset(4, 5, 0), 5);
        assert_eq!(offset(0, 0, 3), 0);
    }

    #[test]
    fn a_click_finds_its_item() {
        let items: Vec<(&str, &str)> =
            ["a", "b", "c", "d", "e"].iter().map(|name| (*name, "")).collect();
        let list = Dropdown::new(&items).cursor(3);
        let area = Rect::new(4, 2, 9, 4);
        // Scrolled by two: the rows show c and d.
        assert_eq!(list.item_at(area, Position::new(6, 3)), Some(2));
        assert_eq!(list.item_at(area, Position::new(11, 4)), Some(3));
        for on_the_border in [(4, 3), (12, 3), (6, 2), (6, 5), (3, 3), (6, 6)] {
            assert_eq!(list.item_at(area, on_the_border.into()), None, "{on_the_border:?}");
        }
        // Rows left empty under a short list hold no item.
        let list = Dropdown::new(&items[..1]);
        assert_eq!(list.item_at(area, Position::new(6, 3)), Some(0));
        assert_eq!(list.item_at(area, Position::new(6, 4)), None);
    }

    #[test]
    fn the_list_goes_where_the_screen_has_room() {
        let screen = Rect::new(0, 0, 40, 12);
        let anchor = |x, y| Rect::new(x, y, 8, 1);
        // Under the control, at its left edge.
        assert_eq!(place(anchor(5, 2), screen, 20, 5), Rect::new(5, 3, 20, 5));
        // Moved left where the screen ends.
        assert_eq!(place(anchor(30, 2), screen, 20, 5), Rect::new(20, 3, 20, 5));
        // Above, where there are more rows than under the control.
        assert_eq!(place(anchor(5, 9), screen, 20, 5), Rect::new(5, 4, 20, 5));
        // Cut off where neither side has the rows: on the side with more of them.
        assert_eq!(place(anchor(5, 5), screen, 20, 9), Rect::new(5, 6, 20, 6));
        assert_eq!(place(anchor(5, 7), screen, 20, 9), Rect::new(5, 0, 20, 7));
        // Wider than the screen.
        assert_eq!(place(anchor(5, 2), screen, 90, 5), Rect::new(0, 3, 40, 5));
        // A control off the screen: the list stays on it.
        assert_eq!(place(anchor(50, 20), screen, 20, 5), Rect::new(20, 7, 20, 5));
        let screen = Rect::new(10, 10, 40, 12);
        assert_eq!(place(anchor(0, 0), screen, 20, 5), Rect::new(10, 10, 20, 5));
        // No items, no notes: a window with nothing in it.
        assert_eq!(Dropdown::new(&[]).area(anchor(0, 0), screen), Rect::new(10, 10, 8, 2));
        let no_notes = [("a", ""), ("bcd", "")];
        assert_eq!(Dropdown::new(&no_notes).area(anchor(0, 0), screen).width, 11);
    }

    #[test]
    fn nothing_is_drawn_outside_the_area() {
        let blank = Buffer::empty(Rect::new(0, 0, 12, 4));
        let mut buf = blank.clone();
        let list = Dropdown::new(LANGUAGES).chosen(0);
        list.clone().render(Rect::new(0, 4, 12, 4), &mut buf);
        list.clone().render(Rect::new(3, 1, 0, 3), &mut buf);
        assert_eq!(buf, blank);
        // Cut off by the buffer and narrower than its items: names and notes end early.
        list.render(Rect::new(0, 1, 30, 9), &mut buf);
        assert_eq!(&rows(&buf)[1..], ["╭──────────╮", "│ (*) Syst │", "╰──────1/3─╯"]);
    }
}
