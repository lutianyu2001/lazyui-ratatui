//! `Field::render`: nothing panics, nothing is drawn off the area's first row, what is drawn
//! is what `Field::view` says, with the cursor there too, and no control character reaches
//! the buffer.
#![no_main]

use lazyui_ratatui::Field;
use lazyui_ratatui::text::width;
use lazyui_ratatui_fuzz::{AnyBuffer, AnyRect, assert_clean, assert_drawn, assert_kept, text};
use libfuzzer_sys::fuzz_target;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};

fuzz_target!(|input: (AnyBuffer, AnyRect, bool, Vec<u16>, u16)| {
    let (buffer, area, far, units, cursor) = input;
    let mut buf = buffer.buffer();
    let area = area.near(far);
    let s = text(&units);
    let before = buf.clone();
    let style = Style::new().add_modifier(Modifier::UNDERLINED);
    let cursor = usize::from(cursor) % (s.chars().count() + 2);
    let at = Field::new(&s, cursor).style(style).render(area, &mut buf);

    let row = Rect { height: area.height.min(1), ..area };
    assert_kept(&before, &buf, |p| row.contains(p));
    let drawn = row.intersection(buf.area);
    assert_clean(&buf);
    if drawn.is_empty() {
        assert_eq!(at, area.as_position());
        return;
    }
    assert!(drawn.contains(at), "cursor {at:?} outside {drawn:?}");
    let view = Field::view(&s, cursor, usize::from(drawn.width));
    assert_eq!(usize::from(at.x - drawn.x), view.cursor, "{s:?}: {view:?}");

    let y = drawn.y;
    let marker = |x: u16, symbol: &str| {
        assert_eq!(buf[(x, y)].symbol(), symbol, "{s:?}: {view:?}");
        assert!(buf[(x, y)].modifier.contains(Modifier::REVERSED), "{s:?}: {view:?}");
    };
    if view.left {
        marker(drawn.x, "<");
    }
    if view.right {
        marker(drawn.right() - 1, ">");
    }
    let first = drawn.x + u16::from(view.left);
    assert_drawn(&buf, first, y, view.shown);
    // Past the text, and short of the `>`, the cells are as they were but for the style.
    let end = first + u16::try_from(width(view.shown)).expect("what fits in a row");
    for x in end..drawn.right() - u16::from(view.right) {
        assert_eq!(buf[(x, y)].symbol(), before[(x, y)].symbol(), "{s:?}: {view:?}");
    }
});
