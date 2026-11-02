# Security

This file says what the security audit of this crate checks, how to run it and what it does not
cover. The README's Security Audit section gives the results. It is an audit by the project's
maintainer, not a third party's.

## Threat Model

The crate draws text into a Ratatui `Buffer`. It starts no process and opens no file or socket,
keeps no state between frames and has no `unsafe` code. So the attacker it has to be read against
is someone who controls **text a program draws with it**: a file name from a server, a key in
someone else's bucket, a message from a peer. Coordinates and widths come from the program and
are not trusted to be sensible either, since terminals come in any size.

| Attack surface | What holds | What covers it |
|---|---|---|
| Terminal escapes in drawn text | C0 and C1 control characters never reach a `Buffer` | fuzz (`text`, `field`, `window`), proptest |
| Text reordering ("Trojan Source") | U+061C, U+200E/F, U+202A–202E and U+2066–2069 never reach a `Buffer`: Ratatui leaves them out as zero-width graphemes | the same |
| Text overrunning its place | in the `Buffer`, a text never leaves its area, and a name or note never touches the corners; a terminal can still disagree, see [Known Remaining Risks](#known-remaining-risks) | fuzz, proptest, Kani (the arithmetic) |
| Sizes and coordinates | any area, empty, off the buffer or past `u16::MAX`, is clipped; nothing panics, overflows or draws outside it | Kani, fuzz; proptest only with rectangles from `Rect::new`; Clippy's `arithmetic_side_effects` |
| A cursor out of range | any `usize` cursor is accepted; the column stays in the field and off the markers, and never falls as the cursor moves right | Kani (the arithmetic), fuzz |
| Very long text | time is linear in the text: under 80 ms per MB, per field or window, per frame, in a release build | measured from 1 to 8 MB, with text of many shapes, including one 4 MB grapheme |

The items of a usual threat model that do not apply: secrets in logs, arguments or environment
(there is no logging, and no process), subprocesses, files and permissions, network, proxies and
redirects, host keys, fallbacks, retries and queues, names joined into paths, expansion of `${}`.

### What the Crate Leaves to the Program

- **`FieldView::shown` and `take_width` return the text as it came**, control characters included.
  Only a `Buffer` leaves them out, so a program that prints these itself has to filter them.
- **Look-alikes are not flagged.** Text that looks like other text is drawn as it is: homoglyphs,
  blank characters such as U+115F, invisible ones that stay in a cell with a base character (tag
  characters, zero-width joiners, variation selectors), U+2028 and U+2029, which take a cell, and
  right-to-left letters. Control characters are left out silently, so `ad\u{7}min` shows as
  `admin`. A name can draw what looks like a border and another window's key: only bold sets it
  apart from the real one. What to mark as suspicious depends on the program.
- **Names too wide are left out whole**, the focus key with them, as is a note.
- **`Window::inner` cannot see the buffer.** Of an area that reaches outside the buffer, give it
  the part inside, or what is drawn there goes over the border drawn at the buffer's edge.
- **A field's cursor can be off the buffer.** With nothing to draw on, `Field::render` returns the
  area's corner, which a program should not pass on unchecked.
- **What is already in the buffer stays.** A field clears no cells, and a wide character left of
  a window or a marker makes Ratatui's diff skip the cell after it.
- **A field has no password mode.** `Field` draws what it is given, and its `Debug` output prints
  it: a program with a secret to edit passes a masked copy.
- **Long text is drawn every frame.** Its cost is linear, not bounded: a program that shows
  names from elsewhere should cap their length.

### Known Remaining Risks

**A terminal can disagree with Ratatui on a text's width, and then text leaves its window on the
screen.** The `Buffer` is right and the terminal is not told otherwise: the backend sends a cursor
move only between cells that are not adjacent, so the shift carries along the row. On tmux 3.4,
with Ratatui 0.30.2:

| Text | Cells in Ratatui | Columns in tmux |
|---|---|---|
| U+231A then U+FE0E | 1 | 2 |
| a lone regional indicator | 1 | 2 |
| 12 emoji joined by U+200D | 2 | 8 |
| U+FFF9 | 1 | 0 |

So 92 bytes (13 times U+231A U+FE0E, then text) in a field 28 cells wide, as a name from a server
might be, paint their end into the window to its right, with both borders intact, and move the
terminal's cursor as well. Any Ratatui widget does the same, other terminals have not been tested,
and this crate draws no text of its own that could be measured apart from Ratatui's. A program that
shows untrusted text has to replace such sequences before drawing it.

**Ratatui's width of a grapheme wraps at 65,536 cells.** ratatui-core 0.1.2 casts it to `u16`
(`CellWidth for str`): 32,769 Hangul leading consonants (98 KB, one grapheme) report 2 cells, and
the terminal prints 65,538 columns, over every window on the screen. U+FF9E marks after one base
character wrap likewise (`count_halfwidth_sound_marks`). Capping the length of text avoids this.

## Audit

| Check | What it covers | Toolchain |
|---|---|---|
| `deny` | sources, licenses, duplicate versions and known vulnerabilities of the dependencies; the fuzz workspace's vulnerabilities and sources | stable |
| `vet` | every dependency has an audit on record, or an exemption | stable |
| `geiger` | how much `unsafe` code the dependencies have | stable |
| `miri` | undefined behavior in the unit tests of all modules (`field::`, `text::`, `window::`), which run Ratatui's and its dependencies' `unsafe` code | nightly |
| `coverage` | branches the tests do not reach | nightly |
| `kani` | the proofs in `src/proofs.rs`: the layout arithmetic, for every value of the integers | Kani's own |
| `fuzz` | the four targets below, an hour each | nightly |

Unit and property tests run with `cargo test`, and Clippy's pedantic lints with
`arithmetic_side_effects`, `unwrap_used` and `expect_used` are denied. The crate forbids `unsafe`
code, and the workspace's release profile turns overflow checks on (a library's users set their
own).

