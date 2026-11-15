# Orchestration 6 State (`planning/STATE_6.md`)

**Goal:** G11 — R0 programs that R0 accepts match GNU R under the canonical comparison, or the difference is an allowlisted, documented restriction (`EXECUTION_PLAN_6.md`)
**Branch:** `main`

## Baseline (Pre-Wave 1)
- [x] `cargo test`: 123 lib, 80 conformance passed (0 failed) — HEAD `aa2f806`
- [x] `cargo clippy --all-targets -- -D warnings`: exit 0
- [x] `cargo build --target wasm32-unknown-unknown --release`: exit 0
- [x] `Rscript --version`: R 4.6.2 at `/usr/bin/Rscript`
- [ ] `cargo fmt --check`: already reports differences (out of scope; tracked as G16)
- [ ] `make test-gnu-r`: does not exist yet (task `6.2.3`)

## Wave Board

| Wave | Task ID | Title | Status | Commit |
| :--- | :--- | :--- | :--- | :--- |
| 1 | `6.1.1` | Canonical form on both sides, plus self-test | done | `37360fc` |
| 2 | `6.1.2` | Runner, allowlist, and skip rule | done | `6f8a7fb` |
| 3 | `6.2.1` | Corpus A: scalars, NA, broadcasting, indexing, intrinsics | pending | — |
| 3 | `6.2.2` | Corpus B: data frames, lists, loops | pending | — |
| 3 | `6.2.3` | Makefile target `test-gnu-r` | pending | — |
| 4 | `6.3.1` | Triage: allowlist or STOP; spec section | pending | — |

## Task Reports

## 6.1.1
- Status: done. Canonical form implemented on both sides; self-test passes with Rscript 4.6.2.
- Files: `tests/gnu_r.rs` (new, 1-264: `fmt_g15`, `canon_escape`, `canon_r0`, `canon_via_rscript`, `fixed_cases`, 2 tests); `tests/gnu_r/canon.R` (new, 1-42: `canon`, `canon_container`, `canon_elem`, `canon_escape`).
- Code commit: `37360fc` (`test(gnu-r): add canonical value form for R0 and GNU R with self-test`).
- `cargo test --test gnu_r canon_`: 2 passed, 0 failed (`canon_matches_on_fixed_values` runs 11 fixed cases against Rscript; `canon_fmt_g15_edge_cases`).
- `cargo clippy --all-targets -- -D warnings`: exit 0.
- Deviations: none. Format is as in the 6.1.1 Notes: atomic `<typeof>[<n>]: <elems>`, containers `list[<n>]` / `data.frame[<ncols>]` with `  "<name>"` headers and 4-space child indent; Df header has column count only.
- Gotchas for 6.1.2: `canon_r0` takes `&R0Val` and returns lines joined by `\n` with no trailing newline; the R side uses `writeLines(canon(v))`, so compare after trimming trailing `\n`. `canon_via_rscript` takes an R expression string, not a value; the runner in 6.1.2 needs to build its own script that sources `CANON_R` (also a const in gnu_r.rs). Helpers are private `fn`s; 6.1.2 will add `canon_r0` callers in the same file.
- Range is materialized as `start + i*step` using `R0Val::len`; `end` is ignored.

## 6.1.2
- Status: done. Runner, allowlist parsing, stale-entry checks, and Rscript skip rule implemented; corpus empty, run passes.
- Files: `tests/gnu_r.rs` (edit: imports 3-14 incl. `R0Vm` and `R0_DIR`; `load_allowlist` 270-299; `run_rscript_file` 301-323; `r0_corpus_matches_gnu_r` 325-383); `tests/r0/allowlist.txt` (new, 3 comment lines, no entries).
- Code commit: `6f8a7fb` (`test(gnu-r): run R0 corpus against GNU R with allowlist and stale-entry checks`).
- `cargo test --test gnu_r`: 3 passed, 0 failed (`canon_matches_on_fixed_values`, `canon_fmt_g15_edge_cases`, `r0_corpus_matches_gnu_r`); exit 0.
- `cargo clippy --all-targets -- -D warnings`: exit 0.
- Deviations: none. Answers applied: missing-.R0 allowlist entry fails with `(no such .R0 file)`; malformed line (no `|` or empty stem/reason) fails naming the line number; no temp-corpus self-test.
- Gotchas for 6.2.x: runner reads `tests/r0/*.R0` by extension `R0` (uppercase); allowlist stems are matched against file stems; `reject:` lines print to stdout. Mismatch and stale branches are untested until a `.R0` file exists (per answer 3, exercised in 6.2.1).
- Runner evaluates the file with `parse(file=)` in a fresh `new.env()`; an empty file yields `canon(NULL)`.
- Skip rule (Rscript absent) is code-path only; not exercised, since Rscript is installed.

## Discovered
- `canon_escape` relies on R `gsub(..., fixed = TRUE)` treating the replacement literally. Verified by the chr-escapes case; no issue seen.
- Lgl values other than 0/1/2 map to `NA` on the Rust side. Not reachable from R0 under normal construction.

## 6.2.1
- Status: done. 11 corpus programs added; all match GNU R and none are rejected by R0. No allowlist entries.
- Files: `tests/r0/num_*.R0` (new, 11): scalar_arith, na_arith, na_compare, kleene_and, kleene_or, kleene_not, broadcast_equal, index_mask, index_oob, reduce_na, which_round.
- Code commit: `af4d924` (`test(gnu-r): add numeric and vector corpus programs for R0`), corpus files only.
- `cargo test --test gnu_r`: 3 passed, 0 failed (`r0_corpus_matches_gnu_r` ok, no `reject:` lines, no mismatches); exit 0.
- Signature check (src/vm.rs 2124-2140): `pmin`/`pmax(x, y, na.rm)`, `round(x, digits)`, `atan2(y, x)`. `na.rm` accepted, so no rewrites were needed.
- Deviations: file count is 11, not 12. The plan's item list has 11 file items (2-12); I miscounted "12" in the plan. 11 is within the 8-12 spec range. Not padded to 12.
- Gotchas: the runner's corpus test is a single test, so a pass covers all files. Check the `--nocapture` output for `reject:` lines to confirm acceptance.
- `index_oob` uses `x[5]` (Dbl index) rather than `99L`, per spec.
## Discovered
- none
