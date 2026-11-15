# Execution Plan 6 (`planning/EXECUTION_PLAN_6.md`)

## Goal & Scope

**Goal (G11):** Every R0 program in the new `tests/r0/` corpus that R0 accepts produces the same canonical result under GNU R (`Rscript`), or its difference is listed as an explicit R0 restriction. This is the acceptance test for the one-way guarantee: R0 is a strict subset that GNU R runs. It is also the gate the later `.R0 -> .R` transpiler (G12) must pass.

**In scope**
- A harness, `tests/gnu_r.rs` plus `tests/gnu_r/canon.R`, that evaluates each `tests/r0/*.R0` program with R0 and with `Rscript`, serializes both results to one canonical text form, and diffs them.
- A corpus of `.R0` programs in `tests/r0/`, each valid GNU R syntax, covering the R0 surface that exists today (arithmetic, NA handling, broadcasting, indexing, data frames, lists, loops with `break`/`next`, intrinsics).
- An allowlist `tests/r0/allowlist.txt` for known differences, each with a reason.
- A Makefile target `test-gnu-r`.
- A section in `docs/LANGUAGE_SPEC.md` listing R0 restrictions that come from GNU R equivalence.

**Out of scope**
- The `.R0 -> .R` transpiler (G12) and any syntax R0 uses that GNU R lacks (`fn`, `->`, type hints). Corpus programs must not use them.
- Recycling of any kind (no mode, no support). Short-mask and length-mismatch behaviour stays strict.
- Programs R0 rejects. Only accepted programs are compared.
- Changes to `src/**`. If the harness finds a source defect, the worker STOPs and reports it; it is not fixed here.
- Migrating the 528 Rust assertions in `tests/conformance.rs` into `.R0` files.

**Verbatim constraints (from the user)**
- "I think we need to provide some sort of support for recycling ... it may be we need strictness modes -- my main concern is shortening the pathway to being able to run canonical GNU R code in R0" (superseded by the next quote).
- "I agree it is one-way, im not sure about building in support for recycling, it is a simple enough to be explicit. i like the idea of R0 being runnable in GNU R but not necessarily vice versa."
- "we can provide some sort of transpiler that can go from .R0 to .R files to strip specific things like fn or -> or type hints."
- "agrees yes" (approval to write GOALS.md and hand G11 to plan-wave).

## Decision Log
- **2026-11-14 (direction):** R0 is a strict subset that GNU R runs. No recycling mode. Canonical GNU R running unchanged in R0 is not a goal.
- **2026-11-14 (compare):** Results are compared as a canonical dump, not as printed output. The same text form is produced by a Rust function on the R0 side and an R function on the GNU R side (`tests/gnu_r/canon.R`).
- **2026-11-14 (programs):** A new corpus `tests/r0/*.R0` holds one program per file. The existing 528 Rust assertions in `tests/conformance.rs` are not migrated.
- **2026-11-14 (rejects):** Programs that R0 rejects (trap) are excluded. GNU R is not asked to reject them.
- **2026-11-14 (mismatch):** Any mismatch fails the harness unless listed in `tests/r0/allowlist.txt` with a reason. An allowlisted case that no longer differs also fails, so stale entries get removed.
- **2026-11-14 (no-R):** `cargo test --test gnu_r` skips with a printed notice when `Rscript` is not on `PATH`. `make test-gnu-r` requires `Rscript` and fails without it.
- **2026-11-14 (program result):** Each program's result is the value of its last top-level expression. R0 uses `eval_script`. GNU R parses the file and evaluates the expressions in order in one environment, and the value of the last one is the result.
- **2026-11-14 (tracking):** Planning artifacts are tracked; `docs(planning):` commits stay separate from code.
- **2026-11-14 (models):** Workers run on Haiku, one wave at a time, no worktrees (waves are serial). Escalation ladder applies (see wave-orchestration).

## Recon / Hazards

