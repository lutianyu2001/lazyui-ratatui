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
| Text overrunning its place | in the `Buffer`, a text never leaves its area, and a name or note never touches the corners | fuzz, proptest, Kani (the arithmetic) |
| A terminal measuring text otherwise than Ratatui | what `Field` and `Window` draw goes through `text::sanitize`, which replaces every grapheme whose width was seen to differ, cell for cell; other terminals are not covered, see [Known Remaining Risks](#known-remaining-risks) | measured on tmux 3.4: all 1,112,064 characters, and 400,000 random texts, of which 140,589 disagreed before and none after; fuzz (`text`) |
| Sizes and coordinates | any area, empty, off the buffer or past `u16::MAX`, is clipped; nothing panics, overflows or draws outside it | Kani, fuzz; proptest only with rectangles from `Rect::new`; Clippy's `arithmetic_side_effects` |
| A cursor out of range | any `usize` cursor is accepted; the column stays in the field and off the markers, never falls as the cursor moves right, and what is under it is shown if the field can hold it | Kani (the arithmetic), fuzz |
| Very long text | time is linear in the text: under 80 ms per MB, per field or window, per frame, in a release build; `text::sanitize` under 80 ms per MB | measured from 1 to 8 MB, with text of many shapes, including one 4 MB grapheme |

The items of a usual threat model that do not apply: secrets in logs, arguments or environment
(there is no logging, and no process), subprocesses, files and permissions, network, proxies and
redirects, host keys, fallbacks, retries and queues, names joined into paths, expansion of `${}`.

### What the Crate Leaves to the Program

- **`FieldView::shown` and `take_width` return the text as it came**, control characters included.
  A program that draws these itself, or draws text from elsewhere with another widget on the same
  screen, passes it through `text::sanitize` first.
- **Look-alikes are not flagged.** Text that looks like other text is drawn as it is: homoglyphs,
  blank characters such as U+115F, invisible ones that stay in a cell with a base character (tag
  characters), U+2028 and U+2029, which take a cell, and
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

**Emoji are drawn plainer than a terminal could show them.** `sanitize` trades them for a width
terminals agree on: joined emoji show as their first emoji, an emoji with a skin tone without it,
a flag as `U+FFFD` twice, and a heart with U+FE0F as a plain heart and a space. The text itself,
`FieldView::shown` and the cursor do not change.

**Only tmux 3.4 was measured.** A terminal with older Unicode tables can disagree on characters
that tmux knows, and one set to give East Asian ambiguous characters two cells disagrees on the
border itself. `src/unsteady.rs` lists what disagreed on tmux 3.4, and has to be measured again
with a new Ratatui, whose tables may change. What a program draws with other widgets, or prints
itself, is not covered unless it goes through `text::sanitize`.

## Audit

| Check | What it covers | Toolchain |
|---|---|---|
| `deny` | sources, licenses, duplicate versions and known vulnerabilities of the dependencies; the fuzz workspace's vulnerabilities and sources | stable |
| `vet` | every dependency has an audit on record, or an exemption | stable |
| `geiger` | how much `unsafe` code the dependencies have | stable |
| `miri` | undefined behavior in the unit tests of all modules (`field::`, `text::`, `unsteady::`, `window::`), which run Ratatui's and its dependencies' `unsafe` code; one test of very long text is left out, as it takes hours under Miri | nightly |
| `coverage` | branches the tests do not reach | nightly |
| `kani` | the proofs in `src/proofs.rs`: the layout arithmetic, for every value of the integers | Kani's own |
| `fuzz` | the four targets below, an hour each on a workstation, four hours each on GitHub Actions | nightly |

Unit and property tests run with `cargo test`, and Clippy's pedantic lints with
`arithmetic_side_effects`, `unwrap_used` and `expect_used` are denied. The crate forbids `unsafe`
code, and the workspace's release profile turns overflow checks on (a library's users set their
own).

### Fuzz Targets

Text is built so that any character of the basic plane is as easy to reach as any other, along with
the emoji blocks and the characters that join into graphemes (`fuzz/src/lib.rs`).

