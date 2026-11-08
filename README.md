# lazyui-ratatui

<p align="right">
    <strong>English</strong> | <a href="./README/README_CN.md">中文</a>
</p>

[![License](https://img.shields.io/badge/License-Apache%202.0-blue.svg)](https://opensource.org/licenses/Apache-2.0)
[![Release](https://img.shields.io/github/release/lutianyu2001/lazyui-ratatui)](https://github.com/lutianyu2001/lazyui-ratatui/releases)
[![Crates.io Version](https://img.shields.io/crates/v/lazyui-ratatui)](https://crates.io/crates/lazyui-ratatui)<br>
[![CI](https://github.com/lutianyu2001/lazyui-ratatui/actions/workflows/ci.yml/badge.svg)](https://github.com/lutianyu2001/lazyui-ratatui/actions/workflows/ci.yml)
[![audit](https://img.shields.io/endpoint?url=https%3A%2F%2Fraw.githubusercontent.com%2Flutianyu2001%2Flazyui-ratatui%2Fbadges%2Faudit.json)](SECURITY.md#ci)
[![pins](https://img.shields.io/endpoint?url=https%3A%2F%2Fraw.githubusercontent.com%2Flutianyu2001%2Flazyui-ratatui%2Fbadges%2Fpins.json)](xtask/src/pins.rs)

A small widget library that brings lazygit-style panels to [Ratatui](https://ratatui.rs):
rounded windows named with their focus key, corner notes, arrows between windows and
scrolling one-line text fields.

## Table of Contents

- [Overview](#overview)
- [Features](#features)
- [Security Audit](#security-audit)
- [Prerequisites](#prerequisites)
- [Installation](#installation)
- [Usage](#usage)
- [API](#api)
- [Testing](#testing)
- [Compatibility](#compatibility)
- [Disclaimer](#disclaimer)
- [License](#license)
- [Author](#author)

## Overview

The widgets cover the parts of a lazygit-style screen that Ratatui's own `Block` and
`Paragraph` leave to the program: window names tied to focus keys, notes on the
bottom border, arrows from one window to another and a text field that scrolls with
its cursor. They draw into a Ratatui `Buffer` like any other widget and depend only on
Ratatui and unicode-segmentation.

```text
╭────────[1] Files─╮   Window::new().name("1", "Files").note("[2 marked]")
│                  │
╰───────[2 marked]─╯

    ╰─┬────╯           arrow_down(buf, x, upper, lower, style)
╭─────▼────╮

<56789a>               Field::new(text, cursor).render(area, buf)
```

Note: "Lazy" in the name refers to lazygit, not lazy evaluation.

## Features

- Rounded windows with the focus key and name in the top right corner (`[1] Files`)
  and an optional note in the bottom right corner, both in the border's style
- Arrows that join a window to one below it (`┬`, `│`, `▼`)
- One-line text fields that scroll to keep the cursor visible and mark cut-off text
  with a highlighted `<` / `>`, as readline's horizontal-scroll-mode does
- Widths measured per grapheme, the way Ratatui draws text, so CJK text and single
  emoji line up; what terminals measure differently is replaced as it is drawn, so joined
  emoji and flags show plainer (see [SECURITY.md](SECURITY.md))
- Safe at any size: areas that are empty or reach outside the buffer are clipped
  instead of causing a panic

## Security Audit

[![deny](https://img.shields.io/endpoint?url=https%3A%2F%2Fraw.githubusercontent.com%2Flutianyu2001%2Flazyui-ratatui%2Fbadges%2Fdeny.json)](https://github.com/lutianyu2001/lazyui-ratatui/actions/workflows/advisories.yml)
[![vet](https://img.shields.io/endpoint?url=https%3A%2F%2Fraw.githubusercontent.com%2Flutianyu2001%2Flazyui-ratatui%2Fbadges%2Fvet.json)](audit/vet)
[![geiger](https://img.shields.io/endpoint?url=https%3A%2F%2Fraw.githubusercontent.com%2Flutianyu2001%2Flazyui-ratatui%2Fbadges%2Fgeiger.json)](audit/geiger)
[![miri](https://img.shields.io/endpoint?url=https%3A%2F%2Fraw.githubusercontent.com%2Flutianyu2001%2Flazyui-ratatui%2Fbadges%2Fmiri.json)](audit/miri)
[![coverage](https://img.shields.io/endpoint?url=https%3A%2F%2Fraw.githubusercontent.com%2Flutianyu2001%2Flazyui-ratatui%2Fbadges%2Fcoverage.json)](audit/coverage)
[![kani](https://img.shields.io/endpoint?url=https%3A%2F%2Fraw.githubusercontent.com%2Flutianyu2001%2Flazyui-ratatui%2Fbadges%2Fkani.json)](audit/kani)
[![fuzz](https://img.shields.io/endpoint?url=https%3A%2F%2Fraw.githubusercontent.com%2Flutianyu2001%2Flazyui-ratatui%2Fbadges%2Ffuzz.json)](audit/fuzz)

This is an audit by the project's maintainer, not by a third party, and it is no promise that the
crate has no bugs. It covers the versions in `Cargo.lock` at the commit named below.

The crate draws text and nothing else: it starts no process and opens no file or socket. So the
audit asks what someone who controls the text a program draws can do with it, and what happens at
sizes a terminal should never have.

| Audit | What was done | Result |
|---|---|---|
| Threat model | text from others: terminal escapes, text reordering, overrunning a window, very long text; sizes and cursors out of range | 7 problems found and fixed; what is left is below |
| `unsafe` code | forbidden in this crate | none |
| Dependencies | 40 crates at run and build time: sources, licenses and known vulnerabilities with cargo-deny, audit records with cargo-vet | no advisory; 27 of all 82 audited by others, 55 exempted by the maintainer |
| Miri | undefined behavior in 26 of the 27 unit tests of all four modules; the other, of very long text, takes hours and passed once | none found |
| Fuzzing | 4 targets, an hour each on a workstation and four hours each on GitHub Actions at each release: text widths, a field's layout, a field drawn into a buffer, windows and arrows drawn into a buffer, with what each must draw checked | 78 million runs in an hour each (2026-11-02), no failure |
| Kani proofs | 7 proofs of the layout arithmetic, for every value of the integers, with no bound | 7 of 7 pass |
| Branch coverage | tests, on all four modules | 86 of 90 branches reached; the other 4 are a test's own assertions failing |
| Review | the code, the threat model and the audit's own tooling reviewed apart from their writing, and the old and new code compared on the same inputs | findings fixed, or listed in SECURITY.md |

The proofs, all with no bound on their integers, as run on 2026-11-02 at commit `9470670` with Kani 0.68.0:

| Proof | What holds | Time (s) | Peak memory (MiB) |
|---|---|---|---|
| `a_text_on_the_border_leaves_the_corners_alone` | a name or note takes exactly its width, with a `─` and the corner left on each side | <1 | 22 |
| `an_arrow_stays_on_the_screen` | every row of an arrow is on the screen | <1 | 26 |
| `an_arrow_joins_the_borders_of_both_windows` | an arrow runs from the upper window's bottom border to the lower one's top border, off their corners | <1 | 46 |
| `an_arrow_is_drawn_when_nothing_stops_it` | with both borders on the screen, off their corners, the arrow is drawn | 1 | 43 |
| `the_cursor_goes_on_the_field` | the cursor is on the field, whatever its column | <1 | 22 |
| `a_scrolled_field_uses_its_cells_exactly` | markers and text add up to the field's width | <1 | 22 |
| `a_scrolled_cursor_keeps_off_the_markers` | a scrolled cursor is never on a marker, and what is under it fits before the right marker | 18 | 22 |

What this says, and what it doesn't:

- Rust rules out memory errors in this crate. It does not rule out mistakes in its logic, which is
  what the tests, the fuzzing and the proofs are for.
- `unsafe` code in the dependencies is out of this crate's hands: it is counted in
  [SUPPLY_CHAIN.md](SUPPLY_CHAIN.md) and rests on their authors and on audits on record. Ratatui
  itself is among the 55 crates exempted, so no audit of it is on record here.
- Fuzzing samples inputs, it proves nothing. The proofs cover integers only: text goes through
  Unicode tables that are out of Kani's reach, so nothing about text is proved.
- `Field` and `Window` replace the text that terminals measure differently from Ratatui, so that
  it cannot leave its window. This was measured on tmux 3.4 alone: of 400,000 random texts, 140,589
  disagreed before and none after. Other terminals, and text a program draws with other widgets
  without `text::sanitize`, are not covered, see [SECURITY.md](SECURITY.md#known-remaining-risks).
- `FieldView::shown` and `take_width` return text as it came, control characters included, and
  look-alike text is drawn as it is. [SECURITY.md](SECURITY.md#what-the-crate-leaves-to-the-program)
  has the whole list of what is left to the program.

The results of each check are committed with the code in `audit/<task>/`, and SECURITY.md says how
to run them. The badges above show whether each result still stands for the code on `main`
(green), no longer does (yellow) or failed (red); the audit badge under the title sums them up, and
the pins badge beside it shows whether a pinned tool or action is due for an update. GitHub Actions runs the checks at each release; Kani and Miri are among them while they are
short, as one change can make them take hours. An audit out of date holds no release back.

## Prerequisites

- Rust ≥1.88
- Ratatui 0.30

## Installation

### From crates.io (Recommended)

1. Add the crate to your project:

   ```bash
   cargo add lazyui-ratatui
   ```

   Or add it to `Cargo.toml` by hand:

   ```toml
   [dependencies]
   lazyui-ratatui = "0.2"
   ```

### Building from Source

1. Clone the repository and build it:

   ```bash
   git clone https://github.com/lutianyu2001/lazyui-ratatui.git
   cd lazyui-ratatui
   cargo build --locked
   ```

2. Depend on the local copy in your project's `Cargo.toml`:

   ```toml
   [dependencies]
   lazyui-ratatui = { path = "<path/to/lazyui-ratatui>" }
   ```

## Usage

A focused window with a text field in it:

```rust
use lazyui_ratatui::{Field, Window};
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};

/// A focused `[1] Search` window with a text field in it.
fn draw_search(frame: &mut Frame, area: Rect, text: &str, cursor: usize) {
    let window = Window::new().name("1", "Search").border_style(Style::new().fg(Color::Green));
    let inner = window.inner(area);
    frame.render_widget(window, area);
    let at = Field::new(text, cursor).render(inner, frame.buffer_mut());
    frame.set_cursor_position(at);
}
```

Two windows joined by an arrow:

```rust
use lazyui_ratatui::{Window, arrow_down};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::widgets::Widget;

let mut buf = Buffer::empty(Rect::new(0, 0, 20, 6));
let (upper, lower) = (Rect::new(0, 0, 20, 3), Rect::new(0, 3, 20, 3));
Window::new().name("1", "Address").render(upper, &mut buf);
Window::new().name("2", "Files").note("[2 marked]").render(lower, &mut buf);
arrow_down(&mut buf, 4, upper, lower, Style::new());
```

`Field::view` gives a field's layout without drawing it, for programs that draw text
their own way:

```rust
use lazyui_ratatui::Field;

// "0123456789" with the cursor at the end, in a field 6 cells wide.
let view = Field::view("0123456789", 10, 6);
assert!(view.left && !view.right);
assert_eq!((view.shown, view.cursor), ("6789", 5));
```

## API

| Item | Description |
|------|-------------|
| `Window` | Rounded window; `name(key, name)`, `note(text)`, `border_style(style)`, `inner(area)` |
| `arrow_down` | Draws `┬` / `│` / `▼` at column `x` from one window down to the next; does nothing if `x` is not on both borders |
| `Field` | One-line text field; `new(text, cursor)` takes the cursor as a char index, `render(area, buf)` returns where the terminal cursor goes |
| `FieldView` | What `Field::view(text, cursor, width)` returns: the visible text, both markers and the cursor's column |
| `text::width` | Cells a string takes in a `Buffer` |
| `text::take_width` | The longest prefix of a string that fits in a number of cells |
| `text::grapheme_width` | Cells one grapheme takes; 0 for control characters, which Ratatui skips |
| `text::sanitize` | Text as terminals can be trusted to draw it, in the same cells; for what other widgets draw |

Full documentation is on [docs.rs](https://docs.rs/lazyui-ratatui).

## Testing

```bash
cargo test
cargo clippy --all-targets
```

The examples in this README are compiled and run as doctests, as are those of the
crate documentation.

The crate forbids `unsafe` code, and Clippy rejects arithmetic that doesn't say how it
handles overflow. `tests/no_panic.rs` uses [proptest](https://crates.io/crates/proptest)
to draw random text into random areas and buffers, some next to `u16::MAX`, and checks
that nothing panics and nothing is drawn outside the area. Set `PROPTEST_CASES` to run
more cases than the default 256.

## Compatibility

Tested on:

- Ubuntu 24.04.4 LTS
  - Rust 1.99.1
  - [Ratatui](https://ratatui.rs) 0.30.2

## Disclaimer

This project is not affiliated with, endorsed by, or sponsored by lazygit or its maintainers. "lazygit" refers to the software developed by Jesse Duffield (https://github.com/jesseduffield/lazygit) and is referenced here solely for descriptive purposes.

This project is not affiliated with, endorsed by, or sponsored by Ratatui or its maintainers. "Ratatui" refers to the software developed by The Ratatui Developers (https://github.com/ratatui/ratatui) and is referenced here solely for descriptive purposes.

## License

Copyright 2026 Tianyu (Sky) Lu

The entire repository is licensed under the Apache License, Version 2.0 
(the "License"); you may not use any file in this repository except in 
compliance with the License. You may obtain a copy of the License at

```plaintext
http://www.apache.org/licenses/LICENSE-2.0
```

Unless required by applicable law or agreed to in writing, software
distributed under the License is distributed on an "AS IS" BASIS,
WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
See the License for the specific language governing permissions and
limitations under the License.

## Author

Tianyu (Sky) Lu
