//! Message popups, and where a popup goes on the screen.

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::widgets::{Clear, Widget};
use unicode_segmentation::UnicodeSegmentation;

use crate::Window;
use crate::dropdown::{saturate, uncover};
use crate::text::{grapheme_width, sanitize, take_width, width};

/// The width a popup keeps while the screen has it, and the cells kept free on each side of
/// a popup while the screen has those too: lazygit's.
const MIN_WIDTH: u16 = 80;
const MARGIN: u16 = 3;

/// A message in the middle of the screen, as lazygit's welcome page and its confirmations
/// look: a rounded window, an optional title on its top border, and the text, wrapped at
/// spaces to the window's width. The window is as wide and as high as its text needs, up to
/// [`Message::width`] and to the screen; of a text with more rows than that, it shows those
/// from [`Message::scroll`] on.
///
/// It is drawn over what the buffer holds, as a [`Dropdown`](crate::Dropdown) is: the cells
/// of its area are cleared first, and a wide character that reaches into the area from the
/// cell before it is blanked there.
///
/// `render` takes the screen and puts the window in its middle; [`Message::area`] says where
/// that is.
#[derive(Debug, Clone)]
pub struct Message<'a> {
    title: &'a str,
    text: &'a str,
    border: Style,
    style: Style,
    width: u16,
    scroll: usize,
}

impl<'a> Message<'a> {
    /// A message saying `text`, whose lines end at `\n`; a line too long for the window goes
    /// on at its last space, and where it has none, at the window's edge.
    pub fn new(text: &'a str) -> Self {
        Self {
            title: "",
            text,
            border: Style::new(),
            style: Style::new(),
            width: MIN_WIDTH,
            scroll: 0,
        }
    }

    /// The title on the top border.
    pub fn title(mut self, title: &'a str) -> Self {
        self.title = title;
        self
    }

    /// Style of the border and the title on it.
    pub fn border_style(mut self, style: Style) -> Self {
        self.border = style;
        self
    }

    /// Style of the text.
    pub fn style(mut self, style: Style) -> Self {
        self.style = style;
        self
    }

    /// The widest the window gets, borders included; 80 cells unless set, as in lazygit.
    pub fn width(mut self, cells: u16) -> Self {
        self.width = cells;
        self
    }

    /// The first row of text shown where the window can't hold them all; past the last
    /// screenful, that one (see [`Message::overflow`]).
    pub fn scroll(mut self, row: usize) -> Self {
        self.scroll = row;
        self
    }

    /// Where the message goes on `screen`: in its middle, as wide as its longest line and
    /// its title need and as high as its text, wrapped, has rows. A screen wider than the
    /// message keeps 3 cells free beside it, a narrower one a single cell, as lazygit does;
    /// a screen with fewer rows than the text gets a window of its own height.
    pub fn area(&self, screen: Rect) -> Rect {
        let width = self.outer_width(screen);
        let rows = wrap(self.text, usize::from(width.saturating_sub(2))).len();
        centered(screen, width, saturate(rows.saturating_add(2)))
    }

    /// The rows of text the window on `screen` has no room for: the furthest it scrolls.
    pub fn overflow(&self, screen: Rect) -> usize {
        let inner = Window::new().inner(self.area(screen));
        let rows = wrap(self.text, usize::from(inner.width)).len();
        rows.saturating_sub(usize::from(inner.height))
    }

    /// The window's width on `screen`, borders included.
    fn outer_width(&self, screen: Rect) -> u16 {
        let longest = self.text.lines().map(width).max().unwrap_or(0);
        // The text between the borders; the title with a `─` beside each corner too.
        let needed = longest.saturating_add(2).max(match width(self.title) {
            0 => 0,
            title => title.saturating_add(4),
        });
        saturate(needed).min(popup_width(self.width, screen.width))
    }
}