| Target | Input | Properties checked |
|---|---|---|
| `text` | any text and width | `width` equals the cells Ratatui really takes (for texts under 4096 cells); `take_width` returns the longest prefix of whole graphemes that fits; `sanitize` keeps the width, leaves nothing that takes no cell, and has nothing left to do on what it returns; no control character gets into a buffer |
| `view` | any text, cursor and width | what is shown fits with its markers and is a piece of the text cut between graphemes; a marker means something visible is cut off on that side, and with room for what is under the cursor and both markers, whatever is cut off gets one; the cursor stays in the field, off the markers, at the start of its grapheme, among the text shown, in the column of the text before it, and what is under it is shown if the field can hold it |
| `field` | any text and cursor, any buffer and area | no panic; nothing is drawn off the area's first row; the cells hold what `Field::view` says, sanitized, each grapheme in its place and the markers highlighted, and the rest of the row keeps its text; the cursor goes where `view` says; no control character gets into a buffer |
| `window` | any key, name and note, any buffers and areas, any arrow | no panic; nothing is drawn off the area, or off the arrow's column; `inner` stays in the area; the corners and the `─` beside them stay, and a text that fits is there in its place, sanitized; an arrow is made of `┬`, `│` and `▼` and leaves the corners of both windows alone; no control character gets into a buffer |

Areas and buffers are written without `Rect::new`, so their right and bottom edges may lie past
`u16::MAX`, and some of each input keeps them next to it. Non-BMP text is limited to the emoji
blocks and the characters above. The wide windows of `corner_place` (next to 65,535 cells) are out
of the buffers' reach here: Kani and a unit test cover them.

### Kani Proofs

The proofs cover what depends only on integers, so they hold for **every** value of those
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
how to run some proofs (`kani <proof...>`) or fuzz for less than an hour. A fuzz run longer than
an hour is recorded, and stands for the task. `--if-needed` skips a task whose result stands.

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

The maintainer's decision is to run on GitHub Actions every task whose length is known before it
starts, to make full use of it, and to leave the others to a person:

- `.github/workflows/verify.yml` runs by itself when a release is published on GitHub: `deny`,
  `vet`, `geiger`, `coverage`, and `fuzz` for four hours a target, all four at once on one
  runner, which stops on time whatever it finds.
- `kani` and `miri` never start by themselves: one new proof or test can take them from seconds
  to hours (one test took Miri from a minute to three hours), and a run left to time out at each
  release would waste the runners. They are tried on a workstation first, with `--no-record`, and
  then run there, or on GitHub by starting the workflow by hand with the tasks `kani miri`.
- Nothing runs on a push: the audit takes hours, and no push needs it.
- A release does not wait for the audit, and a result out of date holds no release back. The
  badges at the top of the README show which results stand.

Each task runs only when its result no longer stands, one job each, and the last job commits the
results to `audit/` on `main`, as results for the release's commit. Results committed since that
commit are newer and stay. That job is the only one that may write, it runs no project code, and
it restores no cache. The rest have `contents: read`, and their checkouts keep no token. Fuzzing
goes on from the corpus of the last run on GitHub, kept for 90 days as an artifact.

`.github/workflows/advisories.yml` runs daily, since a new advisory needs no change to the
repository: it runs `cargo xtask deny --no-record` and checks the pinned tools and actions for
known vulnerabilities, and weekly whether they are due for an update. It also writes the badges:
whether each task's latest result stands for the code on `main`, how its own `deny` check went, and
whether a pinned version is due. It does so daily, after each run of `verify.yml`, and when started
by hand. A job that cannot write produces them, and a job that runs no project code lets through
only the files and words a badge can have, and pushes them to the `badges` branch, which holds
nothing else and is replaced each time.

Actions are pinned to commits, with the release in a comment, so that a tag moved upstream cannot
change what runs with a token. `cargo xtask outdated` checks them like the pinned tools: a release
a month newer, a known vulnerability, an action not pinned, or a tag that no longer names the
commit pinned.

GitHub Actions is off on this repository for now. Until it is turned on, neither workflow runs, the
badges have nothing to show, and the results come from runs on the maintainer's machine.

Because `verify.yml` pushes to `main`, run `git pull --rebase` before pushing, and do not amend what
has been pushed. The repository has no branch rules, on purpose.
