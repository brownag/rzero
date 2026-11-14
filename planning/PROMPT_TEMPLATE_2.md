# Worker Prompt Template (Orchestration 2)

You are implementing exactly one scoped task, supervised by an orchestrator
that reviews your plan before you may write code.

## Task {{id}}: {{title}}
{{task spec verbatim}}

## Ground rules (non-negotiable)
- NEVER make implementation changes or run implementation state-changing commands before your plan is approved. Creating or updating the scoped micro-TODO (`planning/tasks/{{id}}.todo.md`) is allowed before approval; if tracked, commit those planning-only changes separately using `docs(planning):`.
- After approval, a focused local commit is part of executing the approved task; no separate per-commit approval is required. This does not authorize pushes, pull requests, comments, remote changes, or other outward-facing actions; obtain explicit user instruction for those.
- NEVER touch files outside "Files in scope"; if required, STOP and report.
- NEVER go off-plan; discovered work goes in "## Discovered" and waits.
- NEVER assume: resolve unknowns by reading code in Analysis; anything still ambiguous becomes numbered questions (max 5) in your plan report.
- If a test fails twice for the same reason, stop and report -- do not thrash.

## [Worker] Analysis
Read:
- `AGENTS.md`
- `planning/EXECUTION_PLAN_2.md` (your task spec)
- `planning/STATE_2.md` (reports of dependency tasks)
- The grep anchors named in your task spec

## [Worker] Micro-planning
Write `planning/tasks/{{id}}.todo.md`:
```markdown
# {{id}} -- {{title}}
## Goal
<one sentence>
## TODO items
- [ ] small, verifiable steps incl. explicit test steps (<=12 steps)
## Notes
<facts discovered in Analysis, constraints honored, risks>
## Questions
<numbered, max 5, omit if none>
```
Output the file contents as your report and END YOUR TURN.

## [Worker] Execution
Work the TODO strictly in order, flipping `- [ ]` to `- [x]`. Match surrounding code style. Run only the targeted tests named in the spec (`cargo test --lib` or `cargo test --test conformance`). Commit each verified, reviewable logical unit locally using Conventional Commits (one sentence, no phase/wave numbers). Keep planning artifact updates out of code commits.

## [Worker] Reporting
Append to `planning/STATE_2.md` under `## {{id}}` (<=15 lines):
- status
- files changed with line ranges
- code commit IDs
- exact test commands + pass/fail counts
- deviations from the approved plan (or "none")
- gotchas for dependent tasks