impl Widget for Message<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let area = self.area(area.intersection(buf.area));
        if area.is_empty() {
            return;
        }
        uncover(area, buf);
        Clear.render(area, buf);
        let window = Window::new().name("", self.title).border_style(self.border);
        let inner = window.inner(area);
        window.render(area, buf);
        let lines = wrap(self.text, usize::from(inner.width));
        let first = first_row(self.scroll, lines.len(), usize::from(inner.height));
        for (line, y) in lines.iter().skip(first).zip(inner.y..inner.bottom()) {
            // A line holding one grapheme wider than the window stays empty.
            let line = take_width(line, usize::from(inner.width));
            buf.set_stringn(inner.x, y, sanitize(line), width(line), self.style);
        }
    }
}

/// The lines of `text` in `w` cells: each line of the text as it is while it fits, else cut
/// after its last space that leaves a word before it, and where there is none, before the
/// grapheme that passes the edge. A space a line is cut at is left out, and a grapheme wider
/// than `w` gets a line of its own. No lines at all in no cells.
pub(crate) fn wrap(text: &str, w: usize) -> Vec<&str> {
    let mut lines = Vec::new();
    if w > 0 {
        for line in text.lines() {
            wrap_line(line, w, &mut lines);
        }
    }
    lines
}

fn wrap_line<'a>(line: &'a str, w: usize, lines: &mut Vec<&'a str>) {
    // Where the row being filled starts, its cells so far, and its last space after a word.
    let (mut start, mut cells, mut space) = (0_usize, 0_usize, None);
    let mut word = false;
    for (i, g) in line.grapheme_indices(true) {
        let wide = grapheme_width(g);
        if cells.saturating_add(wide) > w && i > start {
            if g == " " {
                lines.push(&line[start..i]);
                (start, cells, space, word) = (i.saturating_add(1), 0, None, false);
                continue;
            }
            if let Some(at) = space.take() {
                lines.push(&line[start..at]);
                start = at.saturating_add(1);
                cells = width(&line[start..i]);
                word = i > start;
            }
            // No space to cut at, or a wide grapheme that passes the edge even so.
            if cells.saturating_add(wide) > w && i > start {
                lines.push(&line[start..i]);
                (start, cells) = (i, 0);
            }
        }
        cells = cells.saturating_add(wide);
        if g != " " {
            word = true;
        } else if word {
            space = Some(i);
        }
    }
    lines.push(&line[start.min(line.len())..]);
}

/// The first of `lines` rows of text shown in `rows` rows when asked to scroll to `scroll`:
/// no further than where the last row of text is in the last row.
pub(crate) fn first_row(scroll: usize, lines: usize, rows: usize) -> usize {
    scroll.min(lines.saturating_sub(rows))
}

/// The width of a popup that asks for `asked` cells on a screen `screen` cells wide, as
/// lazygit sizes its popups: what it asks for while `MARGIN` cells stay free on each side;
/// then it gives the margin up before it gets narrower than `MIN_WIDTH`, and in the end it
/// leaves a cell on each side.
pub(crate) fn popup_width(asked: u16, screen: u16) -> u16 {
    let with_margin = screen.saturating_sub(MARGIN.saturating_mul(2));
    let at_min = MIN_WIDTH.min(screen.saturating_sub(2));
    asked.min(with_margin.max(at_min))
}

/// A popup `w` cells wide and `h` rows high in the middle of `screen`, cut to the screen's
/// size. Where the cells left over are uneven, the odd one goes to the right and under the
/// popup, and a popup with an uneven number of rows sits a row higher still, as lazygit's.
pub(crate) fn centered(screen: Rect, w: u16, h: u16) -> Rect {
    // A screen written out to reach past `u16::MAX` ends there.
    let screen = Rect::new(screen.x, screen.y, screen.width, screen.height);
    let (width, height) = (w.min(screen.width), h.min(screen.height));
    let left = screen.width.saturating_sub(width) / 2;
    let above = (screen.height / 2).saturating_sub(height / 2).saturating_sub(height % 2);
    Rect { x: screen.x.saturating_add(left), y: screen.y.saturating_add(above), width, height }
}

