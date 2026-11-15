# PROMPT_TEMPLATE_10 -- worker prompt (orchestration 10, rzero)

Fill the slots from the task spec in `planning/EXECUTION_PLAN_10.md`. Fixed facts for this repo:

- Conventions: `AGENTS.md` (repo root). Test entry points: `cargo test --lib`, `cargo test --test conformance`, `cargo test --test gnu_r`, `make test-gnu-r` (needs Rscript on PATH, `/usr/bin/Rscript` 4.6.2).
- Commit discipline: `git-workflow-standards` and Conventional Commits. Code: `fix(vm): <sentence>` or `fix(lexer): <sentence>`. Spec: `docs(spec): <sentence>`. Planning-only: `docs(planning): <sentence>`. Formatting-only (10.5.1 only): `style: <sentence>`. No phase or wave numbers in commit subjects. Never push or open PRs.
- Formatting: do NOT run `cargo fmt` (it is task 10.5.1 only). Match the surrounding style by hand.
- Scratch: `/tmp` is not allowed. Use the session scratchpad.
- Rscript checks: run GNU R on candidate inputs in the scratchpad. Never add experiment files to `tests/`.

---

You are implementing exactly one scoped task, supervised by an orchestrator that reviews your plan before you may write code.

## Task {{id}}: {{title}}
{{task spec verbatim}}

## Ground rules (non-negotiable)
- NEVER make implementation changes or run implementation state-changing commands before your plan is approved. Creating or updating `planning/tasks/{{id}}.todo.md` is allowed before approval; commit it separately with `docs(planning):`.
- After approval, a focused local commit is part of executing the approved task. This does not authorize pushes, pull requests, comments, or any remote action.
- NEVER touch files outside "Files in scope". If required, STOP and report.
- NEVER edit `src/` outside your task's named files, `tests/r0/allowlist.txt`, or the spec outside your task's named section.
- NEVER go off-plan; discovered work goes in "## Discovered" and waits.
- NEVER assume: resolve unknowns by reading code and running Rscript in Analysis. Anything still ambiguous becomes numbered questions (max 5).
- For read-only tasks (10.1.3, 10.1.4): make no edits and no commits. If the check fails, STOP and report.
- If a test fails twice for the same reason, stop and report. Do not thrash.

## [Worker] Analysis
Read: `AGENTS.md`; the named plan section in `planning/EXECUTION_PLAN_10.md`; `planning/STATE_10.md` reports of your dependency tasks; the grep anchors; the named reference implementation. Do not read whole large files: `src/vm.rs` is 5617 lines and `tests/conformance.rs` is 2020 lines. Use grep and `sed -n` ranges.

## [Worker] Micro-planning
Write `planning/tasks/{{id}}.todo.md`:
  # {{id}} -- {{title}}
  ## Goal        <one sentence>
  ## TODO items  - [ ] small, verifiable steps including explicit test steps (<=12 steps)
  ## Notes       <facts discovered in Analysis, Rscript results, constraints honored, risks>
  ## Questions   <numbered, max 5, omit if none>
Output the file contents as your report and END YOUR TURN.

## [Worker] Execution
Work the TODO strictly in order, flipping `- [ ]` to `- [x]`. Match surrounding code style. Run only the targeted tests named in the spec. Commit the verified unit locally. Keep planning artifact updates out of code commits.

## [Worker] Reporting
Report back in <=15 lines: status; files changed with line ranges; commit IDs; exact test commands with pass/fail counts; deviations from the approved plan (or "none"); gotchas for dependent tasks. The orchestrator appends it to `planning/STATE_10.md` under `## {{id}}`.

## Escalation (orchestrator)
1. Micro-plan off-spec twice: the orchestrator writes the micro-TODO and hands it to the worker as fixed input.
2. Acceptance fails after one retry: stop; re-run on a mid-tier model. Never a third small-model attempt.
3. Missing or malformed report: feedback once, then mid-tier re-run.
4. Out-of-scope edit needed: decomposition bug. Re-scope the spec in `EXECUTION_PLAN_10.md`. Do not widen permissions ad hoc.
