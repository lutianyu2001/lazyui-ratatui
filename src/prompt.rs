//! Prompts with suggestions.

use ratatui::buffer::Buffer;
use ratatui::layout::{Position, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::widgets::{Clear, Widget};

use crate::dropdown::{offset, uncover};
use crate::field::cursor_cell;
use crate::message::{centered, popup_width};
use crate::text::{sanitize, take_width, width};
use crate::{Field, Window};

/// Rows of a window around one line of text.
const INPUT_ROWS: u16 = 3;

/// A prompt in the middle of the screen, as lazygit asks for a branch's name: a rounded
/// window named on its top border, holding one line of text to edit, and under it, with the
/// same left and right edges, a second window listing suggestions, one a row. Of the two,
/// the one with the focus has its border in the focus style. While the suggestions have it,
/// the row under their cursor is highlighted; the list scrolls to keep that row visible and
/// says where it is in the bottom right corner (`2 of 15`).
///
/// It is drawn over what the buffer holds, as a [`Dropdown`](crate::Dropdown) is: the cells
/// of both windows are cleared first, and a wide character that reaches into one from the
/// cell before it is blanked there.
///
/// The program holding the prompt keeps the text, both cursors and the focus, and changes
/// them. `render` takes the screen; [`Prompt::areas`] says where on it the two windows go,
/// [`Prompt::cursor_at`] where the terminal's cursor goes, and [`Prompt::suggestion_at`]
/// which suggestion a click is on.
#[derive(Debug, Clone)]
pub struct Prompt<'a> {
    title: &'a str,
    text: &'a str,
    cursor: usize,
    list: Option<(&'a str, &'a [&'a str])>,
    selected: usize,
    on_list: bool,
    border: Style,
    focus: Style,
    highlight: Style,
    width: u16,
    rows: u16,
    gap: u16,
}

/// Where the two windows of a [`Prompt`] go. Without suggestions, theirs is empty.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct PromptAreas {
    /// The window holding the text.
    pub input: Rect,
    /// The window holding the suggestions.
    pub suggestions: Rect,
}

impl<'a> Prompt<'a> {
    /// A prompt named `title` holding `text`, with the cursor before char `cursor`, and no
    /// suggestions.
    pub fn new(title: &'a str, text: &'a str, cursor: usize) -> Self {
        Self {
            title,
            text,
            cursor,
            list: None,
            selected: 0,
            on_list: false,
            border: Style::new(),
            focus: Style::new().fg(Color::Green),
            highlight: Style::new().add_modifier(Modifier::REVERSED),
            width: 80,
            rows: 10,
            gap: 0,
        }
    }

    /// The window of suggestions, named `title`. It is there with no suggestions too, empty.
    pub fn suggestions(mut self, title: &'a str, suggestions: &'a [&'a str]) -> Self {
        self.list = Some((title, suggestions));
        self
    }

    /// The suggestion under the cursor; past the last one, the last.
    pub fn selected(mut self, suggestion: usize) -> Self {
        self.selected = suggestion;
        self
    }

    /// Whether the suggestions have the focus, not the text.
    pub fn focus_suggestions(mut self, focused: bool) -> Self {
        self.on_list = focused;
        self
    }

    /// Style of the border, and the texts on it, of the window without the focus.
    pub fn border_style(mut self, style: Style) -> Self {
        self.border = style;
        self
    }

    /// The same of the window with the focus; green unless set, as in lazygit.
    pub fn focus_style(mut self, style: Style) -> Self {
        self.focus = style;
        self
    }

    /// Style of the suggestion under the cursor while the suggestions have the focus, over
    /// the list's own; reverse video unless set.
    pub fn highlight_style(mut self, style: Style) -> Self {
        self.highlight = style;
        self
    }

    /// The width of both windows, borders included, where the screen has it; 80 cells unless
    /// set, as in lazygit.
    pub fn width(mut self, cells: u16) -> Self {
        self.width = cells;
        self
    }

    /// The rows of suggestions shown at a time; 10 unless set, as in lazygit.
    pub fn rows(mut self, rows: u16) -> Self {
        self.rows = rows;
        self
    }

    /// Rows left free between the two windows, which keep what the buffer holds; none unless
    /// set, as in lazygit.
    pub fn gap(mut self, rows: u16) -> Self {
        self.gap = rows;
        self
    }

    /// Where the windows go on `screen`: in its middle, the suggestions under the text with
    /// the same left and right edges. A screen wider than the prompt keeps 3 cells free
    /// beside it, a narrower one a single cell, as lazygit does. On a screen with too few
    /// rows, the suggestions lose theirs first.
    pub fn areas(&self, screen: Rect) -> PromptAreas {
        let width = popup_width(self.width, screen.width);
        let (input, suggestions) = boxes(screen, width, self.list.map(|_| self.rows), self.gap);
        PromptAreas { input, suggestions }
    }

    /// Where the terminal's cursor goes for a prompt drawn on `screen`; nowhere while the
    /// suggestions have the focus, or where the screen has no room for the text. `screen` is
    /// the part of it inside the buffer, which is what `render` draws on.
    pub fn cursor_at(&self, screen: Rect) -> Option<Position> {
        let inner = Window::new().inner(self.areas(screen).input);
        if self.on_list || inner.is_empty() {
            return None;
        }
        let view = Field::view(self.text, self.cursor, usize::from(inner.width));
        Some(cursor_cell(inner, view.cursor))
    }

    /// The suggestion drawn at `at` by a prompt on `screen`, for a click there; none on a
    /// border, on a row without one or off the list. Which of the windows a click is on,
    /// [`Prompt::areas`] says.
    pub fn suggestion_at(&self, screen: Rect, at: Position) -> Option<usize> {
        let (_, suggestions) = self.list?;
        let inner = Window::new().inner(self.areas(screen).suggestions);
        if !inner.contains(at) {
            return None;
        }
        let row = usize::from(at.y.saturating_sub(inner.y));
        let first = offset(self.selected, suggestions.len(), usize::from(inner.height));
        let suggestion = first.saturating_add(row);
        (suggestion < suggestions.len()).then_some(suggestion)
    }

    /// The style of a window's border, by whether the window has the focus.
    fn border(&self, focused: bool) -> Style {
        if focused { self.focus } else { self.border }
    }

    fn draw_input(&self, area: Rect, buf: &mut Buffer) {
        if area.is_empty() {
            return;
        }
        uncover(area, buf);
        Clear.render(area, buf);
        let window = Window::new().name("", self.title).border_style(self.border(!self.on_list));
        let inner = window.inner(area);
        window.render(area, buf);
        Field::new(self.text, self.cursor).render(inner, buf);
    }

    fn draw_suggestions(&self, title: &str, suggestions: &[&str], area: Rect, buf: &mut Buffer) {
        if area.is_empty() {
            return;
        }
        uncover(area, buf);
        Clear.render(area, buf);
        let window = Window::new().name("", title).border_style(self.border(self.on_list));
        let inner = window.inner(area);
        let selected = self.selected.min(suggestions.len().saturating_sub(1));
        let nth = if suggestions.is_empty() { 0 } else { selected.saturating_add(1) };
        window.note(&format!("{nth} of {}", suggestions.len())).render(area, buf);
        let first = offset(selected, suggestions.len(), usize::from(inner.height));
        let shown = suggestions.iter().enumerate().skip(first);
        for ((suggestion, text), y) in shown.zip(inner.y..inner.bottom()) {
            let highlighted = self.on_list && suggestion == selected;
            let style = if highlighted { self.highlight } else { Style::new() };
            buf.set_style(Rect { y, height: 1, ..inner }, style);
            let text = take_width(text, usize::from(inner.width));
            buf.set_stringn(inner.x, y, sanitize(text), width(text), style);
        }
    }
}

impl Widget for Prompt<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let areas = self.areas(area.intersection(buf.area));
        self.draw_input(areas.input, buf);
        if let Some((title, suggestions)) = self.list {
            self.draw_suggestions(title, suggestions, areas.suggestions, buf);
        }
    }
}