#[cfg(test)]
mod tests {
    use ratatui::style::{Color, Modifier};

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

    const WELCOME: &str = "Thanks for using this! Three things to share with you:\n\n \
        1) Read the release notes, which say what has changed since the last version\n \
        2) 简体中文也可以\n\nPress <enter> to get started.";

    #[test]
    fn a_message_is_as_large_as_its_text() {
        let mut buf = Buffer::empty(Rect::new(0, 0, 30, 7));
        let message = Message::new("Delete this branch?\nIt is not merged.").title("Confirm");
        assert_eq!(message.area(buf.area), Rect::new(4, 1, 21, 4));
        assert_eq!(message.overflow(buf.area), 0);
        message.render(buf.area, &mut buf);
        assert_eq!(
            &rows(&buf)[1..5],
            [
                "    ╭─Confirm───────────╮     ",
                "    │Delete this branch?│     ",
                "    │It is not merged.  │     ",
                "    ╰───────────────────╯     ",
            ]
        );
        // A title longer than the text is what the window is as wide as.
        let message = Message::new("No").title("A question");
        assert_eq!(message.area(buf.area), Rect::new(8, 1, 14, 3));
        // No text and no title: a window with nothing in it.
        assert_eq!(Message::new("").area(buf.area), Rect::new(14, 2, 2, 2));
    }

    #[test]
    fn long_lines_go_on_at_their_last_space() {
        let mut buf = Buffer::empty(Rect::new(0, 0, 60, 10));
        let message = Message::new(WELCOME).title("Welcome").width(50);
        assert_eq!(message.area(buf.area), Rect::new(5, 0, 50, 10));
        message.render(buf.area, &mut buf);
        assert_eq!(
            rows(&buf),
            [
                "     ╭─Welcome────────────────────────────────────────╮     ",
                "     │Thanks for using this! Three things to share    │     ",
                "     │with you:                                       │     ",
                "     │                                                │     ",
                "     │ 1) Read the release notes, which say what has  │     ",
                "     │changed since the last version                  │     ",
                "     │ 2) 简体中文也可以                              │     ",
                "     │                                                │     ",
                "     │Press <enter> to get started.                   │     ",
                "     ╰────────────────────────────────────────────────╯     ",
            ]
        );
    }

    #[test]
    fn lines_are_cut_where_they_have_to_be() {
        assert_eq!(wrap("one two three", 7), ["one two", "three"]);
        assert_eq!(wrap("one two three", 6), ["one", "two", "three"]);
        // A space past the edge ends the row without starting the next.
        assert_eq!(wrap("abc def", 3), ["abc", "def"]);
        // A word longer than a row is cut at the edge, after what fits before it.
        assert_eq!(wrap("a bcdefghij k", 4), ["a", "bcde", "fghi", "j k"]);
        // The spaces a line starts with are no place to cut it.
        assert_eq!(wrap("  abcdef", 4), ["  ab", "cdef"]);
        // Text without spaces, and a wide character with one cell left for it.
        assert_eq!(wrap("简体中文也可以", 5), ["简体", "中文", "也可", "以"]);
        assert_eq!(wrap("a b文", 3), ["a", "b文"]);
        assert_eq!(wrap(" ab文", 3), [" ab", "文"]);
        // One wider than the row gets a row, which stays empty when drawn.
        assert_eq!(wrap("a文b", 1), ["a", "文", "b"]);
        // Lines end at `\n` and at `\r\n`; empty ones stay, but for one at the very end.
        assert_eq!(wrap("a\r\n\nb\n", 9), ["a", "", "b"]);
        assert_eq!(wrap("", 9), [""; 0]);
        assert_eq!(wrap("abc", 0), [""; 0]);
        // What takes no cell goes along with its row.
        assert_eq!(wrap("ab\u{1b}\u{301} cd", 2), ["ab\u{1b}\u{301}", "cd"]);
    }

