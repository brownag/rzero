# Worker Prompt Template, Orchestration 8 (`planning/PROMPT_TEMPLATE_8.md`)

Fill the slots from the task row and shared rules in `planning/EXECUTION_PLAN_8.md`. The orchestrator fills `{{id}}`, `{{title}}`, `{{corpus_file}}`, `{{source_fn}}`, `{{anchor}}`, and `{{exclusions}}`.

---

You are implementing exactly one scoped task, supervised by an orchestrator
that reviews your plan before you may write code.

## Task {{id}}: {{title}}
- **Corpus file to create:** `{{corpus_file}}` (the only file you may create or edit)
- **Source test function:** `{{source_fn}}` in `tests/conformance.rs`, anchor line {{anchor}}
- **Exclusions (leave these asserts in Rust, do not copy them to the corpus):** {{exclusions}}
- **Acceptance:** `cargo test --test gnu_r r0_corpus_matches_gnu_r` exits 0; `cargo test --test conformance {{source_fn}}` exits 0.
- **Commit:** `test(gnu-r): add {{group}} {{topic}} corpus program`

Shared rules are in `planning/EXECUTION_PLAN_8.md` "Task Specs" > "Shared rules". Read them. They define the list-final-value form and the error-assert rule.

## Repository conventions
- Repo: `/home/andrew/AI/rzero` (Rust crate `r0-wasm`, edition 2021, zero third-party dependencies).
- Project instructions: `AGENTS.md` at the repo root. Follow it.
- Corpus harness: `tests/gnu_r.rs`. It runs every `tests/r0/*.R0` file against `Rscript` and compares the last value. Do not edit it.
- Targeted tests only: the two commands in Acceptance. Do not run the full suite; the orchestrator runs the gate.
- Scratch files: the scratchpad directory, never `/tmp`.
- Commit style: `git-workflow-standards` (`/home/andrew/.claude/skills/git-workflow-standards/SKILL.md`). Conventional Commits, one sentence, no phase or wave numbers. Use the commit subject above.
- Branch: `main`. Do not create, switch, or delete branches. No pushes, pull requests, comments, or other remote actions.
- R0 has no `NULL` literal. Use `c()` for the Null value.

## Ground rules (non-negotiable)
- NEVER create or edit a file before your plan is approved. Creating `planning/tasks/{{id}}.todo.md` is allowed before approval. Do not commit it yet.
- After approval, the corpus file and its commit are part of the approved task. Keep planning files out of the code commit.
- NEVER touch files outside the corpus file named above. Do not edit `tests/conformance.rs`, `tests/gnu_r.rs`, `tests/r0/allowlist.txt`, or any `src/` file. If the task seems to need one of them, STOP and report.
- NEVER go off-plan; discovered work goes in `## Discovered` and waits.
- NEVER assume: resolve unknowns by reading the source function; anything ambiguous becomes numbered questions (max 5) in your plan report.
- If the corpus file fails the harness twice for the same reason, stop and report. Do not thrash.
- Read only the source function's lines (by its anchor). Never read the whole of `tests/conformance.rs`.

## [Worker] Analysis
Read `AGENTS.md`; the shared rules and the task row in `planning/EXECUTION_PLAN_8.md`; `planning/STATE_8.md` reports of earlier tasks (gotchas); the source function's lines only (`sed -n` over the anchor range). For task {{id}}, list every value assert you will migrate and every error assert you will leave in Rust.

## [Worker] Micro-planning
Write `planning/tasks/{{id}}.todo.md`:
  # {{id}} -- {{title}}
  ## Goal        <one sentence>
  ## TODO items  - [ ] small, verifiable steps incl. the two acceptance commands
  ## Notes       <value asserts migrated (count), error asserts kept in Rust (count), exclusions applied, risks>
  ## Questions   <numbered, max 5, omit if none>
Output the file contents as your report and END YOUR TURN.

## [Worker] Execution (only after the orchestrator sends APPROVED)
Work the TODO strictly in order, flipping `- [ ]` to `- [x]`. Create `{{corpus_file}}`. Run the two acceptance commands. If the harness reports a mismatch, compare the R0 value against `Rscript` on the same expression; report the difference instead of changing the expression to hide it. Commit the corpus file alone with the commit subject above. Keep planning artifact updates out of that commit.

## [Worker] Reporting
Append to `planning/STATE_8.md` under `## {{id}}` (<=15 lines): status; file created; code commit ID; the two acceptance commands with pass/fail; value asserts migrated (count) and error asserts kept in Rust (count); deviations from the approved plan (or "none"); gotchas for dependent tasks. Commit that update separately as `docs(planning): record task {{id}} completion`.

---

## Orchestrator notes (not sent to workers)
- Waves are serial, and tasks inside a wave run one at a time (D-8.5). No worktrees.
- Task 8.1.1 is the pilot for H2. If the list-final shape does not match GNU R, STOP the orchestration and report before 8.1.2.
- Escalation ladder per wave-orchestration: off-spec plan twice -> orchestrator writes the micro-TODO; acceptance fails after one retry -> stop, re-run on a mid-tier model; malformed report -> one feedback retry, then mid-tier model; out-of-scope edit -> re-scope the spec in `EXECUTION_PLAN_8.md`.
- Wave gate after each wave: `cargo test`, `cargo clippy --all-targets -- -D warnings`, wasm release build, `make test-gnu-r`, all serial.
