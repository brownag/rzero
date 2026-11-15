# EXECUTION_PLAN_9 -- Close G11 corpus coverage

Status: approved by the user 2026-11-15 (plan-wave). Execute with wave-orchestration.

## Goal & Scope

**Goal.** Close the circled goal G11 in `planning/GOALS.md`: the `tests/r0/` corpus covers every GNU-compatible R0 program in `tests/conformance.rs`, and each corpus program matches GNU R or its difference is an explicit R0 restriction.

**In scope.**
- GNU-valid pedon corpus file (`PEDON_SCRIPT`, `tests/conformance.rs:212`).
- Errata audit B1–B15 (`docs/LANGUAGE_SPEC.md` §9), with coverage added where GNU R agrees.
- R0 restrictions recorded in `docs/LANGUAGE_SPEC.md` "GNU R equivalence" section.
- Orchestration-8 exclusions recorded (as restrictions, or linked to G20/G21).
- Correction of the 8.4.3 expectation in `planning/EXECUTION_PLAN_8.md`.

**Out of scope.**
- Any change under `src/` (G20, G21, G14 are separate goals).
- Edits to `tests/r0/allowlist.txt` (it stays empty).
- Pushing or any remote action.
- G12 transpiler, G13, G15, G16.
- Rewriting the history in `EXECUTION_PLAN_8.md` beyond the 8.4.3 correction.

**Verbatim constraints.**
- "Do not change src/ (G20, G21, G14 are separate goals). Nothing is pushed."
- "Exit: cargo test --test gnu_r and make test-gnu-r exit 0."
- G11 exit criteria: a difference is allowed only if it is "listed as an explicit R0 restriction".

## Decision log

- 2026-11-15: Planning artifacts are **tracked**. Plan-file commits are `docs(planning):`, separate from code.
- 2026-11-15: R0 restrictions are recorded in the `docs/LANGUAGE_SPEC.md` "GNU R equivalence" section (line 328) (currently "empty; see the errata").
- 2026-11-15: `B2` (`1.5L`, `1e3L`) is a restriction, not a corpus file. Rscript 4.6.2 warns "integer literal 1.5L contains decimal; using numeric value" and returns `1.5`. R0 rejects with `ParseError`, which is out of harness scope, so it is recorded in the "GNU R equivalence" section.
- 2026-11-15: The pedon twin uses plain `function(...)`, drops type hints and `->`, and keeps `|>`. R0 accepts `function` and `fn` (`src/lexer.rs:239`), and it already runs the `|>` form, so the twin is a normal corpus file that both R0 and GNU run.
- 2026-11-15: The 8.4.3 correction is an annotated note in `EXECUTION_PLAN_8.md`. The historical plan is not rewritten.
- 2026-11-15: A corpus file that R0 accepts but that disagrees with GNU R is **not** fixed in this orchestration. The worker stops and reports, and the difference becomes a restriction or a new goal.
- 2026-11-15: The errata ledger has no B7 entry (B1–B6 and B8–B15 are present). This is reported, not invented.

## Recon facts

- `tests/gnu_r.rs` discovers corpus files with `read_dir(tests/r0)`. New files need no registry edit, so there is no hidden coupling between tasks that create corpus files.
- Corpus state: 82 `.R0` files plus `allowlist.txt` (empty). Existing errata-named files: `spec_errata_b1`, `b3`, `b4`, `b5`, `b6`.
- Errata with no errata-named file: B2, B8, B9, B10, B11, B12, B13, B14, B15. Some may already be covered by `g*` files, so the worker checks the assert content, not the filename.
- `PEDON_SCRIPT` is at `tests/conformance.rs:212`. Its R0-only parts are `hz: df`, `z_min: dbl`, `z_max: dbl`, and `-> dbl`.
- Rscript 4.6.2 is on PATH. The green baseline is recorded in `STATE_9.md`.

## Waves