- `tests/conformance.rs` (2,014 lines) compares `format!("{:?}")` of `R0Val`. The harness must not reuse that comparison.
- `src/value.rs` `R0Val` variants: `Dbl`, `Int`, `Lgl`, `Chr`, `Range { start, end, step }`, `Df { names, cols }`, `List { names, cols }`, `Null`. `Range` must be materialized and tagged `integer` to match R's `1:n`.
- R's `for` returns `NULL` invisibly. The Decision Log says an R0 loop returns the last body value (or `Null`). This is a likely divergence. Do not write corpus programs that depend on a loop's value; if one does, record it as a finding in 6.3.1.
- R distinguishes `NA` from `NaN` in doubles. The canonical form must keep both (`NA`, `NaN`) and handle `Inf`, `-Inf`, and `-0`.
- Doubles: the canonical form uses 15 significant digits (`%.15g` semantics). Rust has no `%g`; the worker implements it and tests it on both sides with the same values.
- `Rscript` is at `/usr/bin/Rscript` (R 4.6.2). `Rscript` must be run with `-e`-free file arguments so it does not depend on shell quoting.
- `src/main.rs` already prints `R0Val` via `Display`; the harness must not depend on it.
- Baseline (pre-wave-1, HEAD `aa2f806`): `cargo test` 123 lib + 80 conformance pass; `cargo clippy --all-targets -- -D warnings` exits 0; `cargo build --target wasm32-unknown-unknown --release` exits 0.
- `cargo fmt --check` already reports differences (not in scope; see G16).

## Canonical form (owned by task 6.1.1)

One line per value: `<typeof>[<len>]: <elements>`, with elements space-separated.
- `double`: `%.15g`, `NA`, `NaN`, `Inf`, `-Inf`, `-0` (kept as `-0`).
- `integer`: decimal, `NA` for `NA_INT`.
- `logical`: `TRUE`, `FALSE`, `NA`.
- `character`: double-quoted with `\\`, `\"`, `\n` escaped; `NA` unquoted.
- `NULL`: `NULL`.
- `list`: `list[<n>]` followed by one indented line per element, prefixed by its name (or `""`).
- `data.frame`: same as list, with the `data.frame` tag and column names.

Both sides must produce identical strings for the same value; 6.1.1's self-test enforces this.

## Wave Table

| Wave | Task | Title | Files | Depends on |
| :--- | :--- | :--- | :--- | :--- |
| 1 | `6.1.1` | Canonical form on both sides, plus self-test | `tests/gnu_r.rs` (create), `tests/gnu_r/canon.R` (create) | — |
| 2 | `6.1.2` | Runner, allowlist, and skip rule | `tests/gnu_r.rs` (same file as 6.1.1, so serial) | `6.1.1` |
| 3 | `6.2.1` | Corpus A: scalars, NA, broadcasting, vector indexing, intrinsics | `tests/r0/num_*.R0` (create) | `6.1.2` |
| 3 | `6.2.2` | Corpus B: data frames, lists, loops, break/next | `tests/r0/df_*.R0`, `tests/r0/ctl_*.R0` (create) | `6.1.2` |
| 3 | `6.2.3` | Makefile target `test-gnu-r` | `Makefile` | `6.1.2` |
| 4 | `6.3.1` | Triage: allowlist each mismatch with a reason, or STOP on a src defect; add spec section | `tests/r0/allowlist.txt` (create), `docs/LANGUAGE_SPEC.md` | `6.2.1`, `6.2.2`, `6.2.3` |

Waves are run serially (Haiku workers, no worktrees). Tasks within a wave touch disjoint files, so they could be parallel later if needed.

## Task Specs

### Task `6.1.1`: Canonical form on both sides
- **Repo:** `/home/andrew/AI/rzero`
- **Files in scope:** `tests/gnu_r.rs` (create), `tests/gnu_r/canon.R` (create)
- **Depends on:** —
- **Do not touch:** `src/**`, `tests/conformance.rs`, `Makefile`, `docs/**`, `tests/r0/**`
- **Grep anchors:** `use r0_wasm::` lines at top of `tests/conformance.rs`; `pub fn eval_script` in `src/vm.rs`; `pub enum R0Val` in `src/value.rs`; `NA_INT`, `is_na_real` in `src/value.rs`
- **Spec:**
  1. Create `tests/gnu_r/canon.R` defining `canon(x)` returning the canonical form (see "Canonical form") as a character vector of lines. It must handle NA vs NaN, `Inf`, `-0`, `%.15g` doubles, character escaping, and recursion over lists and data frames.
  2. In `tests/gnu_r.rs`, implement `canon_r0(&R0Val) -> String` with the same output. `Range` is materialized and tagged `integer`. Implement `%.15g` by hand (Rust has no `%g`).
  3. Self-test (`#[test] fn canon_matches_on_fixed_values`): build about 10 fixed values (Dbl with NA, NaN, Inf, -0, 0.1+0.2; Int with NA; Lgl with NA; Chr with a quote and newline; Null; a named List; a Df). For each, compare `canon_r0` with the R-side `canon` by writing a small R script to a unique temp file that sources `tests/gnu_r/canon.R`, builds the same value, and prints `canon(value)`. Skip with a notice if `Rscript` is not on `PATH`.
