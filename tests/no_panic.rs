//! Random areas, buffers and text: nothing panics, and nothing is drawn outside the area
//! it was asked for. Coordinates include ones next to `u16::MAX`, where overflow would be.

// In a test an overflow panics, and so fails it, as it should.
#![allow(clippy::arithmetic_side_effects)]

use lazyui_ratatui::text::{sanitize, take_width, width};
use lazyui_ratatui::{Choice, Dropdown, Field, Message, Prompt, Side, Window, arrow_down};
use proptest::prelude::*;
use ratatui::buffer::Buffer;
use ratatui::layout::{Position, Rect};
use ratatui::style::Style;
use ratatui::widgets::Widget;

fn coord() -> impl Strategy<Value = u16> {
    prop_oneof![0..24u16, (u16::MAX - 24)..=u16::MAX]
}

/// Rects as `Rect::new` makes them, so ones near `u16::MAX` are clipped to fit.
fn rect() -> impl Strategy<Value = Rect> {
    (coord(), coord(), 0..24u16, 0..8u16).prop_map(|(x, y, w, h)| Rect::new(x, y, w, h))
}

/// A buffer small enough to compare cell by cell, somewhere on the screen.
fn buffer() -> impl Strategy<Value = Buffer> {
    (coord(), coord(), 0..24u16, 0..8u16)
        .prop_map(|(x, y, w, h)| Buffer::empty(Rect::new(x, y, w, h)))
}

/// A buffer and an area, mostly a few cells from it: picked apart, the two rarely meet, and
/// then nothing is drawn at all.
fn buffer_and_area() -> impl Strategy<Value = (Buffer, Rect)> {
    let near = (buffer(), 0..8u16, 0..4u16, 0..24u16, 0..24u16, 0..4u16, 0..4u16).prop_map(
        |(buf, x, y, w, h, left, up)| {
            let (at_x, at_y) = (
                buf.area.x.saturating_sub(left).saturating_add(x),
                buf.area.y.saturating_sub(up).saturating_add(y),
            );
            (buf, Rect::new(at_x, at_y, w, h))
        },
    );
    prop_oneof![1 => (buffer(), rect()), 3 => near]
}

/// A buffer, a column and two windows, mostly from the buffer's top left corner with one
/// over the other, so that an arrow gets drawn.
fn arrow() -> impl Strategy<Value = (Buffer, u16, Rect, Rect)> {
    let joined = (buffer(), 0..24u16, 0..24u16, 0..4u16, 0..3u16, 0..4u16).prop_map(
        |(buf, along, w, upper_height, gap, lower_height)| {
            let Position { x, y } = buf.area.as_position();
            let upper = Rect::new(x, y, w, upper_height);
            let lower = Rect::new(x, upper.bottom().saturating_add(gap), w, lower_height);
            (buf, x.saturating_add(along), upper, lower)
        },
    );
    prop_oneof![1 => (buffer(), coord(), rect(), rect()), 3 => joined]
}

/// Text mixing ASCII, wide characters, emoji, combining accents and control characters.
fn text() -> impl Strategy<Value = String> {
    let special = prop::sample::select(vec![
        '文',
        '件',
        '❤',
        '\u{fe0f}',
        '\u{301}',
        'ｶ',
        'ﾞ',
        '\t',
        '\u{1b}',
        '\0',
        '\u{231a}',
        '\u{fe0e}',
        '\u{1100}',
        '\u{202e}',
        '\u{2066}',
        '\u{200f}',
        '\u{9b}',
        '\u{200d}',
        '\u{1f3fb}',
        '\u{1f1e6}',
        '\u{1f469}',
    ]);
    let c = prop_oneof![3 => prop::char::range('a', 'z'), 2 => special, 1 => any::<char>()];
    prop::collection::vec(c, 0..24).prop_map(String::from_iter)
}

/// Cells holding what must not reach a terminal: C0 and C1 controls, and the controls that
/// reorder text.
fn unclean(buf: &Buffer) -> Vec<Position> {
    let bad = |c: char| {
        c.is_control()
            || matches!(c, '\u{61c}' | '\u{200e}' | '\u{200f}' | '\u{202a}'..='\u{202e}')
            || matches!(c, '\u{2066}'..='\u{2069}')
    };
    buf.area.positions().filter(|&p| buf[p].symbol().chars().any(bad)).collect()
}

/// Cells of `after` outside `kept` that differ from `before`.
fn changed_outside(
    before: &Buffer,
    after: &Buffer,
    kept: impl Fn(Position) -> bool,
) -> Vec<Position> {
    before.area.positions().filter(|&p| !kept(p) && before[p] != after[p]).collect()
}

