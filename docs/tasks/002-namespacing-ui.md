# Task: Namespacing and Scope Tags in UI

Status: Complete

## Overview
Enrich custom prompts with scope and namespace and display them in the TUI slash popup.

- Scope sources: `project` (`.codex/prompts` under project root) and `user` (`$CODEX_HOME/prompts`)
- Namespace: subdirectory components under the respective root (e.g., `frontend/component.md` → namespace `frontend`)
- Display tags: `(project:frontend)` or `(user)` when no namespace

## Implementation Details

### Data Model (codex-protocol)
- Added `PromptScope` enum with serde lower-case variants: `project`, `user`.
- Added `CustomPromptMeta` struct:
  - `name: String`
  - `path: PathBuf`
  - `scope: PromptScope`
  - `namespace: Vec<String>` — subdirectory components under the prompts root
  - `description: Option<String>` — reserved for frontmatter parsing (future task)
  - `argument_hint: Option<String>` — reserved for frontmatter parsing (future task)

- Extended protocol event:
  - `ListCustomPromptsResponseEvent` now includes a parallel field:
    - `custom_prompts_meta: Vec<CustomPromptMeta>`
  - The existing `custom_prompts: Vec<CustomPrompt>` is retained for backward compatibility (content used to submit prompt text).

Files:
- `codex-rs/protocol/src/custom_prompts.rs`
- `codex-rs/protocol/src/protocol.rs`

### Discovery & Aggregation (codex-core)
- Existing recursive discovery now feeds a new adapter that builds metadata:
  - `discover_user_and_project_custom_prompt_meta(cwd) -> Vec<CustomPromptMeta>`
  - Computes `scope` based on root directory (`project` vs `user`).
  - Computes `namespace` from the relative directory path (split by `/`, excluding the filename).
  - Conflict policy: entries are keyed by basename; project entries insert first; user entries use `entry.or_insert(...)` so project wins on collisions.
  - Sorted by `name` for stable UI ordering.

- `Op::ListCustomPrompts` now returns both shapes:
  - `custom_prompts` (legacy content shape) and
  - `custom_prompts_meta` (new metadata shape).

Files:
- `codex-rs/core/src/custom_prompts.rs`
- `codex-rs/core/src/codex.rs`

### UI Rendering (codex-tui)
- `chatwidget.rs` consumes `custom_prompts_meta` and builds a map keyed by `name`.
- Bottom pane plumbs metadata to the chat composer and command popup.
- `CommandPopup` builds rows as before, but augments description with a dimmed tag:
  - Tag formatting helper produces:
    - `(project)` when namespace is empty
    - `(project:frontend/foo)` when namespace components are present
    - Same for `user`
  - The tag is appended to the description text (kept dim), not the main label.

- Fuzzy search behavior remains unchanged (matches only against the primary command/prompt name), so ranking is unaffected by tags.

Files:
- `codex-rs/tui/src/chatwidget.rs`
- `codex-rs/tui/src/bottom_pane/mod.rs`
- `codex-rs/tui/src/bottom_pane/chat_composer.rs`
- `codex-rs/tui/src/bottom_pane/command_popup.rs`

### Conflict Policy
- Basename collisions are resolved by preferring `project` scope over `user`.
- Built-in slash command names take precedence: any prompt with a name equal to a built-in command is filtered out from the list.

### Styling & Conventions
- TUI uses ratatui `Stylize` helpers; tag text is dimmed via existing `selection_popup_common` behavior for the description column.
- Avoids impacting fuzzy search keys; tags are appended only to the description for readability.

## Tests & Validation
- `codex-protocol` unit tests pass.
- `codex-core` unit tests run; two pre-existing suite tests unrelated to this change failed as before.
- `codex-tui` unit tests pass; no snapshot changes required for this task.

Commands run:
- `just fmt`
- `just fix -p codex-protocol`
- `just fix -p codex-core`
- `just fix -p codex-tui`
- `cargo test -p codex-protocol`
- `cargo test -p codex-core`
- `cargo test -p codex-tui`

## Usage Notes
- Prompt tags appear in the slash popup as descriptive suffixes, e.g. `/component  send saved prompt (project:frontend)`.
- Selecting a prompt still inserts its content into the composer (unchanged behavior).

## Future Work
- Populate `description` and `argument_hint` from frontmatter metadata (separate task).
- Consider deprecating the legacy `custom_prompts` shape in the protocol once downstream consumers fully migrate to `CustomPromptMeta` for discovery and show the prompt content via a separate fetch when needed.

---

References
- Protocol model: `codex-rs/protocol/src/custom_prompts.rs`
- Event wiring: `codex-rs/protocol/src/protocol.rs` (ListCustomPromptsResponse)
- UI: `codex-rs/tui/src/bottom_pane/command_popup.rs`, `chatwidget.rs`
