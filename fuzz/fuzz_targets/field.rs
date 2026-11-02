//! `Field::render`: nothing panics, nothing is drawn off the area's first row, the cursor
//! stays on what was drawn, and no control character reaches the buffer.
#![no_main]

use lazyui_ratatui::Field;
use lazyui_ratatui_fuzz::{AnyBuffer, AnyRect, assert_clean, assert_kept, text};
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
    if drawn.is_empty() {
        assert_eq!(at, area.as_position());
    } else {
        assert!(drawn.contains(at), "cursor {at:?} outside {drawn:?}");
    }
    assert_clean(&buf);
});
