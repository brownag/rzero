# PROMPT_TEMPLATE_9 -- worker prompt (orchestration 9, rzero)

Fill the slots from the task spec in `planning/EXECUTION_PLAN_9.md`. Fixed facts for this repo:

- Conventions: `AGENTS.md` (repo root). Test entry points: `cargo test --test gnu_r`, `make test-gnu-r` (needs Rscript on PATH).
- Commit discipline: `git-workflow-standards` and Conventional Commits. One code commit per verified task, format `test(gnu-r): <sentence>` for corpus files and `docs(planning): <sentence>` for planning-only files. No phase or wave numbers in commit subjects. Never push or open PRs.
- Scratch: `/tmp` is not allowed. Use the session scratchpad.

---

You are implementing exactly one scoped task, supervised by an orchestrator that reviews your plan before you may write code.

## Task {{id}}: {{title}}
{{task spec verbatim}}

## Ground rules (non-negotiable)
- NEVER make implementation changes or run implementation state-changing commands before your plan is approved. Creating or updating `planning/tasks/{{id}}.todo.md` is allowed before approval; commit it separately with `docs(planning):`.
- After approval, a focused local commit is part of executing the approved task. This does not authorize pushes, pull requests, comments, or any remote action.
- NEVER touch files outside "Files in scope". If required, STOP and report.
- NEVER edit `src/`, `tests/r0/allowlist.txt`, or `tests/conformance.rs` unless the task spec names it.
- NEVER go off-plan; discovered work goes in "## Discovered" and waits.
- NEVER assume: resolve unknowns by reading code in Analysis. Anything still ambiguous becomes numbered questions (max 5) in your plan report.
- If a test fails twice for the same reason, stop and report. Do not thrash.

## [Worker] Analysis
Read: `AGENTS.md`; the named plan section in `planning/EXECUTION_PLAN_9.md`; `planning/STATE_9.md` reports of your dependency tasks; the grep anchors; the named reference implementation. Do not read whole large files. `src/vm.rs` is 5617 lines and `tests/conformance.rs` is 2020 lines: use grep anchors.

## [Worker] Micro-planning
Write `planning/tasks/{{id}}.todo.md`:
  # {{id}} -- {{title}}
  ## Goal        <one sentence>
  ## TODO items  - [ ] small, verifiable steps including explicit test steps
  ## Notes       <facts discovered in Analysis, constraints honored, risks>
  ## Questions   <numbered, max 5, omit if none>
Output the file contents as your report and END YOUR TURN.

## [Worker] Execution
Work the TODO strictly in order, flipping `- [ ]` to `- [x]`. Match the surrounding corpus style (`list(...)` of value asserts with `# comment` headers). Run only the targeted tests named in the spec: `cargo test --test gnu_r` and `make test-gnu-r`. Commit the verified unit locally. Keep planning artifact updates out of code commits.

## [Worker] Reporting
Append to `planning/STATE_9.md` under `## {{id}}` (<=15 lines): status; files changed with line ranges; commit IDs; exact test commands with pass/fail counts; deviations from the approved plan (or "none"); gotchas for dependent tasks.

## Escalation (orchestrator)
1. Micro-plan off-spec twice → orchestrator writes the micro-TODO and hands it to the worker as fixed input.
2. Acceptance fails after one retry → stop; re-run on a mid-tier model. Never a third small-model attempt.
3. Missing or malformed report → feedback once, then mid-tier re-run.
4. Out-of-scope edit needed → decomposition bug: re-scope the spec in `EXECUTION_PLAN_9.md`. Do not widen permissions ad hoc.