- **Constraints:** `Rscript` is invoked with a file argument, never with `-e` quoting. Temporary files use `std::env::temp_dir()` with a unique name. No new dependencies in `Cargo.toml`.
- **Acceptance:** `cargo test --test gnu_r canon_` exits 0 with Rscript installed, and `cargo clippy --all-targets -- -D warnings` exits 0.

### Task `6.1.2`: Runner, allowlist, and skip rule
- **Repo:** `/home/andrew/AI/rzero`
- **Files in scope:** `tests/gnu_r.rs` (edit), `tests/r0/allowlist.txt` (create, comments only)
- **Depends on:** `6.1.1`
- **Do not touch:** `src/**`, `tests/gnu_r/canon.R` (unless a helper is missing; STOP and report), `Makefile`, `docs/**`, `tests/r0/*.R0`
- **Grep anchors:** `fn canon_r0` and `fn canon_matches_on_fixed_values` in `tests/gnu_r.rs`; `R0Vm::new` in `tests/conformance.rs`
- **Spec:**
  1. Runner: for each `tests/r0/*.R0` (sorted), evaluate with `R0Vm::new(1_000_000).eval_script(src)`. If R0 returns `Err`, print `reject: <stem>` and continue (rejects are out of scope). Otherwise run `Rscript` on a generated runner script that sources `tests/gnu_r/canon.R`, evaluates each top-level expression of the file in one environment, and prints `canon(value of the last expression)`. Compare the two strings; a mismatch fails the run unless allowlisted.
  2. Allowlist: parse `tests/r0/allowlist.txt` (one entry per line, `<file-stem> | <reason>`, `#` comments). An allowlisted file that still matches fails the run with `stale allowlist entry: <stem>`.
  3. Skip rule: if `Rscript` is not on `PATH`, print `gnu_r: Rscript not found, skipping` and return without failing.
  4. The run passes with zero `.R0` files present.
- **Constraints:** `Rscript` is invoked with a file argument, never with `-e` quoting. Temporary files use `std::env::temp_dir()` with unique names. No new dependencies.
- **Acceptance:** `cargo test --test gnu_r` exits 0 with Rscript installed and the corpus empty, and `cargo clippy --all-targets -- -D warnings` exits 0.

### Task `6.2.1`: Corpus A (numeric and vector)
- **Repo:** `/home/andrew/AI/rzero`
- **Files in scope:** `tests/r0/num_*.R0` (create, 8–12 files)
- **Depends on:** `6.1.2`
- **Do not touch:** `src/**`, `tests/gnu_r.rs`, `tests/gnu_r/canon.R`, `tests/r0/allowlist.txt`, `docs/**`, `Makefile`, other `tests/r0/*` files
- **Grep anchors:** `fn broadcast_scalar_vector` and `fn index_positive` and `fn index_oob_na_padding` in `tests/conformance.rs` (use as source of valid programs); the intrinsic list in `docs/LANGUAGE_SPEC.md` §7.1
- **Spec:**
  1. Write programs, one per file, each a short script whose final expression is the result. Cover: scalar arithmetic with Int and Dbl; NA propagation in arithmetic and comparison; Kleene `&`, `|`, `!` on logical vectors; scalar-vector broadcasting (`1 + c(10, 20)`); equal-length vector arithmetic; positive, negative, and logical-mask indexing; out-of-range Dbl index padding with NA; `sum`, `mean`, `min`, `max`, `which`, `seq_along`, `pmin`, `pmax` with `na.rm`; `round` ties; `atan2` with equal lengths.
  2. Every program must be valid GNU R syntax. Use only `<-` assignment and common functions. No `fn`, no `->`, no type hints.
  3. Do not write any program whose result depends on a loop's value, or on integer overflow (both are reserved for triage in 6.3.1 if they appear elsewhere).
  4. Run `cargo test --test gnu_r`. Every new file must match, or the worker STOPs and reports the mismatch (it must not be allowlisted here).
- **Acceptance:** `cargo test --test gnu_r` exits 0 with the new files present; each new file is accepted by R0 (not a reject).

