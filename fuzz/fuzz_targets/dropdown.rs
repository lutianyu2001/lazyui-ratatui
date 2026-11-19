//! `Dropdown`: nothing panics, nothing is drawn off the area but in the column before it,
//! where only a wide character goes, the corners stay, each row shown starts with the mark of
//! its item, the list goes on the screen and off the rows of its control (with a head: from
//! the head's row or up to it), a click finds an item the list has, and no control character
//! reaches the buffer.
#![no_main]

use lazyui_ratatui::Dropdown;
use lazyui_ratatui::text::width;
use lazyui_ratatui_fuzz::{AnyBuffer, AnyRect, assert_clean, assert_kept, corners, text};
use libfuzzer_sys::fuzz_target;
use ratatui::layout::Position;
use ratatui::style::Style;
use ratatui::widgets::Widget;

type Head = Option<(Vec<u16>, Vec<u16>, u8)>;
type Input =
    (AnyBuffer, AnyRect, bool, Vec<(Vec<u16>, Vec<u16>)>, (u8, u8), AnyRect, (u16, u16), Vec<u16>, Head);

fuzz_target!(|input: Input| {
    let (buffer, area, far, items, (chosen, cursor), anchor, at, under, head) = input;
    let mut buf = buffer.buffer();
    let area = area.near(far);
    let items: Vec<(String, String)> =
        items.iter().take(24).map(|(name, note)| (text(name), text(note))).collect();
    let items: Vec<(&str, &str)> =
        items.iter().map(|(name, note)| (name.as_str(), note.as_str())).collect();
    let (chosen, cursor) = (usize::from(chosen), usize::from(cursor));
    let mut list = Dropdown::new(&items).chosen(chosen).cursor(cursor);
    // A head on a row near the area's first and last, or on neither.
    let head = head.map(|(name, note, row)| {
        let row = match row % 4 {
            0 => area.y,
            1 => area.bottom().saturating_sub(1),
            _ => area.y.saturating_add(u16::from(row)),
        };
        (text(&name), text(&note), row)
    });
    if let Some((name, note, row)) = &head {
        list = list.head(name, note, *row);
    }

    // Over text already there, wide characters among it.
    for y in buf.area.top()..buf.area.bottom() {
        buf.set_stringn(buf.area.x, y, text(&under), usize::from(buf.area.width), Style::new());
    }
    let before = buf.clone();
    list.clone().render(area, &mut buf);
    let drawn = area.intersection(buf.area);
    assert_kept(&before, &buf, |p| {
        let cut = p.x.checked_add(1) == Some(drawn.x) && p.y >= drawn.y && p.y < drawn.bottom();
        drawn.contains(p) || (cut && width(before[p].symbol()) > 1 && buf[p].symbol() == " ")
    });
    assert_clean(&buf);
    if drawn.width >= 2 && drawn.height >= 2 {
        for (at, corner) in corners(drawn) {
            assert_eq!(buf[at].symbol(), corner, "{drawn:?}");
        }
    }
    // Each row shown holds the item a click on it finds, marked as that item is.
    // In a list wide enough for the mark between the cells kept free beside the borders.
    if drawn.width >= 7 {
        for y in drawn.y..drawn.bottom() {
            let Some(item) = list.item_at(drawn, Position::new(drawn.x + 1, y)) else { continue };
            assert!(item < items.len(), "{item} of {}", items.len());
            let mark: String = (2..5).map(|x| buf[(drawn.x + x, y)].symbol()).collect();
            assert_eq!(mark, if item == chosen { "(*)" } else { "( )" }, "row {y}");
        }
    }

    // Where the list goes for a control: on the screen, on none of the control's rows, or
    // with a head from its row or up to it.
    let anchor = anchor.near(far);
    let placed = list.area(anchor, buf.area);
    assert_eq!(placed.intersection(buf.area), placed, "{anchor:?} on {:?}", buf.area);
    if !placed.is_empty() {
        match &head {
            Some((_, _, row)) => {
                assert!(placed.y == *row || placed.bottom() - 1 == *row, "{placed:?}: {row}");
            }
            None => {
                assert!(placed.y >= anchor.bottom() || placed.bottom() <= anchor.y, "{placed:?}");
            }
        }
    }
    if let Some(item) = list.item_at(placed, Position::new(at.0, at.1)) {
        assert!(item < items.len());
    }
});
