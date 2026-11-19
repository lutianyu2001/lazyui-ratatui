//! Windows and controls for Ratatui, based on lazygit's look.
//!
//! Not a copy of lazygit's interface: what a program needs beyond it is here too (arrows
//! between windows, drop-down lists, a name at either end of a border).
//!
//! - [`Window`]: rounded borders, the key that focuses the window and its name in the
//!   top left corner (`[1] Files`; [`Side::Right`] puts it in the right one), a note in the
//!   bottom right corner.
//! - [`arrow_down`]: joins a window to one below it.
//! - [`Field`]: a one-line text field that scrolls to keep its cursor visible and marks
//!   text it cuts off with `<` and `>`, as readline's horizontal-scroll-mode does.
//! - [`Choice`]: one of a few values between the arrows that change it (`◂ English ▸`),
//!   in the middle of a width that controls under one another share.
//! - [`Dropdown`]: a list of choices that opens at a control, the one in effect marked
//!   `(*)`, the cursor's row highlighted; joined to the control, as a combo box, when given
//!   a head.
//! - [`Prompt`]: a window with one line of text to edit in the middle of the screen, and
//!   under it, with the same edges, a window of suggestions; the one with the focus has its
//!   border in the focus style.
//! - [`Message`]: a window in the middle of the screen, as large as the text it says, which
//!   it wraps at spaces.
//! - [`text`]: the widths these measure text in, the way Ratatui draws it, and
//!   [`text::sanitize`], which they draw text through.
//!
//! # Examples
//!
//! The READMEs show these too, and their examples are compiled and run as doctests.
//!
//! ```rust
//! use lazyui_ratatui::{Field, Window};
//! use ratatui::Frame;
//! use ratatui::layout::Rect;
//! use ratatui::style::{Color, Style};
//!
//! /// A focused `[1] Search` window with a text field in it.
//! fn draw_search(frame: &mut Frame, area: Rect, text: &str, cursor: usize) {
//!     let window = Window::new().name("1", "Search").border_style(Style::new().fg(Color::Green));
//!     let inner = window.inner(area);
//!     frame.render_widget(window, area);
//!     let at = Field::new(text, cursor).render(inner, frame.buffer_mut());
//!     frame.set_cursor_position(at);
//! }
//! ```
//!
//! ```rust
//! use lazyui_ratatui::{Window, arrow_down};
//! use ratatui::buffer::Buffer;
//! use ratatui::layout::Rect;
//! use ratatui::style::Style;
//! use ratatui::widgets::Widget;
//!
//! let mut buf = Buffer::empty(Rect::new(0, 0, 20, 6));
//! let (upper, lower) = (Rect::new(0, 0, 20, 3), Rect::new(0, 3, 20, 3));
//! Window::new().name("1", "Address").render(upper, &mut buf);
//! Window::new().name("2", "Files").note("[2 marked]").render(lower, &mut buf);
//! arrow_down(&mut buf, 4, upper, lower, Style::new());
//! ```
//!
//! ```rust
//! use lazyui_ratatui::{Choice, Dropdown};
//! use ratatui::buffer::Buffer;
//! use ratatui::layout::Rect;
//! use ratatui::widgets::Widget;
//!
//! let screen = Rect::new(0, 0, 40, 10);
//! let mut buf = Buffer::empty(screen);
//! // A value between its arrows, in the middle of eight cells.
//! let control = Rect::new(4, 2, 30, 1);
//! let choice = Choice::new("English").note("en").width(8);
//! let (back, forward) = choice.arrows(control).expect("room for the arrows");
//! assert_eq!((back.x, forward.x), (4, 14));
//! choice.render(control, &mut buf);
//! // Opened, its list is joined to it, the list's arrow where the left one was.
//! let items = [("System", "auto"), ("English", "en")];
//! let list = Dropdown::new(&items).chosen(1).cursor(1).head("English", "en", control.y);
//! let area = list.area(control, screen);
//! assert_eq!((area.x + 3, area.y), (control.x, control.y));
//! list.render(area, &mut buf);
//! ```
//!
//! ```rust
//! use lazyui_ratatui::Dropdown;
//! use ratatui::buffer::Buffer;
//! use ratatui::layout::{Position, Rect};
//! use ratatui::widgets::Widget;
//!
//! let screen = Rect::new(0, 0, 40, 10);
//! let mut buf = Buffer::empty(screen);
//! let items = [("System", "auto"), ("English", "en")];
//! let list = Dropdown::new(&items).chosen(1).cursor(0);
//! // Under the control in row 2, or above it where the screen ends too soon.
//! let area = list.area(Rect::new(4, 2, 12, 1), screen);
//! assert_eq!(list.item_at(area, Position::new(8, 5)), Some(1));
//! list.render(area, &mut buf);
//! ```
//!
//! ```rust
//! use lazyui_ratatui::Prompt;
//! use ratatui::Frame;
//! use ratatui::layout::Position;
//!
//! /// A prompt for a branch's name, with the branches that match it under it.
//! fn draw_prompt(frame: &mut Frame, name: &str, cursor: usize, found: &[&str]) {
//!     let prompt = Prompt::new("New branch name", name, cursor)
//!         .suggestions("Suggestions (press <tab> to focus)", found)
//!         .selected(0);
//!     // The terminal's cursor, while the text has the focus.
//!     if let Some(at) = prompt.cursor_at(frame.area()) {
//!         frame.set_cursor_position(at);
//!     }
//!     // For a click at the screen's top left corner: no suggestion there.
//!     assert_eq!(prompt.suggestion_at(frame.area(), Position::new(0, 0)), None);
//!     frame.render_widget(prompt, frame.area());
//! }
//! ```
//!
//! ```rust
//! use lazyui_ratatui::Message;
//! use ratatui::buffer::Buffer;
//! use ratatui::layout::Rect;
//! use ratatui::widgets::Widget;
//!
//! let screen = Rect::new(0, 0, 40, 10);
//! let mut buf = Buffer::empty(screen);
//! let message = Message::new("Delete this branch?\nIt is not merged.").title("Confirm");
//! // As wide as its longest line and as high as its lines, with the border.
//! assert_eq!(message.area(screen), Rect::new(9, 3, 21, 4));
//! message.render(screen, &mut buf);
//! ```
//!
//! ```rust
//! use lazyui_ratatui::Field;
//!
//! // "0123456789" with the cursor at the end, in a field 6 cells wide.
//! let view = Field::view("0123456789", 10, 6);
//! assert!(view.left && !view.right);
//! assert_eq!((view.shown, view.cursor), ("6789", 5));
//! ```

#![forbid(unsafe_code)]

mod choice;
mod dropdown;
mod field;
mod message;
mod prompt;
#[cfg(kani)]
mod proofs;
pub mod text;
mod unsteady;
mod window;

pub use choice::Choice;
pub use dropdown::Dropdown;
pub use field::{Field, FieldView};
pub use message::Message;
pub use prompt::{Prompt, PromptAreas};
pub use window::{Side, Window, arrow_down};

/// The examples of README.md, as doctests.
#[cfg(doctest)]
#[doc = include_str!("../README.md")]
pub struct ReadmeDoctests;

/// The examples of README/README_CN.md, as doctests.
#[cfg(doctest)]
#[doc = include_str!("../README/README_CN.md")]
pub struct ReadmeCnDoctests;
