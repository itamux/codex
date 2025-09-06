# Task: Project and User Prompt Discovery

## Overview
Enable discovery of custom prompts from both user and project scopes.

- User scope (existing): `~/.codex/prompts` (aka `$CODEX_HOME/prompts`)
- Project scope (new): `./.codex/prompts` at the repository/project root
- Recurse subdirectories to enumerate prompts (namespacing handled in a separate task)
- Aggregate into a single list returned to clients

## Goals
- Scan both roots, tolerate missing directories, and return a unified list
- Prefer project-level prompts over user-level when basenames collide
- Preserve subdirectory structure for the Namespacing task to consume later

## Exact Files To Modify/Add

- Modify: `codex-rs/core/src/custom_prompts.rs`
  - Add recursive directory walk for prompts under a given root
  - Add a new function to aggregate prompts across user + project roots
  - Keep existing `default_prompts_dir()` behavior for user scope
- Modify: `codex-rs/core/src/codex.rs`
  - In `submission_loop`, under `Op::ListCustomPrompts`, switch to the new aggregator that merges user + project prompts
  - Determine project root using existing helpers and `config.cwd` (see `codex-rs/core/src/git_info.rs`)
- No immediate protocol changes in this task (see Namespacing task for adding scope/namespace metadata)

## Technical Plan

1) Discovery helpers (core/src/custom_prompts.rs)
- Add `fn project_prompts_dir(project_root: &Path) -> PathBuf { project_root.join(".codex/prompts") }`
- Add `async fn discover_prompts_recursive(dir: &Path) -> Vec<DiscoveredFile>` where `DiscoveredFile` captures:
  - `path: PathBuf`
  - `name: String` (file stem)
  - `rel_dir: PathBuf` (relative directory under root, used later for namespacing)
- Reuse existing `.md` filtering + UTF‑8 loading; ignore non-files; skip unreadable

2) Aggregation
- Add `async fn discover_user_and_project_prompts(cwd: &Path) -> Vec<DiscoveredFile>`
  - Resolve `$CODEX_HOME/prompts` via `default_prompts_dir()` → user set
  - Resolve project root (see `get_git_repo_root(cwd)` in `git_info.rs`); if found, compute `./.codex/prompts`
  - Recurse both, then deduplicate by basename with project precedence

3) Wire-up (core/src/codex.rs)
- In `Op::ListCustomPrompts` handling, call `discover_user_and_project_prompts(&turn_context.cwd)`
- Convert `DiscoveredFile` to the current protocol shape (for now): `CustomPrompt { name, path, content }`
  - Note: `rel_dir` will be preserved in-memory for the Namespacing task; for this task, it is unused in the response

## Acceptance Criteria
- If only user prompts exist, they are listed as today
- If `./.codex/prompts` exists, its prompts are included
- When both scopes contain the same basename (`foo.md`), only the project version appears
- Works with nested directories (to be surfaced in UI in the Namespacing task)

## Tests
- Unit tests in `core/src/custom_prompts.rs` using `tempdir`:
  - Missing directories → empty
  - User only, Project only, Both (with and without collisions)
  - Nested subdirectories

## Risks / Notes
- This task intentionally does not change protocol wire shape; Namespacing metadata will be added next
- Deduplication strategy is by basename; later tasks will formalize scope and namespace presentation

---

References
- User prompts root resolution: `codex-rs/core/src/custom_prompts.rs`
- Git root detection: `codex-rs/core/src/git_info.rs`
- Protocol event wiring: `codex-rs/core/src/codex.rs` (Op::ListCustomPrompts)

---

## Implementation Details (Complete)

Status: Complete

### Summary of Changes
- Added recursive discovery of Markdown prompts under both user and project scopes with project-over-user precedence on basename collisions.
- Wired `Op::ListCustomPrompts` to aggregate across scopes using the session `cwd` to determine project context.
- Preserved subdirectory structure internally for future namespacing while keeping the outward protocol unchanged.

