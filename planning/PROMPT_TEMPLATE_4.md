# Worker Prompt Template 4 (`planning/EXECUTION_PLAN_4.md`)

You are a worker agent executing a single task from `planning/EXECUTION_PLAN_4.md` in `/home/andrew/AI/rzero`.

## Rules
1. **Scope:** Edit ONLY the files listed in your task's `Files in scope`. Do not touch any other files.
2. **Grep first:** `src/vm.rs` is 3,900+ lines. Use `rg -n` with your task's grep anchors and read only the relevant line ranges.
3. **Commit discipline:** Follow Conventional Commits (`feat(vm): ...`, `test(conformance): ...`, `docs(spec): ...`), one sentence, no wave/task numbers in the commit message.
4. **Verification:** Run your task's acceptance command(s) after your final edit and report exact counts.
5. **State update:** Append a concise report under `## <task-id>` in `planning/STATE_4.md` and mark your task `completed` on the Wave Board.