| Wave | Task | Files (must edit / can create) | Depends on |
| --- | --- | --- | --- |
| 1 | 9.1.1 Pedon GNU-valid twin | create `tests/r0/g11_pedon_twin.R0` | none |
| 1 | 9.1.2 Errata audit and coverage | create `tests/r0/spec_errata_bN.R0` for each uncovered B8–B15 that GNU agrees with; the audit table lives in the worker report | none |
| 1 | 9.1.3 Plan-doc corrections | edit `planning/EXECUTION_PLAN_8.md` (8.4.3 note), `planning/tasks/g11-conformance-classification.md` (exclusion links to G20/G21) | none |
| 2 | 9.2.1 Restrictions in the "GNU R equivalence" section | edit `docs/LANGUAGE_SPEC.md` "GNU R equivalence" section only | 9.1.2 |
| Gate | orchestrator (no worker) | runs full suites; updates `GOALS.md` and `STATE_9.md` | 9.1.1, 9.1.2, 9.1.3, 9.2.1 |

Wave 1 file scopes are pairwise disjoint: `tests/r0/g11_pedon_twin.R0`, `tests/r0/spec_errata_bN.R0`, and two planning files. Wave 2 touches only the spec.

---

## Task 9.1.1 -- Pedon GNU-valid twin

- **Repo:** rzero
- **Files in scope:** must edit: none. Can create: `tests/r0/g11_pedon_twin.R0`.
- **Depends on:** none.
- **Spec:** Copy the R0 script in `tests/conformance.rs` `PEDON_SCRIPT` (line 212) into `tests/r0/g11_pedon_twin.R0`. Change `depth_weighted_mean <- function(hz: df, z_min: dbl = 0, z_max: dbl = 100) -> dbl {` to `depth_weighted_mean <- function(hz, z_min = 0, z_max = 100) {`. Keep the body, the `pedon` data frame, and the final `list(...)`. The final expression is the `list(wt_clay_0_100cm, interp_25_50_85, remaining_oob)`.
- **Anchors:** `tests/conformance.rs:212` (`PEDON_SCRIPT`); `tests/conformance.rs` `pedon_depth_weighted_clay_0_100`, `pedon_approx_25_50_85`, `pedon_remaining_oob`, `pedon_result_is_list` (the Rust values to match).
- **Constraints:** do not touch `tests/conformance.rs`, `src/`, or any other file. Do not edit `allowlist.txt`.
- **Acceptance:** `cargo test --test gnu_r` passes. `make test-gnu-r` passes. R0 must accept the file; if R0 rejects it, stop and report. Do not fix `src/`.
- **Reference:** an existing corpus file with a `list(...)` final expression, for example `tests/r0/g8_extensions.R0`.

## Task 9.1.2 -- Errata audit and coverage

- **Repo:** rzero
- **Files in scope:** must edit: none. Can create: `tests/r0/spec_errata_b8.R0` through `spec_errata_b15.R0`, only for ids that have no existing coverage and where GNU R agrees. Audit table goes in the worker report.
- **Depends on:** none.
- **Spec:** For each of B1–B15 in `docs/LANGUAGE_SPEC.md` §9:
  1. Find existing corpus or Rust coverage by reading assert contents, not just filenames.
  2. Record the result in the audit table as `covered: <file or test>`, `needs-file`, or `restriction`.
  3. For `needs-file` where GNU R agrees, create one `spec_errata_bN.R0` with a `list(...)` of value asserts that exercise the errata behaviour.
  4. Mark `restriction` where GNU R differs or errors. B2 is a restriction. Do not create files for restrictions.
  5. B7 has no ledger entry. Report it as "no B7 in ledger"; do not invent a B7.
- **Anchors:** `docs/LANGUAGE_SPEC.md` §9 (lines ~338–352); `tests/conformance.rs` (grep `B8`–`B15`, `na_rm`, `median`, `atan2`, `pmin`, `pmax`, `list`); `tests/r0/`.
- **Constraints:** do not edit `docs/LANGUAGE_SPEC.md` (that is 9.2.1). Do not edit `allowlist.txt`. Do not touch `src/`.
- **Acceptance:** `cargo test --test gnu_r` and `make test-gnu-r` pass. Report includes the audit table (B1–B15, one row each).
- **Reference:** `tests/r0/spec_errata_b5.R0`, `tests/r0/spec_errata_b6.R0`.

## Task 9.1.3 -- Plan-doc corrections

