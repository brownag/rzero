# Worker Prompt Template 5 (`planning/EXECUTION_PLAN_5.md`)

You are implementing exactly one scoped task from `planning/EXECUTION_PLAN_5.md` in `/home/andrew/AI/rzero`, supervised by an orchestrator that reviews your plan before you may write code.

## Task {{id}}: {{title}}
{{task spec verbatim}}

## Ground rules (non-negotiable)
- NEVER make implementation changes before your plan is approved. Writing `planning/tasks/{{id}}.todo.md` is allowed.
- After approval, focused local commits are authorized (Conventional Commits per `git-workflow-standards`: `feat(vm): ...`, `fix(vm): ...`, `test(conformance): ...`, `docs(spec): ...`; one sentence; no wave/task numbers). No pushes or remote actions.
- NEVER touch files outside "Files in scope"; if required, STOP and report.
- NEVER go off-plan; discovered work goes in "## Discovered".
- `src/vm.rs` is 5,000+ lines: use `rg -n` with the grep anchors and read only relevant ranges.
- Read the "Recon / Hazards" and "Decision Log" sections of `EXECUTION_PLAN_5.md`.
- If a test fails twice for the same reason, stop and report.

## Analysis
Read `AGENTS.md`, `planning/EXECUTION_PLAN_5.md` (Decision Log, Hazards, your task), `planning/STATE_5.md` reports of dependency tasks, and the grep anchors.

## Micro-planning
Write `planning/tasks/{{id}}.todo.md` with `## Goal`, `## TODO items` (<=12 checkbox steps incl. tests), `## Notes`, `## Questions` (max 5, omit if none). Output it as your report and END YOUR TURN.

## Execution (only after `APPROVED`)
Work the TODO in order, flipping boxes. Run only your task's acceptance commands. Commit verified work locally; keep planning files out of code commits.

## Reporting
Append to `planning/STATE_5.md` under `## {{id}}` (<=15 lines): status; files changed with line ranges; commit IDs; exact test commands + pass/fail counts; deviations (or "none"); gotchas for dependents.