/// The window of the text and that of the suggestions for a prompt `w` cells wide on
/// `screen`, `gap` rows apart, the second with `rows` rows inside it, or empty with `None`.
/// The pair goes where lazygit puts it: the text's window where a popup one row lower than
/// the pair would start. Both are cut to the screen's width, and what the screen has no rows
/// for is cut off the suggestions, then off the text's window.
pub(crate) fn boxes(screen: Rect, w: u16, rows: Option<u16>, gap: u16) -> (Rect, Rect) {
    // A screen written out to reach past `u16::MAX` ends there.
    let screen = Rect::new(screen.x, screen.y, screen.width, screen.height);
    let Some(rows) = rows else {
        let input = centered(screen, w, INPUT_ROWS);
        return (input, Rect { y: input.bottom(), height: 0, ..input });
    };
    let list = rows.saturating_add(2);
    let pair = INPUT_ROWS.saturating_add(gap).saturating_add(list);
    let popup = centered(screen, w, pair.saturating_sub(1));
    let end = screen.bottom();
    // Up from there, where the pair would reach past the screen's last row.
    let y = popup.y.min(end.saturating_sub(pair)).max(screen.y);
    let input = Rect { y, height: INPUT_ROWS.min(end.saturating_sub(y)), ..popup };
    let top = input.bottom().saturating_add(gap).min(end);
    (input, Rect { y: top, height: list.min(end.saturating_sub(top)), ..popup })
}

