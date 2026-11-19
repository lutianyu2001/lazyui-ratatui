//! `Prompt`: nothing panics, nothing is drawn off its two windows but in the column before
//! them, where only a wide character goes, the windows share their left and right edges on
//! the screen and keep their corners and borders, the text is drawn as `Field::view` says
//! with the cursor there too, each row of suggestions holds the one a click on it finds,
//! highlighted only under the cursor of a focused list, the list says where its cursor is,
//! and no control character reaches the buffer.
#![no_main]

use lazyui_ratatui::text::{sanitize, take_width, width};
use lazyui_ratatui::{Field, Prompt, Window};
use lazyui_ratatui_fuzz::{
    AnyBuffer, AnyRect, assert_clean, assert_drawn, assert_kept, corners, text,
};
use libfuzzer_sys::fuzz_target;
use ratatui::buffer::Buffer;
use ratatui::layout::{Position, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::widgets::Widget;

type Input = (
    AnyBuffer,
    AnyRect,
    bool,
    [Vec<u16>; 3],
    u16,
    Option<Vec<Vec<u16>>>,
    (u8, bool),
    (u16, u8, u8),
    (u16, u16),
    Vec<u16>,
);

/// The corners of a window and the border beside each of its rows.
fn check_window(buf: &Buffer, window: Rect) {
    if window.width < 2 || window.height < 2 {
        return;
    }
    for (at, corner) in corners(window) {
        assert_eq!(buf[at].symbol(), corner, "{window:?}");
    }
    for y in window.y + 1..window.bottom() - 1 {
        assert_eq!(buf[(window.x, y)].symbol(), "│", "{window:?}");
        assert_eq!(buf[(window.right() - 1, y)].symbol(), "│", "{window:?}");
    }
}

fuzz_target!(|input: Input| {
    let (buffer, screen, far, [title, input, list_title], cursor, list, focus, size, at, under) =
        input;
    let mut buf = buffer.buffer();
    let screen = screen.near(far);
    let (title, input, list_title) = (text(&title), text(&input), text(&list_title));
    let cursor = usize::from(cursor) % (input.chars().count() + 2);
    let items: Option<Vec<String>> =
        list.map(|items| items.iter().take(24).map(|item| text(item)).collect());
    let items: Option<Vec<&str>> =
        items.as_ref().map(|items| items.iter().map(String::as_str).collect());
    let ((selected, on_list), (wide, rows, gap)) = (focus, size);
    let selected = usize::from(selected);
    let mut prompt = Prompt::new(&title, &input, cursor)
        .selected(selected)
        .focus_suggestions(on_list)
        .width(if far { wide } else { wide % 96 })
        .rows(u16::from(rows % 16))
        .gap(u16::from(gap % 4));
    if let Some(items) = &items {
        prompt = prompt.suggestions(&list_title, items);
    }

    // Over text already there, wide characters among it.
    for y in buf.area.top()..buf.area.bottom() {
        buf.set_stringn(buf.area.x, y, text(&under), usize::from(buf.area.width), Style::new());
    }
    let before = buf.clone();
    prompt.clone().render(screen, &mut buf);
    let drawn = screen.intersection(buf.area);
    let areas = prompt.areas(drawn);
    let (top, list) = (areas.input, areas.suggestions);

    // The same edges, the suggestions under the text, both on the screen.
    assert_eq!((top.x, top.width), (list.x, list.width), "{drawn:?}");
    assert!(list.y >= top.bottom(), "{top:?} {list:?}");
    assert!(items.is_some() || list.is_empty(), "{list:?}");
    for window in [top, list] {
        assert_eq!(window.intersection(drawn), window, "{window:?} on {drawn:?}");
        check_window(&buf, window);
    }
    assert_kept(&before, &buf, |p| {
        [top, list].iter().any(|window| {
            let cut =
                p.x.checked_add(1) == Some(window.x) && p.y >= window.y && p.y < window.bottom();
            window.contains(p) || (cut && width(before[p].symbol()) > 1 && buf[p].symbol() == " ")
        })
    });
    assert_clean(&buf);

    // The text, as a field draws it, and the cursor in it while it has the focus.
    let inner = Window::new().inner(top);
    let at_cursor = prompt.cursor_at(drawn);
    assert_eq!(at_cursor.is_some(), !on_list && !inner.is_empty(), "{inner:?}");
    if !inner.is_empty() {
        let view = Field::view(&input, cursor, usize::from(inner.width));
        let first = inner.x + u16::from(view.left);
        assert_drawn(&buf, first, inner.y, &sanitize(view.shown));
        if let Some(at_cursor) = at_cursor {
            assert_eq!(at_cursor, Position::new(inner.x + view.cursor as u16, inner.y));
            assert!(inner.contains(at_cursor), "cursor {at_cursor:?} outside {inner:?}");
        }
    }

    // Each row of suggestions holds the one a click on it finds.
    let items: &[&str] = items.as_deref().unwrap_or(&[]);
    let inner = Window::new().inner(list);
    let under_cursor = selected.min(items.len().saturating_sub(1));
    let mut shown = Vec::new();
    for y in inner.y..inner.bottom() {
        if inner.width == 0 {
            break;
        }
        let highlighted = buf[(inner.x, y)].modifier.contains(Modifier::REVERSED);
        let Some(item) = prompt.suggestion_at(drawn, Position::new(inner.x, y)) else {
            assert!(!highlighted, "row {y}");
            continue;
        };
        assert!(item < items.len(), "{item} of {}", items.len());
        let text = take_width(items[item], usize::from(inner.width));
        assert_drawn(&buf, inner.x, y, &sanitize(text));
        assert_eq!(highlighted, on_list && item == under_cursor, "row {y}");
        shown.push(item);
    }
    // One after the other, the one under the cursor among them, no row left empty for one.
    if !inner.is_empty() && !items.is_empty() {
        assert!(shown.windows(2).all(|pair| pair[0] + 1 == pair[1]), "{shown:?}");
        assert!(shown.contains(&under_cursor), "{under_cursor} not in {shown:?}");
        assert_eq!(shown.len(), items.len().min(usize::from(inner.height)));
    }
    // Where the cursor is, in the bottom right corner when the window is wide enough.
    let nth = if items.is_empty() { 0 } else { under_cursor + 1 };
    let note = format!("{nth} of {}", items.len());
    if list.height >= 2 && note.len() + 4 <= usize::from(list.width) {
        let x = list.right() - 2 - u16::try_from(note.len()).expect("fits in a window");
        assert_drawn(&buf, x, list.bottom() - 1, &note);
    }
    if let Some(item) = prompt.suggestion_at(drawn, Position::new(at.0, at.1)) {
        assert!(item < items.len() && inner.contains(Position::new(at.0, at.1)));
    }
});