    #[test]
    fn wrapped_lines_fit_and_leave_nothing_but_the_spaces_they_were_cut_at() {
        let texts = [
            WELCOME,
            "  two  spaces   and a-very-long-word-without-any  \n\n end ",
            "文件 名称文件名称 ab文c  \u{1f469}\u{200d}\u{1f4bb} e\u{301}\u{1b}[0m x\r\ny",
            "\u{1100}\u{1161}\u{11a8} \u{1f1e6}\u{1f1e7}\t\u{200b} \u{231a}\u{fe0e}ab",
        ];
        for (text, w) in texts.iter().flat_map(|text| (1..=12).map(move |w| (text, w))) {
            let mut rest = *text;
            for line in wrap(text, w) {
                let wider = width(line) > w;
                assert!(!wider || line.graphemes(true).count() == 1, "{line:?} in {w}");
                // In the text's order, with at most a space or a line's end left out before it.
                let after = ["\r\n", "\n", " ", ""]
                    .iter()
                    .find_map(|gap| rest.strip_prefix(gap)?.strip_prefix(line));
                rest = after.unwrap_or_else(|| panic!("{line:?} not next in {rest:?}, in {w}"));
            }
            assert!(["", " ", "\n", "\r\n"].contains(&rest), "{rest:?} left of {text:?} in {w}");
        }
    }

    #[test]
    fn a_text_with_too_many_rows_scrolls() {
        let draw = |scroll| {
            let mut buf = Buffer::empty(Rect::new(0, 0, 9, 4));
            let message = Message::new("a\nb\nc\nd\ne").scroll(scroll);
            assert_eq!(message.area(buf.area), Rect::new(3, 0, 3, 4));
            assert_eq!(message.overflow(buf.area), 3);
            message.render(buf.area, &mut buf);
            rows(&buf)
        };
        assert_eq!(draw(0), ["   ╭─╮   ", "   │a│   ", "   │b│   ", "   ╰─╯   "]);
        assert_eq!(draw(2), ["   ╭─╮   ", "   │c│   ", "   │d│   ", "   ╰─╯   "]);
        // Past the last screenful, that one.
        assert_eq!(draw(9), ["   ╭─╮   ", "   │d│   ", "   │e│   ", "   ╰─╯   "]);
        assert_eq!(first_row(0, 5, 2), 0);
        assert_eq!(first_row(3, 5, 2), 3);
        assert_eq!(first_row(4, 5, 2), 3);
        assert_eq!(first_row(4, 5, 9), 0);
        assert_eq!(first_row(4, 0, 0), 0);
    }

    #[test]
    fn a_popup_is_as_wide_as_lazygit_makes_it() {
        // What it asks for, with the margin; without it; then a cell on each side.
        for (screen, wide) in [(200, 80), (86, 80), (85, 80), (82, 80), (81, 79), (50, 48)] {
            assert_eq!(popup_width(80, screen), wide, "{screen}");
        }
        for (screen, wide) in [(200, 20), (26, 20), (25, 20), (22, 20), (21, 19), (3, 1), (2, 0)] {
            assert_eq!(popup_width(20, screen), wide, "{screen}");
        }
        assert_eq!(popup_width(u16::MAX, u16::MAX), u16::MAX.saturating_sub(6));
        assert_eq!(popup_width(0, 80), 0);
    }