### Task `6.2.2`: Corpus B (data frames, lists, loops)
- **Repo:** `/home/andrew/AI/rzero`
- **Files in scope:** `tests/r0/df_*.R0`, `tests/r0/ctl_*.R0` (create, 8–12 files total)
- **Depends on:** `6.1.2`
- **Do not touch:** `src/**`, `tests/gnu_r.rs`, `tests/gnu_r/canon.R`, `tests/r0/allowlist.txt`, `docs/**`, `Makefile`, `tests/r0/num_*.R0`
- **Grep anchors:** `fn null_df_and_list_access`, `fn bracket_df_errors`, `fn bracket_assign_df_cells` in `src/vm.rs` tests (valid programs); `conformance_g9_extract_index_kinds` and `conformance_g9_assign_2d` in `tests/conformance.rs`
- **Spec:**
  1. Data frames: construction with `data.frame(...)`, 2D extraction `d[i, j]` (positive, negative, logical, name, empty slot), drop rule (`d[, "a"]` bare vector; `d[, c("a","b")]` data frame; `d[1, ]` data frame), `d[j]` column selection, 2D and 1-index assignment with recycling of a scalar.
  2. Lists: `lst[j]` on a named list, `lst$name`, `lst[["name"]]` only if valid in R0 (check the parser first; omit if R0 rejects it).
  3. Loops: `for` over `1:n` with `break` and `next`, nested loops (`break` exits only the inner loop), `while` with `break`. Assert results through the final expression only (do not use a loop's value as the result).
  4. Every program must be valid GNU R syntax. No `fn`, no `->`, no type hints.
  5. Run `cargo test --test gnu_r`. A mismatch STOPs the task and is reported; it is not allowlisted here.
- **Acceptance:** `cargo test --test gnu_r` exits 0 with the new files present; each new file is accepted by R0.

### Task `6.2.3`: Makefile target
- **Repo:** `/home/andrew/AI/rzero`
- **Files in scope:** `Makefile`
- **Depends on:** `6.1.2`
- **Do not touch:** everything else
- **Grep anchors:** `test-conformance:` target and its `.PHONY` entry in `Makefile`; `CARGO` variable
- **Spec:**
  1. Add `test-gnu-r` (help text: `Run R0 vs GNU R equivalence harness (requires Rscript)`). It first checks `command -v Rscript` and exits non-zero with a clear message if missing, then runs `$(CARGO) test --test gnu_r`.
  2. Add it to `.PHONY` if the Makefile has one for other targets.
- **Acceptance:** `make test-gnu-r` exits 0 with Rscript installed; `make help` lists the target.

### Task `6.3.1`: Triage and spec
- **Repo:** `/home/andrew/AI/rzero`
- **Files in scope:** `tests/r0/allowlist.txt` (create), `docs/LANGUAGE_SPEC.md`
- **Depends on:** `6.2.1`, `6.2.2`, `6.2.3`
- **Do not touch:** `src/**`, `tests/gnu_r.rs`, `tests/gnu_r/canon.R`, corpus files, `Makefile`
- **Grep anchors:** `## ` headings in `docs/LANGUAGE_SPEC.md` (find the section on loops and the section on subsetting); the existing "Unimplemented" or "not supported" wording
- **Spec:**
  1. Run `cargo test --test gnu_r`. If it passes with no mismatches, create `tests/r0/allowlist.txt` containing only comments explaining that no differences are currently known, and skip step 2.
  2. For each mismatch: if the difference is a GNU R-visible semantic that R0 intentionally differs on (for example a loop's returned value), add an allowlist entry `<stem> | <reason>` and add a matching restriction sentence to `docs/LANGUAGE_SPEC.md` in a new section "GNU R equivalence". If the difference is a defect in `src/**`, STOP and report it with the program, R0 output, and GNU R output; do not allowlist it and do not fix it.
  3. Add the section "GNU R equivalence" to `docs/LANGUAGE_SPEC.md` describing: the one-way guarantee, the canonical comparison, programs R0 rejects are out of scope, the allowlist, and the restriction list.
- **Acceptance:** `make test-gnu-r` exits 0; `cargo test` exits 0; the spec section exists and lists every allowlist entry.

## Gate for the whole orchestration (run by the orchestrator, not a worker)
- `cargo test` exit 0
- `cargo clippy --all-targets -- -D warnings` exit 0
- `cargo build --target wasm32-unknown-unknown --release` exit 0 (no `src/` changes expected)
- `make test-gnu-r` exit 0