#[cfg(test)]
mod tests {
    use super::*;

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

    const BRANCHES: &[&str] = &["master", "alpha", "beta", "delta", "简体中文", "eta"];

    fn prompt() -> Prompt<'static> {
        Prompt::new("Branch name", "al", 2).suggestions("Suggestions", BRANCHES).rows(4)
    }

    #[test]
    fn the_suggestions_go_under_the_text_with_the_same_edges() {
        let mut buf = Buffer::empty(Rect::new(0, 0, 40, 12));
        let prompt = prompt().selected(1).focus_suggestions(true);
        let areas = prompt.areas(buf.area);
        assert_eq!(areas.input, Rect::new(1, 2, 38, 3));
        assert_eq!(areas.suggestions, Rect::new(1, 5, 38, 6));
        prompt.render(buf.area, &mut buf);
        assert_eq!(
            rows(&buf),
            [
                "                                        ",
                "                                        ",
                " ╭─Branch name────────────────────────╮ ",
                " │al                                  │ ",
                " ╰────────────────────────────────────╯ ",
                " ╭─Suggestions────────────────────────╮ ",
                " │master                              │ ",
                " │alpha                               │ ",
                " │beta                                │ ",
                " │delta                               │ ",
                " ╰─────────────────────────────2 of 6─╯ ",
                "                                        ",
            ]
        );
    }

    #[test]
    fn a_prompt_without_suggestions_is_one_window() {
        let mut buf = Buffer::empty(Rect::new(0, 0, 24, 5));
        let prompt = Prompt::new("Name", "abc", 1).width(20);
        let areas = prompt.areas(buf.area);
        assert_eq!(areas.input, Rect::new(2, 0, 20, 3));
        assert!(areas.suggestions.is_empty());
        assert_eq!(prompt.cursor_at(buf.area), Some(Position::new(4, 1)));
        assert_eq!(prompt.suggestion_at(buf.area, Position::new(4, 1)), None);
        prompt.render(buf.area, &mut buf);
        assert_eq!(
            &rows(&buf)[..3],
            ["  ╭─Name─────────────╮  ", "  │abc               │  ", "  ╰──────────────────╯  "]
        );
        // A window of suggestions is there with none in it too.
        let mut buf = Buffer::empty(Rect::new(0, 0, 24, 7));
        Prompt::new("Name", "", 0).suggestions("Found", &[]).rows(1).render(buf.area, &mut buf);
        assert_eq!(
            &rows(&buf)[3..6],
            [" ╭─Found──────────────╮ ", " │                    │ ", " ╰─────────────0 of 0─╯ "]
        );
    }

    #[test]
    fn the_window_with_the_focus_has_the_focus_style() {
        let draw = |on_list| {
            let mut buf = Buffer::empty(Rect::new(0, 0, 40, 12));
            let prompt = prompt().border_style(Style::new().fg(Color::Blue));
            prompt.selected(1).focus_suggestions(on_list).render(buf.area, &mut buf);
            buf
        };
        let buf = draw(false);
        assert_eq!((buf[(1, 2)].fg, buf[(1, 5)].fg), (Color::Green, Color::Blue));
        // No row is highlighted while the text has the focus.
        assert!(!buf[(2, 7)].modifier.contains(Modifier::REVERSED));
        let buf = draw(true);
        assert_eq!((buf[(1, 2)].fg, buf[(1, 5)].fg), (Color::Blue, Color::Green));
        // The title and the note are in their border's style, bold as a window's are.
        assert_eq!((buf[(30, 2)].fg, buf[(32, 10)].fg), (Color::Blue, Color::Green));
        assert!(buf[(32, 10)].modifier.contains(Modifier::BOLD));
        // The whole row under the cursor, between the borders, and no other.
        for x in 2..38 {
            assert!(buf[(x, 7)].modifier.contains(Modifier::REVERSED), "{x}");
            assert!(!buf[(x, 6)].modifier.contains(Modifier::REVERSED), "{x}");
        }
        assert!(!buf[(1, 7)].modifier.contains(Modifier::REVERSED));
        let mut buf = Buffer::empty(Rect::new(0, 0, 40, 12));
        let styled = prompt().focus_style(Style::new().fg(Color::Red));
        let styled = styled.highlight_style(Style::new().bg(Color::Blue)).focus_suggestions(true);
        styled.render(buf.area, &mut buf);
        assert_eq!((buf[(1, 5)].fg, buf[(2, 6)].bg), (Color::Red, Color::Blue));
    }

    #[test]
    fn the_suggestions_scroll_with_their_cursor() {
        let draw = |selected| {
            let mut buf = Buffer::empty(Rect::new(0, 0, 16, 7));
            let prompt = prompt().rows(2).selected(selected);
            prompt.clone().render(buf.area, &mut buf);
            (rows(&buf)[4..7].to_vec(), prompt.suggestion_at(buf.area, Position::new(3, 4)))
        };
        let shown =
            |a: &str, b: &str, note: &str| vec![a.to_owned(), b.to_owned(), note.to_owned()];
        assert_eq!(
            draw(0),
            (shown(" │master      │ ", " │alpha       │ ", " ╰─────1 of 6─╯ "), Some(0))
        );
        assert_eq!(
            draw(4),
            (shown(" │delta       │ ", " │简体中文    │ ", " ╰─────5 of 6─╯ "), Some(3))
        );
        // A cursor past the last suggestion is on the last.
        assert_eq!(
            draw(9),
            (shown(" │简体中文    │ ", " │eta         │ ", " ╰─────6 of 6─╯ "), Some(4))
        );
    }

    #[test]
    fn a_click_finds_its_suggestion() {
        let screen = Rect::new(0, 0, 40, 12);
        let prompt = prompt().selected(5);
        // Scrolled by two: the rows show beta to eta.
        assert_eq!(prompt.suggestion_at(screen, Position::new(2, 6)), Some(2));
        assert_eq!(prompt.suggestion_at(screen, Position::new(37, 9)), Some(5));
        for off_the_list in [(1, 6), (38, 6), (2, 5), (2, 10), (2, 3), (0, 0), (2, 11)] {
            assert_eq!(prompt.suggestion_at(screen, off_the_list.into()), None, "{off_the_list:?}");
        }
        // Rows left empty under a short list hold no suggestion.
        let prompt = Prompt::new("", "", 0).suggestions("", &BRANCHES[..1]).rows(4);
        assert_eq!(prompt.suggestion_at(screen, Position::new(2, 6)), Some(0));
        assert_eq!(prompt.suggestion_at(screen, Position::new(2, 7)), None);
        // Which window a click is on is in the areas.
        let areas = prompt.areas(screen);
        assert!(areas.input.contains(Position::new(1, 2)));
        assert!(areas.suggestions.contains(Position::new(1, 5)));
    }

    #[test]
    fn the_cursor_goes_where_the_text_has_it() {
        let screen = Rect::new(0, 0, 14, 12);
        // 12 cells wide, 10 inside: the text scrolls as a field does.
        let prompt = Prompt::new("", "0123456789abcdef", 16).suggestions("", BRANCHES).rows(4);
        assert_eq!(prompt.cursor_at(screen), Some(Position::new(11, 3)));
        let mut buf = Buffer::empty(screen);
        prompt.clone().render(screen, &mut buf);
        assert_eq!(rows(&buf)[3], " │<89abcdef │ ");
        // No cursor while the suggestions have the focus, nor without a row for the text.
        assert_eq!(prompt.clone().focus_suggestions(true).cursor_at(screen), None);
        assert_eq!(prompt.cursor_at(Rect::new(0, 0, 14, 2)), None);
        assert_eq!(prompt.cursor_at(Rect::new(0, 0, 3, 12)), None);
        // On a screen that does not start at the origin.
        let prompt = Prompt::new("", "文件", 1).width(8);
        assert_eq!(prompt.cursor_at(Rect::new(10, 20, 20, 5)), Some(Position::new(19, 21)));
    }

    #[test]
    fn the_windows_go_where_lazygit_puts_them() {
        // As lazygit 0.63.0 draws them in a terminal of 110 by 30.
        let screen = Rect::new(0, 0, 110, 30);
        let (input, list) = boxes(screen, popup_width(80, 110), Some(10), 0);
        assert_eq!((input, list), (Rect::new(15, 8, 80, 3), Rect::new(15, 11, 80, 12)));
        let (input, list) = boxes(screen, 80, None, 0);
        assert_eq!((input, list), (Rect::new(15, 13, 80, 3), Rect::new(15, 16, 80, 0)));
        // With rows between them.
        let (input, list) = boxes(screen, 80, Some(10), 1);
        assert_eq!((input, list), (Rect::new(15, 7, 80, 3), Rect::new(15, 11, 80, 12)));
        // The pair fills the screen exactly.
        let screen = Rect::new(5, 5, 20, 9);
        let (input, list) = boxes(screen, 30, Some(4), 0);
        assert_eq!((input, list), (Rect::new(5, 5, 20, 3), Rect::new(5, 8, 20, 6)));
        // Too few rows: the suggestions lose theirs, then the text's window does.
        let rows_of = |height| {
            let (input, list) = boxes(Rect { height, ..screen }, 30, Some(4), 1);
            ((input.y, input.height), (list.y, list.height))
        };
        assert_eq!(rows_of(9), ((5, 3), (9, 5)));
        assert_eq!(rows_of(5), ((5, 3), (9, 1)));
        assert_eq!(rows_of(4), ((5, 3), (9, 0)));
        assert_eq!(rows_of(3), ((5, 3), (8, 0)));
        assert_eq!(rows_of(2), ((5, 2), (7, 0)));
        assert_eq!(rows_of(0), ((5, 0), (5, 0)));
        // More rows and a wider gap than any screen has.
        let (input, list) = boxes(screen, 30, Some(u16::MAX), u16::MAX);
        assert_eq!((input, list), (Rect::new(5, 5, 20, 3), Rect::new(5, 14, 20, 0)));
    }

    #[test]
    fn text_from_elsewhere_stays_in_its_window() {
        let names = ["a\u{1b}[31mb\tc", "文件文件文件文", "x\u{200b}y\u{202e}z"];
        let mut buf = Buffer::empty(Rect::new(0, 0, 17, 9));
        let prompt = Prompt::new("a\u{7}b", "q\u{1b}w", 3).suggestions("件\r件", &names).rows(3);
        prompt.render(buf.area, &mut buf);
        assert_eq!(
            rows(&buf)[..7],
            [
                " ╭─ab──────────╮ ",
                " │qw           │ ",
                " ╰─────────────╯ ",
                " ╭─件件────────╮ ",
                " │a[31mbc      │ ",
                // The seventh character has one cell left, and stays out.
                " │文件文件文件 │ ",
                " │xyz          │ ",
            ]
        );
    }

    #[test]
    fn what_was_drawn_under_the_prompt_is_gone() {
        let mut buf = Buffer::empty(Rect::new(0, 0, 14, 8));
        for y in 0..8 {
            buf.set_string(0, y, "文件文件文件文", Style::new().fg(Color::Red));
        }
        // Both windows start on the second half of a character on each of their rows.
        Prompt::new("", "a", 1).suggestions("", &["b"]).rows(1).render(buf.area, &mut buf);
        assert_eq!(
            rows(&buf),
            [
                "文件文件文件文",
                " ╭──────────╮ ",
                " │a         │ ",
                " ╰──────────╯ ",
                " ╭──────────╮ ",
                " │b         │ ",
                " ╰───1 of 1─╯ ",
                "文件文件文件文",
            ]
        );
        assert_eq!((buf[(3, 2)].fg, buf[(0, 2)].fg), (Color::Reset, Color::Red));
        assert_eq!((buf[(0, 2)].symbol(), buf[(0, 0)].symbol()), (" ", "文"));
    }

    #[test]
    fn nothing_is_drawn_outside_the_buffer() {
        let blank = Buffer::empty(Rect::new(0, 0, 12, 4));
        let mut buf = blank.clone();
        prompt().render(Rect::new(0, 4, 12, 8), &mut buf);
        prompt().render(Rect::new(12, 0, 30, 4), &mut buf);
        prompt().render(Rect::new(3, 1, 0, 3), &mut buf);
        assert_eq!(buf, blank);
        // A screen that reaches past the buffer is the part inside it.
        prompt().selected(3).render(Rect::new(0, 0, 90, 90), &mut buf);
        assert_eq!(rows(&buf), [" ╭────────╮ ", " │al      │ ", " ╰────────╯ ", " ╭────────╮ "]);
    }
}
