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
| 2 | `6.1.2` | Runner, allowlist, and skip rule | pending | — |
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

## Discovered
- `canon_escape` relies on R `gsub(..., fixed = TRUE)` treating the replacement literally. Verified by the chr-escapes case; no issue seen.
- Lgl values other than 0/1/2 map to `NA` on the Rust side. Not reachable from R0 under normal construction.
