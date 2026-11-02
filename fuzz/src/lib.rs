//! What the targets share: how bytes become text and rectangles, and what must hold of any
//! buffer that untrusted text was drawn into.

use arbitrary::Arbitrary;
use ratatui::buffer::Buffer;
use ratatui::layout::{Position, Rect};

/// Characters that take part in grapheme clusters or in what a width depends on, which plain
/// mutations would seldom put together.
const SPECIAL: [char; 26] = [
    '\u{200d}',  // zero width joiner
    '\u{200c}',  // zero width non-joiner
    '\u{fe0e}',  // text presentation
    '\u{fe0f}',  // emoji presentation
    '\u{301}',   // combining accent
    '\u{3099}',  // combining dakuten
    '\u{ff9e}',  // halfwidth dakuten
    '\u{ff9f}',  // halfwidth handakuten
    '\u{1f3fb}', // skin tone
    '\u{1f1e6}', // regional indicators
    '\u{1f1e7}',
    '\u{1100}', // Hangul jamo: leading, vowel, trailing
    '\u{1161}',
    '\u{11a8}',
    '\u{600}', // prepended
    '\u{93f}', // spacing mark
    '\u{94d}', // virama, which joins conjuncts
    '\u{915}',
    '\u{231a}',  // watch: emoji presentation by default
    '\u{2764}',  // heavy black heart: text presentation by default
    '\u{1f469}', // woman
    '\u{e0061}', // tag
    '\u{ad}',    // soft hyphen
    '\u{2028}',  // line separator
    '\u{202e}',  // right-to-left override
    '\u{1b}',    // escape
];

/// Text in which any character of the basic plane is as easy to reach as any other. Where
/// the surrogates would be: `SPECIAL`, and the emoji blocks.
pub fn text(units: &[u16]) -> String {
    units
        .iter()
        .map(|&u| {
            char::from_u32(u32::from(u)).unwrap_or_else(|| {
                let low = usize::from(u & 0x3ff);
                if u < 0xdc00 {
                    SPECIAL[low % SPECIAL.len()]
                } else {
                    char::from_u32(0x1f300 + u32::from(u & 0x3ff)).unwrap_or('?')
                }
            })
        })
        .collect()
}

/// A rectangle as a program may write one: without `Rect::new`, which clips it to fit, so
/// that its right or bottom edge may lie past `u16::MAX`.
#[derive(Arbitrary, Debug, Clone, Copy)]
pub struct AnyRect {
    x: u16,
    y: u16,
    w: u16,
    h: u16,
}

impl AnyRect {
    pub fn rect(self) -> Rect {
        Rect { x: self.x, y: self.y, width: self.w, height: self.h }
    }

    /// Near the buffers' coordinates, as most rectangles are, unless `far`.
    pub fn near(self, far: bool) -> Rect {
        if far {
            return self.rect();
        }
        Rect { x: self.x % 64, y: self.y % 16, width: self.w % 64, height: self.h % 16 }
    }
}

/// A buffer small enough to go through cell by cell: at `x`, `y` anywhere on the screen, as a
/// rectangle written out, so that its edges may lie past `u16::MAX` too.
#[derive(Arbitrary, Debug, Clone, Copy)]
pub struct AnyBuffer {
    x: u16,
    y: u16,
    w: u8,
    h: u8,
    far: bool,
}

impl AnyBuffer {
    pub fn buffer(self) -> Buffer {
        let (width, height) = (u16::from(self.w % 64), u16::from(self.h % 16));
        if self.far {
            return Buffer::empty(Rect { x: self.x, y: self.y, width, height });
        }
        Buffer::empty(Rect { x: self.x % 8, y: self.y % 8, width, height })
    }
}

/// What must not reach a terminal, whatever was drawn: C0 and C1 controls, which ratatui
/// leaves out, and the controls that reorder text (the "Trojan Source" ones). Written out
/// here rather than asked of `char::is_control`, which covers only the first.
pub fn reaches_no_terminal(c: char) -> bool {
    matches!(
        u32::from(c),
        0..=0x1f | 0x7f..=0x9f | 0x61c | 0x200e | 0x200f | 0x202a..=0x202e | 0x2066..=0x2069
    )
}

/// No cell of `buf` holds what must not reach a terminal.
pub fn assert_clean(buf: &Buffer) {
    for p in buf.area.positions() {
        let symbol = buf[p].symbol();
        assert!(!symbol.chars().any(reaches_no_terminal), "{p:?}: {symbol:?}");
    }
}

/// Every cell of `after` not `kept` is as it was in `before`.
pub fn assert_kept(before: &Buffer, after: &Buffer, kept: impl Fn(Position) -> bool) {
    for p in before.area.positions() {
        assert!(kept(p) || before[p] == after[p], "{p:?} changed");
    }
}