    #[test]
    fn a_popup_goes_in_the_middle_of_the_screen() {
        // As lazygit 0.63.0 puts its welcome page in a terminal of 110 by 30.
        let screen = Rect::new(0, 0, 110, 30);
        assert_eq!(centered(screen, 80, 20), Rect::new(15, 5, 80, 20));
        assert_eq!(centered(screen, 80, 21), Rect::new(15, 4, 80, 21));
        // The odd cell goes to the right, the odd row under the popup.
        let screen = Rect::new(10, 20, 9, 5);
        assert_eq!(centered(screen, 4, 2), Rect::new(12, 21, 4, 2));
        assert_eq!(centered(screen, 4, 3), Rect::new(12, 20, 4, 3));
        // Larger than the screen: all of it.
        assert_eq!(centered(screen, 90, 90), screen);
        assert_eq!(centered(Rect::new(3, 4, 0, 0), 5, 5), Rect::new(3, 4, 0, 0));
        // A screen written out to reach past the last column ends there.
        let screen = Rect { x: u16::MAX, y: u16::MAX, width: 9, height: 9 };
        assert_eq!(centered(screen, 4, 4), Rect::new(u16::MAX, u16::MAX, 0, 0));
    }

    #[test]
    fn the_text_and_the_border_have_their_styles() {
        let mut buf = Buffer::empty(Rect::new(0, 0, 12, 3));
        let message = Message::new("abc").title("t").border_style(Style::new().fg(Color::Red));
        message.style(Style::new().add_modifier(Modifier::BOLD)).render(buf.area, &mut buf);
        assert_eq!(rows(&buf), ["   ╭─t─╮    ", "   │abc│    ", "   ╰───╯    "]);
        assert_eq!(
            (buf[(3, 0)].fg, buf[(5, 0)].fg, buf[(4, 1)].fg),
            (Color::Red, Color::Red, Color::Reset)
        );
        assert!(buf[(4, 1)].modifier.contains(Modifier::BOLD));
        assert!(!buf[(7, 1)].modifier.contains(Modifier::BOLD));
        assert!(!buf[(3, 1)].modifier.contains(Modifier::BOLD));
    }

    #[test]
    fn text_from_elsewhere_stays_in_the_window() {
        let mut buf = Buffer::empty(Rect::new(0, 0, 12, 6));
        let text = "a\u{1b}[2Jb\tc\nx\u{202e}y\u{200b}z 文件文件文";
        Message::new(text).title("t\u{7}i").width(10).render(buf.area, &mut buf);
        assert_eq!(
            rows(&buf),
            [
                " ╭─ti─────╮ ",
                " │a[2Jbc  │ ",
                " │xyz     │ ",
                " │文件文件│ ",
                " │文      │ ",
                " ╰────────╯ ",
            ]
        );
    }

    #[test]
    fn what_was_drawn_under_the_message_is_gone() {
        let mut buf = Buffer::empty(Rect::new(0, 0, 9, 3));
        for y in 0..3 {
            buf.set_string(0, y, "文件文件x", Style::new().fg(Color::Red));
        }
        // The window starts on the second half of a character on each of its rows.
        Message::new("a").render(buf.area, &mut buf);
        assert_eq!(rows(&buf), ["文 ╭─╮件x", "文 │a│件x", "文 ╰─╯件x"]);
        assert_eq!((buf[(4, 1)].fg, buf[(2, 1)].fg), (Color::Reset, Color::Red));
    }

    #[test]
    fn nothing_is_drawn_outside_the_buffer() {
        let blank = Buffer::empty(Rect::new(0, 0, 12, 4));
        let mut buf = blank.clone();
        let message = Message::new(WELCOME).title("Welcome");
        message.clone().render(Rect::new(0, 4, 12, 8), &mut buf);
        message.clone().render(Rect::new(12, 0, 30, 4), &mut buf);
        message.clone().render(Rect::new(3, 1, 0, 3), &mut buf);
        assert_eq!(buf, blank);
        // A screen that reaches past the buffer is the part inside it.
        message.scroll(1).render(Rect::new(0, 0, 90, 90), &mut buf);
        assert_eq!(rows(&buf), [" ╭────────╮ ", " │for     │ ", " │using   │ ", " ╰────────╯ "]);
    }
}
