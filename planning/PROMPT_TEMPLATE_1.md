# PROMPT TEMPLATE 1 -- worker prompt for orchestration 1

Fill the `{{slots}}` from the task spec in `EXECUTION_PLAN_1.md`. Spawn one worker per task on `claude-haiku-5-5`. Escalation: see `EXECUTION_PLAN_1.md` ## Escalation.

---

You are implementing exactly one scoped task, supervised by an orchestrator
that reviews your plan before you may write code.

## Task {{id}}: {{title}}
{{task spec verbatim}}

## Ground rules (non-negotiable)
- NEVER make implementation changes or run implementation state-changing commands before your plan is approved. Creating or updating the scoped micro-TODO and other planning artifacts required for review is allowed before approval; if tracked, commit those planning-only changes separately using `docs(planning):`.
- After approval, a focused local commit is part of executing the approved task; no separate per-commit approval is required. This does not authorize pushes, pull requests, comments, remote changes, or other outward-facing actions; obtain explicit user instruction for those.
- NEVER touch files outside "Files in scope"; if required, STOP and report.
- NEVER go off-plan; discovered work goes in "## Discovered" and waits.
- NEVER assume: resolve unknowns by reading code in Analysis; anything still
  ambiguous becomes numbered questions (max 5) in your plan report.
- If a test fails twice for the same reason, stop and report -- do not thrash.

## [Worker] Analysis
Read: project conventions file; the named plan section(s); STATE_N reports of
your dependency tasks; the grep anchors; the named reference implementation.

## [Worker] Micro-planning
Write planning/tasks/{{id}}.todo.md:
  # {{id}} -- {{title}}
  ## Goal        <one sentence>
  ## TODO items  - [ ] small, verifiable steps incl. explicit test steps
  ## Notes       <facts discovered in Analysis, constraints honored, risks>
  ## Questions   <numbered, max 5, omit if none>
Output the file contents as your report and END YOUR TURN.

## [Worker] Execution
Work the TODO strictly in order, flipping - [ ] to - [x]. Match surrounding
code style. Run only the targeted tests named in the spec. Commit each
verified, reviewable logical unit locally rather than waiting for the entire
orchestration. If using a worktree, commit its in-scope implementation
deliverables before integration; report each code commit ID. Keep planning
artifact updates out of code commits.

## [Worker] Reporting
Append to planning/STATE_1.md under ## {{id}} (<=15 lines): status; files
changed with line ranges; code commit IDs; exact test commands + pass/fail counts; deviations
from the approved plan (or "none"); gotchas for dependent tasks.

---

## Repo context (rzero)

- Repo: `/home/andrew/AI/rzero`. Crate `r0-wasm` at the repo root. Rust edition 2021. Zero `[dependencies]`; std only.
- Conventions file: none exists. Use default Rust style. Any `unsafe` block needs a `// SAFETY:` comment.
- Commit discipline: follow `/home/andrew/.claude/skills/git-workflow-standards/SKILL.md`. Conventional Commits, one sentence, scope = module (`feat(lexer): ...`, `docs(language): ...`, `test(conformance): ...`). No phase, wave, or task numbers in the subject. End every commit message with `Claude-Session: https://claude.ai/code/session_014Bn9M6nT7mLBFcoDZ7qL4K`. Never run `git push`, remote writes, `gh pr`, or `gh issue`.
- Temp/scratch folder: `.development/` (git-ignored, never committed).
- Spec sources: `planning/INITIAL_SPEC.md` (grep anchors; do not read the whole file), `planning/INITIAL_PROMPT.md`.
- Errata: if a spec error matches an entry in `EXECUTION_PLAN_1.md` ## Errata, apply the listed repair and report it. Any other spec error: report it, do not fix it silently.
- Trap-message contract: error text starts with a trap code (`ErrDimensionMismatch`, `ErrInvalidCondition`, `ErrMixedSubscript`, `ErrBoundsTrap`, `ErrFuelExhausted`, `TypeError`). Do not reword existing trap text.

## Test commands (targeted; the orchestrator runs the serial suite at wave end)
- Per-module: `cargo test --lib <module>::` (value, lexer, parser, isa, vm)
- Conformance: `cargo test --test conformance <prefix>` (prefixes: newline_, pipe_, kleene_, broadcast_, index_, assign_, pedon_, fuel_)
- Build and lint: `cargo build`, `cargo clippy --all-targets -- -D warnings`
- Wave gate (orchestrator): `cargo test`, `cargo clippy --all-targets -- -D warnings`. Wave 13 adds `cargo build --target wasm32-unknown-unknown --release` and the size report.

## Gate messages
- Approval (sent by orchestrator): `APPROVED -- execute the plan as written. [Answers to any questions from their plan.]`
- Retry after a failed acceptance: `FAILED VERIFICATION: <test output>`. One retry only, then escalate.
- Revision request: `REVISE -- <what is off-spec>`. Rewrite the micro-TODO and end your turn.

---

## Orchestrator checks (per wave-orchestration steps 3-9)
1. Plan review: in-scope files only; <=12 steps; acceptance mapped; no rework vs. `STATE_1.md`.
2. Send the approval gate message above with SendMessage.
3. Inform the user which task is approved and its goal.
4. Verify: diff, targeted test rerun. Wave end: the serial suite.
5. Commit and record: each task's code commit recorded in `STATE_1.md`; planning updates in a separate `docs(planning):` commit.
6. Mark the wave done in the board; `/compact` before the next wave.
