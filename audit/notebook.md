# lazyui-ratatui Security Audit

## Environment / HPC Node Information

- Ubuntu 24.04, Linux 6.8.0, 31 GB RAM, 99 GB swap
- Rust 1.99.1 (stable), nightly-2026-10-28
- Kani 0.68.0 (CBMC 6.11.0), cargo-fuzz 0.13.2, cargo-deny 0.20.2, cargo-vet 0.10.3, cargo-geiger 0.14.0,
  cargo-llvm-cov 0.9.1, tmux 3.4
- Ratatui 0.30.2, ratatui-core 0.1.2; 84 packages in `Cargo.lock`, 52 in `fuzz/Cargo.lock`
- Every heavy task ran in a `systemd-run --user --scope` with `MemoryMax`, one at a time

All commands below were run from the repository root, on branch `security-audit`. The latest logs of each task are
in `audit/<task>/`.

## Task 1: Threat Model and Review

###### Motivation
- **Why**: The crate is published on crates.io with no security evidence. Most of what matters for a widget library
  is not memory safety but what untrusted text and odd sizes can do.
- **Source**: The user's `/rust-security-audit` request, 2026-11-02.

#### Findings Fixed

| Finding | Input | Found by | Fix |
|---|---|---|---|
| Cursor column past the field | `Field::view("⌚\u{fe0e}", 1, 2)` | fuzz, probe | `cells_before` sums whole graphemes |
| Text over a corner | 65,532+ cells in a window 65,535 wide | Kani | `corner_place` uses `checked_add(4)` |
| Cursor on the `<` marker | scrolled field | fuzz | as the first |
| Cursor moving left as it moves right | cursor inside a grapheme | review | as the first |
| Arrow on the corner of a clipped window | window cut off by the buffer | security review | `arrow_ends` clips to the screen |

The threat model, what the crate leaves to programs and the remaining risks are in `SECURITY.md`.

#### Verification
Passed: 2 of 2 mutants of the window oracle caught (arrow without clipping, corner off by one), and the unfixed
`view` bug caught by the strengthened `view` target. The old and new code were compared on about 1.1 billion
`view` and `render` calls, plus windows and arrows: 0 panics, and every difference was one of the fixes above
(cursor inside a grapheme, texts of 65,532 to 65,535 cells, arrows at clipped edges).

###### Note
The terminal width disagreement was reproduced on tmux 3.4: 92 bytes of text in a field 28 cells wide painted into
the next window, borders intact. It is not fixed: any Ratatui widget does the same.

## Task 2: Dependencies

###### Motivation
- **Why**: `forbid(unsafe_code)` does not reach dependencies, which are where this crate's `unsafe` code runs.
- **Source**: Step 3 of the audit.

```sh
cargo xtask deny
cargo xtask vet
cargo xtask geiger
```

Output (`audit/deny/run.log`, `audit/vet/run.log`):
```plaintext
advisories ok, bans ok, licenses ok, sources ok
advisories ok, sources ok
Vetting Succeeded (27 fully audited, 55 exempted)
```

###### Note
The one warning from `cargo deny` is two versions of `syn`, both only used at build time. Exemptions were not
certified in the user's name. `SUPPLY_CHAIN.md` has the `unsafe` counts and the code run at build time.

## Task 3: Fuzzing

###### Motivation
- **Why**: All external input of this crate is text, and Kani cannot reach it (Task 4.1).
- **Source**: Step 5 of the audit.

### 3.1 First Run

Run from a clean tree at `e9e8c25`:

```sh
cargo xtask all
```

| Target | Runs | Seconds |
|---|---|---|
| text | 34,709,580 | 3601 |
| view | 36,794,785 | 3601 |
| field | 17,373,328 | 3601 |
| window | 8,186,074 | 3601 |

No failure, but the commit's Kani run failed (Task 4.3), so this run was replaced by the next.

### 3.2 Second Run

Run at `8ed5335`:

```sh
cargo xtask all --if-needed
```

| Target | Runs | Seconds |
|---|---|---|
| text | 34,783,550 | 3601 |
| view | 35,431,682 | 3601 |
| field | 15,303,966 | 3601 |
| window | 7,741,869 | 3601 |

Total 93,261,067 runs, no failure, no artifact.

## Task 4: Kani

###### Motivation
- **Why**: Layout arithmetic near `u16::MAX` is where sampling is weakest, and it depends only on integers.
- **Source**: Step 6 of the audit.

### 4.1 [Failed] Proofs on Text

`Field::view` on real text goes through the tables of unicode-segmentation and unicode-width, which Kani cannot
get through.

<details>
<summary>Archived</summary>