- **Repo:** rzero
- **Files in scope:** must edit: `planning/EXECUTION_PLAN_8.md`, `planning/tasks/g11-conformance-classification.md`. Can create: none.
- **Depends on:** none.
- **Spec:**
  1. In `EXECUTION_PLAN_8.md`, at the 8.4.3 spec (anchor: the line `| 4 | 8.4.3 |` and the paragraph starting "**8.4.3 (`conformance_g9_assign_1index`)**", around line 111), add a correction note under the existing text. The corpus value for `l["b"]` after `l["b"] <- c(1, 2, 3)` is `1`, per the 7.5.1 list `[<-` change. Do not rewrite the original text.
  2. In `EXECUTION_PLAN_8.md`, add a short "Orchestration 8 exclusions" note listing the seven exclusions, each marked as a restriction (spec "GNU R equivalence" section) or linked to G20 or G21.
  3. In `g11-conformance-classification.md`, the section "New R0 gap to track" (around line 110) and the §D known gap: link `atan2(NaN, NA_real_)` to G20, and link the integer-type items (`d[c(1,2)] <- 1:3`, `diff(1:5, lag = 2)`, `diff(c())`) to G21. Add a line noting the restriction list lives in the "GNU R equivalence" section.
- **Anchors:** `planning/EXECUTION_PLAN_8.md` (grep `8.4.3`); `planning/tasks/g11-conformance-classification.md` (grep `8.4.3`, `New R0 gap`, `known gap`).
- **Constraints:** do not edit `docs/LANGUAGE_SPEC.md`, `tests/`, or `src/`.
- **Acceptance:** `git diff --stat` shows only the two planning files. Grep `planning/` for `8.4.3` and confirm the note is present. No test command required (planning-only), but the orchestrator runs the gate.
- **Reference:** the note style of existing `planning/tasks/7.6.1.todo.md`.

## Task 9.2.1 -- Restrictions in the "GNU R equivalence" section

- **Repo:** rzero
- **Files in scope:** must edit: `docs/LANGUAGE_SPEC.md` "GNU R equivalence" section only. Can create: none.
- **Depends on:** 9.1.2 (its audit table decides which items are restrictions).
- **Spec:** Replace the line "The restriction list is empty; see the errata." (around line 336) with a bullet list of restrictions. Each bullet names the input, GNU R's result, and R0's result, in the spec's style:
  - Bare `c()` passed to `sin`, `order`, or `tabulate` (GNU R errors; R0 result differs).
  - `atan2(NaN, NA_real_)`: GNU R gives `NA`, R0 gives `NaN`. Link to G20.
  - `approx(..., 2)`: GNU R reads the 4th positional argument as `method` and errors.
  - Integer-type differences (linked to G21): `d[c(1, 2)] <- 1:3`, `diff(1:5, lag = 2)`, `diff(c())`.
  - B2: `1.5L` and `1e3L` (GNU R warns and uses numeric; R0 rejects with `ParseError`).
  - Any `restriction` items from the 9.1.2 audit table.
- **Anchors:** `docs/LANGUAGE_SPEC.md` "GNU R equivalence" section (grep `Restrictions`); §9 B2 entry.
- **Constraints:** do not edit §9 errata text. Do not edit `tests/`, `src/`, or `allowlist.txt`.
- **Acceptance:** `grep -n "Restrictions" -A10 docs/LANGUAGE_SPEC.md` shows the list. `cargo test --test gnu_r` still passes (no code change, sanity check).
- **Reference:** the bullet style of the existing bullets in the "GNU R equivalence" section ("Scope", "Canonical comparison").

---

## Gate (orchestrator, after wave 2)

1. `cargo test` and `cargo test --test gnu_r`
2. `make test-gnu-r`
3. `cargo clippy --all-targets -- -D warnings`
4. `cargo build --target wasm32-unknown-unknown --release` (no `lib.rs` change, but the gate standard includes it)
5. Confirm `tests/r0/allowlist.txt` is still empty and `git diff --stat -- src/` is empty.
6. Update `planning/GOALS.md`: G11 closure criteria checked off. Move G11 to Completed and promote G12 to Circled.

## Escalation

- Worker needs `src/` changes → stop, report, and add a new goal (per the escalation rule: decomposition bug).
- A corpus file disagrees with GNU R → stop, report the R0 output and the Rscript output, and wait for a decision.
- Two identical test failures → stop and report (no thrash).
