# Worker Prompt Template, Orchestration 7 (`planning/PROMPT_TEMPLATE_7.md`)

Fill the slots from the task spec in `planning/EXECUTION_PLAN_7.md`. The orchestrator fills `{{id}}`, `{{title}}`, and `{{task spec verbatim}}`.

---

You are implementing exactly one scoped task, supervised by an orchestrator
that reviews your plan before you may write code.

## Task {{id}}: {{title}}
{{task spec verbatim}}

## Repository conventions
- Repo: `/home/andrew/AI/rzero` (Rust crate `r0-wasm`, edition 2021, zero third-party dependencies).
- Project instructions: `AGENTS.md` at the repo root. Follow it.
- Targeted tests only: run the commands named in the task spec's Acceptance line. Do not run the full suite; the orchestrator runs the gate.
- Gate commands (orchestrator only): `cargo test`, `cargo clippy --all-targets -- -D warnings`, `cargo build --target wasm32-unknown-unknown --release`, `make test-gnu-r`.
- Scratch files: the scratchpad directory, never `/tmp`.
- Commit style: follow `/home/andrew/.claude/skills/git-workflow-standards/SKILL.md`. Conventional Commits, one sentence, no phase or wave numbers in the message. Use the commit subject in the task spec.
- Branch: `main`. Do not create, switch, or delete branches. No pushes, pull requests, comments, or other remote actions.

## Ground rules (non-negotiable)
- NEVER make implementation changes or run implementation state-changing commands before your plan is approved. Creating or updating the scoped micro-TODO and other planning artifacts required for review is allowed before approval; if tracked, commit those planning-only changes separately using `docs(planning):`.
- After approval, a focused local commit is part of executing the approved task; no separate per-commit approval is required. This does not authorize pushes, pull requests, comments, remote changes, or other outward-facing actions; obtain explicit user instruction for those.
- NEVER touch files outside "Files in scope"; if required, STOP and report.
- NEVER go off-plan; discovered work goes in "## Discovered" and waits.
- NEVER assume: resolve unknowns by reading code in Analysis; anything still
  ambiguous becomes numbered questions (max 5) in your plan report.
- `src/vm.rs` is over 5,500 lines. Enter it through the grep anchors only. Never read it whole.
- If a test fails twice for the same reason, stop and report -- do not thrash.

## [Worker] Analysis
Read: `AGENTS.md`; the named plan section(s) in `planning/EXECUTION_PLAN_7.md`
for your task; `planning/STATE_7.md` reports of your dependency tasks; the
grep anchors from your task spec. There is no named reference implementation;
the existing code near each anchor is the pattern to match.

## [Worker] Micro-planning
Write `planning/tasks/{{id}}.todo.md`:
  # {{id}} -- {{title}}
  ## Goal        <one sentence>
  ## TODO items  - [ ] small, verifiable steps incl. explicit test steps
  ## Notes       <facts discovered in Analysis, constraints honored, risks>
  ## Questions   <numbered, max 5, omit if none>
Output the file contents as your report and END YOUR TURN.

## [Worker] Execution
Work the TODO strictly in order, flipping `- [ ]` to `- [x]`. Match surrounding
code style. Run only the targeted tests named in the spec. Commit each
verified, reviewable logical unit locally rather than waiting for the entire
orchestration. Use the commit subject from the task spec. Keep planning
artifact updates out of code commits.

## [Worker] Reporting
Append to `planning/STATE_7.md` under `## {{id}}` (<=15 lines): status; files
changed with line ranges; code commit IDs; exact test commands + pass/fail
counts; deviations from the approved plan (or "none"); gotchas for dependent
tasks.

---

## Orchestrator notes (not sent to workers)
- Waves are serial (one task each). Spawn one Haiku worker per wave. No worktrees.
- Escalation ladder per wave-orchestration: off-spec plan twice -> orchestrator writes the micro-TODO; acceptance fails after one retry -> stop, re-run on a mid-tier model; malformed report -> one feedback retry, then mid-tier model; out-of-scope edit -> re-scope the spec.
- Task 7.6.1 is docs-only. Its acceptance is grep-based.
