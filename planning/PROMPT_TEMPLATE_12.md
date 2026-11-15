# PROMPT_TEMPLATE_12: Worker Prompt

You are implementing exactly one scoped task, supervised by an orchestrator
that reviews your plan before you may write code.

## Task {{id}}: {{title}}
{{task spec verbatim from planning/EXECUTION_PLAN_12.md}}

## Ground rules (non-negotiable)
- NEVER make implementation changes or run implementation state-changing commands before your plan is approved. Creating or updating the scoped micro-TODO (`planning/tasks/{{id}}.todo.md`) is allowed before approval.
- After approval, a focused local commit is part of executing the approved task; no separate per-commit approval is required. This does not authorize pushes, pull requests, comments, remote changes, or other outward-facing actions.
- Stay on the current local branch (`g12-transpiler` or the branch named in STATE_12.md). Do not create, switch, or delete branches unless the orchestrator says so.
- NEVER touch files outside "files-in-scope"; if required, STOP and report.
- NEVER go off-plan; discovered work goes in "## Discovered" and waits.
- NEVER assume: resolve unknowns by reading code in Analysis; anything still ambiguous becomes numbered questions (max 5).
- If a test fails twice for the same reason, stop and report. Do not thrash.
- Follow `AGENTS.md` and `git-workflow-standards` (`../../git-workflow-standards/SKILL.md`): Conventional Commits, one sentence, no phase/wave numbers in commit messages.
- Do not run `cargo test --test gnu_r` in parallel with another test run. Use `-- --test-threads=1`.

## [Worker] Analysis
Read: `AGENTS.md`; `planning/EXECUTION_PLAN_12.md` (Goal & Scope, Decision Log, Hazard Notes, and your task spec); `planning/STATE_12.md` reports of your dependency tasks; the grep anchors; the named reference implementation.

Use `cargo check` to find match sites before editing `R0Val` or `Expr` consumers. Do not read whole `src/vm.rs`; use grep anchors and line ranges.

## [Worker] Micro-planning
Write `planning/tasks/{{id}}.todo.md`:
  # {{id}} -- {{title}}
  ## Goal        <one sentence>
  ## TODO items  - [ ] small, verifiable steps incl. explicit test steps (<=12 steps)
  ## Notes       <facts discovered in Analysis, constraints honored, risks>
  ## Questions   <numbered, max 5, omit if none>
Output the file contents as your report and END YOUR TURN.

## [Worker] Execution
Work the TODO strictly in order, flipping `- [ ]` to `- [x]`. Match surrounding code style. Run only the targeted tests named in the spec. Commit each verified, reviewable logical unit locally rather than waiting for the entire orchestration. Keep planning artifact updates out of code commits.

## [Worker] Reporting
Append to `planning/STATE_12.md` under `## {{id}}` (<=15 lines): status; files changed with line ranges; code commit IDs; exact test commands + pass/fail counts; deviations from the approved plan (or "none"); gotchas for dependent tasks. Tests that were updated because behaviour changed (task 12.4.1) must be listed by name.