### Core Code Changes
- File: `codex-rs/core/src/custom_prompts.rs`
  - New helper: `pub fn project_prompts_dir(project_root: &Path) -> PathBuf`
    - Computes the project prompt root: `project_root/.codex/prompts`.
  - New struct: `DiscoveredFile { path: PathBuf, name: String, rel_dir: PathBuf, content: String }`
    - `rel_dir` is the directory relative to the scanned root; used later for namespacing.
  - New function: `pub async fn discover_prompts_recursive(root: &Path) -> Vec<DiscoveredFile>`
    - Iterative DFS using a stack (non-recursive to avoid `async fn` recursion boxing).
    - Filters `.md` files only, UTF-8 content only; skips unreadable entries.
  - New function: `pub async fn discover_user_and_project_prompts(cwd: &Path) -> Vec<DiscoveredFile>`
    - Determines project root via `git_info::get_git_repo_root(cwd)`; falls back to `cwd` if not inside a Git repo.
    - Scans `project_root/.codex/prompts` and the user prompts (`default_prompts_dir()` → `$CODEX_HOME/prompts`).
    - Deduplicates by basename with project precedence; sorts by `name`.
  - New adapter: `pub async fn discover_user_and_project_custom_prompts(cwd: &Path) -> Vec<CustomPrompt>`
    - Converts internal `DiscoveredFile` items into the existing protocol struct `CustomPrompt`.
  - Backwards compatibility: existing `discover_prompts_in(_/excluding)` kept intact.

- File: `codex-rs/core/src/codex.rs`
  - In `submission_loop`, branch `Op::ListCustomPrompts` now calls:
    - `custom_prompts::discover_user_and_project_custom_prompts(&turn_context.cwd).await`
  - Uses the turn context’s `cwd` to resolve the project root; no protocol changes required.

### Behavior Notes
- Scope resolution
  - Project: `<git-root>/.codex/prompts`; when not inside a Git repository, falls back to `<cwd>/.codex/prompts`.
  - User: `$CODEX_HOME/prompts` (aka `~/.codex/prompts` by default).
- Deduplication
  - Keyed by basename (`file_stem`). When a collision occurs, the project-scope entry overrides the user-scope entry.
  - Final list is sorted lexicographically by `name`.
- Filtering & robustness
  - Includes only `.md` files; ignores non-files and unreadable entries; includes UTF‑8 only.
  - Preserves subdirectories via `rel_dir` internally for a follow-up namespacing task.

### Tests Added (Unit)
- Location: `codex-rs/core/src/custom_prompts.rs`
  - `recursive_discovery_with_subdirs` – verifies recursive enumeration and `rel_dir` capture.
  - `aggregate_user_only` – returns user prompts only, sorted.
  - `aggregate_project_only` – returns project prompts only, sorted.
  - `aggregate_both_with_collision_project_wins` – dedup works with project precedence; non-colliding entries retained.
  - `fallback_to_cwd_when_no_git_repo` – validates cwd fallback when no Git repository is present.

Run:
- `cargo test -p codex-core`

### Manual Verification (TUI)
1) Create prompts:
   - Project: `<repo-root>/.codex/prompts/hello.md`
   - User: `$CODEX_HOME/prompts/hello.md` (for collision precedence testing)
2) Run TUI with project root:
   - `cd codex-rs && just tui -C <repo-root>`
3) Open slash menu by typing `/`.
   - Prompts appear after built-in commands, sorted by name.
   - On basename collision, the project prompt is selected.

### Known Limitations
- Protocol is intentionally unchanged; namespacing metadata adoption is deferred to the follow-up task.
- Prompt list is loaded at session start; changes on disk require a new session to refresh in the UI.

### Rationale
- Falling back to `cwd` when not in a Git repo provides a smoother developer experience for ad-hoc directories without changing behavior for normal Git projects.