A proof of `Field::view` on up to 3 ASCII characters ran out its 15 minutes at a peak of 7 GB. The exact command
was not kept. Text is left to fuzzing and property tests; the arithmetic was moved into functions of integers
(`scroll`, `scrolled_cursor`, `cursor_cell`, `corner_place`, `arrow_ends`).

</details>

### 4.2 Proofs of the Arithmetic

Seven proofs in `src/proofs.rs`, for every value of their integers, with no bound.

```sh
cargo xtask all --if-needed
```

Output (`audit/kani/harnesses.txt`, commit `8ed5335`, 9 s in all):
```plaintext
proofs::a_scrolled_cursor_keeps_off_the_markers: succeeded, 2 s, peak memory 21 MiB
proofs::a_scrolled_field_uses_its_cells_exactly: succeeded, 0 s, peak memory 22 MiB
proofs::a_text_on_the_border_leaves_the_corners_alone: succeeded, 0 s, peak memory 22 MiB
proofs::an_arrow_is_drawn_when_nothing_stops_it: succeeded, 1 s, peak memory 49 MiB
proofs::an_arrow_joins_the_borders_of_both_windows: succeeded, 1 s, peak memory 49 MiB
proofs::an_arrow_stays_on_the_screen: succeeded, 0 s, peak memory 27 MiB
proofs::the_cursor_goes_on_the_field: succeeded, 0 s, peak memory 21 MiB
```

#### Verification
Passed: both `kani::cover!` conditions are satisfied, so the arrow proofs are not vacuous, and none of the checks
reported `UNREACHABLE` is in this crate's code (2 are in `core::fmt`).

### 4.3 A Proof That Failed

`any_scroll` was widened so that the proofs would not depend on when `Field::view` scrolls, and the full audit was
started without running Kani on the change first. At `e9e8c25`:

```plaintext
Failed Checks: assertion failed: left <= column && column < w - right
 File: "src/proofs.rs", line 114, in proofs::a_scrolled_cursor_keeps_off_the_markers
VERIFICATION:- FAILED
```

`scrolled_cursor` added the marker's cell and then took off the skipped cells, so with `before == usize::MAX` the
addition saturated and the column came out as 0, on the marker. No text has that many cells, as a text's cells do
not exceed its bytes. The operations were swapped in `8ed5335`, which agrees with the old code for every `before`
that is reachable and needs no assumption in the proof, and a unit test was added.

## Task 5: Tests, Miri and Coverage

###### Motivation
- **Why**: Miri runs the `unsafe` code of the dependencies that the tests reach, and coverage shows which conditions
  no test tells apart.
- **Source**: Step 4 of the audit.

Miri at `8ed5335` (`audit/miri/run.log`):
```plaintext
test result: ok. 18 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 56.83s
```

### 5.1 Property Tests That Drew Nothing

Branch coverage fell from 93.55% to 90.32% between two runs, when only a test had been added. The text report
showed why:

```sh
cargo +nightly-2026-10-28 llvm-cov report --text
```

Output, for `if area.is_empty()` in `Field::render`, in the binary of `tests/no_panic.rs`:
```plaintext
|  Branch (51:12): [True: 257, False: 0]
```

All 257 cases of `fields_stay_in_their_first_row` returned early: the buffer and the area were picked apart, and
rarely meet. Arrows were never drawn at all (`window.rs:102`). `bb7fab1` starts areas near their buffer and windows
in it:

```plaintext
|  Branch (51:12): [True: 204, False: 53]    field drawn in 53 of 257 cases
|  Branch (102:9): [True: 11, False: 246]    arrow drawn in 11 of 257 cases
```

###### Note
The final figure is in `audit/coverage/summary.txt`. The 4 branches left unreached are in `field.rs:226` and
`field.rs:231`, a test's own assertions failing. `Field::style` and `Window::border_style` are the 2 functions no
test here calls; the second is run by a doctest, which coverage does not count.

###### Analysis from AI
The audit gives real evidence for the crate's own arithmetic and for what it does with text, within what was
sampled; it says little about Ratatui, which nobody has audited for this project.

- Firm: no `unsafe` code in the crate; the integer layout holds for all values; 93 million fuzz runs found no failure
  with oracles that were shown to catch real bugs.
- Not settled: 55 exempted crates; other terminals than tmux 3.4; the width disagreement, which remains open.
- Change first: decide whether the crate should offer a way to clean text before drawing, since every program that
  shows untrusted text has to solve this on its own now.

Two of this audit's mistakes were found only because a number moved: a proof changed and not rerun, and a property
test passing without drawing. A run that exercises nothing passes as quietly as one that holds.

---

*Timestamp: 2026-11-02*

---