proptest! {
    #[test]
    fn windows_stay_in_their_area(
        (mut buf, area) in buffer_and_area(),
        (key, name, note) in (text(), text(), text()),
        right in any::<bool>(),
    ) {
        let blank = buf.clone();
        let side = if right { Side::Right } else { Side::Left };
        Window::new().name(&key, &name).name_side(side).note(&note).render(area, &mut buf);
        prop_assert_eq!(changed_outside(&blank, &buf, |p| area.contains(p)), []);
        prop_assert_eq!(unclean(&buf), []);
    }

    #[test]
    fn arrows_stay_in_their_column((mut buf, x, upper, lower) in arrow()) {
        let blank = buf.clone();
        arrow_down(&mut buf, x, upper, lower, Style::new());
        let between = |p: Position| p.x == x && p.y >= upper.y && p.y <= lower.y;
        prop_assert_eq!(changed_outside(&blank, &buf, between), []);
    }

    #[test]
    fn choices_stay_in_their_first_row(
        (mut buf, area) in buffer_and_area(),
        (name, note) in (text(), text()),
        cells in prop::option::of(0..32usize),
    ) {
        let blank = buf.clone();
        let mut choice = Choice::new(&name).note(&note);
        if let Some(cells) = cells {
            choice = choice.width(cells);
        }
        choice.clone().render(area, &mut buf);
        let row = Rect { height: area.height.min(1), ..area };
        prop_assert_eq!(changed_outside(&blank, &buf, |p| row.contains(p)), []);
        prop_assert_eq!(unclean(&buf), []);
        // The arrows are in the part of the row on the screen.
        let drawn = row.intersection(buf.area);
        if drawn.is_empty() {
            prop_assert_eq!(&blank, &buf);
        }
        if let Some((back, forward)) = choice.arrows(drawn) {
            prop_assert_eq!(back.intersection(drawn), back);
            prop_assert_eq!(forward.intersection(drawn), forward);
        }
    }

    #[test]
    fn fields_stay_in_their_first_row((mut buf, area) in buffer_and_area(), text in text(), cursor in 0..32usize) {
        let blank = buf.clone();
        let at = Field::new(&text, cursor).render(area, &mut buf);
        let row = Rect { height: area.height.min(1), ..area };
        prop_assert_eq!(changed_outside(&blank, &buf, |p| row.contains(p)), []);
        let drawn = row.intersection(buf.area);
        if drawn.is_empty() {
            prop_assert_eq!(at, area.as_position());
        } else {
            prop_assert!(drawn.contains(at), "cursor {at:?} outside {drawn:?}");
        }
        prop_assert_eq!(unclean(&buf), []);
    }

    #[test]
    fn lists_stay_in_their_area_but_for_the_column_before_it(
        (mut buf, area) in buffer_and_area(),
        items in prop::collection::vec((text(), text()), 0..6),
        (chosen, cursor) in (0..8usize, 0..8usize),
        (anchor, at) in (rect(), (coord(), coord())),
        head in prop::option::of((text(), text(), coord())),
    ) {
        let items: Vec<(&str, &str)> = items.iter().map(|(a, b)| (a.as_str(), b.as_str())).collect();
        let mut list = Dropdown::new(&items).chosen(chosen).cursor(cursor);
        if let Some((name, note, row)) = &head {
            // On the list's first row as often as anywhere else.
            list = list.head(name, note, if row % 2 == 0 { area.y } else { *row });
        }
        let blank = buf.clone();
        list.clone().render(area, &mut buf);
        // Only a wide character cut by the list's left edge goes, in the column before it.
        let before = |p: Position| p.x.checked_add(1) == Some(area.x) && p.y >= area.y;
        prop_assert_eq!(changed_outside(&blank, &buf, |p| area.contains(p) || before(p)), []);
        prop_assert_eq!(unclean(&buf), []);
        // Where it goes for a control is on the screen, and a click finds an item it has.
        let placed = list.area(anchor, buf.area);
        prop_assert_eq!(placed.intersection(buf.area), placed);
        if let Some(item) = list.item_at(placed, Position::new(at.0, at.1)) {
            prop_assert!(item < items.len());
        }
    }

    #[test]
    fn prompts_stay_in_their_windows_but_for_the_column_before_them(
        (mut buf, screen) in buffer_and_area(),
        (title, input, cursor) in (text(), text(), 0..32usize),
        list in prop::option::of((text(), prop::collection::vec(text(), 0..6))),
        (selected, on_list) in (0..8usize, any::<bool>()),
        (wide, rows, gap) in (0..40u16, 0..6u16, 0..3u16),
        at in (coord(), coord()),
    ) {
        let list = list.as_ref().map(|(title, items)| {
            (title.as_str(), items.iter().map(String::as_str).collect::<Vec<&str>>())
        });
        let mut prompt = Prompt::new(&title, &input, cursor).selected(selected);
        prompt = prompt.focus_suggestions(on_list).width(wide).rows(rows).gap(gap);
        if let Some((title, items)) = &list {
            prompt = prompt.suggestions(title, items);
        }
        let blank = buf.clone();
        prompt.clone().render(screen, &mut buf);
        // The two windows share their edges, on the part of the screen inside the buffer.
        let drawn = screen.intersection(buf.area);
        let areas = prompt.areas(drawn);
        let (top, under) = (areas.input, areas.suggestions);
        prop_assert_eq!((top.x, top.width), (under.x, under.width));
        prop_assert!(under.y >= top.bottom());
        for window in [top, under] {
            prop_assert_eq!(window.intersection(drawn), window);
        }
        // Only a wide character cut by a window's left edge goes, in the column before it.
        let kept = |p: Position| [top, under].iter().any(|window| {
            let rows = window.y..window.bottom();
            window.contains(p) || (p.x.checked_add(1) == Some(window.x) && rows.contains(&p.y))
        });
        prop_assert_eq!(changed_outside(&blank, &buf, kept), []);
        prop_assert_eq!(unclean(&buf), []);
        // The cursor is in the text's window, a click finds a suggestion the prompt has.
        if let Some(cursor) = prompt.cursor_at(drawn) {
            prop_assert!(!on_list && areas.input.contains(cursor), "cursor {cursor:?}");
        }
        let items = list.as_ref().map_or(0, |(_, items)| items.len());
        let at = Position::new(at.0, at.1);
        if let Some(suggestion) = prompt.suggestion_at(drawn, at) {
            prop_assert!(suggestion < items && areas.suggestions.contains(at));
        }
    }

    #[test]
    fn messages_stay_in_their_window_but_for_the_column_before_it(
        (mut buf, screen) in buffer_and_area(),
        (title, lines) in (text(), prop::collection::vec(text(), 0..6)),
        (wide, scroll) in (0..40u16, 0..8usize),
    ) {
        let text = lines.join("\n");
        let message = Message::new(&text).title(&title).width(wide).scroll(scroll);
        let blank = buf.clone();
        message.clone().render(screen, &mut buf);
        let drawn = screen.intersection(buf.area);
        let area = message.area(drawn);
        prop_assert_eq!(area.intersection(drawn), area);
        prop_assert!(area.width <= wide);
        // Only a wide character cut by the window's left edge goes, in the column before it.
        let rows = area.y..area.bottom();
        let before = |p: Position| p.x.checked_add(1) == Some(area.x) && rows.contains(&p.y);
        prop_assert_eq!(changed_outside(&blank, &buf, |p| area.contains(p) || before(p)), []);
        prop_assert_eq!(unclean(&buf), []);
        // A window that holds all its text has nothing to scroll.
        let (lines, rows) = (text.lines().count(), usize::from(area.height.saturating_sub(2)));
        if area.width > 2 && message.overflow(drawn) == 0 {
            prop_assert!(lines <= rows, "{lines} lines in {rows} rows");
        }
    }

    #[test]
    fn views_fit_their_width(text in text(), cursor in 0..32usize, w in 0..32usize) {
        let v = Field::view(&text, cursor, w);
        let (left, right) = (usize::from(v.left), usize::from(v.right));
        prop_assert!(left + width(v.shown) + right <= w, "{v:?}");
        prop_assert!(text.contains(v.shown));
        if w > 0 {
            prop_assert!(v.cursor < w - right, "{v:?}");
        }
    }

    #[test]
    fn sanitized_text_takes_the_same_cells(text in text()) {
        let clean = sanitize(&text);
        prop_assert_eq!(width(&clean), width(&text));
        let again = sanitize(&clean);
        prop_assert_eq!(&again, &clean);
    }

    #[test]
    fn take_width_takes_a_prefix_that_fits(text in text(), w in 0..32usize) {
        let taken = take_width(&text, w);
        prop_assert!(text.starts_with(taken));
        prop_assert!(width(taken) <= w);
    }
}