### Fuzz Targets

Text is built so that any character of the basic plane is as easy to reach as any other, along with
the emoji blocks and the characters that join into graphemes (`fuzz/src/lib.rs`).

| Target | Input | Properties checked |
|---|---|---|
| `text` | any text and width | `width` equals the cells Ratatui really takes (for texts under 4096 cells); `take_width` returns the longest prefix of whole graphemes that fits; no control character gets into a buffer |
| `view` | any text, cursor and width | what is shown fits with its markers and is a piece of the text cut between graphemes; a marker means something visible is cut off on that side, and from 3 cells on, whatever is cut off gets one; the cursor stays in the field, off the markers, at the start of its grapheme, among the text shown, in the column of the text before it |
| `field` | any text and cursor, any buffer and area | no panic; nothing is drawn off the area's first row; the cells hold what `Field::view` says, each grapheme in its place and the markers highlighted, and the rest of the row keeps its text; the cursor goes where `view` says; no control character gets into a buffer |
| `window` | any key, name and note, any buffers and areas, any arrow | no panic; nothing is drawn off the area, or off the arrow's column; `inner` stays in the area; the corners and the `─` beside them stay, and a text that fits is there in its place; an arrow is made of `┬`, `│` and `▼` and leaves the corners of both windows alone; no control character gets into a buffer |

Areas and buffers are written without `Rect::new`, so their right and bottom edges may lie past
`u16::MAX`, and some of each input keeps them next to it. Non-BMP text is limited to the emoji
blocks and the characters above. The wide windows of `corner_place` (next to 65,535 cells) are out
of the buffers' reach here: Kani and a unit test cover them.

### Kani Proofs

The seven proofs cover what depends only on integers, so they hold for **every** value of those
integers, with no bound on them: where a name goes on a border, the rows of an arrow, how a
scrolled field divides its cells and where its cursor goes. They do not cover text: `Field::view`
on real text goes through the tables of unicode-segmentation and unicode-width, and a proof on
just 3 ASCII characters ran out 15 minutes at 7 GB, so text is left to fuzzing and property tests.
That the crate's calls on a `Buffer` stay inside it is covered by fuzzing, not by the proofs.

### Running the Audit

The tools and the nightly toolchain are pinned in `xtask/src/pins.rs`, and nothing here is part of
`cargo build` or `cargo test`.

```bash
cargo xtask setup                 # install the pinned tools
cargo xtask status                # each task's latest result, and whether it still stands
cargo xtask all --if-needed       # run what no longer stands
cargo xtask <task>                # deny, vet, geiger, miri, coverage, kani [proof...], fuzz [seconds]
cargo xtask outdated              # whether a pinned version is due for an update
```

`--no-record` runs on any working tree and leaves the results in `target/audit/<task>/`: that is
how to run some proofs (`kani <proof...>`) or fuzz for less than an hour. `--if-needed` skips
a task whose result stands.

### Results

The latest run of each task is committed with the code in `audit/<task>/`: its log, its time, tools
and arguments (`run.txt`), the files it read (`inputs.txt`) and what it produced. When the latest
run failed, the last one that succeeded is kept in `audit/<task>/last_succeeded/`.

A result stands while it succeeded, the tools and arguments are the same and the files the task
reads have not changed: Rust files compare by their tokens, so comments and layout do not count,
and doc comments do; other files compare as they are. `deny` also lapses after a day, as the
advisory database changes daily. Runs that record need a clean working tree, as a result stands
for a commit.

### CI

`.github/workflows/verify.yml` runs after each push to `main`, runs only the tasks whose result no
longer stands, one job each, and its last job commits the results to `audit/`. That job is the only
one that may write, and it runs no project code. The rest have `contents: read`.

GitHub Actions is off on this repository for now. Until it is turned on, neither workflow runs, and
the results come from runs on the maintainer's machine.

`.github/workflows/advisories.yml` runs daily, since a new advisory needs no change to the
repository: it runs `cargo xtask deny --no-record` and checks the pinned tools for known
vulnerabilities, and weekly whether they are due for an update.

Because CI pushes to `main`, run `git pull --rebase` before pushing, and do not amend what has
been pushed. Branch protection that requires pull requests or checks would stop that push.
