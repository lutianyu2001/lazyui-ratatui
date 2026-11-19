//! `Choice`: nothing panics, nothing is drawn off the first row of the area, the arrows are
//! where `arrows` says, in the area, the value's cells between them, a value that fits is
//! there in the middle, a note is there whole or not at all, and no control character
//! reaches the buffer.
#![no_main]

use lazyui_ratatui::Choice;
use lazyui_ratatui::text::{sanitize, width};
use lazyui_ratatui_fuzz::{AnyBuffer, AnyRect, assert_clean, assert_drawn, assert_kept, text};
use libfuzzer_sys::fuzz_target;
use ratatui::layout::Rect;
use ratatui::widgets::Widget;

type Input = (AnyBuffer, AnyRect, bool, Vec<u16>, Vec<u16>, Option<u8>);

fuzz_target!(|input: Input| {
    let (buffer, area, far, name, note, cells) = input;
    let mut buf = buffer.buffer();
    let area = area.near(far);
    let (name, note) = (text(&name), text(&note));
    let mut choice = Choice::new(&name).note(&note);
    if let Some(cells) = cells {
        choice = choice.width(usize::from(cells));
    }

    let before = buf.clone();
    choice.clone().render(area, &mut buf);
    let row = Rect { height: area.height.min(1), ..area }.intersection(buf.area);
    assert_kept(&before, &buf, |p| row.contains(p));
    assert_clean(&buf);

    let Some((back, forward)) = choice.arrows(row) else {
        // No room for the arrows: nothing is drawn.
        assert_eq!(before, buf);
        return;
    };
    assert_eq!(back.intersection(row), back, "{back:?} in {row:?}");
    assert_eq!(forward.intersection(row), forward, "{forward:?} in {row:?}");
    assert_eq!(back.x, row.x);
    assert!(back.right() <= forward.x);
    assert_eq!(buf[(back.x, row.y)].symbol(), "◂");
    assert_eq!(buf[(forward.x + 1, row.y)].symbol(), "▸");
    // A value that fits between the arrows is there, as many cells before it as after it,
    // or one fewer.
    let (between, cells) = (usize::from(forward.x - back.right()), width(&name));
    if cells <= between {
        let lead = u16::try_from((between - cells) / 2).expect("fits in a row");
        assert_drawn(&buf, back.right() + lead, row.y, &sanitize(&name));
    }
    // The note two cells after the arrow, or the cells there as they were.
    let (end, cells) = (forward.right(), width(&note));
    if cells > 0 && usize::from(row.right() - end) >= cells + 2 {
        assert_drawn(&buf, end + 2, row.y, &sanitize(&note));
    } else {
        for x in end..row.right() {
            assert_eq!(buf[(x, row.y)], before[(x, row.y)], "{x}");
        }
    }
});
