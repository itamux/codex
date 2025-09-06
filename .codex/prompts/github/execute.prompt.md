# Execute the next implementation phase from a tasks file


Execute the next phase from a tasks.md file, implementing its tasks and updating the file.

This is the fourth step in the Spec-Driven Development lifecycle.

Given the absolute path to a tasks file provided as an argument, do this:

1. Resolve the tasks file path
   - Let `TASKS_FILE` be the path from the arguments. If it is not absolute, resolve it relative to the repository root.
   - Read the entire file. Parse Phases by the `## Phase X.Y:` headings and tasks by Markdown checkboxes `- [ ]` / `- [x]` with task codes like `T001`.

2. Select the next unimplemented Phase
   - Find the first Phase that is not fully complete (i.e., contains any unchecked task boxes).
   - Within that Phase, respect its intended order. If the Phase describes TDD (tests first), ensure tests are written and fail before implementing.

3. Execute the Phase tasks
   - For each task in Phase order:
     - Read referenced files and directories. Use absolute paths. Create files that do not exist when specified.
     - Follow any commands listed with the task as validation steps (e.g., `cargo test -p ...`).
     - Maintain repository conventions (see `/home/iatzmon/workspace/codex/AGENTS.md`). For Rust changes under `codex-rs`:
       - Run `just fmt` after making code changes.
       - Run `just fix -p <project>` for the changed crate to address lints.
       - Run project-specific tests mentioned in the task (e.g., `cargo test -p codex-core`). If tasks modify common, core, or protocol crates, run the full suite with `cargo test --all-features` after project tests pass.
       - Prefer scoping to the changed project to keep runs fast unless the task explicitly requires workspace-wide verification.
     - For TUI snapshot tasks: follow the pending/accept flow with `cargo insta` as described in the spec when appropriate.
     - If unrelated tests fail due to environment constraints, filter to the relevant package or module tests and document the exception in the implementation notes.

4. Update the tasks file for the Phase
   - Mark each completed task in the Phase by changing `- [ ]` to `- [x]`.
   - Immediately after the Phase block, add a short "Implementation details" note summarizing:
     - Key decisions or additions (dependencies, helpers, doc updates).
     - Any environment/test caveats (e.g., filtered tests), and how to run validations locally.
     - Anything required by subsequent Phases so the next steps have the necessary context.

5. Verify and report
   - Re-run the Phase's validation commands to ensure success.
   - Summarize results: which Phase was executed, tasks completed, important changed files, and any follow-ups for the next Phase.

Use absolute paths with the repository root for all file operations to avoid path issues. Keep changes additive and backward compatible unless the tasks specify otherwise.

